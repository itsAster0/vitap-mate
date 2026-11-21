package vitapmate.ui

import android.os.Build
import android.os.ext.SdkExtensions
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.core.net.toUri
import androidx.pdf.ExperimentalPdfApi
import androidx.pdf.PdfDocument
import androidx.pdf.SandboxedPdfLoader
import androidx.pdf.compose.PdfViewer
import androidx.pdf.compose.rememberPdfViewerState
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.launch

/** Whether [SystemPdf] runs here: Android 12+ with SDK extension 13, the platform's sandboxed PDF renderer. */
fun systemPdfSupported(): Boolean = Build.VERSION.SDK_INT >= 31 && SdkExtensions.getExtensionVersion(Build.VERSION_CODES.S) >= 13

/**
 * The PDF at [uri] in Jetpack PDF's viewer (only where [systemPdfSupported]): sharp at any zoom,
 * text to select and copy, tappable links and a fast-scroll thumb. Opens on [page]; [onPage] hears
 * the first visible page as it settles.
 */
@OptIn(ExperimentalPdfApi::class, FlowPreview::class)
@Composable
fun SystemPdf(uri: String, modifier: Modifier = Modifier, page: Int = 0, padding: PaddingValues = PaddingValues(), onPage: (suspend (Int) -> Unit)? = null, onFailed: (Throwable) -> Unit) {
    val context = LocalContext.current
    val document by produceState<PdfDocument?>(null, uri) {
        value = try {
            SandboxedPdfLoader(context).openDocument(uri.toUri())
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            onFailed(e)
            null
        }
    }
    DisposableEffect(document) { onDispose { document?.close() } }
    val state = rememberPdfViewerState()
    val scope = rememberCoroutineScope()
    if (onPage != null) LaunchedEffect(state) { snapshotFlow { state.firstVisiblePage }.debounce(500).collect { onPage(it) } }
    PdfViewer(
        document,
        state,
        modifier,
        contentPadding = padding,
        onFirstContentLoad = { if (page > 0) scope.launch { state.scrollToPage(page) } },
    )
}
