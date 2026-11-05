package vitapmate.features.vtop

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.background
import androidx.compose.material3.SearchBarDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.SheetValue
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import com.composables.icons.lucide.X
import vitapmate.ui.Space

/**
 * Every VTOP sidebar page, searchable, with recently opened pages first (the Flutter app's
 * `VtopPageSearchSheet`): words match title and section, title matches first; empty, it lists
 * Recent then every section. [pages] is null while VTOP's sidebar is read.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun VtopSearch(pages: List<VtopPage>?, recent: List<VtopPage>, onSelect: (VtopPage) -> Unit, onDismiss: () -> Unit) {
    var query by remember { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    val all = pages.orEmpty()
    val words = query.trim().lowercase().split(Regex("\\s+")).filter { it.isNotEmpty() }
    val matches = all.filter { p -> "${p.title} ${p.section}".lowercase().let { t -> words.all(t::contains) } }
        .sortedBy { if (words.isNotEmpty() && it.title.lowercase().contains(words.first())) 0 else 1 }
    val sections = all.groupBy { it.section }.let { g -> g.filterKeys { it.isNotEmpty() } + (g[""]?.let { mapOf("Other" to it) } ?: emptyMap()) }
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    // The sheet is its own window: its tags need their own resource-id switch.
    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.semantics { testTagsAsResourceId = true }.testTag("vtop.search"), sheet) {
        Column(Modifier.fillMaxHeight(0.88f)) {
            // M3's search field, in the sheet as the Flutter app has it.
            SearchBarDefaults.InputField(
                query = query,
                onQueryChange = { query = it },
                onSearch = {},
                expanded = false,
                onExpandedChange = {},
                modifier = Modifier.fillMaxWidth().padding(horizontal = Space.page).clip(SearchBarDefaults.inputFieldShape)
                    .background(MaterialTheme.colorScheme.surfaceContainerHigh).focusRequester(focus).testTag("vtop.search.field"),
                placeholder = { Text("Search VTOP pages") },
                leadingIcon = { Icon(Lucide.Search, null, Modifier.size(20.dp)) },
                trailingIcon = {
                    if (query.isNotEmpty()) IconButton({ query = "" }, Modifier.testTag("vtop.search.clear")) { Icon(Lucide.X, "Clear search", Modifier.size(18.dp)) }
                },
            )
            LazyColumn(Modifier.fillMaxWidth().testTag("vtop.search.list")) {
                fun group(title: String, list: List<VtopPage>, tag: String, showSection: Boolean, recentIcon: Boolean = false) {
                    item(key = "h:$tag") {
                        Text(
                            title.uppercase(),
                            Modifier.padding(start = Space.page + Space.xs, top = Space.lg, bottom = Space.xs),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.SemiBold,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    itemsIndexed(list, key = { _, it -> "$tag:${it.url}" }) { i, p ->
                        // Segmented: one rounded group, small corners between rows.
                        val big = 20.dp
                        val small = 4.dp
                        val shape = RoundedCornerShape(
                            topStart = if (i == 0) big else small, topEnd = if (i == 0) big else small,
                            bottomStart = if (i == list.lastIndex) big else small, bottomEnd = if (i == list.lastIndex) big else small,
                        )
                        ListItem(
                            headlineContent = { Text(p.title) },
                            supportingContent = if (showSection && p.section.isNotEmpty()) ({ Text(p.section) }) else null,
                            leadingContent = if (recentIcon) ({ Icon(Lucide.History, null, Modifier.size(18.dp)) }) else null,
                            trailingContent = { Icon(Lucide.ChevronRight, null, Modifier.size(18.dp)) },
                            colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
                            modifier = Modifier.padding(horizontal = Space.page, vertical = 1.dp).clip(shape)
                                .clickable { onSelect(p) }.testTag("vtop.search.page"),
                        )
                    }
                }
                when {
                    words.isNotEmpty() && matches.isEmpty() -> item { Message("No VTOP page matches \"${query.trim()}\".") }
                    words.isNotEmpty() -> group("${matches.size} pages", matches, "match", showSection = true)
                    else -> {
                        if (recent.isNotEmpty()) group("Recent", recent, "recent", showSection = true, recentIcon = true)
                        sections.forEach { (section, list) -> group(section, list, "s:$section", showSection = false) }
                        if (pages == null) item { Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) { LoadingIndicator() } }
                        else if (all.isEmpty()) item { Message("VTOP pages appear once VTOP has finished loading.") }
                    }
                }
            }
        }
    }
}

@Composable
private fun Message(text: String) = Text(
    text,
    Modifier.fillMaxWidth().padding(Space.xl).testTag("vtop.search.empty"),
    style = MaterialTheme.typography.bodyMedium,
    color = MaterialTheme.colorScheme.onSurfaceVariant,
    textAlign = TextAlign.Center,
)
