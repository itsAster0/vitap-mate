package vitapmate.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import java.util.Locale
import java.util.Date
import java.util.Calendar
import java.text.SimpleDateFormat
import kotlin.time.Duration.Companion.milliseconds

/** The data's age for [UpdatedLine] at the end of a page's content; [PageScaffold] provides it. */
val LocalUpdated = staticCompositionLocalOf<Pair<Long, Boolean>?> { null }

/**
 * "Updated 3 hr ago" from [LocalUpdated], centred: put it last in a page's list (activity in
 * the app bar shows a refresh as it runs).
 */
@Composable
fun UpdatedFooter(modifier: Modifier = Modifier) {
    // The app bar shows a refresh while it runs, so only the age here.
    val (updatedAt, _) = LocalUpdated.current ?: return
    Box(modifier.fillMaxWidth().padding(vertical = Space.md), Alignment.Center) { UpdatedLine(updatedAt, refreshing = false) }
}

/** "Updated 3 hr ago" ([updatedAt] in epoch seconds), or [busy] ("Updating…") with a small indicator while [refreshing]. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatedLine(updatedAt: Long, refreshing: Boolean, modifier: Modifier = Modifier, busy: String = "Updating…") {
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(60_000.milliseconds)
            now = System.currentTimeMillis()
        }
    }
    Row(modifier.testTag("page.updated"), verticalAlignment = Alignment.CenterVertically) {
        if (refreshing) {
            LoadingIndicator(Modifier.size(18.dp))
            Spacer(Modifier.width(Space.xs))
        }
        Text(
            when {
                refreshing -> busy
                updatedAt > 0 -> "Updated ${relativeTime(updatedAt * 1000, now)}"
                else -> ""
            },
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** "just now", "5 min ago", "2 hr ago", "yesterday 4:10 PM", "25 Sep" (times in epoch ms). */
fun relativeTime(then: Long, now: Long): String {
    val minutes = (now - then) / 60_000
    if (minutes < 1) return "just now"
    if (minutes < 60) return "$minutes min ago"
    if (minutes < 12 * 60) return "${minutes / 60} hr ago"
    fun day(ms: Long) = Calendar.getInstance().apply {
        timeInMillis = ms
        set(Calendar.HOUR_OF_DAY, 12) // noon, so DST shifts don't change the day count
        set(Calendar.MINUTE, 0)
    }
    val a = day(then)
    val b = day(now)
    val days = Math.round((b.timeInMillis - a.timeInMillis) / 86_400_000.0)
    fun fmt(pattern: String) = SimpleDateFormat(pattern, Locale.US).format(Date(then))
    return when {
        days == 0L -> "today ${fmt("h:mm a")}"
        days == 1L -> "yesterday ${fmt("h:mm a")}"
        a.get(Calendar.YEAR) == b.get(Calendar.YEAR) -> fmt("d MMM")
        else -> fmt("d MMM y")
    }
}
