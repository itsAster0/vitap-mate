package vitapmate.features.calendar

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import vitapmate.core.CalendarEvent
import vitapmate.core.CalendarEventPart
import vitapmate.core.CalendarMonth
import vitapmate.ui.AppTheme
import vitapmate.ui.Dot
import vitapmate.ui.Radii
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge
import vitapmate.ui.formatDay
import vitapmate.ui.neutralTone
import vitapmate.ui.nums

/**
 * The rows touching [month] (a run from a neighbouring month included whole) as segments, each led
 * by its date in the mark's tone; past ones fade.
 */
@Composable
fun MonthEvents(month: CalendarMonth, today: Int, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    if (month.events.isEmpty()) {
        Text(
            "Regular classes all month.",
            modifier.fillMaxWidth().padding(vertical = Space.lg),
            style = MaterialTheme.typography.bodyMedium,
            color = scheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        return
    }
    val firstOfMonth = month.days.first().day
    // Segments: the group's outer corners are large, each row's own corners small.
    Column(modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        month.events.forEach { event ->
            EventRow(event, startsBefore = event.first < firstOfMonth, past = event.last < today)
        }
    }
}

private val DATE_WIDTH = 48.dp

@Composable
private fun EventRow(event: CalendarEvent, startsBefore: Boolean, past: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tone = markTone(event.mark) ?: neutralTone
    val days = event.last - event.first + 1
    val range = event.last != event.first
    val mixed = event.parts.size > 1
    val subtitle = listOfNotNull(
        if (range) "${formatDay(event.first, if (sameMonth(event)) "EEE d" else "EEE d MMM")} – ${formatDay(event.last, "EEE d MMM")}" else null,
        if (range && !mixed) "$days days" else null,
    ).joinToString(" · ")
    Column(
        Modifier.fillMaxWidth().alpha(if (past) 0.5f else 1f).background(AppTheme.colors.card, RoundedCornerShape(6.dp))
            .testTag("calendar.event.${event.firstDate}").padding(horizontal = Space.lg, vertical = Space.md),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(
                Modifier.size(DATE_WIDTH, 52.dp).background(tone.subtle, RoundedCornerShape(16.dp)),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                // A run begun last month shows that month, not a weekday.
                Text(
                    formatDay(event.first, if (startsBefore) "MMM" else "EEE"),
                    style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                    color = tone.onSubtle,
                )
                Text(formatDay(event.first, "d"), style = MaterialTheme.typography.titleLarge.nums(fontWeight = FontWeight.ExtraBold, color = tone.onSubtle, lineHeight = 22.sp))
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(event.title, style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (subtitle.isNotEmpty()) {
                    Text(subtitle, Modifier.padding(top = 2.dp), style = MaterialTheme.typography.bodySmall.nums(color = scheme.onSurfaceVariant))
                }
            }
            Spacer(Modifier.width(Space.sm))
            ToneBadge(markLabel(event.mark), tone = tone)
        }
        // Days off for different reasons: one row, each reason on its own line.
        if (mixed) {
            Spacer(Modifier.height(Space.sm))
            Column(Modifier.padding(start = DATE_WIDTH + 14.dp), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                event.parts.forEach { PartLine(it) }
            }
        }
    }
}

@Composable
private fun PartLine(part: CalendarEventPart) {
    val scheme = MaterialTheme.colorScheme
    val tone = markTone(part.mark) ?: neutralTone
    val days = if (part.first == part.last) formatDay(part.first, "EEE d") else "${formatDay(part.first, "EEE d")} – ${formatDay(part.last, "EEE d")}"
    Row(
        Modifier.fillMaxWidth().background(scheme.surfaceContainerHigh, RoundedCornerShape(Radii.sm)).padding(horizontal = Space.md, vertical = Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Dot(size = 6.dp, color = if (hollow(part.mark)) Color.Transparent else tone.base, ring = if (hollow(part.mark)) tone.base else null)
        Spacer(Modifier.width(Space.sm))
        Text(days, Modifier.width(96.dp), style = MaterialTheme.typography.labelMedium.nums(fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant))
        Text(part.title, style = MaterialTheme.typography.bodyMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}

private fun sameMonth(event: CalendarEvent) = event.firstDate.take(7) == event.lastDate.take(7)
