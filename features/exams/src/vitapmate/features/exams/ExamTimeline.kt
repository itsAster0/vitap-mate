package vitapmate.features.exams

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.core.ExamGroup
import vitapmate.core.ScheduledPaper
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.SectionCard
import vitapmate.ui.SectionHeader
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter

/** One line of the timeline. */
private sealed interface Line {
    val key: String

    data class Header(val day: Int, val count: Int) : Line { override val key = "day.$day" }
    data class Gap(val day: Int, val days: Int) : Line { override val key = "gap.$day" }
    data class Paper(val paper: ScheduledPaper, val next: Boolean) : Line { override val key = "paper.${paper.key}" }
}

/**
 * Days to come under a header (date, TODAY / TOMORROW / IN 3 DAYS, count) with the free days
 * between them; past days without headers, their papers carrying the date. Papers without a
 * date close the list, then the data's age. The first paper still to come is [Line.Paper.next].
 */
private fun lines(papers: List<ScheduledPaper>, today: Int, minute: Int): List<Line> {
    val next = papers.firstOrNull { it.day != null && !it.done(today, minute) }
    val days = papers.mapNotNull { it.day }.distinct().sorted()
    val out = mutableListOf<Line>()
    days.forEachIndexed { i, day ->
        if (day >= today) {
            val prev = days.getOrNull(i - 1)
            if (prev != null && prev >= today && day - prev > 1) out += Line.Gap(day, day - prev - 1)
            out += Line.Header(day, papers.count { it.day == day })
        }
        papers.filter { it.day == day }.forEach { out += Line.Paper(it, it == next) }
    }
    return out
}

@Composable
internal fun ExamTimeline(
    group: ExamGroup,
    state: LazyListState,
    today: Int,
    minute: Int,
    expanded: List<String>,
    onToggle: (ScheduledPaper) -> Unit,
    modifier: Modifier = Modifier,
) {
    val lines = lines(group.papers, today, minute)
    val undated = group.papers.filter { it.day == null }
    LazyColumn(
        modifier.fillMaxSize(),
        state,
        contentPadding = PaddingValues(
            start = Space.sm,
            top = Space.xs,
            end = Space.sm,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
        ),
    ) {
        // Written papers that stay folded, by line: consecutive ones join into one block.
        val folded = lines.map { it is Line.Paper && it.paper.done(today, minute) && "${group.name}/${it.paper.key}" !in expanded }
        itemsIndexed(lines, key = { _, it -> it.key }) { i, line ->
            val joinedAbove = folded[i] && folded.getOrElse(i - 1) { false }
            val joinedBelow = folded[i] && folded.getOrElse(i + 1) { false }
            val item = Modifier.animateItem().padding(top = if (i == 0) 0.dp else if (joinedAbove) 2.dp else Space.sm)
            when (line) {
                is Line.Header -> DayHeader(line.day, today, line.count, item)
                is Line.Gap -> GapRow(line.days, item)
                is Line.Paper -> {
                    val paper = line.paper
                    val done = paper.done(today, minute)
                    ExamCard(
                        paper,
                        today,
                        minute,
                        done,
                        line.next,
                        open = !done || "${group.name}/${paper.key}" in expanded,
                        onToggle = { onToggle(paper) },
                        item.testTag("exams.paper.${group.name.tagKey()}.${paper.key}"),
                        shape = RoundedCornerShape(
                            topStart = if (joinedAbove) 4.dp else 16.dp,
                            topEnd = if (joinedAbove) 4.dp else 16.dp,
                            bottomEnd = if (joinedBelow) 4.dp else 16.dp,
                            bottomStart = if (joinedBelow) 4.dp else 16.dp,
                        ),
                    )
                }
            }
        }
        if (undated.isNotEmpty()) {
            item("undated") {
                Column(Modifier.animateItem().padding(top = Space.sm).testTag("exams.undated")) {
                    SectionHeader("Date not announced", trailing = { Text("${undated.size} ${if (undated.size == 1) "course" else "courses"}") })
                    SectionCard {
                        undated.forEachIndexed { i, paper ->
                            if (i > 0) HorizontalDivider(Modifier.padding(vertical = Space.md), color = MaterialTheme.colorScheme.outlineVariant)
                            CourseLine(paper)
                        }
                    }
                }
            }
        }
        item("updated") {
            Spacer(Modifier.height(Space.xs + Space.sm))
            UpdatedFooter()
        }
    }
}

/** Written already: the day is past, or today's start (the day's end without a time) has gone. */
internal fun ScheduledPaper.done(today: Int, minute: Int): Boolean {
    val day = day ?: return false
    return day < today || day == today && (start ?: (23 * 60 + 59)) <= minute
}
