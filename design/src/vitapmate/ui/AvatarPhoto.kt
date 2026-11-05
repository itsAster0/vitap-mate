package vitapmate.ui

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import java.io.File
import kotlin.math.min
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.Settings

/**
 * The avatar photo the student chose: a square, [SIZE] px WebP in the app's private files,
 * named by time so a new pick never shows a stale copy. [Settings.avatarPath] points at it.
 */
object AvatarPhoto {
    private const val SIZE = 256

    /** Crops [uri] (from the Photo Picker) to its centre square and saves it; its EXIF rotation is applied on API 28+. */
    suspend fun pick(context: Context, uri: Uri) = save(context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            ImageDecoder.decodeBitmap(ImageDecoder.createSource(context.contentResolver, uri)) { decoder, info, _ ->
                // Decode near the target size; the crop below finishes the job.
                val scale = min(info.size.width, info.size.height) / SIZE
                if (scale > 1) decoder.setTargetSize(info.size.width / scale, info.size.height / scale)
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            }
        } else {
            context.contentResolver.openInputStream(uri)?.use(BitmapFactory::decodeStream)
        }
    }

    /** Saves an image's bytes (VTOP's photo) the same way. */
    suspend fun use(context: Context, bytes: ByteArray) = save(context) { BitmapFactory.decodeByteArray(bytes, 0, bytes.size) }

    /** Back to the initial: forgets the photo and deletes the file. */
    fun remove() {
        Settings.avatarPath?.let { File(it).delete() }
        Settings.avatarPath = null
    }

    private suspend fun save(context: Context, decode: () -> Bitmap?) {
        val path = withContext(Dispatchers.IO) {
            val source = decode() ?: error("not an image")
            val side = min(source.width, source.height)
            val square = Bitmap.createBitmap(source, (source.width - side) / 2, (source.height - side) / 2, side, side)
            val scaled = Bitmap.createScaledBitmap(square, SIZE, SIZE, true)
            val file = File(context.filesDir, "avatar-${System.currentTimeMillis()}.webp")
            @Suppress("DEPRECATION") // WEBP_LOSSY needs API 30; WEBP is the same before it.
            val format = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) Bitmap.CompressFormat.WEBP_LOSSY else Bitmap.CompressFormat.WEBP
            file.outputStream().use { scaled.compress(format, 90, it) }
            file.path
        }
        Settings.avatarPath?.takeIf { it != path }?.let { File(it).delete() }
        Settings.avatarPath = path
    }
}

/** The saved avatar photo as an image, read off the main thread; null without one. */
@Composable
fun rememberAvatarBitmap(): State<ImageBitmap?> {
    val path = Settings.avatarPath
    return produceState<ImageBitmap?>(null, path) {
        value = path?.let { withContext(Dispatchers.IO) { BitmapFactory.decodeFile(it)?.asImageBitmap() } }
    }
}
