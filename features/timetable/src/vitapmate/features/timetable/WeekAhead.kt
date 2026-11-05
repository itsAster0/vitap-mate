package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarMark
import vitapmate.core.ClassDay
import vitapmate.core.ClassSlot
import vitapmate.core.TimetableDay
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.nums

private class AheadRow(val date: Int, val till: Int?, val title: String, val detail: String?, val tone: Tone?, val slots: List<ClassSlot>)

/**
 * The seven days after a free day [from], one row each: your classes on a shared time scale, or
 * what the calendar calls the day (a run of the same day off on one row). A row in the week
 * strip opens that day ([onOpen] gives null for the others).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun WeekAhead(from: Int, days: (Int) -> TimetableDay?, onOpen: (Int) -> (() -> Unit)?, h24: Boolean) {
    val tones = AppTheme.tones
    val rows = mutableListOf<AheadRow>()
    for (date in from + 1..from + 7) {
        val day = days(date)
        val papers = day?.papers.orEmpty()
        val slots = day?.let(::meeting).orEmpty()
        if (papers.isNotEmpty()) {
            // A day with one of your papers is an exam day whatever the calendar says.
            val paper = papers.first()
            rows += AheadRow(
                date, null, "${paper.name} · ${paper.course}",
                listOfNotNull(paper.start?.let { clock(it, h24) }, if (papers.size > 1) "+${papers.size - 1} more" else null).joinToString("  ·  "),
                tones.warning, emptyList(),
            )
        } else if (slots.isNotEmpty()) {
            val fats = slots.filter { it.state == ClassDay.LAB_FAT }.map { it.slot.code }.distinct()
            rows += AheadRow(
                date, null, "${slots.size} ${if (slots.size == 1) "class" else "classes"}",
                listOfNotNull(
                    "${clock(slots.minOf { it.slot.start }, h24)} – ${clock(slots.maxOf { it.slot.end }, h24)}",
                    if (fats.isEmpty()) null else "Lab FAT ${fats.joinToString(", ")}",
                    if (day!!.lastClassDay) "Last class day" else null,
                ).joinToString("  ·  "),
                null, slots.map { it.slot },
            )
        } else {
            val mark = day?.mark
            val named = mark == CalendarMark.HOLIDAY || mark == CalendarMark.NO_CLASSES || mark == CalendarMark.EXAM
            val title = if (named) day.title.orEmpty().ifBlank { "No classes" } else "No classes"
            // A run of the same day off is one row.
            val prev = rows.lastOrNull()
            if (prev != null && prev.slots.isEmpty() && prev.title == title && date - 1 == (prev.till ?: prev.date)) {
                rows[rows.lastIndex] = AheadRow(prev.date, date, title, prev.detail, prev.tone, emptyList())
                continue
            }
            rows += AheadRow(
                date, null, title,
                if (mark == CalendarMark.HOLIDAY && title != "Holiday") "Holiday" else null,
                when (mark) {
                    CalendarMark.HOLIDAY -> tones.success
                    CalendarMark.EXAM -> tones.warning
                    else -> null
                },
                emptyList(),
            )
        }
    }
    // A run of days off that carries on past the week shows its real end (as far as the window goes).
    rows.lastOrNull()?.takeIf { it.slots.isEmpty() }?.let { tail ->
        var till = tail.till ?: tail.date
        while (true) {
            val next = days(till + 1) ?: break
            if (next.mark == null || next.title != tail.title || meeting(next).isNotEmpty()) break
            till++
        }
        if (till != tail.date) rows[rows.lastIndex] = AheadRow(tail.date, till, tail.title, tail.detail, tail.tone, emptyList())
    }
    // Every day's bar on the same clock, so the days compare at a glance.
    val classRows = rows.filter { it.slots.isNotEmpty() }
    val span = if (classRows.isEmpty()) null else classRows.minOf { r -> r.slots.minOf { it.start } } to classRows.maxOf { r -> r.slots.maxOf { it.end } }
    val scheme = MaterialTheme.colorScheme

    Column(Modifier.fillMaxWidth().testTag("timetable.weekahead")) {
        SectionTitle("Week ahead", "${formatDay(from + 1, "d MMM")} – ${formatDay(from + 7, "d MMM")}")
        Card(shape = MaterialTheme.shapes.extraLarge, colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow)) {
            Column(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
                for (r in rows) {
                    val open = onOpen(r.date)
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .then(if (open == null) Modifier else Modifier.clickable(onClickLabel = "Open ${formatDay(r.date, "EEEE")}", onClick = open))
                            .padding(horizontal = 16.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(
                            Modifier.width(48.dp).background(r.tone?.subtle ?: scheme.surfaceContainerHigh, MaterialTheme.shapes.medium).padding(vertical = 6.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(formatDay(r.date, "EEE"), style = MaterialTheme.typography.labelSmall, color = r.tone?.onSubtle ?: scheme.onSurfaceVariant)
                            Text("${dayOfMonth(r.date)}", style = MaterialTheme.typography.titleMediumEmphasized.nums(), color = r.tone?.onSubtle ?: scheme.onSurface)
                        }
                        Spacer(Modifier.width(14.dp))
                        Column(Modifier.weight(1f)) {
                            Text(r.title, style = MaterialTheme.typography.titleSmallEmphasized, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            val detail = listOfNotNull(r.till?.let { "Till ${formatDay(it, "EEE d MMM")}" }, r.detail).filter { it.isNotEmpty() }.joinToString("  ·  ")
                            if (detail.isNotEmpty()) Text(detail, style = MaterialTheme.typography.bodySmall.nums(), color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (r.slots.isNotEmpty()) {
                                Spacer(Modifier.height(4.dp))
                                DayBar(r.slots, minuteNow = null, span = span)
                            }
                        }
                    }
                }
            }
        }
    }
}
