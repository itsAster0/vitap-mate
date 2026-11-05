package vitapmate.features.attendance

import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.floor
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.data.AppCore
import vitapmate.observability.flow
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * Each course's plan by key (until the next exam if one comes first, else the semester's end),
 * from Rust's saved attendance, class histories, timetable and calendar; read again when [key]
 * (the attendance and the histories that arrived) changes. Empty without a calendar.
 */
@Composable
internal fun rememberOutlook(key: Any?): Map<String, AttendancePlan> {
    val outlook by produceState(emptyMap(), key) {
        value = withContext(Dispatchers.IO) {
            runCatching { flow("attendance.outlook") { id -> AppCore.await().attendanceOutlook(id) } }
                .onFailure { if (it is CancellationException) throw it }.getOrDefault(emptyList())
        }.associate { it.key to it.plan }
    }
    return outlook
}

/**
 * "1+16 left · 8 to spare · 95% max" under a hairline, centred: the muted "1+" is held but not
 * posted yet, then the classes to come (till the exam when the plan stops at one); "N needed"
 * below 75%, "short of 75%" in [alert] when even all of them are not enough. Without
 * [showSkip] the middle part is left out. Labs count in sessions.
 */
@Composable
internal fun OutlookLine(course: AttendanceCourse, plan: AttendancePlan, showSkip: Boolean, alert: Color, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val left = plan.upcoming + plan.unposted
    val must = plan.mustAttend
    val warn = left > 0 && must == null
    val strong = SpanStyle(fontWeight = FontWeight.SemiBold, color = if (warn) alert else scheme.onSurface)
    val text = buildAnnotatedString {
        if (left == 0) {
            append(if (plan.until.isEmpty()) "No classes left this semester" else "No classes before ${plan.until}")
            return@buildAnnotatedString
        }
        if (plan.unposted > 0) append("${plan.unposted}+")
        withStyle(strong) { append("${plan.upcoming}") }
        append(if (plan.until.isEmpty()) " left" else " left till ${plan.until}")
        if (showSkip) {
            append("  ·  ")
            when {
                must == null -> withStyle(SpanStyle(color = alert)) { append("short of 75%") }
                !course.safe -> { withStyle(strong) { append("$must") }; append(" needed") }
                else -> { withStyle(strong) { append("${plan.canMiss}") }; append(" to spare") }
            }
        }
        append("  ·  ")
        withStyle(strong) { append("${floor(plan.bestPercent).toInt()}%") }
        append(" max")
    }
    Column(modifier.fillMaxWidth().testTag("attendance.course.${course.key}.outlook")) {
        HorizontalDivider(color = scheme.outlineVariant)
        Text(
            text,
            Modifier.fillMaxWidth().padding(top = Space.sm),
            style = MaterialTheme.typography.labelMedium.nums(),
            color = scheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}
