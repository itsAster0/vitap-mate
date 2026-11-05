package vitapmate.features.biometric

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.LogIn
import com.composables.icons.lucide.LogOut
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.ScanFace
import vitapmate.core.BiometricDay
import vitapmate.core.Punch
import vitapmate.core.PunchKind
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.SectionHeader
import vitapmate.ui.Space
import vitapmate.ui.Surface
import vitapmate.ui.Tone
import vitapmate.ui.ToneBadge
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.nums

/** One day: where the student last punched, the punches newest first on a timeline, the age. */
@Composable
internal fun DayView(data: BiometricDay, isToday: Boolean) {
    val list = rememberLazyListState()
    ScrollToTopOnReselect(list)
    val h24 = is24Hour()
    LazyColumn(
        Modifier.fillMaxSize().testTag("biometric.list"),
        list,
        contentPadding = PaddingValues(start = Space.page, end = Space.page, top = Space.sm, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl),
    ) {
        val latest = data.punches.firstOrNull()
        if (latest == null) {
            item("empty") {
                EmptyView(Lucide.ScanFace, "No punches", Modifier.fillMaxWidth().testTag("biometric.empty"), message = formatDay(data.day, "EEE d MMM"))
            }
        } else {
            item("status") { Status(latest, isToday, h24, Modifier.animateItem()) }
            item("header") {
                SectionHeader(formatDay(data.day, "EEEE, d MMMM"), Modifier.animateItem()) { Text("${data.entries} in · ${data.exits} out") }
            }
            item("punches") {
                Surface(Modifier.animateItem().testTag("biometric.punches"), padding = PaddingValues(horizontal = Space.md + 2.dp, vertical = Space.sm)) {
                    Column {
                        data.punches.forEachIndexed { i, punch ->
                            PunchRow(punch, h24, first = i == 0, last = i == data.punches.lastIndex, Modifier.testTag("biometric.punch.$i"))
                        }
                    }
                }
            }
        }
        item("footer") { UpdatedFooter(Modifier.animateItem()) }
    }
}

private data class Look(val tone: Tone, val label: String, val icon: ImageVector)

@Composable
private fun PunchKind.look(): Look = AppTheme.tones.let { t ->
    when (this) {
        PunchKind.ENTRY -> Look(t.success, "ENTRY", Lucide.LogIn)
        PunchKind.EXIT -> Look(t.warning, "EXIT", Lucide.LogOut)
        PunchKind.OTHER -> Look(t.accentTone, "PUNCH", Lucide.ScanFace)
    }
}

private fun Punch.at(h24: Boolean) = minute?.let { clock(it, h24) } ?: time

/** Today: inside or out of the last place since its punch; another day: that day's last punch. */
@Composable
private fun Status(latest: Punch, isToday: Boolean, h24: Boolean, modifier: Modifier = Modifier) {
    val tone = latest.kind.look().tone
    val at = latest.at(h24)
    val (title, detail) = if (isToday) {
        when (latest.kind) {
            PunchKind.ENTRY -> "Inside ${latest.place}" to "since $at"
            PunchKind.EXIT -> "Out of ${latest.place}" to "since $at"
            PunchKind.OTHER -> "Last seen at ${latest.place}" to at
        }
    } else {
        "Last punch that day" to "${if (latest.kind == PunchKind.EXIT) "Exit" else "Entry"} at ${latest.place}, $at"
    }
    Surface(modifier.fillMaxWidth().testTag("biometric.status")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(10.dp).shadow(6.dp, CircleShape, ambientColor = tone.base, spotColor = tone.base).background(tone.base, CircleShape))
            Spacer(Modifier.width(Space.md))
            Column {
                Text(title, style = MaterialTheme.typography.bodyLarge.copy(fontWeight = FontWeight.W600))
                Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

/** Time, a dot on the day's line, then the kind and VTOP's venue. */
@Composable
private fun PunchRow(punch: Punch, h24: Boolean, first: Boolean, last: Boolean, modifier: Modifier = Modifier) {
    val look = punch.kind.look()
    val line = MaterialTheme.colorScheme.outlineVariant
    Row(modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
        Text(
            punch.at(h24),
            Modifier.width(64.dp).padding(top = Space.md),
            style = MaterialTheme.typography.bodyMedium.nums(fontWeight = FontWeight.W500),
        )
        Box(Modifier.width(24.dp).fillMaxHeight()) {
            Column(Modifier.align(Alignment.TopCenter).fillMaxHeight()) {
                Box(Modifier.width(1.5.dp).height(18.dp).background(if (first) line.copy(alpha = 0f) else line))
                Box(Modifier.width(1.5.dp).weight(1f).background(if (last) line.copy(alpha = 0f) else line))
            }
            Box(Modifier.align(Alignment.TopCenter).offset(y = 14.dp).size(9.dp).background(look.tone.base, CircleShape))
        }
        Spacer(Modifier.width(Space.sm))
        Column(Modifier.weight(1f).padding(vertical = Space.sm + 2.dp), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
            ToneBadge(look.label, tone = look.tone, icon = look.icon)
            Text(punch.venue, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
