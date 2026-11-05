package vitapmate.data

import android.app.DownloadManager
import android.content.Context
import android.net.Uri
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
 * A saved document: [file] is its name under `files/docs/`; [source] is where it came from
 * ("import", "vtop", "outing", ...). [page] and [zoom] are where the viewer was left.
 */
data class Doc(
    val id: String,
    val name: String,
    val file: String,
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
            JSONObject().put("id", d.id).put("name", d.name).put("file", d.file).put("kind", d.kind.name).put("size", d.size)
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
                    o.getString("id"), o.getString("name"), o.getString("file"),
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
 * Files the app keeps (VTOP downloads, outing passes, imported files) in app-private
 * `files/docs/`, with `index.json` beside them; the Flutter app's Docs repository and
 * `download_to_docs.dart`. Any feature saves into it with [save] or [keep]. Log src `docs`:
 * counts, sizes and kinds only, never file names.
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

    /** Loads the index and the mess timings and resumes VTOP downloads still to copy; later calls do nothing. */
    @Synchronized
    fun init(context: Context) {
        if (app != null) return
        val ctx = context.applicationContext
        MessTimings.load(ctx)
        Downloads.init(ctx)
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
            downloads(ctx).all.forEach { (key, value) ->
                val saved = value as? String ?: return@forEach
                key.toLongOrNull()?.let { watchDownload(ctx, it, saved.substringAfter('/'), saved.substringBefore('/')) }
            }
        }
    }

    /** Where [doc]'s file is. */
    fun fileOf(doc: Doc): File = File(dir ?: error("Docs.init not called"), doc.file)

    fun byId(id: String): Doc? = docs.firstOrNull { it.id == id }

    /**
     * The one save for fetched files: [bytes] as [name] (an extension from [mime] is added when
     * the name has none) into Docs with its [source] ("vtop", "coursepage", "outing", ...) and, for
     * anything but an import, into the download folder ([Downloads]). An [id] saved before is
     * replaced.
     */
    suspend fun save(name: String, mime: String?, bytes: ByteArray, source: String, id: String? = null, flow: String? = null): Doc =
        add(name, mime, source, id, flow) { it.writeBytes(bytes) }

    /** Copies [file] in, as [save]. */
    suspend fun save(name: String, mime: String?, file: File, source: String, id: String? = null, flow: String? = null): Doc =
        add(name, mime, source, id, flow) { file.copyTo(it, overwrite = true) }

    /** Copies a picked file (Storage Access Framework) in, under its display name without the extension. */
    suspend fun import(context: Context, uri: Uri, flow: String? = null): Doc {
        val resolver = context.contentResolver
        val display = withContext(Dispatchers.IO) {
            resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
        } ?: "Document"
        return add(display, resolver.getType(uri), "import", null, flow) { out ->
            (resolver.openInputStream(uri) ?: error("can't read the file")).use { input -> out.outputStream().use(input::copyTo) }
        }
    }

    /**
     * The document [id] from Docs, or [download]ed and saved the first time (Flutter's
     * `keepInDocs`), so app-fetched files (outing passes) open offline later.
     */
    suspend fun keep(id: String, name: String, mime: String?, source: String, flow: String? = null, download: suspend () -> ByteArray): Doc {
        byId(id)?.takeIf { fileOf(it).exists() }?.let { return it }
        return save(name, mime, download(), source, id, flow)
    }

    suspend fun rename(id: String, name: String) = update(id) { it.copy(name = name.trim().ifEmpty { it.name }) }

    suspend fun setMessMenu(id: String, on: Boolean) = update(id) { it.copy(asMessMenu = on) }

    suspend fun opened(id: String) = update(id) { it.copy(lastOpenedAt = System.currentTimeMillis()) }

    suspend fun saveView(id: String, page: Int, zoom: Float) = update(id) { it.copy(page = page, zoom = zoom) }

    suspend fun remove(id: String) = lock.withLock {
        val doc = docs.firstOrNull { it.id == id } ?: return@withLock
        withContext(Dispatchers.IO) { deleteFile(doc) }
        state.value = docs - doc
        write()
    }

    /**
     * Once DownloadManager download [downloadId] finishes, [save]s it as [name] from [source] (so
     * into Docs and the download folder; the same name from the same source replaces the earlier
     * one), then removes the DownloadManager copy. Kept across launches until done.
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

    /** [doc]'s file, and its own folder when it has one (`doc-<stamp>/<name>`). */
    private fun deleteFile(doc: Doc) {
        val file = fileOf(doc)
        file.delete()
        if (doc.file.contains('/')) file.parentFile?.delete()
    }

    private fun downloads(context: Context) = context.getSharedPreferences("docs_downloads", Context.MODE_PRIVATE)

    private suspend fun add(name: String, mime: String?, source: String, id: String?, flow: String?, write: (File) -> Unit): Doc = lock.withLock {
        // Saving an [id] again replaces it (a re-download); [keep] is the way to reuse a copy.
        docs.firstOrNull { id != null && it.id == id }?.let { withContext(Dispatchers.IO) { deleteFile(it) } }
        val clean = name.substringAfterLast('/').substringAfterLast('\\').trim()
        val ext = clean.substringAfterLast('.', "").lowercase().takeIf { it.isNotEmpty() && it.length <= 5 }
            ?: mime?.let { MimeTypeMap.getSingleton().getExtensionFromMimeType(it) }
        val stamp = "${System.currentTimeMillis()}-${counter++}"
        // In its own folder under its real name, so other apps (share, open with) show that name.
        val base = clean.substringBeforeLast('.').ifEmpty { clean }.replace(Regex("""[\\/:*?"<>|\u0000-\u001f]"""), "_").take(120).ifEmpty { "document" }
        val stored = "doc-$stamp/$base" + (ext?.let { ".$it" } ?: "")
        val target = File(dir ?: error("Docs.init not called"), stored).apply { parentFile?.mkdirs() }
        withContext(Dispatchers.IO) {
            runCatching { write(target) }.onFailure { target.parentFile?.deleteRecursively() }.getOrThrow()
        }
        val doc = Doc(
            id ?: "doc-$stamp",
            clean.substringBeforeLast('.').ifEmpty { clean }.ifEmpty { base },
            stored,
            DocKind.of(stored),
            target.length(),
            source,
            System.currentTimeMillis(),
        )
        state.value = docs.filterNot { it.id == doc.id } + doc
        write()
        // What the app fetched also goes to the download folder; imports are on the phone already.
        if (source != "import") {
            val type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext.orEmpty()) ?: mime ?: "application/octet-stream"
            Downloads.saveLater(doc.name + (ext?.let { ".$it" } ?: ""), type, withContext(Dispatchers.IO) { target.readBytes() }, flow)
        }
        Log.info("docs", "saved", flow, mapOf("kind" to doc.kind.name, "bytes" to "${doc.size}", "source" to source, "count" to "${docs.size}"))
        doc
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
