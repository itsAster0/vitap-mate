package vitapmate.features.attendance

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.BadgeCheck
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.CircleDot
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
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
 * Every class, newest first, by month (`attendance.detail.history`, rows
 * `attendance.detail.history.<epoch day>`): the month and how many were attended of those posted,
 * then the rows as a connected list: a date tile (tinted for an absence or on duty), slot and
 * time, VTOP's remark, status.
 */
internal fun LazyListScope.classHistory(classes: List<AttendedClass>) {
    if (classes.isEmpty()) {
        item("history") {
            SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.history"), title = "Classes", shape = MaterialTheme.shapes.extraLargeIncreased) {
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
            Column(Modifier.fillMaxWidth().then(if (i == 0) Modifier.testTag("attendance.detail.history") else Modifier)) {
                Row(Modifier.padding(start = Space.sm, end = Space.sm, bottom = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                    Text(month, Modifier.weight(1f), style = MaterialTheme.typography.titleMediumEmphasized)
                    Text("$present/$posted", style = MaterialTheme.typography.labelLarge.nums(), fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    rows.forEachIndexed { r, row -> ClassRow(row, connected(r, rows.size, 22.dp, 6.dp, vertical = true)) }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ClassRow(row: AttendedClass, shape: Shape) {
    val scheme = MaterialTheme.colorScheme
    val h24 = is24Hour()
    val (label, tone) = row.status.look()
    val tint = row.status == ClassStatus.ABSENT || row.status == ClassStatus.ON_DUTY
    Row(
        Modifier.fillMaxWidth().background(AppTheme.colors.card, shape).padding(start = 10.dp, end = Space.md, top = 10.dp, bottom = 10.dp)
            .testTag("attendance.detail.history.${row.day}"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            Modifier.size(44.dp).background(if (tint) tone.subtle else scheme.surfaceContainer, RoundedCornerShape(14.dp)),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            val ink = if (tint) tone.onSubtle else scheme.onSurface
            Text(formatDay(row.day, "d"), style = MaterialTheme.typography.titleSmallEmphasized.nums(fontWeight = FontWeight.W800, lineHeight = 18.sp), color = ink)
            Text(formatDay(row.day, "EEE"), style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold, color = ink.copy(alpha = 0.8f))
        }
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f)) {
            val time = row.start?.let { start -> row.end?.let { "${clock(start, h24)}–${clock(it, h24)}" } ?: clock(start, h24) }
            Text(
                listOfNotNull(row.slot.ifBlank { null }, time).joinToString(" · "),
                style = MaterialTheme.typography.bodyMedium.nums(),
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (row.remark.isNotBlank()) {
                Text(row.remark, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            }
        }
        Spacer(Modifier.width(Space.sm))
        StatusChip(
            label,
            tone,
            icon = when (row.status) {
                ClassStatus.PRESENT -> Lucide.Check
                ClassStatus.ON_DUTY -> Lucide.BadgeCheck
                ClassStatus.ABSENT -> Lucide.X
                ClassStatus.OTHER -> Lucide.CircleDot
                null -> Lucide.Clock
            },
        )
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
