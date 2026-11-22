package vitapmate.features.docs

import android.net.Uri
import androidx.activity.compose.BackHandler
import com.composables.icons.lucide.Trash2
import vitapmate.ui.AppDialog
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.itemsIndexed
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.File
import com.composables.icons.lucide.FilePlus
import com.composables.icons.lucide.Files
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import kotlin.coroutines.cancellation.CancellationException
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
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.LocalOpenDetail
import vitapmate.ui.Space
import vitapmate.ui.showToast

/** What the picker offers: the kinds Docs can show (Flutter's `docExtensions`). */
private val pickable = arrayOf(
    "application/pdf", "image/*", "text/*", "application/json", "application/xml",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "application/vnd.ms-excel.sheet.macroEnabled.12",
    "application/vnd.ms-excel", "application/vnd.oasis.opendocument.spreadsheet",
)

/**
 * Docs (the Flutter app's `docs_page.dart`): saved files in a grid, searchable, sorted and
 * filtered by kind, the last opened one in a "Continue viewing" card; tap opens the viewer, long
 * press starts selecting (share, delete, or the actions for one).
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
    var kind by rememberSaveable { mutableStateOf<DocKind?>(null) }
    // Ids picked by a long press; non-empty is selection mode.
    var selected by remember { mutableStateOf(emptySet<String>()) }
    var confirmDelete by remember { mutableStateOf(false) }
    val all = Docs.docs
    val sort = Docs.sort
    val kinds = remember(all) { all.map { it.kind }.distinct().sorted() }
    val shown = remember(all, sort, query, kind) { queryDocs(all, sort, query, kind) }
    val searching = query.isNotBlank() || kind != null
    val picked = remember(all, selected) { all.filter { it.id in selected } }
    val selecting = picked.isNotEmpty()
    BackHandler(selecting) { selected = emptySet() }
    val toggle = { doc: Doc -> selected = if (doc.id in selected) selected - doc.id else selected + doc.id }

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

    val recent = remember(shown) { shown.filter { it.kind != DocKind.File && it.lastOpenedAt != null }.maxByOrNull { it.lastOpenedAt ?: 0L } }
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
        if (kinds.size > 1) item(span = { GridItemSpan(maxLineSpan) }, key = "kinds") { KindChips(kinds, kind) { kind = it } }
        item(span = { GridItemSpan(maxLineSpan) }, key = "count") {
            if (selecting) return@item SelectionBar(
                picked.size,
                allShown = shown.all { it.id in selected },
                onClear = { selected = emptySet() },
                onAll = { selected = if (shown.all { it.id in selected }) emptySet() else selected + shown.map { it.id } },
                onShare = {
                    scope.logAction("docs.share", SRC, "share ${picked.size}")
                    shareAll(context, picked)
                },
                onDelete = { confirmDelete = true },
                onMore = picked.singleOrNull()?.let { { actions = it.id; selected = emptySet() } },
            )
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
        itemsIndexed(shown, key = { _, d -> d.id }) { i, doc ->
            DocCard(doc, i, Modifier.animateItem(), doc.id in selected, onOpen = { if (selecting) toggle(doc) else open(doc) }, onLongPress = { toggle(doc) })
        }
    }
    actions?.let(Docs::byId)?.let { DocActions(it, scope) { actions = null } }
    if (confirmDelete) {
        val gone = picked
        AppDialog(
            { confirmDelete = false },
            if (gone.size == 1) "Delete \"${gone.single().name}\"?" else "Delete ${gone.size} documents?",
            "Delete",
            {
                confirmDelete = false
                selected = emptySet()
                scope.launch {
                    try {
                        flow("docs.delete") { f ->
                            gone.forEach { Docs.remove(it.id) }
                            Log.info(SRC, "delete", f, mapOf("deleted" to "${gone.size}", "count" to "${Docs.docs.size}"))
                        }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Throwable) {
                        Log.warn(SRC, "delete failed: ${e::class.simpleName}")
                        showToast("Something went wrong", "Please try again.")
                    }
                }
            },
            icon = Lucide.Trash2,
            danger = true,
            confirmTag = "docs.delete.confirm",
        ) { Text("Their files are removed from the download folder too.") }
    }
}
