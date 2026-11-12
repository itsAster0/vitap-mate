package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.Dp
import com.composables.icons.lucide.Plus
import vitapmate.ui.AppTheme
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowDown
import com.composables.icons.lucide.ArrowUp
import com.composables.icons.lucide.CircleMinus
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import vitapmate.data.NavTabs
import vitapmate.data.Settings
import vitapmate.observability.Log

/** Settings → Tabs in the bar: pick up to 5 and order them (core's [NavTabs] rules apply). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun TabsScreen() {
    val tabs = navTabs
    val full = tabs.size >= NavTabs.MAX
    fun save(next: List<String>) {
        Settings.setNavTabs(next)
        Log.info("ui.settings", "tabs ${Settings.navTabs.joinToString(",")}")
    }
    Page("Tabs in the bar", "settings.tabs", actions = {
        TextButton({ save(NavTabs.DEFAULT) }, Modifier.testTag("settings.tabs.reset")) { Text("Reset") }
    }) { padding ->
        LazyColumn(contentPadding = padding) {
            item("preview") {
                Column(
                    Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).background(AppTheme.colors.card, MaterialTheme.shapes.extraLarge).padding(16.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text("Preview", Modifier.fillMaxWidth(), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    BarPreview(iconSize = 22.dp, gap = 22.dp)
                }
            }
            item {
                ListGroup("In the bar (${tabs.size} of ${NavTabs.MAX})", footer = "More always stays: it holds everything else.") {
                    tabs.forEachIndexed { i, id ->
                        val f = featureById(id) ?: return@forEachIndexed
                        GroupItem(f.label, "settings.tab.$id", f.icon, trailing = {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                // Up and down as one joined pair.
                                Row(horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                                    FilledTonalIconButton(
                                        { save(tabs.toMutableList().apply { add(i - 1, removeAt(i)) }) },
                                        Modifier.size(40.dp).testTag("settings.tab.$id.up"),
                                        enabled = i > 0,
                                        shape = RoundedCornerShape(topStart = 20.dp, bottomStart = 20.dp, topEnd = 6.dp, bottomEnd = 6.dp),
                                    ) {
                                        Icon(Lucide.ArrowUp, "Move ${f.label} up", Modifier.size(20.dp))
                                    }
                                    FilledTonalIconButton(
                                        { save(tabs.toMutableList().apply { add(i + 1, removeAt(i)) }) },
                                        Modifier.size(40.dp).testTag("settings.tab.$id.down"),
                                        enabled = i < tabs.lastIndex,
                                        shape = RoundedCornerShape(topStart = 6.dp, bottomStart = 6.dp, topEnd = 20.dp, bottomEnd = 20.dp),
                                    ) {
                                        Icon(Lucide.ArrowDown, "Move ${f.label} down", Modifier.size(20.dp))
                                    }
                                }
                                if (id == NavTabs.MORE) {
                                    IconButton({}, Modifier.testTag("settings.tab.$id.locked"), enabled = false) { Icon(Lucide.Lock, "More can't be removed") }
                                } else {
                                    IconButton(
                                        { save(tabs - id) },
                                        Modifier.testTag("settings.tab.$id.remove"),
                                        colors = IconButtonDefaults.iconButtonColors(contentColor = AppTheme.tones.danger.base),
                                    ) { Icon(Lucide.CircleMinus, "Remove ${f.label}") }
                                }
                            }
                        })
                    }
                }
            }
            item {
                ListGroup("Not in the bar", Modifier.padding(top = 8.dp), footer = if (full) "The bar is full; remove a tab to add another." else null) {
                    tabChoices.filter { it.id !in tabs }.forEach { f ->
                        GroupItem(f.label, "settings.add.${f.id}", f.icon, enabled = !full, trailing = {
                            FilledTonalButton(onClick = { save(NavTabs.add(tabs, f.id)) }, modifier = Modifier.testTag("settings.add.${f.id}.add"), enabled = !full, shapes = ButtonDefaults.shapes()) {
                                Icon(Lucide.Plus, "Add ${f.label}", Modifier.size(18.dp))
                                Spacer(Modifier.size(6.dp))
                                Text("Add")
                            }
                        })
                    }
                }
            }
        }
    }
}

/** A small copy of the bar: the icons of the tabs in it, in order (Appearance's row, this page's preview). */
@Composable
fun BarPreview(modifier: Modifier = Modifier, iconSize: Dp = 15.dp, gap: Dp = 12.dp) {
    Row(
        modifier.background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(iconSize - 1.dp)).padding(horizontal = iconSize * 2 / 3, vertical = iconSize / 3),
        horizontalArrangement = Arrangement.spacedBy(gap),
    ) {
        navTabs.forEach { id -> featureById(id)?.let { Icon(it.icon, null, Modifier.size(iconSize), tint = MaterialTheme.colorScheme.onSurface) } }
    }
}
