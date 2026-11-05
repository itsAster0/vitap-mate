package vitapmate

import androidx.compose.foundation.layout.Row
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
import com.composables.icons.lucide.CirclePlus
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import vitapmate.data.NavTabs
import vitapmate.data.Settings
import vitapmate.observability.Log

/** Settings → Tabs in the bar: pick up to 5 and order them (core's [NavTabs] rules apply). */
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
            item {
                ListGroup("In the bar (${tabs.size} of ${NavTabs.MAX})", footer = "More always stays: it holds everything else.") {
                    tabs.forEachIndexed { i, id ->
                        val f = featureById(id) ?: return@forEachIndexed
                        GroupItem(f.label, "settings.tab.$id", f.icon, trailing = {
                            Row {
                                IconButton({ save(tabs.toMutableList().apply { add(i - 1, removeAt(i)) }) }, Modifier.testTag("settings.tab.$id.up"), enabled = i > 0) {
                                    Icon(Lucide.ArrowUp, "Move ${f.label} up")
                                }
                                IconButton({ save(tabs.toMutableList().apply { add(i + 1, removeAt(i)) }) }, Modifier.testTag("settings.tab.$id.down"), enabled = i < tabs.lastIndex) {
                                    Icon(Lucide.ArrowDown, "Move ${f.label} down")
                                }
                                if (id == NavTabs.MORE) {
                                    IconButton({}, Modifier.testTag("settings.tab.$id.locked"), enabled = false) { Icon(Lucide.Lock, "More can't be removed") }
                                } else {
                                    IconButton({ save(tabs - id) }, Modifier.testTag("settings.tab.$id.remove")) { Icon(Lucide.CircleMinus, "Remove ${f.label}") }
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
                            IconButton({ save(NavTabs.add(tabs, f.id)) }, Modifier.testTag("settings.add.${f.id}.add"), enabled = !full) {
                                Icon(Lucide.CirclePlus, "Add ${f.label}")
                            }
                        })
                    }
                }
            }
        }
    }
}
