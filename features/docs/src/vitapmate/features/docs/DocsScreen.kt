package vitapmate.features.docs

import vitapmate.ui.rememberHaptics
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.toShape
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.itemsIndexed
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowUpDown
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.File
import com.composables.icons.lucide.FileCode
import com.composables.icons.lucide.FilePlus
import com.composables.icons.lucide.FileSpreadsheet
import com.composables.icons.lucide.FileText
import com.composables.icons.lucide.Files
import com.composables.icons.lucide.Image
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import com.composables.icons.lucide.Utensils
import com.composables.icons.lucide.X
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.data.Doc
import vitapmate.data.DocKind
import vitapmate.data.DocSort
import vitapmate.data.Docs
import vitapmate.data.MessMenuFormatException
import vitapmate.data.queryDocs
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.LocalOpenDetail
import vitapmate.ui.Radii
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.ToneBadge
import vitapmate.ui.neutralTone
import vitapmate.ui.showToast

/** What the picker offers: the kinds Docs can show (Flutter's `docExtensions`). */
private val pickable = arrayOf(
    "application/pdf", "image/*", "text/*", "application/json", "application/xml",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "application/vnd.ms-excel.sheet.macroEnabled.12",
    "application/vnd.ms-excel", "application/vnd.oasis.opendocument.spreadsheet",
)

/**
 * Docs (the Flutter app's `docs_page.dart`): saved files in a grid, searchable and sorted, the
 * last opened one in a "Continue viewing" card; tap opens the viewer, long press the actions.
 * Flows `docs.open/import/view/rename/delete/share`, log src `ui.docs` (counts and kinds only).
 */
@Composable
internal fun DocsScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val opener = LocalOpenDetail.current
    var query by rememberSaveable { mutableStateOf("") }
    var actions by remember { mutableStateOf<String?>(null) }
    var asMenu by rememberSaveable { mutableStateOf(false) }
    val all = Docs.docs
    val sort = Docs.sort
    val shown = remember(all, sort, query) { queryDocs(all, sort, query) }
    val searching = query.isNotBlank()

    // What the student changed in the folder outside the app, caught up each time Docs opens.
    LaunchedEffect(Unit) { Docs.refresh() }
    LaunchedEffect(all.size, shown.size, sort) {
        StateRegistry.report(SRC, mapOf("docs" to all.size, "shown" to shown.size, "sort" to sort.name, "bytes" to all.sumOf { it.size }, "kinds" to all.groupingBy { it.kind.name }.eachCount()))
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
        val menu = asMenu
        scope.launch {
            try {
                flow("docs.import") { f ->
                    if (uri == null) return@flow Log.info(SRC, "import cancelled", f)
                    val doc = Docs.import(context, uri, f)
                    Log.info(SRC, "imported", f, mapOf("kind" to doc.kind.name, "bytes" to "${doc.size}", "count" to "${Docs.docs.size}"))
                    if (menu) {
                        if (doc.kind != DocKind.Spreadsheet) return@flow showToast("Added as a document", "Only the .xlsx mess menu can be shown as a menu.")
                        loadMessMenu(doc)
                        Docs.setMessMenu(doc.id, true)
                    }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: MessMenuFormatException) {
                showToast("Not a mess menu", "${e.message} It stays a spreadsheet.")
            } catch (e: Throwable) {
                Log.warn(SRC, "import failed: ${e::class.simpleName}")
                showToast("Couldn't add that file", "Please try another file.")
            }
        }
    }
    fun import(menu: Boolean) {
        asMenu = menu
        picker.launch(pickable)
    }
    fun open(doc: Doc) {
        scope.launch {
            flow("docs.open") { f -> Log.info(SRC, "open", f, mapOf("kind" to doc.kind.name, "menu" to "${doc.asMessMenu}")) }
            if (doc.kind == DocKind.File) {
                openWith(context, doc)
                Docs.opened(doc.id)
            } else {
                opener.open(doc.id, doc.name)
            }
        }
    }

    if (all.isEmpty()) {
        return Box(Modifier.fillMaxSize().padding(LocalContentPadding.current), Alignment.Center) {
            EmptyView(Lucide.Files, "Your personal board", Modifier.testTag("docs.empty"), action = {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Button({ import(false) }, Modifier.testTag("docs.import")) {
                        Icon(Lucide.FilePlus, null, Modifier.size(16.dp))
                        Spacer(Modifier.width(Space.sm))
                        Text("Import a document")
                    }
                    TextButton({ import(true) }, Modifier.testTag("docs.import_menu")) { Text("or add the mess menu (.xlsx)") }
                }
            })
        }
    }

    val recent = remember(shown) { shown.filter { it.kind != DocKind.File && it.lastOpenedAt != null }.maxByOrNull { it.lastOpenedAt!! } }
    val grid = rememberLazyGridState()
    val bottom = LocalContentPadding.current.calculateBottomPadding()
    LazyVerticalGrid(
        GridCells.Adaptive(150.dp),
        Modifier.fillMaxSize().testTag("docs.grid"),
        grid,
        PaddingValues(start = Space.sm, end = Space.sm, bottom = bottom + Space.lg),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item(span = { GridItemSpan(maxLineSpan) }, key = "search") { SearchRow(query, { query = it }, sort) }
        item(span = { GridItemSpan(maxLineSpan) }, key = "count") {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f).padding(start = Space.xs)) {
                    Text(
                        if (searching) "${shown.size} of ${all.size} documents" else "${all.size} document${if (all.size == 1) "" else "s"}",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        if (sort == DocSort.RecentlyOpened || sort == DocSort.RecentlyAdded) "${sort.label} first" else sort.label,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Button({ import(false) }, Modifier.testTag("docs.add")) {
                    Icon(Lucide.FilePlus, null, Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Add")
                }
            }
        }
        if (recent != null && !searching) {
            item(span = { GridItemSpan(maxLineSpan) }, key = "recent-${recent.id}") { Recent(recent) { open(recent) } }
        }
        if (shown.isEmpty()) {
            item(span = { GridItemSpan(maxLineSpan) }, key = "none") {
                EmptyView(Lucide.Search, "No documents found", action = { TextButton({ query = "" }) { Text("Clear search") } })
            }
        }
        itemsIndexed(shown, key = { _, it -> it.id }) { i, doc ->
            DocCard(doc, i, Modifier.animateItem(), onOpen = { open(doc) }, onLongPress = { actions = doc.id })
        }
    }
    actions?.let(Docs::byId)?.let { DocActions(it, scope) { actions = null } }
}

@Composable
private fun SearchRow(query: String, onQuery: (String) -> Unit, sort: DocSort) {
    var menu by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    Row(Modifier.padding(bottom = Space.xs), verticalAlignment = Alignment.CenterVertically) {
        TextField(
            query,
            onQuery,
            Modifier.weight(1f).testTag("docs.search"),
            placeholder = { Text("Search documents") },
            leadingIcon = { Icon(Lucide.Search, null, Modifier.size(18.dp)) },
            trailingIcon = { if (query.isNotEmpty()) IconButton({ onQuery("") }) { Icon(Lucide.X, "Clear search", Modifier.size(18.dp)) } },
            singleLine = true,
            shape = RoundedCornerShape(Radii.pill),
            colors = TextFieldDefaults.colors(focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent),
        )
        Box {
            IconButton({ menu = true }, Modifier.testTag("docs.sort")) { Icon(Lucide.ArrowUpDown, "Sort") }
            DropdownMenu(menu, { menu = false }, Modifier.testTag("docs.sort.menu")) {
                DocSort.entries.forEach { option ->
                    val picked = option == sort
                    DropdownMenuItem(
                        text = { Text(option.label, fontWeight = if (picked) FontWeight.SemiBold else null) },
                        onClick = {
                            menu = false
                            Docs.sort = option
                            scope.logAction("docs.sort", SRC, "sort ${option.name}")
                        },
                        leadingIcon = { Icon(if (picked) Lucide.Check else Lucide.ArrowUpDown, null, Modifier.size(18.dp)) },
                        modifier = Modifier.padding(horizontal = 6.dp).clip(RoundedCornerShape(14.dp))
                            .background(if (picked) MaterialTheme.colorScheme.primaryContainer else Color.Transparent)
                            .testTag("docs.sort.${option.name.lowercase()}"),
                        colors = if (picked) MenuDefaults.itemColors(
                            textColor = MaterialTheme.colorScheme.onPrimaryContainer,
                            leadingIconColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        ) else MenuDefaults.itemColors(),
                    )
                }
            }
        }
    }
}

/** Icon and tone for [doc]'s kind (the Flutter app's `visualFor`). */
@Composable
private fun visualOf(doc: Doc): Pair<Tone, ImageVector> {
    val t = AppTheme.tones
    if (doc.asMessMenu) return t.warning to Lucide.Utensils
    return when (doc.kind) {
        DocKind.Pdf -> t.danger to Lucide.FileText
        DocKind.Image -> t.accentTone to Lucide.Image
        DocKind.Spreadsheet -> t.success to Lucide.FileSpreadsheet
        DocKind.Text -> t.lab to Lucide.FileCode
        DocKind.File -> neutralTone to Lucide.File
    }
}

private fun kindLabel(doc: Doc) = if (doc.asMessMenu) "Menu" else when (doc.kind) {
    DocKind.Pdf -> "PDF"
    DocKind.Image -> "Image"
    DocKind.Spreadsheet -> "Sheet"
    DocKind.Text -> "Text"
    DocKind.File -> "File"
}

private fun openedLabel(ms: Long?): String {
    if (ms == null || ms == 0L) return "never opened"
    val mins = (System.currentTimeMillis() - ms) / 60_000
    return when {
        mins < 1 -> "opened just now"
        mins < 60 -> "opened ${mins}m ago"
        mins < 60 * 24 -> "opened ${mins / 60}h ago"
        else -> "opened ${mins / 60 / 24}d ago"
    }
}

@OptIn(ExperimentalFoundationApi::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun DocCard(doc: Doc, index: Int, modifier: Modifier, onOpen: () -> Unit, onLongPress: () -> Unit) {
    val (tone, icon) = visualOf(doc)
    val haptics = rememberHaptics()
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val press by androidx.compose.animation.core.animateFloatAsState(if (pressed) 0.96f else 1f, spring(Spring.DampingRatioMediumBouncy, Spring.StiffnessMedium))
    // Cards rise in one after another the first time the grid shows.
    val enter = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay((index.coerceAtMost(10) * 40).toLong())
        enter.animateTo(1f, spring(Spring.DampingRatioLowBouncy, Spring.StiffnessMediumLow))
    }
    Column(
        modifier.height(152.dp).fillMaxWidth()
            .graphicsLayer {
                alpha = enter.value.coerceIn(0f, 1f)
                translationY = (1 - enter.value) * 40f
                scaleX = press
                scaleY = press
            }
            .clip(RoundedCornerShape(26.dp))
            .background(AppTheme.colors.card)
            .combinedClickable(interaction, null, onClick = onOpen, onLongClick = {
                haptics.longPress()
                onLongPress()
            })
            .testTag("docs.card.${doc.id}"),
    ) {
        // A band in the kind's tone, so kinds tell apart across the grid; the icon in a shape
        // (its own for the mess menu and for images).
        Row(Modifier.fillMaxWidth().height(66.dp).background(tone.subtle).padding(horizontal = 14.dp), verticalAlignment = Alignment.CenterVertically) {
            val shape = when {
                doc.asMessMenu -> MaterialShapes.Cookie9Sided.toShape()
                doc.kind == DocKind.Image -> MaterialShapes.Cookie7Sided.toShape()
                else -> CircleShape
            }
            Box(Modifier.size(42.dp).background(MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), shape), Alignment.Center) {
                Icon(icon, null, Modifier.size(22.dp), tint = tone.base)
            }
            Spacer(Modifier.weight(1f))
            ToneBadge(kindLabel(doc), tone = Tone(tone.base, MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), tone.onSubtle))
        }
        Column(Modifier.weight(1f).padding(start = 14.dp, top = 10.dp, end = 14.dp, bottom = 14.dp)) {
            Text(doc.name, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(Space.xs))
            Text(openedLabel(doc.lastOpenedAt), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
        }
    }
}

/**
 * "Continue viewing": the last opened document in its kind's tone, live in a short window, with
 * Full screen under it; header and window are one stack.
 */
@Composable
private fun Recent(doc: Doc, onOpen: () -> Unit) {
    val (tone, icon) = visualOf(doc)
    val inner = RoundedCornerShape(6.dp)
    Column(
        Modifier.fillMaxWidth().padding(bottom = Space.xs).clip(MaterialTheme.shapes.extraLarge).testTag("docs.recent"),
        verticalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        Row(Modifier.fillMaxWidth().background(tone.subtle, inner).padding(horizontal = Space.lg, vertical = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(46.dp).background(MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), RoundedCornerShape(15.dp)), Alignment.Center) {
                Icon(icon, null, Modifier.size(22.dp), tint = tone.base)
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text("Continue viewing", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, color = tone.onSubtle)
                Text(doc.name, style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Column(Modifier.fillMaxWidth().background(AppTheme.colors.card, inner)) {
            Box(Modifier.fillMaxWidth().height(210.dp)) { DocContent(doc, embedded = true) }
            Button(
                onOpen,
                Modifier.fillMaxWidth().padding(start = Space.md, end = Space.md, bottom = Space.md).height(46.dp).testTag("docs.recent.open"),
                colors = ButtonDefaults.buttonColors(containerColor = tone.base, contentColor = AppTheme.colors.background),
            ) {
                Icon(Lucide.ExternalLink, null, Modifier.size(16.dp))
                Spacer(Modifier.width(Space.sm))
                Text("Full screen")
            }
        }
    }
}
