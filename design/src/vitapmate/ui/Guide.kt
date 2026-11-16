package vitapmate.ui

import android.graphics.BitmapFactory
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import vitapmate.data.Guides
import vitapmate.data.Problem
import vitapmate.data.ProblemKind
import vitapmate.observability.flow

/** A guide from the edge: its [value] once read (the kept copy first), or why it couldn't be, and [retry]. */
class LoadedGuide<T>(val value: T?, val problem: Problem?, val retry: () -> Unit)

/**
 * The guide at [path] ([Guides]) read with [parse]: the kept copy at once, then the edge's if it
 * changed. A problem shows only when there is nothing to show. Flow `guide.open`.
 */
@Composable
fun <T> rememberGuide(path: String, parse: (JSONObject) -> T): LoadedGuide<T> {
    val context = LocalContext.current.applicationContext
    var value by remember(path) { mutableStateOf<T?>(null) }
    var problem by remember(path) { mutableStateOf<Problem?>(null) }
    var attempt by remember(path) { mutableIntStateOf(0) }
    LaunchedEffect(path, attempt) {
        problem = null
        flow("guide.open") { f ->
            fun read(file: File): T {
                val json = JSONObject(file.readText())
                if (json.optInt("schema") != Guides.SCHEMA) throw UnsupportedOperationException("schema ${json.optInt("schema")}")
                return parse(json)
            }
            withContext(Dispatchers.IO) { Guides.cached(context, path)?.let { runCatching { read(it) }.getOrNull() } }?.let { value = it }
            runCatching { withContext(Dispatchers.IO) { read(Guides.refresh(context, path, f)) } }
                .onSuccess { value = it }
                .onFailure { e ->
                    if (value != null) return@onFailure
                    problem = if (e is UnsupportedOperationException) Problem(ProblemKind.Unexpected, "Update the App", "This page needs a newer version of the app.", f, retryable = false)
                    else Problem(ProblemKind.Offline, "Not Loaded", "Couldn't load this page. Check your connection and try again.", f, retryable = true)
                }
        }
    }
    return LoadedGuide(value, problem) { attempt++ }
}

/** The image at [path] next to a guide, once it's kept (fetched the first time); null until then or when offline. */
@Composable
fun rememberGuideImage(path: String): ImageBitmap? {
    val context = LocalContext.current.applicationContext
    var image by remember(path) { mutableStateOf<ImageBitmap?>(null) }
    LaunchedEffect(path) {
        fun decode(file: File) = BitmapFactory.decodeFile(file.path)?.asImageBitmap()
        image = withContext(Dispatchers.IO) { Guides.cached(context, path)?.let(::decode) }
        flow("guide.image") { f ->
            runCatching { withContext(Dispatchers.IO) { decode(Guides.refresh(context, path, f)) } }.getOrNull()?.let { image = it }
        }
    }
    return image
}
