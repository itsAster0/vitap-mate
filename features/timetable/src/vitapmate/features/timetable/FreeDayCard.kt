package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarOff
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.PartyPopper
import com.composables.icons.lucide.PenLine
import com.composables.icons.lucide.SunMedium
import vitapmate.core.CalendarMark
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.nums

/** A day without classes: why (your exam, the calendar's holiday or no-class day, or nothing on your timetable). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun FreeDayCard(
    d: AgendaDay,
    countdown: String?,
    h24: Boolean,
) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    val papers = d.day.papers
    val exam = papers.firstOrNull()
    val off = d.freeReason
    val mark = if (exam != null) CalendarMark.EXAM else off?.mark
    val (icon, tone) = when (mark) {
        CalendarMark.EXAM -> Lucide.PenLine to tones.warning
        CalendarMark.HOLIDAY -> Lucide.PartyPopper to tones.success
        null -> Lucide.SunMedium to tones.accentTone
        else -> Lucide.CalendarOff to tones.accentTone
    }
    val title = when {
        exam != null -> exam.course + if (papers.size > 1) " +${papers.size - 1} more" else ""
        off != null -> off.title.ifBlank { "No classes" }
        d.schedule.isEmpty() -> "No classes"
        // Your classes that day don't meet (the calendar holds no theory).
        else -> "Free day"
    }
    val eyebrow = when (mark) {
        CalendarMark.HOLIDAY -> if (title == "Holiday") null else "Holiday"
        CalendarMark.EXAM -> if (exam != null) listOfNotNull(exam.name, exam.start?.let { clock(it, h24) }).joinToString(" · ") else "Exam · no classes"
        null -> null
        else -> "No classes"
    }
    Card(
        Modifier.fillMaxWidth().testTag("timetable.free"),
        shape = MaterialTheme.shapes.extraLargeIncreased,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(Modifier.padding(20.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(56.dp).background(tone.subtle, MaterialShapes.Sunny.toShape()), Alignment.Center) {
                    Icon(icon, null, Modifier.size(24.dp), tint = tone.base)
                }
                Spacer(Modifier.width(16.dp))
                Column(Modifier.weight(1f)) {
                    if (eyebrow != null) {
                        StatusChip(eyebrow, tone)
                        Spacer(Modifier.height(6.dp))
                    }
                    Text(title, style = MaterialTheme.typography.titleLargeEmphasized, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    if (countdown != null) {
                        Text(countdown, style = MaterialTheme.typography.bodySmall.nums(), color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
        }
    }
}
