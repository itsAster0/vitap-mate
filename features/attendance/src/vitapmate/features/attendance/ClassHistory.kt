package vitapmate.features.attendance

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.core.AttendedClass
import vitapmate.core.ClassStatus
import vitapmate.ui.AppTheme
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.nums

/**
 * Every class, newest first, a card per month (`attendance.detail.history`, rows
 * `attendance.detail.history.<epoch day>`): date and weekday, slot and time, VTOP's remark, status.
 */
internal fun LazyListScope.classHistory(classes: List<AttendedClass>) {
    if (classes.isEmpty()) {
        item("history") {
            SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.history"), title = "Classes") {
                Text("None posted yet", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        return
    }
    val months = classes.groupBy { formatDay(it.day, "MMMM yyyy") }
    months.entries.forEachIndexed { i, (month, rows) ->
        item("history.$month") {
            val posted = rows.count { it.status != null && it.status != ClassStatus.OTHER }
            val present = rows.count { it.status == ClassStatus.PRESENT || it.status == ClassStatus.ON_DUTY }
            SectionCard(
                Modifier.fillMaxWidth().then(if (i == 0) Modifier.testTag("attendance.detail.history") else Modifier),
                title = month,
                action = {
                    Text("$present/$posted", style = MaterialTheme.typography.labelLarge.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant)
                },
            ) {
                rows.forEachIndexed { r, row ->
                    if (r > 0) HorizontalDivider(Modifier.padding(vertical = Space.sm), color = MaterialTheme.colorScheme.outlineVariant)
                    ClassRow(row)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ClassRow(row: AttendedClass) {
    val scheme = MaterialTheme.colorScheme
    val h24 = is24Hour()
    val (label, tone) = row.status.look()
    Row(Modifier.fillMaxWidth().testTag("attendance.detail.history.${row.day}"), verticalAlignment = Alignment.Top) {
        Column(Modifier.widthIn(min = 40.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Text(formatDay(row.day, "d"), style = MaterialTheme.typography.titleMediumEmphasized.nums(), textAlign = TextAlign.Center)
            Text(formatDay(row.day, "EEE"), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
        }
        Spacer(Modifier.width(Space.md))
        Column(Modifier.weight(1f).padding(top = 2.dp)) {
            val time = row.start?.let { start -> row.end?.let { "${clock(start, h24)}–${clock(it, h24)}" } ?: clock(start, h24) }
            Text(
                listOfNotNull(row.slot.ifBlank { null }, time).joinToString(" · "),
                style = MaterialTheme.typography.bodyMedium.nums(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (row.remark.isNotBlank()) {
                Text(row.remark, Modifier.padding(top = 2.dp), style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            }
        }
        Spacer(Modifier.width(Space.sm))
        StatusChip(label, tone)
    }
}

/** A class's status as a label in its tone; null is "Not posted". */
@Composable
internal fun ClassStatus?.look(): Pair<String, Tone> {
    val tones = AppTheme.tones
    val neutral = MaterialTheme.colorScheme.run { Tone(outline, surfaceContainerHigh, onSurfaceVariant) }
    return when (this) {
        ClassStatus.PRESENT -> "Present" to tones.success
        ClassStatus.ON_DUTY -> "On duty" to tones.accentTone
        ClassStatus.ABSENT -> "Absent" to tones.danger
        ClassStatus.OTHER -> "Marked" to neutral
        null -> "Not posted" to neutral
    }
}
