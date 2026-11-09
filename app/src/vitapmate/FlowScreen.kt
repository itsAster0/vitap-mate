package vitapmate

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Share2
import kotlinx.coroutines.launch
import vitapmate.observability.Inspector
import vitapmate.observability.LogFilter
import vitapmate.observability.LogLine
import vitapmate.observability.Log
import vitapmate.observability.fallbackOf
import vitapmate.observability.routeOf
import vitapmate.ui.AppTheme
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/** One flow: every Kotlin and Rust line it logged, in order, coloured by level. */
@Composable
fun FlowScreen(flow: String) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var min by rememberSaveable { mutableStateOf("trace") }
    // Tailed, so a running flow's lines keep arriving.
    val flows by remember { Inspector.flows() }.collectAsState(emptyList())
    val summary = flows.firstOrNull { it.id == flow }
    val lines = summary?.lines.orEmpty()
    LaunchedEffect(flow) { Log.info("ui.inspector", "open flow", flow) }
    val shown = lines.filter(LogFilter(min)::matches)
    val name = summary?.name ?: flow
    Page(name, "inspector.flow", actions = {
        IconButton({ copy(context, flow, shown) }, Modifier.testTag("inspector.copy")) { Icon(Lucide.Copy, "Copy all lines") }
        IconButton({ scope.launch { shareLogs(context) } }, Modifier.testTag("inspector.share")) { Icon(Lucide.Share2, "Share the log file") }
    }) { padding ->
        SelectionContainer {
            LazyColumn(contentPadding = padding) {
                item {
                    Column(Modifier.padding(horizontal = 16.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Flow $flow · ${lines.size} lines${summary?.ms?.let { " · $it ms" }.orEmpty()}${summary?.outcome?.let { " · $it" }.orEmpty()}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        RouteLine(routeOf(lines), fallbackOf(lines))
                        LevelChips(min, "inspector.flow.level") { min = it }
                    }
                }
                itemsIndexed(shown) { i, line -> LineRow(line, Modifier.testTag("inspector.line.$i")) }
            }
        }
    }
}

@Composable
private fun LineRow(line: LogLine, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    val route = line.msg == "route used" || line.msg.startsWith("route failed")
    val color = when (line.level.lowercase()) {
        "error" -> scheme.error
        "warn" -> AppTheme.palette.warning.base
        "debug" -> scheme.onSurfaceVariant
        // The route switch's lines ("route used") stand out in the timeline.
        else -> if (route) scheme.primary else scheme.onSurface
    }
    val mono = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace, fontSize = 12.sp, lineHeight = 16.sp)
    Column(modifier.padding(horizontal = 16.dp, vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(clock(line.t), style = mono, color = scheme.onSurfaceVariant)
            Text(line.level.uppercase(), style = mono, color = color)
            Surface(shape = RoundedCornerShape(6.dp), color = scheme.surfaceContainerHighest) {
                Text(line.src, Modifier.padding(horizontal = 6.dp), style = mono, color = scheme.onSurfaceVariant, maxLines = 1)
            }
        }
        Text(line.msg, style = mono, color = color)
        if (line.fields.isNotEmpty()) {
            Text(
                line.fields.entries.joinToString("  ") { (k, v) -> "$k=$v" },
                Modifier.horizontalScroll(rememberScrollState()),
                style = mono,
                color = scheme.onSurfaceVariant,
                softWrap = false,
            )
        }
    }
}

private fun copy(context: Context, flow: String, lines: List<LogLine>) {
    val text = lines.joinToString("\n") { l ->
        "${clock(l.t)} ${l.level.uppercase()} ${l.src} ${l.msg}" + if (l.fields.isEmpty()) "" else " " + l.fields.entries.joinToString(" ") { (k, v) -> "$k=$v" }
    }
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("flow $flow", text))
    showToast("Copied", "${lines.size} lines of flow $flow", tone = ToastTone.Success)
}
