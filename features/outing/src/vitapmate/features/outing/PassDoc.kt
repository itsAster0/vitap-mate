package vitapmate.features.outing

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.OutingKind
import vitapmate.data.AppCore
import vitapmate.data.Doc
import vitapmate.data.Docs
import vitapmate.observability.Log

/**
 * The pass as document `outing:<pass id>` in Docs (source `outing`; the file is in the download
 * folder): kept there after the first download, so it opens offline at the gate. Android's
 * DownloadManager fetches it from the link Rust signs ([Docs.download]).
 */
internal suspend fun passDoc(context: Context, kind: OutingKind, passId: String, flow: String): Doc {
    val (doc, kept) = Docs.download(context, "outing:$passId", "outing", flow, reuse = true) {
        withContext(Dispatchers.IO) { AppCore.await().outingPassLink(flow, kind, passId) }
    }
    Log.info("ui.outing", if (kept) "pass from docs" else "pass saved to docs", flow, mapOf("kind" to kind.key, "bytes" to "${doc.size}"))
    return doc
}
