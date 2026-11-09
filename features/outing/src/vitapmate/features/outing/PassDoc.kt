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
    val (doc, kept) = Docs.keep("outing:$passId", "outing", flow, "Outing pass $passId", "application/pdf") {
        File(withContext(Dispatchers.IO) { AppCore.await().outingPass(flow, kind, passId) })
    }
    Log.info("ui.outing", if (kept) "pass from docs" else "pass saved to docs", flow, mapOf("kind" to kind.key, "bytes" to "${doc.size}"))
    return doc
}
