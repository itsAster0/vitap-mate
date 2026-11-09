package vitapmate.features.outing

import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.OutingKind
import vitapmate.data.AppCore
import vitapmate.data.Doc
import vitapmate.data.Docs
import vitapmate.observability.Log

/**
 * The pass as document `outing:<pass id>` in Docs (source `outing`; the file is in the download
 * folder): kept there after the first download, so it opens offline at the gate. Rust fetches it
 * to a temporary file, which goes once saved.
 */
internal suspend fun passDoc(kind: OutingKind, passId: String, flow: String): Doc {
    val id = "outing:$passId"
    Docs.byId(id)?.takeIf { withContext(Dispatchers.IO) { Docs.exists(it) } }?.let {
        Log.info("ui.outing", "pass from docs", flow, mapOf("kind" to kind.key))
        return it
    }
    val file = File(withContext(Dispatchers.IO) { AppCore.await().outingPass(flow, kind, passId) })
    return try {
        Docs.save("Outing pass $passId", "application/pdf", file, "outing", id, flow)
    } finally {
        withContext(Dispatchers.IO) { file.delete() }
    }.also { Log.info("ui.outing", "pass saved to docs", flow, mapOf("kind" to kind.key, "bytes" to "${it.size}")) }
}
