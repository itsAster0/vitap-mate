package vitapmate.features.grades

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.CircularWavyProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowRight
import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minus
import com.composables.icons.lucide.Plus
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.GpaPlan
import vitapmate.core.GradeHistory
import vitapmate.core.PlannedCourse
import vitapmate.core.TargetOutlook
import vitapmate.core.planGpa
import vitapmate.data.AppCore
import vitapmate.data.SemesterChoice
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.IconButton
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.SectionCard
import vitapmate.ui.SectionHeader
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * The GPA planner: planned courses (this semester's from the saved timetable to start) give the
 * semester GPA; with the CGPA and credits so far (from [history], editable) the CGPA after it and
 * the semester GPA a target needs. Arithmetic in Rust ([planGpa]). Flows `grades.plan.*`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun PlannerView(planner: Planner, history: GradeHistory?, list: LazyListState) {
    val scope = rememberCoroutineScope()
    fun act(what: String, fields: Map<String, String> = emptyMap()) {
        scope.launch { flow("grades.plan.$what") { id -> Log.info("ui.grades", "plan $what", id, fields + ("rows" to "${planner.rows.size}")) } }
    }
    LaunchedEffect(history) {
        if (history == null || planner.prefilled) return@LaunchedEffect
        planner.prefilled = true
        if (planner.cgpa.isEmpty()) planner.cgpa = history.cgpa
        if (planner.completed.isEmpty()) planner.completed = history.creditsRegistered
    }
    LaunchedEffect(SemesterChoice.selected) {
        try {
            planner.semester = flow("grades.plan.courses") { id -> withContext(Dispatchers.IO) { AppCore.await().plannerCourses(id) } }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // In the log already.
        }
        if (!planner.autoLoaded && planner.untouched && planner.semester.isNotEmpty()) {
            planner.autoLoaded = true
            planner.loadSemester()
            act("load", mapOf("by" to "open"))
        }
    }
    val rows = planner.rows.toList()
    val plan = remember(rows, planner.cgpa, planner.completed, planner.target) {
        planGpa(rows.map { PlannedCourse(it.credits, it.grade) }, planner.cgpa, planner.completed, planner.target)
    }
    LazyColumn(
        Modifier.fillMaxSize().testTag("grades.list.planner"),
        list,
        contentPadding = PaddingValues(
            start = Space.sm,
            top = Space.xs,
            end = Space.sm,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
        ),
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        item("summary") { PlanSummary(planner, plan, rows.size) }
        if (plan.needed != null) {
            item("target") {
                TargetCard(plan, Modifier.animateItem()) { target ->
                    planner.target = target
                    act("target", mapOf("target" to target.fixed2()))
                }
            }
        }
        item("courses") {
            SectionHeader("Courses", trailing = {
                TextButton({ planner.loadSemester(); act("load", mapOf("by" to "button")) }, Modifier.testTag("grades.plan.semester"), enabled = planner.semester.isNotEmpty()) {
                    Icon(Lucide.CalendarDays, null, Modifier.size(16.dp))
                    Spacer(Modifier.width(Space.xs))
                    Text("Current semester")
                }
            })
        }
        itemsIndexed(rows, key = { _, row -> row.id }) { i, row ->
            PlanCourseCard(
                i,
                row,
                canRemove = rows.size > 1,
                onCredits = { c -> planner.update(row.id) { it.copy(credits = c) }; act("credits", mapOf("credits" to c.creditsLabel())) },
                onGrade = { g -> planner.update(row.id) { it.copy(grade = g) }; act("grade", mapOf("grade" to g)) },
                onRemove = { planner.rows.removeAll { it.id == row.id }; act("remove") },
                Modifier.animateItem(),
            )
        }
        item("add") {
            Button({ planner.add(); act("add") }, Modifier.fillMaxWidth().testTag("grades.plan.add"), ButtonVariant.Outline, prefix = Lucide.Plus) { Text("Add course") }
        }
    }
}

/** Semester GPA in a ring in its letter's tone; the CGPA now → after with the change; the base it counts from. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun PlanSummary(planner: Planner, plan: GpaPlan, count: Int) {
    val scheme = MaterialTheme.colorScheme
    val tone = gradeTone(plan.letter)
    val label = MaterialTheme.typography.labelSmall
    val ring by animateFloatAsState((plan.semesterGpa / 10).toFloat(), MaterialTheme.motionScheme.defaultSpatialSpec())
    SectionCard(Modifier.fillMaxWidth().testTag("grades.plan.summary")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(88.dp), Alignment.Center) {
                CircularWavyProgressIndicator(progress = { ring.coerceIn(0f, 1f) }, Modifier.size(88.dp), color = tone.base, trackColor = tone.subtle)
                CountUp(plan.semesterGpa, MaterialTheme.typography.titleLargeEmphasized.nums(), Modifier.testTag("grades.plan.gpa"))
            }
            Spacer(Modifier.width(Space.lg))
            Column(Modifier.weight(1f)) {
                Text("SEMESTER GPA", style = label, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
                Text("$count ${if (count == 1) "course" else "courses"} · ${plan.credits.creditsLabel()} credits", style = MaterialTheme.typography.bodyMedium)
                Spacer(Modifier.height(Space.md))
                Text("PROJECTED CGPA", style = label, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
                val current = plan.current
                val projected = plan.projected
                if (current == null || projected == null) {
                    Text("—", style = MaterialTheme.typography.titleLarge, color = scheme.onSurfaceVariant)
                } else {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(current.fixed2(), style = MaterialTheme.typography.bodyLarge.nums(), color = scheme.onSurfaceVariant)
                        Icon(Lucide.ArrowRight, null, Modifier.padding(horizontal = Space.xs).size(14.dp), tint = scheme.onSurfaceVariant)
                        CountUp(projected, MaterialTheme.typography.titleLargeEmphasized.nums(), Modifier.testTag("grades.plan.projected"))
                        if (Math.abs(plan.delta) >= 0.005) {
                            Spacer(Modifier.width(Space.sm))
                            val up = plan.delta > 0
                            ToneBadge("${if (up) "+" else "−"}${Math.abs(plan.delta).fixed2()}", tone = if (up) AppTheme.tones.success else AppTheme.tones.danger)
                        }
                    }
                }
            }
        }
        Spacer(Modifier.height(Space.md))
        Row(
            Modifier.fillMaxWidth().tappable({ planner.editBase = !planner.editBase }, "Edit current CGPA and credits", Role.Button).testTag("grades.plan.base"),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(Lucide.History, null, Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
            Spacer(Modifier.width(6.dp))
            Text(
                if (plan.current == null) "Current CGPA and credits" else "CGPA ${planner.cgpa} over ${planner.completed.ifEmpty { "0" }} credits",
                Modifier.weight(1f),
                style = MaterialTheme.typography.labelMedium,
                color = scheme.onSurfaceVariant,
            )
            Text(if (planner.editBase) "Done" else "Edit", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, color = AppTheme.tones.accentTone.onSubtle)
        }
        val motion = MaterialTheme.motionScheme
        AnimatedVisibility(
            planner.editBase || plan.current == null,
            enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(motion.defaultEffectsSpec()),
            exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut(motion.fastEffectsSpec()),
        ) {
            Row(Modifier.padding(top = Space.md), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                OutlinedTextField(
                    planner.cgpa,
                    { planner.cgpa = it },
                    Modifier.weight(1f).testTag("grades.plan.cgpa"),
                    label = { Text("Current CGPA") },
                    placeholder = { Text("8.35") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                )
                OutlinedTextField(
                    planner.completed,
                    { planner.completed = it },
                    Modifier.weight(1f).testTag("grades.plan.completed"),
                    label = { Text("Completed credits") },
                    placeholder = { Text("54") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                )
            }
        }
    }
}

/** The target CGPA with a stepper of 0.05, and the semester GPA it needs. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun TargetCard(plan: GpaPlan, modifier: Modifier, onTarget: (Double) -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val needed = plan.needed ?: return
    SectionCard(modifier.fillMaxWidth().testTag("grades.plan.target")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("TARGET CGPA", Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
            IconButton(Lucide.Minus, if (plan.target > 5) ({ onTarget(plan.target - 0.05) }) else null, "Lower target", Modifier.testTag("grades.plan.target.less"))
            Text(plan.target.fixed2(), Modifier.width(56.dp), style = MaterialTheme.typography.titleLargeEmphasized.nums(), textAlign = TextAlign.Center)
            IconButton(Lucide.Plus, if (plan.target < 9.995) ({ onTarget(minOf(10.0, plan.target + 0.05)) }) else null, "Raise target", Modifier.testTag("grades.plan.target.more"))
        }
        val style = MaterialTheme.typography.bodyMedium.nums()
        when (plan.outlook) {
            TargetOutlook.REACHED -> Text("Already there, whatever you score", style = style, color = tones.success.onSubtle)
            TargetOutlook.ON_TRACK -> Text("Your plan gets you there", style = style, color = tones.success.onSubtle)
            TargetOutlook.OUT_OF_REACH -> Text("Out of reach this semester (needs ${needed.fixed2()})", style = style, color = tones.danger.onSubtle)
            else -> Text(
                buildAnnotatedString {
                    append("Needs a ")
                    withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = scheme.onSurface)) { append(needed.fixed2()) }
                    append(" semester GPA")
                },
                Modifier.testTag("grades.plan.needed"),
                style = style,
                color = scheme.onSurfaceVariant,
            )
        }
    }
}
