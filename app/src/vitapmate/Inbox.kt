package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Bell
import com.composables.icons.lucide.BookOpenCheck
import com.composables.icons.lucide.CalendarCheck
import com.composables.icons.lucide.CalendarClock
import com.composables.icons.lucide.ClipboardList
import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Change
import vitapmate.core.ChangeKind
import vitapmate.core.Core
import vitapmate.data.Activity
import vitapmate.data.AppCore
import vitapmate.data.coreFlow
import vitapmate.data.epochDayOf
import vitapmate.data.minuteOf
import vitapmate.data.orNull
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyView
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.tappable

const val INBOX = "inbox"

/**
 * A screen to open from outside (a change notification, a launcher shortcut); the shell opens it
 * and clears it. One per activity: a launcher shortcut clears the task, and the activity on its way
 * out must not take the new one's request.
 */
class OpenRequest {
    var screen by mutableStateOf<String?>(null)
}

val LocalOpenRequest = staticCompositionLocalOf { OpenRequest() }

/** The unread count for the bell, read again after each VTOP run finishes. */
private object Unseen {
    var count by mutableIntStateOf(0)

    suspend fun read() {
        count = orNull { coreFlow("inbox.unseen") { c, id -> c.unseenChanges(id).toInt() } } ?: count
    }
}

/** More → Changes: the unread count beside it; opens the inbox. */
@Composable
fun InboxRow() {
    val nav = LocalNav.current
    val last = Activity.recent.firstOrNull()?.flow
    LaunchedEffect(last) { Unseen.read() }
    val count = Unseen.count
    GroupItem(
        "Changes",
        "more.inbox",
        Lucide.Bell,
        if (count > 0) "$count new since you last looked" else "What changed on VTOP",
        onClick = { nav.push(Route.Screen(INBOX)) },
        trailing = {
            if (count > 0) {
                Box(Modifier.size(24.dp).background(MaterialTheme.colorScheme.error, CircleShape), Alignment.Center) {
                    Text(if (count > 9) "9+" else "$count", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onError)
                }
            } else {
                Chevron()
            }
        },
    )
}

/**
 * What changed on VTOP, newest first, grouped by day; the ones new since the last visit stand
 * out. Opening it marks them seen. Tapping one opens its page. Clear empties the list.
 */
@Composable
fun InboxScreen() {
    val nav = LocalNav.current
    val scope = rememberCoroutineScope()
    var changes by remember { mutableStateOf<List<Change>?>(null) }
    var fresh by remember { mutableIntStateOf(0) }
    LaunchedEffect(Unit) {
        flow("inbox.open") { id ->
            withContext(Dispatchers.IO) {
                val core = AppCore.await()
                fresh = core.unseenChanges(id).toInt()
                changes = core.changes(id)
                core.markChangesSeen(id)
            }
            Unseen.count = 0
            Log.info("ui.inbox", "open", id, mapOf("changes" to "${changes?.size}", "new" to "$fresh"))
        }
    }
    Page("Changes", INBOX, actions = {
        if (!changes.isNullOrEmpty()) {
            TextButton({
                scope.launch {
                    coreFlow("inbox.clear", Core::clearChanges)
                    changes = emptyList()
                }
            }, Modifier.testTag("inbox.clear")) { Text("Clear") }
        }
    }) { padding ->
        val list = changes ?: return@Page
        if (list.isEmpty()) {
            Box(Modifier.fillMaxSize().padding(padding), Alignment.Center) { EmptyView(Lucide.Bell, "No changes yet", Modifier.testTag("inbox.empty")) }
            return@Page
        }
        val newIds = list.take(fresh).map { it.id }.toSet()
        LazyColumn(
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = padding.calculateTopPadding(), bottom = padding.calculateBottomPadding() + 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            list.groupBy { formatDay(epochDayOf(it.atMs), "EEEE, d MMMM") }.forEach { (day, items) ->
                item("day.$day") {
                    Text(day, Modifier.padding(start = 4.dp, top = 8.dp), style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                }
                items(items, key = { it.id }) { change ->
                    ChangeRow(change, change.id in newIds) {
                        scope.logAction("inbox.open_change", "ui.inbox", "open ${change.kind.name.lowercase()}")
                        val target = pageOf(change.kind)
                        if (target in navTabs) nav.goTab(target) else nav.push(Route.Screen(target))
                    }
                }
            }
        }
    }
}

/** One change: its kind's icon, title and body, the time; tinted when new since the last visit. */
@Composable
private fun ChangeRow(change: Change, new: Boolean, onClick: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val tone = when {
        change.title.contains("below 75") -> tones.danger
        new -> tones.accentTone
        else -> null
    }
    Row(
        Modifier.fillMaxWidth()
            .background(if (new) tones.accentTone.subtle.copy(alpha = 0.5f) else AppTheme.colors.card, MaterialTheme.shapes.large)
            .tappable(onClick, change.title)
            .padding(12.dp)
            .testTag("inbox.change"),
        verticalAlignment = Alignment.Top,
    ) {
        Box(Modifier.size(36.dp).background(tone?.subtle ?: scheme.surfaceContainerHighest, CircleShape), Alignment.Center) {
            Icon(iconOf(change.kind), null, Modifier.size(18.dp), tint = tone?.base ?: scheme.onSurfaceVariant)
        }
        Column(Modifier.weight(1f).padding(start = 12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(change.title, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
                Text(clock(minuteOf(change.atMs), is24Hour()), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
            }
            Text(change.body, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
        }
    }
}

private fun iconOf(kind: ChangeKind): ImageVector = when (kind) {
    ChangeKind.ATTENDANCE -> Lucide.CalendarCheck
    ChangeKind.MARKS -> Lucide.ClipboardList
    ChangeKind.TIMETABLE -> Lucide.CalendarClock
    ChangeKind.EXAMS -> Lucide.BookOpenCheck
    ChangeKind.GRADES -> Lucide.GraduationCap
}

/** The feature a change belongs to. */
internal fun pageOf(kind: ChangeKind): String = when (kind) {
    ChangeKind.ATTENDANCE -> "attendance"
    ChangeKind.MARKS -> "marks"
    ChangeKind.TIMETABLE -> "timetable"
    ChangeKind.EXAMS -> "exams"
    ChangeKind.GRADES -> "grades"
}
