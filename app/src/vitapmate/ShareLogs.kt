package vitapmate

import android.content.ClipData
import android.content.Context
import android.content.Intent
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.observability.Inspector
import vitapmate.observability.Log

/** Shares the log file through the system share sheet (the `.logs` FileProvider in the manifest). */
suspend fun shareLogs(context: Context) {
    val file = runCatching { withContext(Dispatchers.IO) { Inspector.exportFile() } }.getOrElse {
        Log.error("ui.inspector", "export failed: $it")
        return
    }
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.logs", file)
    val send = Intent(Intent.ACTION_SEND)
        .setType("text/plain")
        .putExtra(Intent.EXTRA_STREAM, uri)
        .putExtra(Intent.EXTRA_SUBJECT, "vitapmate logs")
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    // The clip carries the read grant to the share sheet too, so it can preview the file.
    send.clipData = ClipData.newRawUri(file.name, uri)
    context.startActivity(Intent.createChooser(send, "Share logs"))
}
