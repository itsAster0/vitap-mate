package vitapmate

import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Search
import com.composables.icons.lucide.Share2
import com.composables.icons.lucide.SquareActivity
import com.composables.icons.lucide.X
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import vitapmate.observability.Inspector
import vitapmate.observability.LogFilter
import vitapmate.ui.EmptyView

fun clock(t: Long): String = SimpleDateFormat("HH:mm:ss", Locale.ROOT).format(Date(t))

/** Settings → Inspector: every flow, newest first, searchable and filtered by level. */
@Composable
fun InspectorScreen() {
    val nav = LocalNav.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val all by remember { Inspector.flows() }.collectAsState(emptyList())
    var query by rememberSaveable { mutableStateOf("") }
    var min by rememberSaveable { mutableStateOf("trace") }
    // A flow shows when one of its lines matches (its name is on its start line).
    val shown = LogFilter(min, text = query.trim()).apply(all)
    Page("Inspector", INSPECTOR, actions = {
        IconButton({ scope.launch { shareLogs(context) } }, Modifier.testTag("inspector.share")) { Icon(Lucide.Share2, "Share the log file") }
    }) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                Column(Modifier.padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        query,
                        { query = it },
                        Modifier.fillMaxWidth().testTag("inspector.search"),
                        placeholder = { Text("Search flows by name or id") },
                        leadingIcon = { Icon(Lucide.Search, null) },
                        trailingIcon = if (query.isEmpty()) null else ({ IconButton({ query = "" }) { Icon(Lucide.X, "Clear") } }),
                        singleLine = true,
                        shape = CircleShape,
                    )
                    LevelChips(min, "inspector.level") { min = it }
                }
            }
            if (shown.isEmpty()) {
                item {
                    if (all.isEmpty()) EmptyView(Lucide.SquareActivity, "No flows yet", message = "Flows appear here as the app works.")
                    else EmptyView(Lucide.Search, "No flows match", message = "Try another word or a lower level.")
                }
            }
            items(shown, key = { it.id }) { f ->
                Box(Modifier.padding(horizontal = 20.dp)) {
                GroupItem(
                    f.name,
                    "inspector.flow.${f.id}",
                    supporting = "${clock(f.startT)} · ${f.id}${f.ms?.let { " · $it ms" }.orEmpty()}",
                    onClick = { nav.push(Route.FlowLog(f.id)) },
                    trailing = { OutcomeChip(f.outcome) },
                )
                }
            }
        }
    }
}

/** Minimum level: All, Info, Warnings, Errors. */
@Composable
fun LevelChips(min: String, tag: String, onPick: (String) -> Unit) {
    val haptics = rememberHaptics()
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        listOf("trace" to "All", "info" to "Info", "warn" to "Warnings", "error" to "Errors").forEach { (level, label) ->
            FilterChip(min == level, { haptics.tick(); onPick(level) }, { Text(label) }, Modifier.testTag("$tag.$level"))
        }
    }
}
