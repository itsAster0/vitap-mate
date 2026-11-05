package vitapmate.features.docs

import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import android.util.LruCache
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
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import java.io.File
import java.util.concurrent.Executors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.withContext
import vitapmate.data.Doc
import vitapmate.data.Docs
import vitapmate.ui.AppTheme
import vitapmate.ui.Space

/** An open PDF: page sizes up front, pages rendered one at a time on its own thread (PdfRenderer allows one). */
private class Pdf(file: File) : AutoCloseable {
    private val renderer = PdfRenderer(ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY))
    private val executor = Executors.newSingleThreadExecutor()
    private val thread = executor.asCoroutineDispatcher()
    private val cache = object : LruCache<Int, Bitmap>(48 * 1024 * 1024) {
        override fun sizeOf(key: Int, value: Bitmap) = value.byteCount
    }

    /** Width / height of each page. */
    val ratios: List<Float> = (0 until renderer.pageCount).map { i -> renderer.openPage(i).use { it.width.toFloat() / it.height } }

    suspend fun page(index: Int, width: Int): Bitmap = cache.get(index)?.takeIf { it.width >= width } ?: withContext(thread) {
        renderer.openPage(index).use { page ->
            val bitmap = Bitmap.createBitmap(width, (width / ratios[index]).toInt().coerceAtLeast(1), Bitmap.Config.ARGB_8888)
            bitmap.eraseColor(Color.WHITE)
            page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
            bitmap.also { cache.put(index, it) }
        }
    }

    /** After any page being rendered, on the same thread. */
    override fun close() {
        executor.execute { renderer.close() }
        executor.shutdown()
    }
}

/** Every page down a list, pinch to zoom; reopens on the page it was left at. */
@OptIn(FlowPreview::class)
@Composable
internal fun PdfView(doc: Doc, embedded: Boolean, onFailed: (Throwable) -> Unit) {
    val pdf by produceState<Pdf?>(null, doc.file) {
        value = runCatching { withContext(Dispatchers.IO) { Pdf(Docs.fileOf(doc)) } }.onFailure(onFailed).getOrNull()
    }
    val open = pdf ?: return Box(Modifier.fillMaxSize())
    DisposableEffect(open) { onDispose { open.close() } }
    val list = rememberLazyListState(doc.page.coerceIn(0, (open.ratios.size - 1).coerceAtLeast(0)))
    val zoom = rememberZoom(if (embedded) 1f else doc.zoom)
    if (!embedded) {
        LaunchedEffect(list, zoom) {
            snapshotFlow { list.firstVisibleItemIndex to zoom.scale }.debounce(500).collect { (page, scale) -> Docs.saveView(doc.id, page, scale) }
        }
    }
    BoxWithConstraints(Modifier.fillMaxSize().zoomGestures(zoom, list = true).testTag("docs.viewer.pdf")) {
        // Pages render at 1.5× the screen width, sharp enough up to about 2× zoom.
        val px = with(LocalDensity.current) { (maxWidth.toPx() * 1.5f).toInt().coerceAtMost(2400) }
        LazyColumn(
            Modifier.fillMaxSize().zoomed(zoom),
            list,
            PaddingValues(Space.sm),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            itemsIndexed(open.ratios) { i, ratio ->
                val bitmap by produceState<Bitmap?>(null, i, px) { value = runCatching { open.page(i, px) }.getOrNull() }
                Box(
                    Modifier.fillMaxWidth().aspectRatio(ratio).clip(RoundedCornerShape(4.dp)).background(AppTheme.colors.card).testTag("docs.viewer.page.$i"),
                ) {
                    bitmap?.let { Image(remember(it) { it.asImageBitmap() }, null, Modifier.fillMaxSize(), contentScale = ContentScale.FillBounds) }
                }
            }
        }
        if (!embedded) Recenter(zoom)
    }
}
