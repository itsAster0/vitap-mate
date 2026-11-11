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
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
import com.composables.icons.lucide.Check
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import vitapmate.ui.formatDay
import androidx.compose.runtime.mutableStateOf
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.ChevronUp
import vitapmate.core.PlanDay
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
import kotlin.math.floor
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.observability.Log
import vitapmate.observability.logAction
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
 * counts at the chosen goal (75–90%), and the best reachable; "Try it" plays that split out in
 * What if. What if also takes on-duty approvals still to come (absences turned present) and the
 * upcoming days marked to attend or miss. Labs count in sessions.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Calculator(course: AttendanceCourse, plans: List<AttendancePlan>, modifier: Modifier = Modifier) {
    val scope = rememberCoroutineScope()
    val tones = AppTheme.tones
    var attend by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var skip by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var od by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var picked by rememberSaveable(course.key) { mutableStateOf(emptyList<Int>()) }
    var going by rememberSaveable(course.key) { mutableStateOf(emptyList<Int>()) }
    var target by rememberSaveable(course.key) { mutableIntStateOf(0) }
    var goal by rememberSaveable(course.key) { mutableIntStateOf(75) }
    var calendar by rememberSaveable(course.key) { mutableStateOf(false) }
    // The days still to come (till the semester's end), for marking the ones to attend ([going]) or miss ([picked]).
    val days = plans.lastOrNull()?.days.orEmpty()
    val pickedClasses = days.filter { it.day in picked }.sumOf { it.classes }
    val missing = skip + pickedClasses
    val goingClasses = days.filter { it.day in going }.sumOf { it.classes }
    val attending = attend + goingClasses
    val log: (String, Map<String, String>) -> Unit = { what, fields ->
        scope.logAction("attendance.detail.calc", "ui.attendance", "calc $what ${course.key}", fields + ("course" to course.key))
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
            val reset = attending > 0 || missing > 0 || od > 0
            val (percent, exactSafe) = course.projected(attending, missing, od)
            val now = course.projected(0, 0).first
            val tone = when {
                percent < 75 -> tones.danger
                exactSafe -> tones.success
                else -> tones.warning
            }
            val card = AppTheme.colors.card
            // The card takes the tone the what-if lands in; neutral while nothing is changed.
            val fill = if (reset) tone.subtle else card
            val ink = if (reset) tone.onSubtle else MaterialTheme.colorScheme.onSurface
            val (attended, counted) = course.sessions
            val unposted = if (course.isLab) (course.total + 1) / 2 - counted else course.total - counted
            Column(Modifier.fillMaxWidth().background(fill, RoundedCornerShape(32.dp)).padding(6.dp)) {
                Column(Modifier.padding(start = Space.md, end = Space.md, top = Space.xs, bottom = Space.lg)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("What if", Modifier.weight(1f), style = MaterialTheme.typography.titleMediumEmphasized, color = ink)
                        // Always there (disabled at zero), so the card doesn't jump when it turns on.
                        TextButton({ attend = 0; skip = 0; od = 0; picked = emptyList(); going = emptyList(); log("reset", emptyMap()) }, Modifier.testTag("attendance.detail.calc.reset"), enabled = reset) { Text("Reset") }
                    }
                    Text(
                        "Using $attended of $counted $unit" + if (unposted > 0) " · $unposted not posted" else "",
                        Modifier.testTag("attendance.detail.calc.basis"),
                        style = MaterialTheme.typography.labelLarge.nums(),
                        color = ink.copy(alpha = 0.85f),
                    )
                    Row(Modifier.padding(top = 6.dp), verticalAlignment = Alignment.Bottom) {
                        Text(
                            buildAnnotatedString {
                                append("$percent")
                                withStyle(SpanStyle(fontSize = 30.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.6).sp)) { append("%") }
                            },
                            Modifier.testTag("attendance.detail.calc.result"),
                            style = MaterialTheme.typography.displayLargeEmphasized.nums(fontSize = 64.sp, lineHeight = 62.sp, letterSpacing = (-3.5).sp),
                            color = ink,
                            maxLines = 1,
                        )
                        if (percent != now) {
                            Text(
                                if (percent > now) "+${percent - now}" else "−${now - percent}",
                                Modifier.padding(start = Space.md, bottom = Space.sm)
                                    .background(tone.base, CircleShape)
                                    .padding(horizontal = 10.dp, vertical = 3.dp)
                                    .testTag("attendance.detail.calc.delta"),
                                style = MaterialTheme.typography.labelLargeEmphasized.nums(fontWeight = FontWeight.W800),
                                color = tone.subtle,
                            )
                        }
                        Spacer(Modifier.weight(1f))
                        if (reset) {
                            Text(
                                "${attended + attending + od} of ${counted + attending + missing}",
                                Modifier.padding(bottom = Space.sm),
                                style = MaterialTheme.typography.titleSmall.nums(),
                                fontWeight = FontWeight.SemiBold,
                                color = ink,
                            )
                        }
                    }
                    AttendanceLine(
                        percent.toDouble(),
                        tone.base,
                        Modifier.padding(top = Space.lg),
                        thickness = 14.dp,
                        from = if (reset) course.percent else null,
                        track = ink.copy(alpha = 0.14f),
                        halo = fill,
                    )
                }
                // The controls on a card-coloured panel inside.
                Column(Modifier.fillMaxWidth().background(card, RoundedCornerShape(26.dp)).padding(start = 14.dp, end = 14.dp, top = 6.dp, bottom = 14.dp)) {
                    val hairline = MaterialTheme.colorScheme.surfaceContainerHigh
                    Stepper("Attend", attend, { attend = it; log("attend", mapOf("attend" to "$it", "skip" to "$skip")) }, "attendance.detail.calc.attend", Modifier.padding(vertical = Space.sm))
                    HorizontalDivider(color = hairline)
                    Stepper("Skip", skip, { skip = it; log("skip", mapOf("attend" to "$attend", "skip" to "$it")) }, "attendance.detail.calc.skip", Modifier.padding(vertical = Space.sm))
                    // On duty already held but marked absent: VTOP turns them present when the OD posts.
                    val absences = counted - attended
                    if (absences > 0) {
                        HorizontalDivider(color = hairline)
                        Stepper("OD pending", od, { od = it; log("od", mapOf("od" to "$it")) }, "attendance.detail.calc.od", Modifier.padding(vertical = Space.sm), max = absences)
                    }
                    if (days.isNotEmpty()) {
                        HorizontalDivider(color = hairline)
                        MarkDays(days, going, picked, goingClasses, pickedClasses, calendar, { calendar = it; log("calendar", mapOf("open" to "$it")) }) { d ->
                            // Each tap moves a day on: unmarked, attend, miss, unmarked.
                            val next = when (d) {
                                in going -> { going = going - d; picked = picked + d; "miss" }
                                in picked -> { picked = picked - d; "none" }
                                else -> { going = going + d; "attend" }
                            }
                            log("day", mapOf("day" to formatDay(d, "yyyy-MM-dd"), "mark" to next))
                        }
                    }
                }
            }
        }
        if (plans.isNotEmpty()) item("plan") {
            val shown = target.coerceIn(0, plans.lastIndex)
            val plan = plans[shown]
            val left = plan.upcoming + plan.unposted
            // Strict arithmetic at the chosen goal, like Rust's 75% plan.
            val total = plan.held + left
            val need = ((goal * total - 100 * plan.attended + 99).floorDiv(100)).coerceAtLeast(0)
            val must = need.takeIf { it <= left }
            val canMiss = (left - need).coerceIn(0, left)
            SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.calc.plan"), title = "To finish at $goal%", shape = MaterialTheme.shapes.extraLargeIncreased) {
                ConnectedChoice(
                    GOALS,
                    goal,
                    { g -> goal = g; log("goal", mapOf("goal" to "$g")) },
                    tag = "attendance.detail.calc.goal",
                    key = { "$it" },
                    label = { "$it%" },
                    modifier = Modifier.padding(bottom = Space.md),
                )
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
                    canMiss == 0 -> tones.warning
                    else -> tones.success
                }
                if (left > 0) LeftStrip(left, must ?: left, tone, Modifier.padding(bottom = Space.lg))
                Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                    PlanCount(
                        if (must == null) "–" else "$must",
                        if (must == null) "can't reach $goal%" else "must attend",
                        if (must == null) tones.danger.base else MaterialTheme.colorScheme.onSurface,
                        MaterialTheme.colorScheme.onSurfaceVariant,
                        Modifier.weight(1f).fillMaxHeight().background(MaterialTheme.colorScheme.surfaceContainer, connected(0, 2, 22.dp, 8.dp)).testTag("attendance.detail.calc.must"),
                    )
                    PlanCount(
                        "$canMiss",
                        "can miss",
                        tone.onSubtle,
                        tone.onSubtle,
                        Modifier.weight(1f).fillMaxHeight().background(tone.subtle, connected(1, 2, 22.dp, 8.dp)).testTag("attendance.detail.calc.miss"),
                    )
                }
                Row(Modifier.padding(top = Space.md), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        listOfNotNull(
                            "$left $unit left",
                            "${plan.unposted} not posted".takeIf { plan.unposted > 0 },
                            "best ${floor(plan.bestPercent).toInt()}%",
                        ).joinToString(" · "),
                        Modifier.weight(1f).testTag("attendance.detail.calc.left"),
                        style = MaterialTheme.typography.bodySmall.nums(),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (must != null && left > 0) {
                        FilledTonalButton(
                            { attend = must; skip = canMiss; od = 0; picked = emptyList(); going = emptyList(); log("try", mapOf("attend" to "$must", "skip" to "$canMiss", "goal" to "$goal")) },
                            Modifier.testTag("attendance.detail.calc.try"),
                        ) { Text("Try it") }
                    }
                }
            }
        }
    }
}

/** The finishing targets on offer; VTOP's line is 75%. */
private val GOALS = listOf(75, 80, 85, 90)

/**
 * The classes left as a strip of cells: the first [must] filled in [tone] (to attend), the rest
 * hollow (can be missed). Too many to tell apart: one bar split in the same two parts.
 */
@Composable
private fun LeftStrip(left: Int, must: Int, tone: Tone, modifier: Modifier = Modifier) {
    val outline = tone.base.copy(alpha = 0.55f)
    Canvas(modifier.fillMaxWidth().height(22.dp).testTag("attendance.detail.calc.strip")) {
        val gap = 3.dp.toPx()
        val r = CornerRadius(6.dp.toPx())
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

/** A large count over its label, in a tile its [modifier] fills. */
@Composable
private fun PlanCount(value: String, label: String, color: Color, labelColor: Color, modifier: Modifier = Modifier) = Column(modifier.padding(horizontal = 14.dp, vertical = Space.md)) {
    Text(value, style = MaterialTheme.typography.headlineLargeEmphasized.nums(fontSize = 34.sp, fontWeight = FontWeight.W800, lineHeight = 36.sp), color = color)
    Text(label, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium, color = labelColor)
}

/**
 * The upcoming class [days] to mark: a tap marks a day attended ([going], green with a tick), a
 * second missed ([picked], red with a cross), a third clears it. "Attending N, missing M" over
 * "Next 7 days" and a Calendar / Week toggle. Collapsed: the days within a week of the first as
 * chips in one scrolling row, and any marked later on an "Also marked later" line. Open ([calendar]): every
 * class day on month grids, "×N" on days with several classes. Days tagged
 * `attendance.detail.calc.day.<day>`.
 */
@Composable
private fun MarkDays(days: List<PlanDay>, going: List<Int>, picked: List<Int>, goingClasses: Int, pickedClasses: Int, calendar: Boolean, onCalendar: (Boolean) -> Unit, onToggle: (Int) -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val classes = days.associate { it.day to it.classes }
    val tones = AppTheme.tones
    val toneOf = { d: Int -> if (d in going) tones.success else if (d in picked) tones.danger else null }
    val chip = @Composable { d: Int, label: String ->
        val tone = toneOf(d)
        FilterChip(
            tone != null,
            { onToggle(d) },
            { Text(label + if ((classes[d] ?: 0) > 1) " ×${classes[d]}" else "", style = MaterialTheme.typography.labelLarge.nums()) },
            Modifier.testTag("attendance.detail.calc.day.$d"),
            leadingIcon = tone?.let { { Icon(if (d in going) Lucide.Check else Lucide.X, null, Modifier.size(16.dp)) } },
            colors = FilterChipDefaults.filterChipColors(
                selectedContainerColor = tone?.subtle ?: Color.Unspecified,
                selectedLabelColor = tone?.onSubtle ?: Color.Unspecified,
                selectedLeadingIconColor = tone?.base ?: Color.Unspecified,
            ),
        )
    }
    Row(Modifier.padding(top = 10.dp, bottom = Space.sm), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(
                listOfNotNull("attending $goingClasses".takeIf { goingClasses > 0 }, "missing $pickedClasses".takeIf { pickedClasses > 0 })
                    .joinToString(", ").replaceFirstChar { it.uppercase() }.ifEmpty { "Days you'll attend or miss" },
                style = MaterialTheme.typography.titleSmall,
            )
            Text((if (calendar) "Every class day" else "Next 7 days") + " · tap to attend, again to miss", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
        }
        FilledTonalIconButton({ onCalendar(!calendar) }, Modifier.testTag("attendance.detail.calc.calendar")) {
            Icon(if (calendar) Lucide.ChevronUp else Lucide.CalendarDays, if (calendar) "Week" else "Calendar", Modifier.size(20.dp))
        }
    }
    val first = days.first().day
    if (calendar) {
        PlanCalendar(days.map { it.day }, "attendance.detail.calc.day", toneOf, onToggle) { d, tone ->
            val n = classes[d] ?: 0
            if (n > 1) {
                Text(
                    "×$n",
                    Modifier.align(Alignment.TopEnd).padding(top = 4.dp, end = 5.dp),
                    style = MaterialTheme.typography.labelSmall.nums(fontSize = 9.sp, fontWeight = FontWeight.Bold),
                    color = tone?.onSubtle ?: scheme.onSurfaceVariant,
                )
            }
        }
        return
    }
    LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        items(days.filter { it.day < first + 7 }, key = { it.day }) { d -> chip(d.day, formatDay(d.day, "EEE d")) }
    }
    val later = (going + picked).filter { it >= first + 7 }.sorted()
    if (later.isNotEmpty()) {
        FlowRow(Modifier.padding(top = Space.sm), horizontalArrangement = Arrangement.spacedBy(6.dp), itemVerticalAlignment = Alignment.CenterVertically) {
            Text("Also marked later:", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            for (d in later) chip(d, formatDay(d, "EEE d MMM"))
        }
    }
}
