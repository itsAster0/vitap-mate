package vitapmate.ui

import android.content.Context
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.FileText
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Printer
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.data.PdfExport
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * The menu a long press on the account button opens on a feature with [Feature.pdf]: the page as
 * a PDF, shared or printed. [build] makes the file off the main thread from what the app has
 * saved, or null when there is nothing yet. Flow and test tag `<feature>.pdf`, log src `ui.<feature>`.
 */
@Composable
fun PdfMenu(feature: String, open: Boolean, onDismiss: () -> Unit, build: suspend (context: Context, flow: String) -> File?) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    fun export(print: Boolean) {
        onDismiss()
        scope.launch {
            flow("$feature.pdf") { f ->
                val file = try {
                    withContext(Dispatchers.IO) { build(context, f) }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.warn("ui.$feature", "pdf failed: ${e::class.simpleName}", f)
                    return@flow showToast("Couldn't make the PDF", "Please try again.")
                }
                if (file == null) return@flow showToast("Nothing to export yet", "Try again once the page has loaded.")
                Log.info("ui.$feature", "pdf", f, mapOf("how" to if (print) "print" else "share", "bytes" to "${file.length()}"))
                if (print) PdfExport.print(context, file) else PdfExport.share(context, file)
            }
        }
    }
    DropdownMenu(open, onDismiss, Modifier.testTag("$feature.pdf")) {
        DropdownMenuItem({ Text("Share as PDF") }, { export(false) }, Modifier.testTag("$feature.pdf.share"), leadingIcon = { Icon(Lucide.FileText, null) })
        DropdownMenuItem({ Text("Print") }, { export(true) }, Modifier.testTag("$feature.pdf.print"), leadingIcon = { Icon(Lucide.Printer, null) })
    }
}
