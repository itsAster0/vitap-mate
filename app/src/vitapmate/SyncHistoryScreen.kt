package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.data.Settings
import vitapmate.data.epochDayOf
import vitapmate.data.minuteOf
import vitapmate.observability.StepState
import vitapmate.ui.AppTheme
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour

/** Route id of Settings → Sync → Background sync. */
const val SYNC_HISTORY = "sync_history"

private const val BACKGROUND_SYNC = "Background sync"

/**
 * Background sync's page: its switch and options (how often, which changes notify), how the last
 * run went and when the next is due, then every run (skipped ones with their reason, finished ones with what they notified),
 * from this session, the logs and the saved history.
 */
@Composable
fun SyncHistoryScreen() {
    val context = LocalContext.current
    val all = rememberAllRuns(older = true)
    val runs = remember(all) { all?.filter { it.title == BACKGROUND_SYNC } }
    val next by produceState<Long?>(null, Settings.backgroundSync, Settings.syncHours) { value = BackgroundSync.nextRunAt(context) }
    val last = runs?.firstOrNull { !it.running }
    val card = AppTheme.colors.card
    val h24 = is24Hour()
    Page(BACKGROUND_SYNC, SYNC_HISTORY) { padding ->
        LazyColumn(contentPadding = padding, verticalArrangement = Arrangement.spacedBy(RUN_GAP)) {
            item("setting") {
                BackgroundSyncSetting { on ->
                    Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        SyncStat("Last sync", "sync.last", last?.let { atText(it.startT, h24) } ?: "None yet", last?.steps?.lastOrNull()?.let { s -> if (s.state == StepState.Skipped) "skipped: ${s.reason}" else s.label })
                        if (Settings.backgroundSync) SyncStat("Next sync", "sync.next", next?.let { atText(it, h24) } ?: "Once the phone is online", null)
                    }
                    // How far along the wait from the last run to the next is.
                    val from = last?.startT
                    val to = next
                    if (on && from != null && to != null && to > from) {
                        val done = ((System.currentTimeMillis() - from).toFloat() / (to - from)).coerceIn(0f, 1f)
                        LinearProgressIndicator({ done }, Modifier.fillMaxWidth().padding(top = 12.dp).testTag("sync.progress"), drawStopIndicator = {})
                    }
                }
            }
            runItems(runs, Int.MAX_VALUE, card, inset = 16.dp, empty = "No background syncs yet")
        }
    }
}

/** One of the status card's two figures: Last or Next sync, its time and an outcome under it. */
@Composable
private fun RowScope.SyncStat(label: String, tag: String, value: String, note: String?) {
    val scheme = MaterialTheme.colorScheme
    Column(Modifier.weight(1f).background(AppTheme.colors.card, RoundedCornerShape(18.dp)).testTag(tag).padding(horizontal = 12.dp, vertical = 11.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.titleSmall, color = scheme.onSurface)
        note?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant) }
    }
}

/** "Today 2:15 PM", "Tomorrow 8:00 AM", "Wed 12 Nov 8:00 AM". */
private fun atText(t: Long, h24: Boolean): String {
    val day = epochDayOf(t)
    val name = when (day - epochDayOf(System.currentTimeMillis())) {
        0 -> "Today"
        1 -> "Tomorrow"
        -1 -> "Yesterday"
        else -> formatDay(day, "EEE d MMM")
    }
    return "$name ${clock(minuteOf(t), h24)}"
}
