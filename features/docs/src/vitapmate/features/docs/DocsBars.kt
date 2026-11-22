package vitapmate.features.docs

import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.FilterChip
import com.composables.icons.lucide.CheckCheck
import com.composables.icons.lucide.EllipsisVertical
import com.composables.icons.lucide.Share2
import com.composables.icons.lucide.SquareCheckBig
import com.composables.icons.lucide.Trash2
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowUpDown
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.File
import com.composables.icons.lucide.Image
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import com.composables.icons.lucide.X
import vitapmate.data.DocKind
import vitapmate.data.DocSort
import vitapmate.data.Docs
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.Radii
import vitapmate.ui.Space

/** Selection mode's bar in place of the count: how many, select all, share, delete, and the actions for one. */
@Composable
internal fun SelectionBar(count: Int, allShown: Boolean, onClear: () -> Unit, onAll: () -> Unit, onShare: () -> Unit, onDelete: () -> Unit, onMore: (() -> Unit)?) {
    val danger = AppTheme.tones.danger
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(Radii.pill)).background(MaterialTheme.colorScheme.secondaryContainer).padding(4.dp).testTag("docs.selection"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClear, Modifier.testTag("docs.selection.clear")) { Icon(Lucide.X, "Stop selecting") }
        Text("$count selected", Modifier.weight(1f), style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSecondaryContainer)
        IconButton(onAll, Modifier.testTag("docs.selection.all")) { Icon(if (allShown) Lucide.SquareCheckBig else Lucide.CheckCheck, if (allShown) "Select none" else "Select all") }
        IconButton(onShare, Modifier.testTag("docs.selection.share")) { Icon(Lucide.Share2, "Share") }
        IconButton(onDelete, Modifier.testTag("docs.selection.delete")) { Icon(Lucide.Trash2, "Delete", tint = danger.base) }
        if (onMore != null) IconButton(onMore, Modifier.testTag("docs.selection.more")) { Icon(Lucide.EllipsisVertical, "More") }
    }
}

/** One chip per kind in Docs, with All; picking one shows only that kind. */
@Composable
internal fun KindChips(kinds: List<DocKind>, picked: DocKind?, onPick: (DocKind?) -> Unit) {
    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(bottom = Space.xs).testTag("docs.kinds")) {
        item { FilterChip(picked == null, { onPick(null) }, { Text("All") }, Modifier.testTag("docs.kind.all")) }
        items(kinds) { k ->
            FilterChip(picked == k, { onPick(if (picked == k) null else k) }, { Text(kindsLabel(k)) }, Modifier.testTag("docs.kind.${k.name.lowercase()}"))
        }
    }
}

private fun kindsLabel(kind: DocKind) = when (kind) {
    DocKind.Pdf -> "PDFs"
    DocKind.Image -> "Images"
    DocKind.Spreadsheet -> "Sheets"
    DocKind.Text -> "Text"
    DocKind.File -> "Other"
}

@Composable
internal fun SearchRow(query: String, onQuery: (String) -> Unit, sort: DocSort) {
    var menu by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    Row(Modifier.padding(bottom = Space.xs), verticalAlignment = Alignment.CenterVertically) {
        TextField(
            vitapmate.ui.rememberSyncedTextFieldState(query, onQuery),
            Modifier.weight(1f).testTag("docs.search"),
            placeholder = { Text("Search documents") },
            leadingIcon = { Icon(Lucide.Search, null, Modifier.size(18.dp)) },
            trailingIcon = { if (query.isNotEmpty()) IconButton({ onQuery("") }) { Icon(Lucide.X, "Clear search", Modifier.size(18.dp)) } },
            lineLimits = TextFieldLineLimits.SingleLine,
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
