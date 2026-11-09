package vitapmate

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarClock
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.TriangleAlert
import vitapmate.data.Settings
import vitapmate.data.epochDayOf
import vitapmate.data.minuteOf
import vitapmate.observability.StepState
import vitapmate.ui.AppTheme
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour

/** Route id of Settings → Sync → Sync history. */
const val SYNC_HISTORY = "sync_history"

private const val BACKGROUND_SYNC = "Background sync"

/**
 * Background sync at a glance: how often it checks, how the last run went and when the next is
 * due, then every run (skipped ones with their reason, finished ones with what they notified),
 * from this session, the logs and the saved history.
 */
@Composable
fun SyncHistoryScreen() {
    val context = LocalContext.current
    val all = rememberAllRuns(older = true)
    val runs = remember(all) { all?.filter { it.title == BACKGROUND_SYNC } }
    val next by produceState<Long?>(null) { value = BackgroundSync.nextRunAt(context) }
    val last = runs?.firstOrNull { !it.running }
    val card = AppTheme.colors.card
    val h24 = is24Hour()
    Page("Sync history", SYNC_HISTORY) { padding ->
        LazyColumn(contentPadding = padding, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item("status") {
                ListGroup(null) {
                    Settings.syncPaused?.let { GroupItem("Paused", "sync.paused", Lucide.TriangleAlert, it, tone = AppTheme.tones.warning) }
                    GroupItem("Checks every", "sync.every", Lucide.RefreshCw, trailing = { TrailingValue("${Settings.syncHours} h") })
                    GroupItem("Last sync", "sync.last", Lucide.History, last?.let { lastText(it.startT, h24, it.steps.lastOrNull()?.let { s -> if (s.state == StepState.Skipped) "skipped: ${s.reason}" else s.label }) } ?: "None yet")
                    GroupItem("Next sync", "sync.next", Lucide.CalendarClock, next?.let { atText(it, h24) } ?: "Once the phone is online")
                }
            }
            runItems(runs, Int.MAX_VALUE, card, inset = 16.dp, empty = "No background syncs yet")
        }
    }
}

/** "Today 2:15 PM · 3 changes notified". */
private fun lastText(t: Long, h24: Boolean, outcome: String?): String = listOfNotNull(atText(t, h24), outcome).joinToString(" · ")

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
