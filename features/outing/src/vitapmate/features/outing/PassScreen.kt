package vitapmate.features.outing

import android.graphics.Bitmap
import android.graphics.Color as AndroidColor
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.FileX
import com.composables.icons.lucide.Lucide
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.OutingKind
import vitapmate.data.Docs
import vitapmate.data.OtpPrompt
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space

/**
 * The outing pass from Docs ([item] `<general|weekend>:<pass id>`) drawn page by page at the screen's
 * width. Read from the saved PDF (Rust downloads it the first time), so it opens offline at the
 * gate. Flow `outing.pass`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun PassScreen(item: String) {
    val kind = if (item.substringBefore(':') == OutingKind.WEEKEND.key) OutingKind.WEEKEND else OutingKind.GENERAL
    val passId = item.substringAfter(':')
    var pages by remember { mutableStateOf<List<Bitmap>?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableIntStateOf(0) }
    BoxWithConstraints(Modifier.fillMaxSize().testTag("outing.pass.screen"), Alignment.Center) {
        val widthPx = with(LocalDensity.current) { (maxWidth - Space.lg * 2).roundToPx() }.coerceAtLeast(1)
        LaunchedEffect(passId, attempt, widthPx) {
            failure = null
            try {
                pages = flow("outing.pass") { id ->
                    val doc = passDoc(kind, passId, id)
                    Docs.opened(doc.id)
                    withContext(Dispatchers.IO) { render(Docs.open(doc), widthPx) }
                        .also { Log.info("ui.outing", "pass shown", id, mapOf("kind" to kind.key, "pages" to "${it.size}")) }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason)
                failure = failureOf(e, "")
            }
            StateRegistry.report("ui.outing.pass", mapOf("kind" to kind.key, "pages" to pages?.size, "failed" to (failure != null)))
        }
        val shown = pages
        when {
            shown != null -> LazyColumn(
                Modifier.fillMaxSize(),
                contentPadding = PaddingValues(start = Space.lg, top = Space.sm, end = Space.lg, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg),
                verticalArrangement = Arrangement.spacedBy(Space.md),
            ) {
                itemsIndexed(shown) { i, page ->
                    Image(
                        page.asImageBitmap(),
                        "Pass page ${i + 1}",
                        Modifier.fillMaxWidth().aspectRatio(page.width.toFloat() / page.height)
                            .clip(MaterialTheme.shapes.medium).background(MaterialTheme.colorScheme.surfaceContainerLowest)
                            .testTag("outing.pass.page"),
                    )
                }
            }
            failure != null -> EmptyView(
                Lucide.FileX,
                "Couldn't get the pass",
                message = failure,
                action = { Button({ attempt++ }, variant = ButtonVariant.Outline) { Text("Retry") } },
            )
            else -> Box(Modifier.fillMaxSize(), Alignment.Center) { LoadingIndicator(Modifier.testTag("outing.pass.loading")) }
        }
    }
}

/** Each page of [file] [width] pixels wide, on white (a PDF page has no background). */
private fun render(fd: ParcelFileDescriptor, width: Int): List<Bitmap> =
    PdfRenderer(fd).use { pdf ->
        (0 until pdf.pageCount).map { i ->
            pdf.openPage(i).use { page ->
                val height = (width.toLong() * page.height / page.width.coerceAtLeast(1)).toInt().coerceAtLeast(1)
                Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888).apply {
                    eraseColor(AndroidColor.WHITE)
                    page.render(this, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                }
            }
        }
    }
