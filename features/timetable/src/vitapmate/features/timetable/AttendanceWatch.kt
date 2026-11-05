package vitapmate.features.timetable

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
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind
import vitapmate.data.Settings
import vitapmate.ui.nums

private const val MAX = 4

/** On a free day: courses that can't spare a class, lowest first (up to four); one calm line when none. Courses VTOP hasn't counted yet are left out. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AttendanceWatch(records: List<AttendanceCourse>) {
    val counted = records.filter { it.total > 0 }.sortedBy { it.percent }
    if (counted.isEmpty()) return
    val skips = Settings.showSkipAdvice
    val watch = counted.filter { !it.safe || (skips && it.canSkip == 0) }
    val scheme = MaterialTheme.colorScheme

    @Composable
    fun course(c: AttendanceCourse, title: String? = null) {
        val tone = toneOf(c)
        val code = c.code + if (c.kind == ClassKind.LAB) " lab" else ""
        val percent = c.percent.roundToInt()
        Row(verticalAlignment = Alignment.CenterVertically) {
            CircularProgressIndicator(
                progress = { (c.percent / 100).toFloat() },
                modifier = Modifier.size(32.dp),
                color = tone.base,
                trackColor = tone.subtle,
                strokeWidth = 4.dp,
            )
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(title ?: code, style = MaterialTheme.typography.titleSmallEmphasized, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(if (title != null) "Lowest: $code · $percent%" else c.name, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            if (title == null) {
                Spacer(Modifier.width(8.dp))
                Text(
                    listOfNotNull("$percent%", if (!skips) null else if (!c.safe) "need ${c.mustAttend}" else "don't skip").joinToString(" · "),
                    style = MaterialTheme.typography.labelLargeEmphasized.nums(),
                    color = tone.onSubtle,
                )
            }
        }
    }

    Column(Modifier.fillMaxWidth().testTag("timetable.watch")) {
        SectionTitle("Attendance", if (watch.isEmpty()) null else "${watch.size} to watch")
        Card(shape = MaterialTheme.shapes.extraLarge, colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow)) {
            Column(Modifier.fillMaxWidth().padding(16.dp)) {
                if (watch.isEmpty()) {
                    course(counted.first(), title = "All courses above 75%")
                } else {
                    watch.take(MAX).forEachIndexed { i, c ->
                        if (i > 0) Spacer(Modifier.height(12.dp))
                        course(c)
                    }
                }
            }
        }
    }
}
