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
import vitapmate.ui.Tone
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.material3.TextButton
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.background
import androidx.compose.foundation.Canvas
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
 * semester's end, the classes left as a strip (filled: must attend, hollow: can miss), both
 * counts, and the best reachable; "Try it" plays that split out in What if. Labs count in sessions.
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
            val reset = (attend > 0 || skip > 0)
            SectionCard(
                Modifier.fillMaxWidth(),
                title = "What if",
                // Always there (disabled at zero), so the card doesn't jump when it turns on.
                action = { TextButton({ attend = 0; skip = 0; log("reset", emptyMap()) }, Modifier.testTag("attendance.detail.calc.reset"), enabled = reset) { Text("Reset") } },
            ) {
                val (percent, exactSafe) = course.projected(attend, skip)
                val now = course.projected(0, 0).first
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
                    Text(
                        "$percent%",
                        Modifier.testTag("attendance.detail.calc.result"),
                        style = MaterialTheme.typography.displaySmallEmphasized.nums(),
                        color = if (reset) tone.base else MaterialTheme.colorScheme.onSurface,
                    )
                    if (percent != now) {
                        Text(
                            if (percent > now) "+${percent - now}" else "−${now - percent}",
                            Modifier.padding(start = Space.md)
                                .background(tone.subtle, RoundedCornerShape(8.dp))
                                .padding(horizontal = 8.dp, vertical = 2.dp)
                                .testTag("attendance.detail.calc.delta"),
                            style = MaterialTheme.typography.labelLargeEmphasized.nums(),
                            color = tone.onSubtle,
                        )
                    }
                    Spacer(Modifier.weight(1f))
                    if (reset) {
                        Text(
                            "${attended + attend} of ${counted + attend + skip}",
                            style = MaterialTheme.typography.titleSmall.nums(),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                AttendanceLine(
                    percent.toDouble(),
                    tone.base,
                    Modifier.padding(top = Space.md, bottom = Space.xl),
                    thickness = 10.dp,
                    from = if (reset) course.percent else null,
                )
                Stepper("Attend", attend, { attend = it; log("attend", mapOf("attend" to "$it", "skip" to "$skip")) }, "attendance.detail.calc.attend")
                Spacer(Modifier.height(Space.md))
                Stepper("Skip", skip, { skip = it; log("skip", mapOf("attend" to "$attend", "skip" to "$it")) }, "attendance.detail.calc.skip")
            }
        }
        if (plans.isNotEmpty()) item("plan") {
            val shown = target.coerceIn(0, plans.lastIndex)
            val plan = plans[shown]
            val left = plan.upcoming + plan.unposted
            val must = plan.mustAttend
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
                val tone = when {
                    must == null -> tones.danger
                    plan.canMiss == 0 -> tones.warning
                    else -> tones.success
                }
                if (left > 0) LeftStrip(left, must ?: left, tone, Modifier.padding(bottom = Space.lg))
                Row(horizontalArrangement = Arrangement.spacedBy(Space.lg)) {
                    PlanCount(
                        if (must == null) "–" else "$must",
                        if (must == null) "can't reach 75%" else "must attend",
                        if (must == null) tones.danger.base else MaterialTheme.colorScheme.onSurface,
                        Modifier.weight(1f).testTag("attendance.detail.calc.must"),
                    )
                    PlanCount("${plan.canMiss}", "can miss", tone.base, Modifier.weight(1f).testTag("attendance.detail.calc.miss"))
                }
                Row(Modifier.padding(top = Space.md), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        listOfNotNull(
                            "$left $unit left",
                            "${plan.unposted} not posted".takeIf { plan.unposted > 0 },
                            "best ${plan.bestPercent.roundToInt()}%",
                        ).joinToString(" · "),
                        Modifier.weight(1f).testTag("attendance.detail.calc.left"),
                        style = MaterialTheme.typography.bodySmall.nums(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (must != null && left > 0) {
                        TextButton(
                            { attend = must; skip = plan.canMiss; log("try", mapOf("attend" to "$must", "skip" to "${plan.canMiss}")) },
                            Modifier.testTag("attendance.detail.calc.try"),
                        ) { Text("Try it") }
                    }
                }
            }
        }
    }
}

/**
 * The classes left as a strip of cells: the first [must] filled in [tone] (to attend), the rest
 * hollow (can be missed). Too many to tell apart: one bar split in the same two parts.
 */
@Composable
private fun LeftStrip(left: Int, must: Int, tone: Tone, modifier: Modifier = Modifier) {
    val outline = tone.base.copy(alpha = 0.55f)
    Canvas(modifier.fillMaxWidth().height(16.dp).testTag("attendance.detail.calc.strip")) {
        val gap = 3.dp.toPx()
        val r = CornerRadius(4.dp.toPx())
        val cell = (size.width - gap * (left - 1)) / left
        if (cell >= 6.dp.toPx()) {
            for (i in 0 until left) {
                val x = i * (cell + gap)
                if (i < must) {
                    drawRoundRect(tone.base, Offset(x, 0f), Size(cell, size.height), r)
                } else {
                    drawRoundRect(tone.subtle, Offset(x, 0f), Size(cell, size.height), r)
                    drawRoundRect(outline, Offset(x, 0f), Size(cell, size.height), r, style = Stroke(1.5.dp.toPx()))
                }
            }
        } else {
            val split = size.width * must / left
            drawRoundRect(tone.subtle, Offset.Zero, size, r)
            drawRoundRect(outline, Offset.Zero, size, r, style = Stroke(1.5.dp.toPx()))
            if (split > 0f) drawRoundRect(tone.base, Offset.Zero, Size(split, size.height), r)
        }
    }
}

/** A large count over its label. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun PlanCount(value: String, label: String, color: Color, modifier: Modifier = Modifier) = Column(modifier) {
    Text(value, style = MaterialTheme.typography.headlineMediumEmphasized.nums(), color = color)
    Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
}
