package vitapmate.features.wrapped

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.GenericShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.ui.rememberHaptics
import kotlin.math.roundToInt
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedTitle

// The title, the summary card and its receipt.

/** Why [t] fits, from the numbers; null when there's nothing to add. */
internal fun reason(t: WrappedTitle, r: WrappedReport): String? = when (t) {
    WrappedTitle.TOPPER -> r.grades?.let { "A ${gpa(it.gpa)} GPA this semester" }
    WrappedTitle.GHOST_GENIUS -> r.grades?.let { g -> r.attendance?.let { "${it.percent}% attendance, and still a ${gpa(g.gpa)} GPA" } }
    WrappedTitle.EXAM_CLUTCH -> r.marks?.catChange?.let { "Up $it points from CAT-1 to CAT-2, on average" }
    WrappedTitle.PERFECTIONIST -> r.fullMarks?.let { "${count(it.count, "full mark")} out of ${count(it.of, "assessment")}" }
    WrappedTitle.COMEBACK -> r.comeback?.let { "From ${it.lowPercent}% in ${month(it.lowDay)} to ${it.finalPercent}% at the end" }
    WrappedTitle.EDGE_WALKER -> r.closeCall?.let { c -> "${titled(c.course)} got down to ${count(c.spare ?: 0, "class", "classes")} to spare, and you never slipped under 75%" }
    WrappedTitle.IRON_STREAK -> r.streak?.let { s -> "${s.length} classes in a row without a miss" + (s.total.takeIf { it >= s.length }?.let { ", ${(s.length * 100f / it).roundToInt()}% of all $it" } ?: "") }
    WrappedTitle.EARLY_REGULAR -> r.habits?.earlyPercent?.let { "$it% of your 8 AM classes, attended" }
    WrappedTitle.REGULAR -> r.attendance?.let { "${it.percent}% attendance, all semester" }
    WrappedTitle.CLUB_CAPTAIN -> r.onDuty?.let { "${it.classes} classes on OD across ${count(it.days, "day")}" }
    WrappedTitle.LAB_RAT -> r.time?.let { "${(it.labMinutes + 30) / 60} of your ${(it.minutes + 30) / 60} class hours in labs" }
    WrappedTitle.EXPLORER -> r.outings?.let { "${it.leaves + it.weekends} trips off campus" }
    WrappedTitle.STAIRMASTER -> r.moves?.let { "${it.floors} floors climbed between classes" }
    WrappedTitle.NIGHT_OWL -> r.dayHalf?.let { "${it.percent}% of your classes started at 2 PM or later" }
    WrappedTitle.STEADY_CRUISER -> r.attendance?.let { "${it.percent}% across ${count(r.courses, "course")}, no drama" } ?: "You did this semester your own way"
}

/** The short line for [t]: the summary's title row and the traits. */
private fun short(t: WrappedTitle, r: WrappedReport): String = when (t) {
    WrappedTitle.TOPPER -> r.grades?.let { "GPA ${gpa(it.gpa)}" }
    WrappedTitle.GHOST_GENIUS -> r.grades?.let { g -> r.attendance?.let { "${it.percent}%, GPA ${gpa(g.gpa)}" } }
    WrappedTitle.EXAM_CLUTCH -> r.marks?.catChange?.let { "CAT-2 up $it" }
    WrappedTitle.PERFECTIONIST -> r.fullMarks?.let { count(it.count, "full mark") }
    WrappedTitle.COMEBACK -> r.comeback?.let { "${it.lowPercent}% to ${it.finalPercent}%" }
    WrappedTitle.EDGE_WALKER -> r.closeCall?.let { "${count(it.spare ?: 0, "class", "classes")} to spare" }
    WrappedTitle.IRON_STREAK -> r.streak?.let { "${it.length} in a row" }
    WrappedTitle.EARLY_REGULAR -> r.habits?.earlyPercent?.let { "8 AM: $it%" }
    WrappedTitle.REGULAR -> r.attendance?.let { "${it.percent}% attendance" }
    WrappedTitle.CLUB_CAPTAIN -> r.onDuty?.let { "${it.classes} classes on OD" }
    WrappedTitle.LAB_RAT -> r.time?.let { "${(it.labMinutes + 30) / 60} h of labs" }
    WrappedTitle.EXPLORER -> r.outings?.let { count(it.leaves + it.weekends, "trip") }
    WrappedTitle.STAIRMASTER -> r.moves?.let { "${it.floors} floors" }
    WrappedTitle.NIGHT_OWL -> r.dayHalf?.let { "${it.percent}% after 2 PM" }
    WrappedTitle.STEADY_CRUISER -> r.attendance?.let { "${it.percent}% attendance" }
}.orEmpty()

/** Each title's shape on the traits list. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun shapeOf(i: Int) = listOf(MaterialShapes.Cookie9Sided, MaterialShapes.Clover8Leaf, MaterialShapes.SoftBurst)[i % 3].toShape()

/**
 * Your title, sealed until held (or tapped): a ring fills as it's held, then the title, why,
 * the runner-up as its "energy", and the top three traits the share's privacy leaves. A share
 * image draws it unsealed. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun titleSlide(r: WrappedReport) = Slide(
    "title",
    Ink.Accent,
    grades = r.traits.take(3).any { it.grades },
    fixed = false,
    pending = Pending("Tap to unseal", { !it.unsealed }) { it.unsealed = true },
) { c ->
    AnimatedContent(
        c.unsealed,
        Modifier.fillMaxSize(),
        transitionSpec = { (fadeIn() + scaleIn(initialScale = 0.85f)) togetherWith (fadeOut() + scaleOut(targetScale = 1.15f)) },
        label = "unseal",
    ) { open ->
        if (open) Unsealed(r, c) else Sealed(c.state)
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Sealed(state: StoryState) {
    val ink = LocalInks.current
    val haptics = rememberHaptics()
    val scope = rememberCoroutineScope()
    val held = remember { Animatable(0f) }
    val unseal = {
        if (!state.unsealed) {
            haptics.confirm()
            state.unsealed = true
        }
    }
    Flow {
        Label("One more before the summary")
        H("Your semester\nearned a title", fits(36f, 30f), Modifier.padding(top = 6.dp))
        P("Press and hold to unseal", 15f, top = 8)
        BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(vertical = 12.dp).heightIn(min = 140.dp), Alignment.Center) {
            val s = minOf(maxWidth * 0.84f, maxHeight)
            Box(
                Modifier.size(s).testTag("wrapped.unseal")
                    .semantics {
                        role = Role.Button
                        contentDescription = "Unseal your title"
                        onClick { unseal(); true }
                    }
                    .pointerInput(Unit) {
                        detectTapGestures(
                            onPress = {
                                // Ticks as the ring fills, the threshold when full; a tap (let go
                                // early) unseals too.
                                val fill = scope.launch {
                                    var ticked = 0
                                    held.animateTo(1f, tween(600, easing = LinearEasing)) {
                                        val step = (value * 3).toInt()
                                        if (step > ticked && step < 3) {
                                            ticked = step
                                            haptics.tick()
                                        }
                                    }
                                    haptics.threshold()
                                    unseal()
                                }
                                tryAwaitRelease()
                                if (!state.unsealed) {
                                    fill.cancel()
                                    unseal()
                                }
                            },
                        )
                    },
                Alignment.Center,
            ) {
                Canvas(Modifier.fillMaxSize()) {
                    val stroke = 8.dp.toPx()
                    val inset = stroke / 2 + 4.dp.toPx()
                    val arc = Size(size.width - inset * 2, size.height - inset * 2)
                    drawArc(ink.text.copy(alpha = 0.16f), 0f, 360f, false, Offset(inset, inset), arc, style = Stroke(stroke))
                    if (held.value > 0f) drawArc(ink.shape, -90f, 360f * held.value, false, Offset(inset, inset), arc, style = Stroke(stroke, cap = StrokeCap.Round))
                }
                val seal = MaterialShapes.Cookie9Sided.toShape()
                Box(Modifier.size(s * 0.72f).rotate(-8f).clip(seal).background(ink.mid), Alignment.Center) {
                    // Hatching across the seal.
                    Canvas(Modifier.fillMaxSize()) {
                        val step = 14.dp.toPx()
                        var x = 0f
                        while (x < size.width * 2) {
                            drawLine(ink.text.copy(alpha = 0.1f), Offset(x, 0f), Offset(x - size.height, size.height), 2.dp.toPx())
                            x += step
                        }
                    }
                    Icon(Lucide.Lock, null, Modifier.size(s * 0.22f), tint = ink.text)
                }
                Box(Modifier.align(Alignment.TopEnd).offset(x = (-s * 0.02f), y = s * 0.12f).size(s * 0.16f).rotate(10f).background(ink.shape, MaterialShapes.SoftBurst.toShape()))
                Box(Modifier.align(Alignment.BottomStart).offset(x = s * 0.04f, y = -s * 0.14f).size(s * 0.1f).background(ink.text, MaterialShapes.Cookie6Sided.toShape()))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Unsealed(r: WrappedReport, c: SlideContext) {
    val ink = LocalInks.current
    val earned = r.shownTraits(c.privacy)
    // The Steady Cruiser only when nothing else is left.
    val traits = earned.filter { it.title != WrappedTitle.STEADY_CRUISER }.ifEmpty { earned.take(1) }
    val title = traits.firstOrNull()?.title ?: WrappedTitle.STEADY_CRUISER
    Flow {
        Label(if (c.share) "This semester, you were" else "Unsealed. You are")
        val label = title.label()
        val base = fits(if (label.count { it == ' ' } >= 2) 46f else 54f, 42f)
        val burst = fits(92f, 72f)
        val measurer = rememberTextMeasurer()
        BoxWithConstraints(Modifier.fillMaxWidth().padding(top = 6.dp)) {
            // The longest word on one line: shrink it to fit beside the burst, or drop the burst
            // when that would make it small.
            val room = constraints.maxWidth.toFloat()
            val word = label.split(' ').maxBy { it.length }
            val widest = measurer.measure(word, type(base), softWrap = false).size.width.toFloat()
            val beside = room - with(LocalDensity.current) { (burst + 8f).dp.toPx() }
            val side = widest * 0.8f <= beside
            val size = base * minOf(1f, (if (side) beside else room) / widest)
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
                H(label.replaceFirst(" ", "\n"), size, Modifier.weight(1f))
                if (side) Box(Modifier.padding(start = 8.dp, top = 4.dp).size(burst.dp).rotate(10f).background(ink.shape, MaterialShapes.SoftBurst.toShape()))
            }
        }
        reason(title, r)?.let { P(it, 15f, top = 12) }
        traits.getOrNull(1)?.let { next ->
            Row(
                Modifier.padding(top = 12.dp).background(ink.shape, RoundedCornerShape(16.dp)).padding(horizontal = 12.dp, vertical = 7.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Box(Modifier.size(16.dp).background(ink.onShape, MaterialShapes.Clover8Leaf.toShape()))
                Text("with ${next.title.label().removePrefix("The ")} energy", color = ink.onShape, style = type(13f, 700, -0.01f, 1.2f), maxLines = 2)
            }
        }
        Spacer(Modifier.weight(1f).heightIn(min = 12.dp))
        if (traits.size > 1) Column(Modifier.fillMaxWidth().background(ink.panel, RoundedCornerShape(28.dp)).padding(horizontal = 16.dp, vertical = 12.dp)) {
            Text("Your top traits", color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, -0.01f, 1.2f))
            traits.take(3).forEachIndexed { i, t ->
                Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Box(Modifier.size(26.dp).background(ink.shape, shapeOf(i)))
                    Column(Modifier.weight(1f)) {
                        Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(t.title.label().removePrefix("The "), Modifier.weight(1f), color = ink.text, style = type(13.5f, 800, -0.02f, 1.2f), maxLines = 2)
                            Text(short(t.title, r), color = ink.text.copy(alpha = 0.75f), style = type(11f, 600, 0f, 1.2f), maxLines = 1)
                        }
                        // How strongly the facts earn it.
                        Box(Modifier.padding(top = 5.dp).fillMaxWidth().height(8.dp).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(4.dp))) {
                            Box(Modifier.fillMaxWidth(t.score.coerceIn(4, 100) / 100f).fillMaxHeight().background(ink.shape, RoundedCornerShape(4.dp)))
                        }
                    }
                }
            }
        }
    }
}

/** Slides whose tiles the summary shows first, in this order; the others' follow in story order. */
private val SUMMARY_ORDER = listOf("attendance", "mosaic", "streak", "arc", "rhythm", "grades", "degree", "exams", "time", "ffcs", "outings", "roi", "traps", "mixtape")

/** The summary's rows of six columns: enough to read at a glance without shrinking. */
private const val SUMMARY_ROWS = 5

/**
 * The best tiles of the slides above it ([SUMMARY_ORDER] first), up to [SUMMARY_ROWS] rows, under
 * the title; or the semester as a receipt. The share's privacy leaves out grade and hostel tiles
 * (the next ones take their place) and the name.
 */
internal fun summarySlide(story: List<Slide>) = Slide("summary", null, grades = story.any { s -> s.tiles.any { it.grades } } || story.any { it.name == "title" && it.grades }, semesterLine = false, fixed = false) { c ->
    val r = c.report
    val frame = LocalFrame.current
    if (c.state.receipt) {
        Box(Modifier.fillMaxSize().padding(top = 18.dp, bottom = 6.dp), Alignment.TopCenter) {
            Box(Modifier.fitHeight((frame.height - 24f).dp)) { Receipt(r, c) }
        }
        return@Slide
    }
    val ink = LocalInks.current
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val t = r.shownTitle(c.privacy)
    val ranked = story.sortedBy { s -> SUMMARY_ORDER.indexOf(s.name).let { if (it < 0) SUMMARY_ORDER.size else it } }
    val tiles = rows(ranked.flatMap { it.tiles }.filter { !(c.hide && it.grades) && !(c.privacy.hostel && it.hostel) }).take(SUMMARY_ROWS)
    // One column, so a title that takes two lines pushes the tiles down; scaled down, centred,
    // only if still taller than the frame.
    Column(Modifier.padding(start = 14.dp, top = 28.dp, end = 14.dp).fillMaxWidth().fitHeight((frame.height - 36f).dp), verticalArrangement = Arrangement.spacedBy(5.dp)) {
      Column(Modifier.padding(start = 10.dp, end = 10.dp, bottom = 5.dp)) {
        Text("${c.shownName?.let { "$it's" } ?: "Your"} ${r.semesterName}", color = ink.text, style = type(23f, 800, -0.04f, 1.1f), maxLines = 2, overflow = TextOverflow.Ellipsis)
        val line = listOfNotNull(
            "${day(r.firstDay)} to ${day(r.lastDay)}",
            r.courses.takeIf { it > 0 }?.let { count(it, "course") },
            r.classDayCount.takeIf { it > 0 }?.let { count(it, "class day") },
        ).joinToString(", ")
        Text(line, Modifier.padding(top = 4.dp), color = muted, style = type(11.5f, 500, 0f, 1.3f))
      }
        val accent = inks(Ink.Accent)
        Row(
            Modifier.fillMaxWidth().background(accent.shape, RoundedCornerShape(18.dp)).padding(horizontal = 11.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(t.label(), Modifier.weight(1f), color = accent.onShape, style = type(22f, 800, -0.04f, 1.1f))
            Text(short(t, r), color = accent.onShape, style = type(11f, 600, 0f, 1.25f), textAlign = TextAlign.End)
        }
        tiles.forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                row.forEach { tile -> TileView(tile, Modifier.weight(tile.span.toFloat())) }
            }
        }
    }
}

/** "OPERATING SYSTEMS", wrapping to a second line rather than cut. */
private fun receiptName(name: String, lab: Boolean) = name.trim().uppercase().let { if (lab && !it.endsWith("LAB")) "$it LAB" else it }

/**
 * The semester as a till receipt: each course part's classes and attendance, the semester's
 * totals as charges, the title as the total, a barcode. In the theme's inverse colours, with
 * zigzag edges. Hide grades leaves out the GPA lines, Hide name the name.
 */
@Composable
private fun Receipt(r: WrappedReport, c: SlideContext) {
    val paper = MaterialTheme.colorScheme.inverseSurface
    val ink = MaterialTheme.colorScheme.inverseOnSurface
    val mono = type(11f, 500, -0.02f, 1.5f).copy(fontFamily = FontFamily.Monospace, color = ink)
    val bold = mono.copy(fontWeight = FontWeight.Bold)
    @Composable
    fun Line(a: String, b: String) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
            Text(a, style = mono, maxLines = 2)
            Canvas(Modifier.weight(1f).height(10.dp).padding(horizontal = 4.dp)) {
                drawLine(ink.copy(alpha = 0.6f), Offset(0f, size.height - 2.dp.toPx()), Offset(size.width, size.height - 2.dp.toPx()), 1.5.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(1.5.dp.toPx(), 3.dp.toPx())))
            }
            Text(b, style = mono, maxLines = 1)
        }
    }
    @Composable
    fun Rule(double: Boolean = false) {
        Canvas(Modifier.fillMaxWidth().padding(vertical = 7.dp).height(if (double) 4.dp else 2.dp)) {
            val dash = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 3.dp.toPx()))
            drawLine(ink.copy(alpha = 0.8f), Offset(0f, 0f), Offset(size.width, 0f), 1.5.dp.toPx(), pathEffect = if (double) null else dash)
            if (double) drawLine(ink.copy(alpha = 0.8f), Offset(0f, size.height), Offset(size.width, size.height), 1.5.dp.toPx())
        }
    }
    val a = r.attendance
    val t = r.shownTitle(c.privacy)
    Column(
        Modifier.width(292.dp).rotate(-1.6f).background(paper, Zigzag).padding(horizontal = 18.dp, vertical = 22.dp).testTag("wrapped.receipt"),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("vitapmate", color = ink, style = type(22f, 800, -0.04f, 1.1f))
        val name = c.shownName?.uppercase()
        Text(
            listOfNotNull(
                r.semesterName.uppercase(),
                listOfNotNull(name, "VIT-AP AMARAVATI").joinToString(" · "),
                "${day(r.firstDay).uppercase()} → ${day(r.lastDay).uppercase()} · ${r.spanDays} DAYS",
            ).joinToString("\n"),
            Modifier.padding(top = 4.dp), style = mono.copy(fontSize = 10.5.sp, textAlign = TextAlign.Center),
        )
        Rule()
        if (a != null && a.lines.isNotEmpty()) {
            Row(Modifier.fillMaxWidth()) {
                Text("QTY", Modifier.width(34.dp), style = bold)
                Text("COURSE", Modifier.weight(1f), style = bold)
                Text("ATT", style = bold)
            }
            a.lines.forEach { line ->
                Row(Modifier.fillMaxWidth()) {
                    Text("${line.attended}", Modifier.width(34.dp), style = mono)
                    Text(receiptName(line.name, line.lab), Modifier.weight(1f).padding(end = 6.dp), style = mono, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    Text("${line.percent}%", style = mono)
                }
            }
            Rule()
            Line("CLASSES ATTENDED", "${a.attended}")
        }
        r.time?.takeIf { it.minutes > 0 }?.let { Line("HOURS IN CLASS", "${(it.minutes + 30) / 60} H") }
        r.habits?.takeIf { it.earlyClasses > 0 }?.let { Line("8 AM TAX", count(it.earlyAttended, "CLASS", "CLASSES")) }
        r.time?.takeIf { it.minutes > 0 && it.labMinutes > 0 }?.let { Line("LAB SURCHARGE", "${(it.labMinutes * 100f / it.minutes).roundToInt()}%") }
        a?.onDuty?.takeIf { it > 0 }?.let { Line("OD DISCOUNT", "−$it CLS") }
        r.streak?.let { Line("LONGEST STREAK", "${it.length}") }
        if (!c.hide) r.grades?.let { g ->
            Line("GPA", gpa(g.gpa))
            if (g.sCount > 0) Line("S GRADES", "${g.sCount}")
        }
        Rule(double = true)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("TOTAL", style = bold.copy(fontSize = 13.sp))
            Text(t.label().uppercase(), Modifier.weight(1f), color = ink, style = type(15f, 800, -0.03f, 1.15f), textAlign = TextAlign.End, maxLines = 2)
        }
        Rule()
        Barcode(r.semesterId + r.firstDay, ink)
        Text(
            "THANK YOU FOR SHOWING UP\n" + if (c.hide || r.grades == null) "*** SEE YOU NEXT SEM ***" else "*** KEEP THIS FOR CGPA ***",
            Modifier.padding(top = 6.dp), style = mono.copy(fontSize = 10.5.sp, textAlign = TextAlign.Center),
        )
    }
}

/** A receipt's top and bottom edges, torn in a zigzag. */
private val Zigzag = GenericShape { size, _ ->
    val tooth = 12f
    val depth = 7f
    val n = (size.width / tooth).toInt().coerceAtLeast(1)
    val w = size.width / n
    moveTo(0f, depth)
    for (i in 0 until n) {
        lineTo(i * w + w / 2, 0f)
        lineTo((i + 1) * w, depth)
    }
    lineTo(size.width, size.height - depth)
    for (i in n downTo 1) {
        lineTo(i * w - w / 2, size.height)
        lineTo((i - 1) * w, size.height - depth)
    }
    close()
}

/** Bars of widths from [seed]'s hash: the same receipt draws the same code. */
@Composable
private fun Barcode(seed: String, color: Color) {
    Canvas(Modifier.width(200.dp).height(34.dp)) {
        val random = java.util.Random(seed.hashCode().toLong())
        var x = 0f
        val unit = 1.dp.toPx()
        while (x < size.width) {
            val bar = unit * listOf(1, 1, 2, 3)[random.nextInt(4)]
            drawRect(color, Offset(x, 0f), Size(bar.coerceAtMost(size.width - x), size.height))
            x += bar + unit * listOf(1, 2, 2, 3)[random.nextInt(4)]
        }
    }
}

/** Measures without a height limit and, when taller than [max], draws scaled down to fit, centred across. */
private fun Modifier.fitHeight(max: Dp) = layout { measurable, constraints ->
    val p = measurable.measure(constraints.copy(maxHeight = Constraints.Infinity))
    val limit = max.roundToPx()
    val s = if (p.height > limit) limit.toFloat() / p.height else 1f
    layout(p.width, minOf(p.height, limit)) {
        p.placeWithLayer(((p.width - p.width * s) / 2).roundToInt(), 0) {
            scaleX = s
            scaleY = s
            transformOrigin = TransformOrigin(0f, 0f)
        }
    }
}

/** Tiles packed into rows of six columns, in order; a short last row stretches. */
private fun rows(tiles: List<Tile>): List<List<Tile>> {
    val rows = mutableListOf<MutableList<Tile>>()
    var used = 6
    for (tile in tiles) {
        if (used + tile.span > 6) {
            rows += mutableListOf<Tile>()
            used = 0
        }
        rows.last() += tile
        used += tile.span
    }
    return rows
}

@Composable
private fun TileView(tile: Tile, modifier: Modifier) {
    val ink = inks(tile.ink)
    val (bg, value, text) = if (tile.key) Triple(ink.shape, ink.onShape, ink.onShape) else Triple(ink.bg, ink.shape, ink.text)
    Column(modifier.defaultMinSize(minHeight = 60.dp).background(bg, RoundedCornerShape(18.dp)).padding(horizontal = 11.dp, vertical = 8.dp)) {
        Text(tile.value, color = value, style = type(if (tile.small) 15f else 22f, 800, -0.04f, 1.1f), maxLines = 2, overflow = TextOverflow.Ellipsis)
        Text(tile.label, Modifier.padding(top = 2.dp), color = text, style = type(11f, 600, 0f, 1.25f), maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}
