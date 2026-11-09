package vitapmate.data

import android.app.DownloadManager
import android.content.Context
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import android.webkit.MimeTypeMap
import androidx.compose.runtime.mutableStateOf
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import vitapmate.observability.Log
import vitapmate.observability.flow

/** What Docs can show a file as, from its extension; anything else is a plain [File]. */
enum class DocKind(val extensions: List<String>) {
    Pdf(listOf("pdf")),
    Image(listOf("png", "jpg", "jpeg", "webp", "gif", "bmp")),
    Spreadsheet(listOf("xlsx", "xlsm", "xls", "ods")),
    Text(listOf("xml", "html", "htm", "txt", "json", "md", "csv")),
    File(emptyList());

    companion object {
        fun of(fileName: String): DocKind = fileName.substringAfterLast('.', "").lowercase().let { ext -> entries.firstOrNull { ext in it.extensions } ?: File }
    }
}

/**
 * A saved document: the file at [uri] in the download folder ([Downloads]), [file] its file name
 * there (for the kind and type); [source] is where it came from ("import", "vtop", "outing", ...).
 * [page] and [zoom] are where the viewer was left.
 */
data class Doc(
    val id: String,
    val name: String,
    val file: String,
    /** Empty only for a document still in app storage from before (moved out at start). */
    val uri: String,
    val kind: DocKind,
    val size: Long,
    val source: String,
    val addedAt: Long,
    val lastOpenedAt: Long? = null,
    /** A spreadsheet read as the mess menu instead of a table; the file is unchanged. */
    val asMessMenu: Boolean = false,
    val page: Int = 0,
    val zoom: Float = 1f,
)

/** The Flutter app's Docs sort options. */
enum class DocSort(val label: String) {
    RecentlyOpened("Recently opened"),
    RecentlyAdded("Recently added"),
    NameAscending("Name A–Z"),
    NameDescending("Name Z–A"),
}

/** [docs] whose name contains [query], in [sort] order; ties by name, then id (Flutter's `queryDocs`). */
fun queryDocs(docs: List<Doc>, sort: DocSort, query: String = ""): List<Doc> {
    val needle = query.trim().lowercase()
    val byName = compareBy<Doc> { it.name.lowercase() }
    val primary: Comparator<Doc> = when (sort) {
        // Never-opened documents go last.
        DocSort.RecentlyOpened -> compareBy(nullsLast(reverseOrder())) { it.lastOpenedAt }
        DocSort.RecentlyAdded -> compareByDescending { it.addedAt }
        DocSort.NameAscending -> byName
        DocSort.NameDescending -> byName.reversed()
    }
    return docs.filter { needle in it.name.lowercase() }.sortedWith(primary.then(byName).thenBy { it.id })
}

/** The index file: the sort and every document, newest last. */
internal object DocIndex {
    fun encode(sort: DocSort, docs: List<Doc>): String = JSONObject()
        .put("sort", sort.name)
        .put("docs", JSONArray(docs.map { d ->
            JSONObject().put("id", d.id).put("name", d.name).put("file", d.file).put("uri", d.uri).put("kind", d.kind.name).put("size", d.size)
                .put("source", d.source).put("addedAt", d.addedAt).put("lastOpenedAt", d.lastOpenedAt ?: JSONObject.NULL)
                .put("asMessMenu", d.asMessMenu).put("page", d.page).put("zoom", d.zoom.toDouble())
        }))
        .toString()

    fun decode(raw: String): Pair<DocSort, List<Doc>> {
        val json = JSONObject(raw)
        val sort = DocSort.entries.firstOrNull { it.name == json.optString("sort") } ?: DocSort.RecentlyOpened
        val list = json.optJSONArray("docs") ?: JSONArray()
        val docs = (0 until list.length()).mapNotNull { i ->
            val o = list.optJSONObject(i) ?: return@mapNotNull null
            runCatching {
                Doc(
                    o.getString("id"), o.getString("name"), o.getString("file"), o.optString("uri"),
                    DocKind.entries.firstOrNull { it.name == o.optString("kind") } ?: DocKind.of(o.getString("file")),
                    o.optLong("size"), o.optString("source", "import"), o.optLong("addedAt"),
                    if (o.isNull("lastOpenedAt")) null else o.optLong("lastOpenedAt"),
                    o.optBoolean("asMessMenu"), o.optInt("page"), o.optDouble("zoom", 1.0).toFloat(),
                )
            }.getOrNull()
        }
        return sort to docs
    }
}

/**
 * The app's view of the files it saved for the student (VTOP downloads, outing passes, imported
 * files): each lives only in the download folder ([Downloads]); `files/docs/index.json` keeps the
 * names, sources and viewer state; the Flutter app's Docs repository and `download_to_docs.dart`.
 * Any feature saves with [save] or [keep]. Log src `docs`: counts, sizes and kinds only, never
 * file names.
 */
object Docs {
    private val state = mutableStateOf<List<Doc>>(emptyList())
    private val sortState = mutableStateOf(DocSort.RecentlyOpened)
    private val lock = Mutex()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var dir: File? = null
    private var app: Context? = null
    private var counter = 0

    /** Every document, in saved order; Compose state. */
    val docs: List<Doc> get() = state.value

    /** The library's sort, kept in the index. */
    var sort: DocSort
        get() = sortState.value
        set(value) {
            sortState.value = value
            scope.launch { lock.withLock { write() } }
        }

    /**
     * Loads the index and the mess timings, moves documents still in app storage (from before)
     * to the download folder, and resumes VTOP downloads still to copy; later calls do nothing.
     */
    @Synchronized
    fun init(context: Context) {
        if (app != null) return
        val ctx = context.applicationContext
        MessTimings.load(ctx)
        Downloads.init(ctx)
        // Docs follow their files when the student picks another folder.
        Downloads.onMoved = { moves -> scope.launch { remap(moves) } }
        val d = File(ctx.filesDir, "docs").apply { mkdirs() }
        dir = d
        // Small; read here so a feature's first [byId] already sees it.
        val index = File(d, "index.json")
        if (index.exists()) runCatching { DocIndex.decode(index.readText()) }
            .onSuccess { (s, list) ->
                sortState.value = s
                state.value = list
            }
            .onFailure { Log.error("docs", "index unreadable: ${it::class.simpleName}") }
        app = ctx
        scope.launch {
            migrate()
            downloads(ctx).all.forEach { (key, value) ->
                val saved = value as? String ?: return@forEach
                key.toLongOrNull()?.let { watchDownload(ctx, it, saved.substringAfter('/'), saved.substringBefore('/')) }
            }
        }
    }

    /** Where [doc]'s file is. */
    fun uriOf(doc: Doc): Uri = Uri.parse(doc.uri)

    /** Whether [doc]'s file is still there (it can be deleted in a file manager). Disk/binder: off the main thread. */
    fun exists(doc: Doc): Boolean = doc.uri.isNotEmpty() && Downloads.exists(uriOf(doc))

    /** [doc]'s file for reading in place (PdfRenderer). */
    fun open(doc: Doc): ParcelFileDescriptor =
        app!!.contentResolver.openFileDescriptor(uriOf(doc), "r") ?: error("can't open the file")

    fun readBytes(doc: Doc): ByteArray = (app!!.contentResolver.openInputStream(uriOf(doc)) ?: error("can't open the file")).use { it.readBytes() }

    /**
     * Runs [block] on a temporary copy of [doc] in the cache, for APIs that need a [File] (a zip
     * reader); the copy is deleted after.
     */
    fun <T> withFile(doc: Doc, block: (File) -> T): T {
        val ctx = app!!
        val tmp = File(File(ctx.cacheDir, "docs").apply { mkdirs() }, "${System.nanoTime()}-${doc.file.substringAfterLast('/')}")
        try {
            (ctx.contentResolver.openInputStream(uriOf(doc)) ?: error("can't open the file")).use { input -> tmp.outputStream().use(input::copyTo) }
            return block(tmp)
        } finally {
            tmp.delete()
        }
    }

    fun byId(id: String): Doc? = docs.firstOrNull { it.id == id }

    /**
     * The one save for fetched files: [bytes] as [name] (an extension from [mime] is added when
     * the name has none) into the download folder ([Downloads], which asks for one the first
     * time) and Docs, with its [source] ("vtop", "coursepage", "outing", ...). An [id] saved
     * before is replaced.
     */
    suspend fun save(name: String, mime: String?, bytes: ByteArray, source: String, id: String? = null, flow: String? = null): Doc =
        add(name, mime, source, id, flow) { bytes }

    /** Reads [file] (a temporary download) in, as [save]. */
    suspend fun save(name: String, mime: String?, file: File, source: String, id: String? = null, flow: String? = null): Doc =
        add(name, mime, source, id, flow) { file.readBytes() }

    /** Copies a picked file (Storage Access Framework) into the download folder, under its display name without the extension. */
    suspend fun import(context: Context, uri: Uri, flow: String? = null): Doc {
        val resolver = context.contentResolver
        val display = withContext(Dispatchers.IO) {
            resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
        } ?: "Document"
        return add(display, resolver.getType(uri), "import", null, flow) {
            (resolver.openInputStream(uri) ?: error("can't read the file")).use { it.readBytes() }
        }
    }

    /**
     * The document [id] from Docs, or [download]ed and saved the first time (Flutter's
     * `keepInDocs`), so app-fetched files (outing passes) open offline later.
     */
    suspend fun keep(id: String, name: String, mime: String?, source: String, flow: String? = null, download: suspend () -> ByteArray): Doc {
        byId(id)?.takeIf { withContext(Dispatchers.IO) { exists(it) } }?.let { return it }
        return save(name, mime, download(), source, id, flow)
    }

    suspend fun rename(id: String, name: String) = update(id) { it.copy(name = name.trim().ifEmpty { it.name }) }

    suspend fun setMessMenu(id: String, on: Boolean) = update(id) { it.copy(asMessMenu = on) }

    suspend fun opened(id: String) = update(id) { it.copy(lastOpenedAt = System.currentTimeMillis()) }

    suspend fun saveView(id: String, page: Int, zoom: Float) = update(id) { it.copy(page = page, zoom = zoom) }

    /** Removes [id] from Docs and its file from the download folder. */
    suspend fun remove(id: String) = lock.withLock {
        val doc = docs.firstOrNull { it.id == id } ?: return@withLock
        if (doc.uri.isNotEmpty()) Downloads.delete(uriOf(doc))
        state.value = docs - doc
        write()
    }

    /**
     * Once DownloadManager download [downloadId] finishes, [save]s it as [name] from [source] (the
     * same name from the same source replaces the earlier one), then removes the DownloadManager
     * copy. Kept across launches until done.
     */
    fun watchDownload(context: Context, downloadId: Long, name: String, source: String) {
        val prefs = downloads(context)
        prefs.edit().putString("$downloadId", "$source/$name").apply()
        scope.launch {
            flow("docs.download") { f ->
                val manager = context.getSystemService(DownloadManager::class.java)
                while (true) {
                    val status = manager.query(DownloadManager.Query().setFilterById(downloadId))?.use { c ->
                        if (c.moveToFirst()) c.getInt(c.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS)) else null
                    }
                    when (status) {
                        DownloadManager.STATUS_SUCCESSFUL -> {
                            runCatching {
                                val bytes = context.contentResolver.openInputStream(manager.getUriForDownloadedFile(downloadId) ?: error("no file"))!!.use { it.readBytes() }
                                save(name, null, bytes, source, "$source:$name", f)
                                manager.remove(downloadId)
                            }.onFailure { Log.error("docs", "download copy failed: ${it::class.simpleName}", f) }
                            break
                        }
                        DownloadManager.STATUS_FAILED, null -> {
                            Log.warn("docs", "download failed", f)
                            break
                        }
                        else -> delay(2_000)
                    }
                }
                prefs.edit().remove("$downloadId").apply()
            }
        }
    }

    private fun downloads(context: Context) = context.getSharedPreferences("docs_downloads", Context.MODE_PRIVATE)

    private fun mimeOf(fileName: String, fallback: String?): String =
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(fileName.substringAfterLast('.', "").lowercase()) ?: fallback ?: "application/octet-stream"

    private suspend fun add(name: String, mime: String?, source: String, id: String?, flow: String?, read: () -> ByteArray): Doc {
        val clean = name.substringAfterLast('/').substringAfterLast('\\').trim()
        val ext = clean.substringAfterLast('.', "").lowercase().takeIf { it.isNotEmpty() && it.length <= 5 }
            ?: mime?.let { MimeTypeMap.getSingleton().getExtensionFromMimeType(it) }
        val base = clean.substringBeforeLast('.').ifEmpty { clean }.replace(Regex("""[\\/:*?"<>|\u0000-\u001f]"""), "_").take(120).ifEmpty { "document" }
        val fileName = base + (ext?.let { ".$it" } ?: "")
        val bytes = withContext(Dispatchers.IO) { read() }
        // Outside the lock: the first save may wait for the student to pick a folder.
        val uri = Downloads.save(fileName, mimeOf(fileName, mime), bytes, flow)
        return lock.withLock {
            // Saving an [id] again replaces it (a re-download); [keep] is the way to reuse a copy.
            docs.firstOrNull { id != null && it.id == id }?.takeIf { it.uri.isNotEmpty() && it.uri != uri.toString() }?.let { Downloads.delete(uriOf(it)) }
            val doc = Doc(
                id ?: "doc-${System.currentTimeMillis()}-${counter++}",
                clean.substringBeforeLast('.').ifEmpty { clean }.ifEmpty { base },
                fileName,
                uri.toString(),
                DocKind.of(fileName),
                bytes.size.toLong(),
                source,
                System.currentTimeMillis(),
            )
            state.value = docs.filterNot { it.id == doc.id } + doc
            write()
            Log.info("docs", "saved", flow, mapOf("kind" to doc.kind.name, "bytes" to "${doc.size}", "source" to source, "count" to "${docs.size}"))
            doc
        }
    }

    /** The folder changed: each document whose file moved points at its new place. */
    private suspend fun remap(moves: Map<String, String>) = lock.withLock {
        state.value = docs.map { d -> moves[d.uri]?.let { d.copy(uri = it) } ?: d }
        write()
    }

    /**
     * Documents saved before Docs became a view (their file under `files/docs/doc-<stamp>/`): each
     * points at its copy in the download folder when that is still there, else its file is written
     * there now (no folder prompt: the chosen folder or Downloads/vitapmate); then the app's copy
     * goes. Counts only in the log.
     */
    private suspend fun migrate() {
        val d = dir ?: return
        val old = docs.filter { it.uri.isEmpty() }
        if (old.isEmpty() && d.listFiles().orEmpty().none { it.isDirectory }) return
        flow("docs.migrate") { f ->
            val byName = Downloads.savedByName()
            var linked = 0
            var written = 0
            var lost = 0
            val moved = old.mapNotNull { doc ->
                val file = File(d, doc.file)
                val fileName = doc.file.substringAfterLast('/')
                val copy = byName[fileName]?.let(Uri::parse)?.takeIf { Downloads.exists(it) }
                val uri = when {
                    copy != null -> copy.also { linked++ }
                    file.exists() -> runCatching { Downloads.saveQuietly(fileName, mimeOf(fileName, null), file.readBytes()) }
                        .onFailure { Log.warn("docs", "move failed: ${it::class.simpleName}", f) }.getOrNull()?.also { written++ }
                    else -> null
                }
                if (uri == null && file.exists()) return@mapNotNull doc // Try again next start.
                if (uri == null) lost++
                uri?.let { doc.copy(file = fileName, uri = it.toString()) }
            }
            lock.withLock {
                val byId = moved.associateBy { it.id }
                // Gone everywhere: dropped. Still in app storage after a failed write: kept for next time.
                state.value = docs.mapNotNull { doc -> if (doc.uri.isNotEmpty()) doc else byId[doc.id] ?: doc.takeIf { File(d, it.file).exists() } }
                write()
            }
            // Only the index stays in app storage.
            val keep = docs.filter { it.uri.isEmpty() }.map { it.file.substringBefore('/') }.toSet()
            d.listFiles().orEmpty().filter { it.isDirectory && it.name !in keep }.forEach { it.deleteRecursively() }
            Log.info("docs", "moved to download folder", f, mapOf("linked" to "$linked", "written" to "$written", "lost" to "$lost", "count" to "${docs.size}"))
        }
    }

    private suspend fun update(id: String, change: (Doc) -> Doc) = lock.withLock {
        if (docs.none { it.id == id }) return@withLock
        state.value = docs.map { if (it.id == id) change(it) else it }
        write()
    }

    private suspend fun write() = withContext(Dispatchers.IO) {
        val d = dir ?: return@withContext
        val tmp = File(d, "index.json.tmp")
        tmp.writeText(DocIndex.encode(sort, docs))
        tmp.renameTo(File(d, "index.json"))
    }
}
