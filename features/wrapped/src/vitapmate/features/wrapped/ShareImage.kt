package vitapmate.features.wrapped

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.core.content.FileProvider
import java.io.File
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The share image's size: a phone story. */
private const val WIDTH = 1080
private const val HEIGHT = 1920

/** The Play listing, under every shared image and in the message beside it. */
internal const val PLAY_LINK = "https://play.google.com/store/apps/details?id=com.vitap_pal.app"

/** A share image being drawn: [slide] as [context] says, done when [image] completes. */
internal class Capture(val slide: Slide, val context: SlideContext) {
    val image = CompletableDeferred<ImageBitmap>()
}

/**
 * Draws [capture]'s slide off screen into a graphics layer at 1080 × 1920 (the design frame at
 * density 3, so it matches the screen), without the progress bar, buttons or switch, and with
 * the first name and the footer. It takes no space and never shows: the layer is recorded, not
 * drawn.
 */
@Composable
internal fun CaptureBox(capture: Capture) {
    val layer = rememberGraphicsLayer()
    val recorded = remember(capture) { CompletableDeferred<Unit>() }
    Box(
        Modifier
            .layout { measurable, _ ->
                val placeable = measurable.measure(Constraints.fixed(WIDTH, HEIGHT))
                layout(0, 0) { placeable.place(0, 0) }
            }
            .drawWithContent {
                layer.record { this@drawWithContent.drawContent() }
                recorded.complete(Unit)
            },
    ) {
        CompositionLocalProvider(LocalDensity provides Density(WIDTH / DESIGN_WIDTH, 1f)) {
            SlideCanvas(capture.slide, capture.context)
        }
    }
    LaunchedEffect(capture) {
        recorded.await()
        capture.image.complete(layer.toImageBitmap())
    }
}

/** Saves [image] as `share/wrapped-<semester>-<slide>.png` in the cache and opens the share sheet (the `.logs` FileProvider). */
internal suspend fun shareImage(context: Context, image: ImageBitmap, semester: String, slide: String) {
    val safe = semester.replace(Regex("[^A-Za-z0-9_-]"), "_")
    val file = withContext(Dispatchers.IO) {
        File(context.cacheDir, "share").apply { mkdirs() }.resolve("wrapped-$safe-$slide.png").also { f ->
            f.outputStream().use { image.asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
        }
    }
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.logs", file)
    val send = Intent(Intent.ACTION_SEND)
        .setType("image/png")
        .putExtra(Intent.EXTRA_STREAM, uri)
        .putExtra(Intent.EXTRA_TEXT, PLAY_LINK)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    // The clip carries the read grant to the share sheet too, so it can preview the image.
    send.clipData = ClipData.newRawUri(file.name, uri)
    context.startActivity(Intent.createChooser(send, "Share"))
}
