package vitapmate.features.docs

import vitapmate.ui.rememberHaptics
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.FileX
import com.composables.icons.lucide.LocateFixed
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import com.composables.icons.lucide.X
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.Doc
import vitapmate.data.DocKind
import vitapmate.data.Docs
import vitapmate.data.readSpreadsheet
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Radii
import vitapmate.ui.Space

/** The full-screen viewer (Feature.detail): logs `docs.view` and marks the document opened. */
@Composable
internal fun DocViewer(id: String) {
    val doc = Docs.byId(id) ?: return EmptyView(Lucide.FileX, "Document not found", Modifier.testTag("docs.viewer.missing"))
    LaunchedEffect(id) {
        flow("docs.view") { f ->
            Log.info(SRC, "view", f, mapOf("kind" to doc.kind.name, "bytes" to "${doc.size}", "menu" to "${doc.asMessMenu}"))
            Docs.opened(id)
        }
    }
    LaunchedEffect(doc) { StateRegistry.report("ui.docs.viewer", mapOf("kind" to doc.kind.name, "menu" to doc.asMessMenu, "page" to doc.page, "zoom" to doc.zoom)) }
    Box(Modifier.fillMaxSize().padding(LocalContentPadding.current).testTag("docs.viewer")) { DocContent(doc, embedded = false) }
}

/** [doc] drawn by its kind; [embedded] is Docs' "Continue viewing" card (no saved position, the menu's next meal only). */
@Composable
internal fun DocContent(doc: Doc, embedded: Boolean) {
    var failed by remember(doc.uri) { mutableStateOf<Throwable?>(null) }
    val fail: (Throwable) -> Unit = { e ->
        if (e !is CancellationException) {
            Log.warn(SRC, "unreadable: ${e::class.simpleName}", fields = mapOf("kind" to doc.kind.name))
            failed = e
        }
    }
    // The file lives in the download folder, where a file manager can delete it.
    val there by produceState<Boolean?>(null, doc.uri) { value = withContext(Dispatchers.IO) { Docs.exists(doc) } }
    when (there) {
        null -> return Loading()
        false -> return EmptyView(Lucide.FileX, "File missing")
        true -> Unit
    }
    failed?.let { return EmptyView(Lucide.FileX, "Couldn't read this file", message = (it as? vitapmate.data.MessMenuFormatException)?.message) }
    when {
        doc.kind == DocKind.Spreadsheet && doc.asMessMenu -> MessMenuView(doc, compact = embedded, onFailed = fail)
        doc.kind == DocKind.Pdf -> PdfView(doc, embedded, fail)
        doc.kind == DocKind.Image -> ImageView(doc, embedded, fail)
        doc.kind == DocKind.Spreadsheet -> SheetView(doc, embedded, fail)
        doc.kind == DocKind.Text -> TextView(doc, embedded, fail)
        else -> {
            val context = LocalContext.current
            EmptyView(Lucide.ExternalLink, doc.name, action = { OutlinedButton({ openWith(context, doc) }) { Text("Open with…") } })
        }
    }
}

@Composable
private fun ImageView(doc: Doc, embedded: Boolean, onFailed: (Throwable) -> Unit) {
    val bitmap by produceState<android.graphics.Bitmap?>(null, doc.uri) {
        value = runCatching { withContext(Dispatchers.IO) { decodeImage(Docs.readBytes(doc)) } }.onFailure(onFailed).getOrNull()
    }
    val zoom = rememberZoom()
    Box(Modifier.fillMaxSize().zoomGestures(zoom, list = false).testTag("docs.viewer.image"), Alignment.Center) {
        bitmap?.let { Image(remember(it) { it.asImageBitmap() }, null, Modifier.fillMaxSize().zoomed(zoom)) } ?: Loading()
        if (!embedded) Recenter(zoom)
    }
}

/** Decoded at most 4096 px on a side, so a camera photo fits a texture. */
private fun decodeImage(bytes: ByteArray): android.graphics.Bitmap {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
    var sample = 1
    while (maxOf(bounds.outWidth, bounds.outHeight) / sample > 4096) sample *= 2
    return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = sample }) ?: error("not an image")
}

@Composable
private fun TextView(doc: Doc, embedded: Boolean, onFailed: (Throwable) -> Unit) {
    val text by produceState<String?>(null, doc.uri) {
        value = runCatching { withContext(Dispatchers.IO) { Docs.readBytes(doc).decodeToString().take(1_000_000) } }.onFailure(onFailed).getOrNull()
    }
    var query by rememberSaveable { mutableStateOf("") }
    val body = text ?: return Loading()
    val needle = query.trim().lowercase()
    val lines = remember(body, needle) {
        if (needle.isEmpty()) body.split('\n') else body.split('\n').filter { needle in it.lowercase() }.take(100)
    }
    val highlight = MaterialTheme.colorScheme.primary.copy(alpha = 0.24f)
    Column(Modifier.fillMaxSize().testTag("docs.viewer.text")) {
        if (!embedded) ViewerSearch(query) { query = it }
        SelectionContainer {
            LazyColumn(Modifier.fillMaxSize().padding(horizontal = Space.sm).background(AppTheme.colors.card, RoundedCornerShape(Radii.md)).padding(Space.md)) {
                if (lines.isEmpty()) item { Text("No matches found.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
                items(lines) { line ->
                    Text(marked(line, needle, highlight), fontFamily = FontFamily.Monospace, fontSize = 13.sp, lineHeight = 19.sp)
                }
            }
        }
    }
}

/** [text] with each [needle] (lowercase) on [color], bold. */
private fun marked(text: String, needle: String, color: Color): AnnotatedString = buildAnnotatedString {
    append(text)
    if (needle.isEmpty()) return@buildAnnotatedString
    var at = text.lowercase().indexOf(needle)
    while (at >= 0) {
        addStyle(SpanStyle(background = color, fontWeight = FontWeight.ExtraBold), at, at + needle.length)
        at = text.lowercase().indexOf(needle, at + needle.length)
    }
}

@Composable
private fun SheetView(doc: Doc, embedded: Boolean, onFailed: (Throwable) -> Unit) {
    val tables by produceState<Map<String, List<List<String>>>?>(null, doc.uri) {
        value = runCatching { withContext(Dispatchers.IO) { Docs.withFile(doc, ::readSpreadsheet) } }.onFailure(onFailed).getOrNull()
    }
    var sheet by rememberSaveable { mutableIntStateOf(0) }
    var query by rememberSaveable { mutableStateOf("") }
    val data = tables ?: return Loading()
    if (data.isEmpty()) return EmptyView(Lucide.FileX, "No readable sheets")
    val names = data.keys.toList()
    val picked = sheet.coerceIn(0, names.lastIndex)
    val needle = query.trim().lowercase()
    val rows = remember(data, picked, needle) {
        data.getValue(names[picked]).filter { row -> row.any { it.isNotEmpty() } && (needle.isEmpty() || row.any { needle in it.lowercase() }) }
    }
    val highlight = MaterialTheme.colorScheme.primary.copy(alpha = 0.24f)
    val border = AppTheme.colors.border
    Column(Modifier.fillMaxSize().testTag("docs.viewer.sheet")) {
        if (!embedded) ViewerSearch(query) { query = it }
        if (names.size > 1) {
            LazyRow(contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = Space.sm), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                itemsIndexed(names) { i, name -> val haptics = rememberHaptics(); FilterChip(i == picked, { haptics.tick(); sheet = i }, { Text(name) }, Modifier.testTag("docs.viewer.sheet.$i")) }
            }
        }
        val tableWidth = CELL * (rows.maxOfOrNull { it.size } ?: 0)
        Box(Modifier.fillMaxSize().horizontalScroll(rememberScrollState())) {
            LazyColumn(Modifier.width(tableWidth + Space.lg).padding(Space.sm)) {
                items(rows) { row ->
                    Row {
                        row.forEach { cell ->
                            Text(
                                marked(cell, needle, highlight),
                                Modifier.width(CELL).border(0.5.dp, border).padding(6.dp),
                                style = MaterialTheme.typography.bodySmall,
                                maxLines = 6,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                }
            }
        }
    }
}

private val CELL = 132.dp

/** Search in the document: lines (text) or rows (sheet) that contain it. */
@Composable
private fun ViewerSearch(query: String, onChange: (String) -> Unit) {
    TextField(
        query,
        onChange,
        Modifier.fillMaxWidth().padding(Space.sm).testTag("docs.viewer.search"),
        placeholder = { Text("Search document") },
        leadingIcon = { Icon(Lucide.Search, null) },
        trailingIcon = { if (query.isNotEmpty()) IconButton({ onChange("") }) { Icon(Lucide.X, "Clear search") } },
        singleLine = true,
        shape = RoundedCornerShape(Radii.pill),
        colors = TextFieldDefaults.colors(focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent),
    )
}

/** Back to 1× (Flutter's re-centre button). */
@Composable
internal fun androidx.compose.foundation.layout.BoxScope.Recenter(zoom: ZoomState) {
    if (zoom.scale > 1.01f) {
        androidx.compose.material3.FilledTonalIconButton(zoom::reset, Modifier.align(Alignment.BottomEnd).padding(Space.md).testTag("docs.viewer.recenter")) {
            Icon(Lucide.LocateFixed, "Re-centre")
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Loading() = Box(Modifier.fillMaxSize(), Alignment.Center) { LoadingIndicator() }

internal const val SRC = "ui.docs"
