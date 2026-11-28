package vitapmate.data

import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.DocumentsContract
import android.provider.MediaStore
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.edit
import java.io.File
import java.io.InputStream
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.decodeSavedFiles
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * The app's one download folder, where saved files (VTOP downloads, course page files, outing
 * passes) land for the student: a folder they chose (Storage Access Framework tree, persistable
 * permission) or, by default, Downloads/vitapmate. The first save asks which ([prompt], drawn by
 * the design module's `DownloadFolderPrompt`). Rust owns the saved-files bookkeeping (`docs/downloads.json`),
 * so changing the folder moves those files (and only those), and [onMoved] tells Docs (a view over
 * this folder) where each went. Log src `downloads`: counts only.
 */
object Downloads {
    const val DEFAULT_LABEL = "Downloads/vitapmate"
    const val SUBDIR = "vitapmate"
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val lock = Mutex()
    private var app: Context? = null

    /** The app, once [init] ran; every use is after it. */
    private val ctx: Context get() = checkNotNull(app) { "Downloads.init not called" }
    private val prefs get() = ctx.getSharedPreferences("downloads", Context.MODE_PRIVATE)
    @Volatile private var imported = false

    /** The chosen folder (a tree URI), or null for Downloads/vitapmate. Compose state; [folder] and [prompt] change under `this`. */
    var folder by mutableStateOf<Uri?>(null)
        private set

    /** Told the old → new URI of each file a folder change moved (Docs follows them). */
    var onMoved: ((Map<String, String>) -> Unit)? = null

    /** Set while a save waits for the student to pick a folder the first time; complete it to go on. */
    var prompt by mutableStateOf<CompletableDeferred<Unit>?>(null)
        private set

    @Synchronized
    fun init(context: Context) {
        if (app != null) return
        app = context.applicationContext
        folder = prefs.getString("tree", null)?.let(Uri::parse)
    }

    /**
     * Waits, the first time, for the student to pick where files go; then the chosen folder, or
     * null for Downloads/vitapmate. Saves at once share one prompt.
     */
    suspend fun target(): Uri? {
        val ask = synchronized(this) {
            if (prefs.getBoolean("asked", false)) null else prompt ?: CompletableDeferred<Unit>().also { prompt = it }
        }
        ask?.await()
        return folder
    }

    /**
     * A file DownloadManager wrote to Downloads/vitapmate itself, as [name]: remembered as saved
     * in Rust, and where it is (its MediaStore entry, else [fallback]).
     */
    suspend fun adopt(name: String, mime: String, fallback: Uri?): Uri = lock.withLock {
        fun find(): Uri? = runCatching {
            val where = "${MediaStore.MediaColumns.RELATIVE_PATH} = ? AND ${MediaStore.MediaColumns.DISPLAY_NAME} = ?"
            ctx.contentResolver.query(MediaStore.Downloads.EXTERNAL_CONTENT_URI, arrayOf(MediaStore.MediaColumns._ID), where, arrayOf("${Environment.DIRECTORY_DOWNLOADS}/$SUBDIR/", name), null)
                ?.use { c -> if (c.moveToFirst()) android.content.ContentUris.withAppendedId(MediaStore.Downloads.EXTERNAL_CONTENT_URI, c.getLong(0)) else null }
        }.getOrNull()
        // DownloadManager publishes the entry a moment after it reports the download done.
        var found: Uri? = null
        var tries = 0
        while (found == null && tries++ < 10) {
            found = withContext(Dispatchers.IO) { find() }
            if (found == null) delay(300)
        }
        val uri = found ?: fallback ?: error("can't find the file")
        val count = record(uri, name, mime, "downloads.adopt")
        Log.info("downloads", "adopted", null, mapOf("media_store" to "${found != null}", "count" to "$count"))
        uri
    }

    /** Where files go, for Settings: the chosen folder's name or [DEFAULT_LABEL]. */
    fun label(): String = folder?.let { DocumentsContract.getTreeDocumentId(it).substringAfterLast(':').ifEmpty { "Chosen folder" } } ?: DEFAULT_LABEL

    /** Saves [bytes] as [name] in the folder, asking for one the first time; returns where. */
    suspend fun save(name: String, mime: String, bytes: ByteArray, flow: String? = null): Uri = save(name, mime, bytes.size.toLong(), flow) { bytes.inputStream() }

    /** Streams what [open] reads ([size] bytes) to [name] in the folder, as [save]; never whole in memory. */
    suspend fun save(name: String, mime: String, size: Long, flow: String? = null, open: () -> InputStream): Uri {
        target()
        return lock.withLock {
            val uri = withContext(Dispatchers.IO) { write(folder, name, mime, open) }
            val count = record(uri, name, mime, flow ?: "downloads.save")
            Log.info("downloads", "saved", flow, mapOf("bytes" to "$size", "chosen" to "${folder != null}", "count" to "$count"))
            uri
        }
    }

    /** Saves without asking for a folder (the chosen one, else Downloads/vitapmate): Docs' move out of app storage. */
    suspend fun saveQuietly(name: String, mime: String, bytes: ByteArray): Uri = lock.withLock {
        withContext(Dispatchers.IO) { write(folder, name, mime) { bytes.inputStream() } }.also { record(it, name, mime, "downloads.save") }
    }

    /** Deletes a file this app saved, and forgets it in Rust. */
    suspend fun delete(uri: Uri) = lock.withLock {
        val context = app ?: return@withLock
        withContext(Dispatchers.IO) { delete(context, uri) }
        val core = coreReady("downloads.delete")
        withContext(Dispatchers.IO) { core.downloadsForget("downloads.delete", uri.toString()) }
    }

    /** Whether [uri] still opens (the student may have deleted the file in their file manager). */
    fun exists(uri: Uri): Boolean = runCatching { ctx.contentResolver.openFileDescriptor(uri, "r")?.use { true } ?: false }.getOrDefault(false)

    /** A file in the folder: where it is, its name and size. */
    class Entry(val uri: String, val name: String, val size: Long)

    /**
     * Every file in the folder now, or null when it can't be read (permission gone). In the
     * default folder Android shows the app only the files it saved itself. Disk/binder: off the
     * main thread.
     */
    fun list(): List<Entry>? = runCatching {
        val resolver = ctx.contentResolver
        val tree = folder
        if (tree != null) {
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
            val columns = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME, DocumentsContract.Document.COLUMN_SIZE, DocumentsContract.Document.COLUMN_MIME_TYPE)
            resolver.query(children, columns, null, null, null)?.use { c ->
                buildList {
                    while (c.moveToNext()) {
                        if (c.getString(3) == DocumentsContract.Document.MIME_TYPE_DIR) continue
                        add(Entry(DocumentsContract.buildDocumentUriUsingTree(tree, c.getString(0)).toString(), c.getString(1), c.getLong(2)))
                    }
                }
            }
        } else {
            val columns = arrayOf(MediaStore.MediaColumns._ID, MediaStore.MediaColumns.DISPLAY_NAME, MediaStore.MediaColumns.SIZE)
            val where = "${MediaStore.MediaColumns.RELATIVE_PATH} = ?"
            resolver.query(MediaStore.Downloads.EXTERNAL_CONTENT_URI, columns, where, arrayOf("${Environment.DIRECTORY_DOWNLOADS}/$SUBDIR/"), null)?.use { c ->
                buildList {
                    while (c.moveToNext()) add(Entry(android.content.ContentUris.withAppendedId(MediaStore.Downloads.EXTERNAL_CONTENT_URI, c.getLong(0)).toString(), c.getString(1), c.getLong(2)))
                }
            }
        }
    }.getOrNull()

    /** What the app saved, by file name: its URI (the newest when a name repeats). */
    suspend fun savedByName(): Map<String, String> = lock.withLock {
        val core = coreReady("downloads.saved_by_name")
        withContext(Dispatchers.IO) { core.downloadsSavedByName("downloads.saved_by_name") }
    }

    /**
     * The student chose [tree] (null: Downloads/vitapmate), from the prompt or Settings. Takes the
     * permission, then moves what the app saved in the background; [onFailed] gets how many files
     * could not be moved. Files whose old place is gone are just forgotten.
     */
    fun choose(tree: Uri?, onFailed: (Int) -> Unit = {}) {
        val context = app ?: return
        val old = folder
        tree?.let { context.contentResolver.takePersistableUriPermission(it, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
        val waiting = synchronized(this) {
            folder = tree
            prefs.edit().putString("tree", tree?.toString()).putBoolean("asked", true).apply()
            prompt.also { prompt = null }
        }
        waiting?.complete(Unit)
        if (old == tree) return
        scope.launch {
            flow("downloads.move") { f ->
                val failed = lock.withLock { move(context, tree, f) }
                if (old != null) runCatching { context.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
                if (failed > 0) onFailed(failed)
            }
        }
    }

    private suspend fun move(context: Context, to: Uri?, flow: String): Int {
        val core = coreReady(flow)
        val items = withContext(Dispatchers.IO) { core.downloadsSavedList(flow) }
        val resolver = context.contentResolver
        val moves = mutableMapOf<String, String>()
        var moved = 0
        var gone = 0
        var failed = 0
        for (item in items) {
            val from = Uri.parse(item.uri)
            val bytes = withContext(Dispatchers.IO) { runCatching { resolver.openInputStream(from)?.use { it.readBytes() } }.getOrNull() }
            if (bytes == null) {
                withContext(Dispatchers.IO) { core.downloadsForget(flow, item.uri) }
                gone++
                continue
            }
            withContext(Dispatchers.IO) { runCatching { write(to, item.name, item.mime) { bytes.inputStream() } } }
                .onSuccess { uri ->
                    withContext(Dispatchers.IO) {
                        delete(context, from)
                        core.downloadsMoved(flow, from.toString(), uri.toString())
                    }
                    moves[from.toString()] = uri.toString()
                    moved++
                }
                .onFailure {
                    failed++
                }
        }
        if (moves.isNotEmpty()) onMoved?.invoke(moves)
        Log.info("downloads", "moved", flow, mapOf("moved" to "$moved", "gone" to "$gone", "failed" to "$failed", "chosen" to "${to != null}"))
        return failed
    }

    private fun write(tree: Uri?, name: String, mime: String, open: () -> InputStream): Uri {
        val context = ctx
        val resolver = context.contentResolver
        if (tree != null) {
            val parent = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
            val doc = DocumentsContract.createDocument(resolver, parent, mime, name) ?: error("can't create the file")
            open().use { input -> (resolver.openOutputStream(doc) ?: error("can't write the file")).use { input.copyTo(it) } }
            return doc
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, name)
                put(MediaStore.MediaColumns.MIME_TYPE, mime)
                put(MediaStore.MediaColumns.RELATIVE_PATH, "${Environment.DIRECTORY_DOWNLOADS}/$SUBDIR")
                put(MediaStore.MediaColumns.IS_PENDING, 1)
            }
            val uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values) ?: error("can't create the file")
            open().use { input -> (resolver.openOutputStream(uri) ?: error("can't write the file")).use { input.copyTo(it) } }
            resolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
            return uri
        }
        // API 28: the public Downloads needs WRITE_EXTERNAL_STORAGE (manifest, maxSdk 28); without
        // it, the app's own external Downloads.
        @Suppress("DEPRECATION")
        val dir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), SUBDIR)
            .takeIf { (it.isDirectory || it.mkdirs()) && it.canWrite() }
            ?: File(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS), SUBDIR).apply { mkdirs() }
        var file = File(dir, name)
        var n = 1
        while (file.exists()) file = File(dir, "${name.substringBeforeLast('.')} (${n++})" + (name.substringAfterLast('.', "").takeIf { it.isNotEmpty() }?.let { ".$it" } ?: ""))
        open().use { input -> file.outputStream().use { input.copyTo(it) } }
        return Uri.fromFile(file)
    }

    private fun delete(context: Context, uri: Uri) {
        runCatching {
            when {
                uri.scheme == "file" -> uri.path?.let { File(it).delete() }
                DocumentsContract.isDocumentUri(context, uri) && uri.authority != MediaStore.AUTHORITY -> DocumentsContract.deleteDocument(context.contentResolver, uri)
                else -> context.contentResolver.delete(uri, null, null)
            }
        }
    }

    /** Returns [Core] after migrating any legacy `"downloads"` SharedPreferences `saved` list once. */
    private suspend fun coreReady(flow: String): Core {
        val core = AppCore.await()
        if (!imported) {
            withContext(Dispatchers.IO) {
                if (core.downloadsNeedsImport(flow)) {
                    val legacy = prefs.getString("saved", null)?.let(::decodeSavedFiles).orEmpty()
                    runCatching { core.downloadsImport(flow, legacy) }
                        .onSuccess {
                            prefs.edit { remove("saved") }
                            imported = true
                        }
                        .onFailure { Log.warn("downloads", "import failed: ${it::class.simpleName}", flow) }
                } else {
                    imported = true
                }
            }
        }
        return core
    }

    private suspend fun record(uri: Uri, name: String, mime: String, flow: String): Int {
        val core = coreReady(flow)
        return withContext(Dispatchers.IO) {
            core.downloadsSaved(flow, name, uri.toString(), mime)
            core.downloadsSavedList(flow).size
        }
    }
}
