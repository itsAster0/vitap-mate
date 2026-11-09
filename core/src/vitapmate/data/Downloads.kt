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
import java.io.File
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * The app's one download folder, where saved files (VTOP downloads, course page files, outing
 * passes) land for the student: a folder they chose (Storage Access Framework tree, persistable
 * permission) or, by default, Downloads/vitapmate. The first save asks which ([prompt], drawn by
 * the design module's `DownloadFolderPrompt`). The app remembers what it saved, so changing the
 * folder moves those files (and only those), and [onMoved] tells Docs (a view over this folder)
 * where each went. Log src `downloads`: counts only.
 */
object Downloads {
    const val DEFAULT_LABEL = "Downloads/vitapmate"
    private const val SUBDIR = "vitapmate"
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val lock = Mutex()
    private var app: Context? = null
    private val prefs get() = app!!.getSharedPreferences("downloads", Context.MODE_PRIVATE)

    /** The chosen folder (a tree URI), or null for Downloads/vitapmate. */
    var folder by mutableStateOf<Uri?>(null)
        private set

    /** Told the old → new URI of each file a folder change moved (Docs follows them). */
    var onMoved: ((Map<String, String>) -> Unit)? = null

    /** Set while a save waits for the student to pick a folder the first time; complete it to go on. */
    var prompt by mutableStateOf<CompletableDeferred<Unit>?>(null)
        private set

    fun init(context: Context) {
        if (app != null) return
        app = context.applicationContext
        folder = prefs.getString("tree", null)?.let(Uri::parse)
    }

    /** Where files go, for Settings: the chosen folder's name or [DEFAULT_LABEL]. */
    fun label(): String = folder?.let { DocumentsContract.getTreeDocumentId(it).substringAfterLast(':').ifEmpty { "Chosen folder" } } ?: DEFAULT_LABEL

    /** Saves [bytes] as [name] in the folder, asking for one the first time; returns where. */
    suspend fun save(name: String, mime: String, bytes: ByteArray, flow: String? = null): Uri {
        if (!prefs.getBoolean("asked", false)) {
            val ask = prompt ?: CompletableDeferred<Unit>().also { prompt = it }
            ask.await()
        }
        return lock.withLock {
            val uri = withContext(Dispatchers.IO) { write(folder, name, mime, bytes) }
            record(uri, name, mime)
            Log.info("downloads", "saved", flow, mapOf("bytes" to "${bytes.size}", "chosen" to "${folder != null}", "count" to "${saved().length()}"))
            uri
        }
    }

    /** Saves without asking for a folder (the chosen one, else Downloads/vitapmate): Docs' move out of app storage. */
    suspend fun saveQuietly(name: String, mime: String, bytes: ByteArray): Uri = lock.withLock {
        withContext(Dispatchers.IO) { write(folder, name, mime, bytes) }.also { record(it, name, mime) }
    }

    /** Deletes a file this app saved, and forgets it. */
    suspend fun delete(uri: Uri) = lock.withLock {
        val context = app ?: return@withLock
        withContext(Dispatchers.IO) { delete(context, uri) }
        val list = saved()
        val kept = JSONArray()
        for (i in 0 until list.length()) list.getJSONObject(i).takeIf { it.optString("uri") != uri.toString() }?.let(kept::put)
        prefs.edit().putString("saved", kept.toString()).apply()
    }

    /** Whether [uri] still opens (the student may have deleted the file in their file manager). */
    fun exists(uri: Uri): Boolean = runCatching { app!!.contentResolver.openFileDescriptor(uri, "r")?.use { true } ?: false }.getOrDefault(false)

    /** A file in the folder: where it is, its name and size. */
    class Entry(val uri: String, val name: String, val size: Long)

    /**
     * Every file in the folder now, or null when it can't be read (permission gone). In the
     * default folder Android shows the app only the files it saved itself. Disk/binder: off the
     * main thread.
     */
    fun list(): List<Entry>? = runCatching {
        val resolver = app!!.contentResolver
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
    fun savedByName(): Map<String, String> {
        val list = saved()
        return (0 until list.length()).associate { i -> list.getJSONObject(i).let { it.optString("name") to it.optString("uri") } }
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
        folder = tree
        prefs.edit().putString("tree", tree?.toString()).putBoolean("asked", true).apply()
        prompt?.complete(Unit)
        prompt = null
        if (old == tree) return
        scope.launch {
            flow("downloads.move") { f ->
                val failed = lock.withLock { move(context, tree, f) }
                if (old != null) runCatching { context.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
                if (failed > 0) onFailed(failed)
            }
        }
    }

    private fun move(context: Context, to: Uri?, flow: String): Int {
        val resolver = context.contentResolver
        val list = saved()
        val kept = JSONArray()
        val moves = mutableMapOf<String, String>()
        var moved = 0
        var gone = 0
        var failed = 0
        for (i in 0 until list.length()) {
            val item = list.getJSONObject(i)
            val from = Uri.parse(item.getString("uri"))
            val bytes = runCatching { resolver.openInputStream(from)?.use { it.readBytes() } }.getOrNull()
            if (bytes == null) {
                gone++
                continue
            }
            runCatching { write(to, item.getString("name"), item.getString("mime"), bytes) }
                .onSuccess { uri ->
                    delete(context, from)
                    moves[from.toString()] = uri.toString()
                    kept.put(JSONObject(item.toString()).put("uri", uri.toString()))
                    moved++
                }
                .onFailure {
                    kept.put(item)
                    failed++
                }
        }
        prefs.edit().putString("saved", kept.toString()).apply()
        if (moves.isNotEmpty()) onMoved?.invoke(moves)
        Log.info("downloads", "moved", flow, mapOf("moved" to "$moved", "gone" to "$gone", "failed" to "$failed", "chosen" to "${to != null}"))
        return failed
    }

    private fun write(tree: Uri?, name: String, mime: String, bytes: ByteArray): Uri {
        val context = app!!
        val resolver = context.contentResolver
        if (tree != null) {
            val parent = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
            val doc = DocumentsContract.createDocument(resolver, parent, mime, name) ?: error("can't create the file")
            resolver.openOutputStream(doc)!!.use { it.write(bytes) }
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
            resolver.openOutputStream(uri)!!.use { it.write(bytes) }
            resolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
            return uri
        }
        // API 24–28: the public Downloads needs WRITE_EXTERNAL_STORAGE (manifest, maxSdk 28); without
        // it, the app's own external Downloads.
        @Suppress("DEPRECATION")
        val dir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), SUBDIR)
            .takeIf { (it.isDirectory || it.mkdirs()) && it.canWrite() }
            ?: File(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS), SUBDIR).apply { mkdirs() }
        var file = File(dir, name)
        var n = 1
        while (file.exists()) file = File(dir, "${name.substringBeforeLast('.')} (${n++})" + (name.substringAfterLast('.', "").takeIf { it.isNotEmpty() }?.let { ".$it" } ?: ""))
        file.writeBytes(bytes)
        return Uri.fromFile(file)
    }

    private fun delete(context: Context, uri: Uri) {
        runCatching {
            when {
                uri.scheme == "file" -> File(uri.path!!).delete()
                DocumentsContract.isDocumentUri(context, uri) && uri.authority != MediaStore.AUTHORITY -> DocumentsContract.deleteDocument(context.contentResolver, uri)
                else -> context.contentResolver.delete(uri, null, null)
            }
        }
    }

    private fun saved(): JSONArray = runCatching { JSONArray(prefs.getString("saved", "[]")) }.getOrElse { JSONArray() }

    private fun record(uri: Uri, name: String, mime: String) {
        prefs.edit().putString("saved", saved().put(JSONObject().put("uri", uri.toString()).put("name", name).put("mime", mime)).toString()).apply()
    }
}
