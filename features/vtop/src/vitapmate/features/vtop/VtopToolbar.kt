package vitapmate.features.vtop

import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.IconButton as M3IconButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowLeft
import com.composables.icons.lucide.EllipsisVertical
import com.composables.icons.lucide.House
import com.composables.icons.lucide.LockKeyhole
import com.composables.icons.lucide.LogIn
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RotateCw
import com.composables.icons.lucide.Search
import vitapmate.ui.Space

/** One of the view preferences in the menu: label, checked, toggle. */
internal data class VtopToggle(val tag: String, val label: String, val on: Boolean, val toggle: () -> Unit)

/**
 * The screen's own slim bar (the Flutter app's nested header): close, the page shown (VTOP's own
 * title) over the host, Search, Home, and the Flutter menu (Reload, view preferences). Pull down at the top of the page reloads too.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun VtopToolbar(
    title: String,
    enabled: Boolean,
    onClose: () -> Unit,
    onSearch: () -> Unit,
    onHome: () -> Unit,
    onReload: () -> Unit,
    toggles: List<VtopToggle>,
) {
    var menu by remember { mutableStateOf(false) }
    TopAppBar(
        title = {
            Column {
                Text(title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.testTag("vtop.title"))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Lucide.LockKeyhole, null, Modifier.size(12.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(
                        "vtop.vitap.ac.in",
                        Modifier.padding(start = Space.xs),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                    )
                }
            }
        },
        navigationIcon = { M3IconButton(onClick = onClose, modifier = Modifier.testTag("vtop.close"), shapes = IconButtonDefaults.shapes()) { Icon(Lucide.ArrowLeft, "Close") } },
        actions = {
            M3IconButton(onClick = onSearch, modifier = Modifier.testTag("vtop.search.open"), enabled = enabled, shapes = IconButtonDefaults.shapes()) { Icon(Lucide.Search, "Search VTOP pages", Modifier.size(20.dp)) }
            M3IconButton(onClick = onHome, modifier = Modifier.testTag("vtop.home"), enabled = enabled, shapes = IconButtonDefaults.shapes()) { Icon(Lucide.House, "VTOP Home", Modifier.size(20.dp)) }
            Box {
                M3IconButton(onClick = { menu = true }, modifier = Modifier.testTag("vtop.menu"), enabled = enabled, shapes = IconButtonDefaults.shapes()) { Icon(Lucide.EllipsisVertical, "More", Modifier.size(20.dp)) }
                // A popup window: its tags need their own resource-id switch.
                DropdownMenu(menu, { menu = false }, Modifier.semantics { testTagsAsResourceId = true }, shape = MaterialTheme.shapes.large) {
                    DropdownMenuItem(
                        { Text("Reload") },
                        { menu = false; onReload() },
                        leadingIcon = { Icon(Lucide.RotateCw, null, Modifier.size(20.dp)) },
                        modifier = Modifier.testTag("vtop.reload"),
                    )
                    HorizontalDivider(Modifier.padding(vertical = Space.xs))
                    val haptics = rememberHaptics()
                    for (t in toggles) {
                        DropdownMenuItem(
                            { Text(t.label) },
                            { haptics.toggle(!t.on); t.toggle() },
                            trailingIcon = { Switch(t.on, null, Modifier.padding(start = Space.md)) },
                            modifier = Modifier.testTag("vtop.menu.${t.tag}"),
                        )
                    }
                }
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
        modifier = Modifier.testTag("vtop.toolbar"),
    )
}
