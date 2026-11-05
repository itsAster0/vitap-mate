package vitapmate.features.exams

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Calendar
import com.composables.icons.lucide.CircleCheck
import com.composables.icons.lucide.Lucide
import vitapmate.core.ScheduledPaper
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.formatMinutes
import vitapmate.ui.is24Hour
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * One paper. To come: course and code, start and end, then venue, seat, seat number and
 * reporting time; the [next] one on the accent fill with how long until it starts today. Written
 * ([done]) and not [open]: one muted row (the date for past days) that taps open; open, it taps shut.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ExamCard(
    paper: ScheduledPaper,
    today: Int,
    minute: Int,
    done: Boolean,
    next: Boolean,
    open: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val motion = MaterialTheme.motionScheme
    val h24 = is24Hour()
    val name = paper.name.ifBlank { paper.code }
    Card(
        modifier.fillMaxWidth()
            .tappable(if (done) onToggle else null, name, Role.Button, pressedScale = 0.99f)
            .semantics {
                if (done) stateDescription = if (open) "expanded" else "finished"
                contentDescription = listOfNotNull(
                    name,
                    paper.day?.let { formatDay(it, "EEEE d MMMM") },
                    paper.start?.let { clock(it, h24) },
                    paper.venue.ifBlank { null },
                    if (next) "next" else null,
                ).joinToString(", ")
            },
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = if (next) AppTheme.tones.accentTone.subtle else AppTheme.colors.card),
    ) {
        AnimatedContent(
            open,
            transitionSpec = {
                fadeIn(motion.defaultEffectsSpec()) togetherWith fadeOut(motion.fastEffectsSpec()) using
                    SizeTransform(clip = true) { _, _ -> motion.defaultSpatialSpec() }
            },
            label = "exam-card",
        ) { full ->
            if (full) Full(paper, today, minute, next, h24) else Folded(paper, today, h24)
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Full(paper: ScheduledPaper, today: Int, minute: Int, next: Boolean, h24: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val day = paper.day
    Column(Modifier.padding(Space.lg)) {
        if (day != null && day < today) {
            StatusChip(formatDay(day, "EEE d MMM"), tones.accentTone, icon = Lucide.Calendar)
            Spacer(Modifier.height(Space.sm))
        }
        Row(verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) {
                Text(paper.name.ifBlank { paper.code }, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(2.dp))
                Text(
                    listOf(paper.code, paper.slot).filter { it.isNotBlank() }.joinToString(" · "),
                    style = MaterialTheme.typography.labelMedium,
                    color = scheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
            Spacer(Modifier.width(Space.md))
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    paper.start?.let { clock(it, h24) } ?: paper.session.ifBlank { "–" },
                    style = MaterialTheme.typography.titleMediumEmphasized.nums(),
                )
                paper.end?.let {
                    Text("to ${clock(it, h24)}", style = MaterialTheme.typography.labelMedium.nums(), color = scheme.onSurfaceVariant)
                }
            }
        }
        if (next) {
            val start = paper.start
            Row(Modifier.padding(top = Space.md), horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
                StatusChip("Next", tones.accentTone, solid = true)
                if (day == today && start != null && start > minute) {
                    Text("in ${formatMinutes(start - minute)}", style = MaterialTheme.typography.labelLarge.nums(), color = tones.accentTone.onSubtle)
                }
            }
        }
        val facts = listOf(
            "Venue" to paper.venue,
            "Seat" to paper.seat,
            "Seat no." to paper.seatNo,
            "Report" to (paper.reporting?.let { clock(it, h24) } ?: ""),
        ).filter { it.second.isNotBlank() }
        facts.chunked(2).forEachIndexed { i, row ->
            Row(Modifier.fillMaxWidth().padding(top = if (i == 0) Space.md else Space.sm)) {
                row.forEach { (label, value) -> Fact(label, value, Modifier.weight(1f)) }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun Fact(label: String, value: String, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(modifier.padding(end = Space.sm)) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
        Spacer(Modifier.height(2.dp))
        Text(value, style = MaterialTheme.typography.titleSmall.nums(), maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/** A written paper in one muted row: the date (or a tick today), course and venue, start. */
@Composable
private fun Folded(paper: ScheduledPaper, today: Int, h24: Boolean) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val day = paper.day
    Row(Modifier.fillMaxWidth().padding(horizontal = Space.lg, vertical = Space.md), verticalAlignment = Alignment.CenterVertically) {
        if (day != null && day < today) {
            Column(Modifier.widthIn(min = 34.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(formatDay(day, "d"), style = MaterialTheme.typography.titleMedium.nums(), color = muted)
                Text(formatDay(day, "EEE").uppercase(), style = MaterialTheme.typography.labelSmall, color = muted, textAlign = TextAlign.Center)
            }
        } else {
            Icon(Lucide.CircleCheck, null, Modifier.size(16.dp), tint = muted)
        }
        Spacer(Modifier.width(Space.md))
        Column(Modifier.weight(1f)) {
            Text(paper.name.ifBlank { paper.code }, style = MaterialTheme.typography.bodyMedium, color = muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (paper.venue.isNotBlank()) Text(paper.venue, style = MaterialTheme.typography.labelMedium, color = muted, maxLines = 1)
        }
        paper.start?.let {
            Spacer(Modifier.width(Space.md))
            Text(clock(it, h24), style = MaterialTheme.typography.labelMedium.nums(), color = muted)
        }
    }
}
