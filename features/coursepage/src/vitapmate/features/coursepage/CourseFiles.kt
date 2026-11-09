package vitapmate.features.coursepage

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.FileLink
import vitapmate.data.AppCore
import vitapmate.data.Docs
import vitapmate.data.askedOtp
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * Flow `coursepage.file`: Android's DownloadManager fetches the VTOP file from the link Rust signs
 * ([Docs.download]), it moves to the download folder (Docs) as document `coursepage:<id>` (source
 * `coursepage`) and opens in the phone's viewer. With [keep], a copy
 * already in Docs opens without fetching (lecture material and syllabi don't change); otherwise the
 * new copy replaces it. Failures are a toast; a re-login's OTP opens its prompt instead.
 */
internal suspend fun openCourseFile(context: Context, id: String, kind: String, keep: Boolean, fetch: suspend (Core, String) -> FileLink) {
    try {
        flow("coursepage.file") { f ->
            val (doc, kept) = Docs.download(context, "coursepage:$id", "coursepage", f, reuse = keep) { withContext(Dispatchers.IO) { fetch(AppCore.await(), f) } }
            Log.info("ui.coursepage", "file ready", f, mapOf("kind" to kind, "kept" to "$kept", "bytes" to "${doc.size}"))
            Docs.opened(doc.id)
            // TODO(docs): open in the Docs viewer once core has a way to; the phone's viewer until then.
            val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(doc.file.substringAfterLast('.', "").lowercase()) ?: "*/*"
            val uri = Docs.uriOf(doc).let { if (it.scheme == "file") FileProvider.getUriForFile(context, "${context.packageName}.logs", File(it.path!!)) else it }
            try {
                context.startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK))
            } catch (_: ActivityNotFoundException) {
                Log.info("ui.coursepage", "no viewer", f, mapOf("mime" to mime))
                showToast("Saved to Docs", "No app on this phone opens this file.")
            }
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        if (askedOtp(e)) return
        showToast("Couldn't get the file", problemOf(e, "").message)
    }
}
