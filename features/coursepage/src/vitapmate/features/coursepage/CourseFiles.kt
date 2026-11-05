package vitapmate.features.coursepage

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.SavedFile
import vitapmate.data.AppCore
import vitapmate.data.Docs
import vitapmate.data.OtpPrompt
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/** A tap that changes only this screen (a filter, a section): its own flow and one log line. */
internal fun CoroutineScope.action(name: String, msg: String, fields: Map<String, String> = emptyMap()) {
    launch { flow(name) { id -> Log.info("ui.coursepage", msg, id, fields) } }
}

/**
 * Flow `coursepage.file`: Rust streams the VTOP file to `files/downloads/`, it moves into Docs as
 * document `coursepage:<id>` (source `vtop`) and opens in the phone's viewer. With [keep], a copy
 * already in Docs opens without fetching (lecture material and syllabi don't change); otherwise the
 * new copy replaces it. Failures are a toast; a re-login's OTP opens its prompt instead.
 */
internal suspend fun openCourseFile(context: Context, id: String, kind: String, keep: Boolean, fetch: suspend (Core, String) -> SavedFile) {
    try {
        flow("coursepage.file") { f ->
            val docId = "coursepage:$id"
            val kept = Docs.byId(docId)?.takeIf { keep && Docs.fileOf(it).exists() }
            val doc = kept ?: run {
                val saved = withContext(Dispatchers.IO) { fetch(AppCore.await(), f) }
                val file = File(saved.path)
                try {
                    Docs.remove(docId)
                    Docs.save(saved.name, saved.mime.ifEmpty { null }, file, "coursepage", docId, f)
                } finally {
                    withContext(Dispatchers.IO) { file.delete() }
                }
            }
            Log.info("ui.coursepage", "file ready", f, mapOf("kind" to kind, "kept" to "${kept != null}", "bytes" to "${doc.size}"))
            Docs.opened(doc.id)
            val file = Docs.fileOf(doc)
            // TODO(docs): open in the Docs viewer once core has a way to; the phone's viewer until then.
            val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "*/*"
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.logs", file)
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
        if (e is CoreException.OtpRequired) return OtpPrompt.ask(e.reason)
        showToast("Couldn't get the file", problemOf(e, "").message)
    }
}
