package vitapmate.features.attendance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * The detail page's Calculator tab. "What if": attend N / skip N more → the percentage VTOP would
 * show, in the tone it lands in (warning when only VTOP's rounding keeps it at 75%). With [plans]
 * (Rust, from the timetable and academic calendar), a second card: until the next exam or the
 * semester's end, what is left, what must be attended to finish at 75%, what can be missed and
 * the best reachable. Labs count in sessions.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Calculator(course: AttendanceCourse, plans: List<AttendancePlan>, modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    val tones = AppTheme.tones
    var attend by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var skip by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var target by rememberSaveable(course.key) { mutableIntStateOf(0) }
    val log: (String, Map<String, String>) -> Unit = { what, fields ->
        scope.launch { flow("attendance.detail.calc") { id -> Log.info("ui.attendance", "calc $what ${course.key}", id, fields + ("course" to course.key)) } }
    }
    val unit = if (course.isLab) "sessions" else "classes"
    LazyColumn(
        modifier.fillMaxSize().testTag("attendance.detail.calc"),
        contentPadding = PaddingValues(
            start = Space.lg,
            top = Space.lg,
            end = Space.lg,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl,
        ),
        verticalArrangement = Arrangement.spacedBy(Space.lg),
    ) {
        item("whatif") {
            SectionCard(Modifier.fillMaxWidth(), title = "What if") {
                val (percent, exactSafe) = course.projected(attend, skip)
                val tone = when {
                    percent < 75 -> tones.danger
                    exactSafe -> tones.success
                    else -> tones.warning
                }
                val (attended, counted) = course.sessions
                val unposted = if (course.isLab) (course.total + 1) / 2 - counted else course.total - counted
                Text(
                    "Using $attended of $counted $unit" + if (unposted > 0) " · $unposted not posted" else "",
                    Modifier.padding(bottom = Space.sm).testTag("attendance.detail.calc.basis"),
                    style = MaterialTheme.typography.labelLarge.nums(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(course.percentLabel, style = MaterialTheme.typography.titleMedium.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text("  →  ", style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text("$percent%", Modifier.testTag("attendance.detail.calc.result"), style = MaterialTheme.typography.displaySmallEmphasized.nums())
                }
                AttendanceLine(percent.toDouble(), tone.base, Modifier.padding(top = Space.md, bottom = Space.xl), thickness = 10.dp)
                Stepper("Attend", attend, { attend = it; log("attend", mapOf("attend" to "$it", "skip" to "$skip")) }, "attendance.detail.calc.attend")
                Spacer(Modifier.height(Space.md))
                Stepper("Skip", skip, { skip = it; log("skip", mapOf("attend" to "$attend", "skip" to "$it")) }, "attendance.detail.calc.skip")
            }
        }
        if (plans.isNotEmpty()) item("plan") {
            val shown = target.coerceIn(0, plans.lastIndex)
            val plan = plans[shown]
            val left = plan.upcoming + plan.unposted
            SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.calc.plan"), title = "To finish at 75%") {
                if (plans.size > 1) {
                    ConnectedChoice(
                        plans.indices.toList(),
                        shown,
                        { i -> target = i; log("target", mapOf("until" to plans[i].until.ifEmpty { "end" })) },
                        tag = "attendance.detail.calc.until",
                        key = { plans[it].until.ifEmpty { "end" } },
                        label = { plans[it].until.ifEmpty { null }?.let { exam -> "Till $exam" } ?: "Semester end" },
                        modifier = Modifier.padding(bottom = Space.lg),
                    )
                }
                Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
                    val must = plan.mustAttend
                    PlanRow(
                        "Attend",
                        when {
                            must == null -> "Can't reach"
                            must == 0 -> "Done"
                            else -> "$must of $left"
                        },
                        if (must == null) tones.danger.onSubtle else null,
                    )
                    PlanRow("Can miss", "${plan.canMiss}")
                    PlanRow("Left", if (plan.unposted > 0) "$left $unit · ${plan.unposted} not posted" else "$left $unit")
                    PlanRow("Best", "${plan.bestPercent.roundToInt()}%")
                }
            }
        }
    }
}

@Composable
private fun PlanRow(label: String, value: String, color: Color? = null) = Row(verticalAlignment = Alignment.CenterVertically) {
    Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    Text(value, style = MaterialTheme.typography.titleSmall.nums(), color = color ?: MaterialTheme.colorScheme.onSurface)
}
