package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import java.time.LocalDate
import kotlin.math.ceil
import kotlin.math.roundToInt
import vitapmate.core.WrappedArcShape
import vitapmate.core.WrappedDay
import vitapmate.core.WrappedPhase
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedTitle

// The semester day by day: the attendance mosaic and the attendance arc.

/** "September" for 9. */
private fun monthName(month: Int) = java.time.Month.of(month.coerceIn(1, 12)).getDisplayName(java.time.format.TextStyle.FULL, java.util.Locale.US)

/** A percent as drawn: rounded up, as VTOP rounds. */
private fun shown(percent: Double) = ceil(percent - 1e-9).toInt()

/** "CAT-I" as "CAT-1". */
private fun cat(exam: String) = exam.replace("CAT-II", "CAT-2").replace("CAT-I", "CAT-1")

/** The mosaic's colours for each kind of day, from the theme. */
private class DayInks(val present: Color, val od: Color, val mixed: Color, val absent: Color, val exam: Color, val faint: Color, val outside: Color, val holiday: Color, val dot: Color)

@Composable
private fun dayInks(): DayInks {
    val ink = LocalInks.current
    return DayInks(
        present = ink.shape, od = Ink.Warning.tone().base, mixed = ink.mid, absent = Ink.Danger.tone().base,
        exam = Ink.Accent.tone().base, faint = ink.text.copy(alpha = 0.09f), outside = ink.text.copy(alpha = 0.04f),
        holiday = ink.text.copy(alpha = 0.45f), dot = ink.onShape.copy(alpha = 0.75f),
    )
}

/** Every day of the semester as a square, Monday to Sunday rows, with each month's attendance beside. Flexible. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun mosaicSlide(r: WrappedReport): Slide? {
    val m = r.mosaic?.takeIf { it.days.size >= 7 } ?: return null
    val best = m.bestMonth?.let { m.months.getOrNull(it) }
    val worst = m.worstMonth?.let { m.months.getOrNull(it) }
    val tiles = listOfNotNull(best?.percent?.let { Tile(Ink.Success, monthName(best.month).take(3), "best month, $it%") })
    return Slide("mosaic", Ink.Success, tiles = tiles, fixed = false) {
        val ink = LocalInks.current
        val colors = dayInks()
        Flow {
            H("Your semester,\none square a day", fits(30f, 26f))
            P(count(m.classDays, "class day") + (r.attendance?.let { ", ${it.percent}% attended" } ?: ""), 14f, top = 6)
            BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(top = 12.dp)) {
                val rows = m.days.size / 7
                val letters = 16.dp
                val step = minOf((maxHeight - letters) / rows, maxWidth * 0.56f / 7, 30.dp)
                val gap = step * 0.16f
                val cell = step - gap
                val gridW = step * 7 - gap
                Row(Modifier.width(step * 7)) {
                    "MTWTFSS".forEach { Text("$it", Modifier.width(step), color = ink.text.copy(alpha = 0.7f), style = type(11f, 700, 0f, 1.2f), textAlign = TextAlign.Center) }
                }
                Canvas(Modifier.offset(y = letters).size(gridW, step * rows - gap)) {
                    val (c, s) = cell.toPx() to step.toPx()
                    m.days.forEachIndexed { i, kind -> drawDay(kind, Offset((i % 7) * s, (i / 7) * s), c, colors) }
                }
                // A rail per month beside its rows, its attendance by it; the best and worst stand out.
                val railX = gridW + 12.dp
                val labelW = maxWidth - railX - 14.dp
                val big = minOf(60.dp, labelW * 0.45f)
                val rails = m.months.map { month ->
                    val top = letters + step * ((month.firstDay - m.firstDay) / 7) + 2.dp
                    top to (letters + step * ((month.lastDay - m.firstDay) / 7) + cell - 2.dp).coerceAtLeast(top + 4.dp)
                }
                val heights = m.months.map {
                    when (it) {
                        best -> big
                        worst -> maxOf(big * 0.62f, 34.dp)
                        else -> 36.dp
                    }
                }
                // Each label centred on its rail, pushed down past the one above, then back up from the bottom.
                val ys = rails.indices.map { (rails[it].first + rails[it].second) / 2 - heights[it] / 2 }.toMutableList()
                var floor = 0.dp
                ys.indices.forEach { ys[it] = maxOf(ys[it], floor); floor = ys[it] + heights[it] + 4.dp }
                var ceiling = maxHeight
                ys.indices.reversed().forEach { ys[it] = minOf(ys[it], ceiling - heights[it]); ceiling = ys[it] - 4.dp }
                m.months.forEachIndexed { i, month ->
                    val railColor = when (month) {
                        best -> ink.shape
                        worst -> colors.absent
                        else -> ink.text.copy(alpha = 0.22f)
                    }
                    val (top, bottom) = rails[i]
                    Box(Modifier.offset(railX, top).size(4.dp, bottom - top).background(railColor, RoundedCornerShape(2.dp)))
                    Box(Modifier.offset(railX + 14.dp, ys[i]).size(labelW, heights[i]), Alignment.CenterStart) {
                        MonthLabel(month.month, month.percent, month == best, month == worst, big)
                    }
                }
            }
            Legend(Modifier.padding(top = 10.dp), colors)
        }
    }
}

/** A month's name and attendance; the best on a shape, the worst on a tilted pill. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun MonthLabel(month: Int, percent: Int?, best: Boolean, worst: Boolean, size: Dp) {
    val ink = LocalInks.current
    val name = monthName(month)
    when {
        percent != null && (best || worst) -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            val shape = if (best) MaterialShapes.Cookie9Sided.toShape() else RoundedCornerShape(50)
            val color = if (best) ink.shape else Ink.Danger.tone().base
            Box(Modifier.size(if (best) size else size * 1.15f, if (best) size else size * 0.62f).rotate(if (worst) -6f else 0f).background(color, shape), Alignment.Center) {
                Text("$percent%", color = ink.onShape, style = type(if (size > 56.dp) 22f else 18f, 800, -0.05f, 1f), maxLines = 1)
            }
            Column {
                Text(name.take(3), color = ink.text, style = type(13f, 700, -0.01f, 1.2f), maxLines = 1)
                Text(if (best) "your best month" else "the slump", color = ink.text.copy(alpha = 0.75f), style = type(12f, 600, -0.01f, 1.2f), maxLines = 2)
            }
        }
        else -> Column {
            Text(name.take(3), color = ink.text.copy(alpha = 0.8f), style = type(12f, 700, 0f, 1.2f), maxLines = 1)
            Text(percent?.let { "$it%" } ?: "no classes", color = ink.text, style = type(if (percent != null) 18f else 13f, 800, -0.04f, 1.1f))
        }
    }
}

private fun DrawScope.drawDay(kind: WrappedDay, at: Offset, cell: Float, c: DayInks) {
    val corner = CornerRadius(cell * 0.32f)
    val size = Size(cell, cell)
    when (kind) {
        WrappedDay.HOLIDAY -> drawRoundRect(c.holiday, at + Offset(0.8f, 0.8f), Size(cell - 1.6f, cell - 1.6f), corner, style = Stroke(1.6f, pathEffect = PathEffect.dashPathEffect(floatArrayOf(2.5f, 2.5f))))
        else -> {
            val fill = when (kind) {
                WrappedDay.ALL_PRESENT -> c.present
                WrappedDay.ON_DUTY -> c.od
                WrappedDay.MIXED -> c.mixed
                WrappedDay.ALL_ABSENT -> c.absent
                WrappedDay.EXAM -> c.exam
                WrappedDay.NO_CLASS -> c.faint
                else -> c.outside
            }
            drawRoundRect(fill, at, size, corner)
            if (kind == WrappedDay.EXAM) drawCircle(c.dot, cell * 0.15f, at + Offset(cell / 2, cell / 2))
        }
    }
}

/** What each square means, three to a row. */
@Composable
private fun Legend(modifier: Modifier, c: DayInks) {
    val ink = LocalInks.current
    val items = listOf(c.present to "All present", c.od to "On duty", c.mixed to "Some missed", c.absent to "All missed", c.exam to "Exam day", null to "Holiday")
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        items.chunked(3).forEach { row ->
            Row(Modifier.fillMaxWidth()) {
                row.forEach { (color, label) ->
                    Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        val swatch = Modifier.size(12.dp)
                        Box(if (color != null) swatch.background(color, RoundedCornerShape(4.dp)) else swatch.border(1.5.dp, c.holiday, RoundedCornerShape(4.dp)))
                        Text(label, color = ink.text, style = type(12f, 600, -0.01f, 1.2f), maxLines = 1)
                    }
                }
            }
        }
    }
}

/** A phase's name: "before CAT-1", "CAT-1 to CAT-2", "after CAT-2". */
private fun WrappedPhase.label(): String {
    val (a, b) = after?.let(::cat) to before?.let(::cat)
    return when {
        a != null && b != null -> "$a to $b"
        b != null -> "before $b"
        a != null -> "after $a"
        else -> "all semester"
    }
}

/**
 * Attendance as it ran: the line over the semester with the exams as bands, its peak, low and
 * biggest fall, where it ended, and attendance between the CATs. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun arcSlide(r: WrappedReport): Slide? {
    val a = r.arc?.takeIf { it.points.size >= 2 } ?: return null
    val first = shown(a.points.first().percent)
    val final = shown(a.points.last().percent)
    val low = shown(a.low.percent)
    val lowest = a.phases.firstOrNull { it.lowest }
    val comeback = r.comeback
    val tiles = when {
        r.title == WrappedTitle.COMEBACK -> emptyList()
        comeback != null -> listOf(Tile(Ink.Accent, "${comeback.lowPercent} → ${comeback.finalPercent}%", "back from ${month(comeback.lowDay)}", span = 3))
        lowest != null -> listOf(Tile(Ink.Accent, "${lowest.percent}%", "${lowest.label()}, your lowest stretch", span = 3))
        else -> emptyList()
    }
    return Slide("arc", Ink.Accent, tiles = tiles, fixed = false) {
        val ink = LocalInks.current
        Flow {
            H("Phase by phase", fits(34f, 28f))
            P(
                when (a.shape) {
                    WrappedArcShape.COMEBACK -> "Down to $low% in ${month(a.low.day)}, then clawed back to $final%."
                    WrappedArcShape.SLIDE -> "Started at $first%, eased to $final% by the end."
                    WrappedArcShape.CLIMB -> "Started at $first%, climbed to $final% by the end."
                    WrappedArcShape.STEADY -> "Held around $final% all semester."
                },
                15f, top = 6,
            )
            ArcChart(r, Modifier.fillMaxWidth().weight(1f).padding(top = 30.dp, bottom = 24.dp).heightIn(min = 90.dp))
            if (a.phases.isNotEmpty()) Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                a.phases.take(3).forEach { phase ->
                    Column(Modifier.weight(1f).fillMaxHeight().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 12.dp, vertical = 11.dp)) {
                        Text("${phase.percent}%", color = ink.shape, style = type(22f, 800, -0.04f, 1.15f), maxLines = 1)
                        Text(phase.label(), color = ink.text, style = type(12f, 600, -0.01f, 1.25f), maxLines = 2)
                        if (phase.lowest) Box(Modifier.padding(top = 4.dp).background(Ink.Danger.tone().base, RoundedCornerShape(8.dp)).padding(horizontal = 6.dp, vertical = 2.dp)) {
                            Text("lowest", color = ink.onShape, style = type(11f, 800, 0f, 1.1f))
                        }
                    }
                }
            } else MiniRow(listOf("${shown(a.peak.percent)}%" to "your peak, ${day(a.peak.day)}", "$low%" to "your low, ${day(a.low.day)}", "$final%" to "at the end"))
        }
    }
}

/** The arc's chart: x over the semester's days, y from under the low (75% at least) to 100. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ArcChart(r: WrappedReport, modifier: Modifier) {
    val a = r.arc ?: return
    val ink = LocalInks.current
    val danger = Ink.Danger.tone().base
    val measurer = rememberTextMeasurer()
    val axis = type(11.5f, 600, 0f, 1.1f).copy(color = ink.text.copy(alpha = 0.85f))
    val bold = type(12f, 800, -0.01f, 1.1f).copy(color = ink.text)
    BoxWithConstraints(modifier) {
        val span = (r.windowEnd - r.firstDay).coerceAtLeast(1).toFloat()
        val lowest = minOf(a.low.percent, 75.0)
        val lo = (((lowest - 5) / 5).toInt() * 5).coerceIn(0, 70).toDouble()
        val xOf = { day: Int -> ((day - r.firstDay) / span).coerceIn(0f, 1f) }
        val yOf = { p: Double -> (1 - ((p - lo) / (100 - lo))).toFloat().coerceIn(0f, 1f) }
        val lowAt = a.points.indexOfFirst { it.day == a.low.day }.coerceAtLeast(0)
        // Where it ended, on a shape by the line's end, on the side the line doesn't come in from;
        // the labels keep clear of it.
        val end = a.points.last()
        val s = fits(76f, 62f).dp
        val ex = maxWidth * xOf(end.day)
        val ey = maxHeight * yOf(end.percent)
        val fromAbove = a.points.getOrNull(a.points.size - 2)?.let { it.percent > end.percent } ?: false
        val endX = (ex - s * 1.15f).coerceIn(0.dp, maxWidth - s)
        val endY = (if (fromAbove) ey + s * 0.25f else ey - s * 1.2f).coerceIn(-24.dp, maxHeight - s)
        Canvas(Modifier.fillMaxSize()) {
            val endShape = Rect(endX.toPx(), endY.toPx(), (endX + s).toPx(), (endY + s).toPx()).inflate(4.dp.toPx())
            /** Draws [text] at the first of [spots] (its left edge, or right edge for an `end` spot) clear of the end shape; none when all overlap. */
            fun clear(text: String, style: TextStyle, vararg spots: Pair<Offset, Boolean>) {
                val size = measurer.measure(text, style).size
                spots.firstOrNull { (at, endAligned) ->
                    val x = (if (endAligned) at.x - size.width else at.x).coerceIn(0f, (this.size.width - size.width).coerceAtLeast(0f))
                    !Rect(x, at.y, x + size.width, at.y + size.height).overlaps(endShape)
                }?.let { (at, endAligned) -> text(measurer, text, style, at, end = endAligned, clampTo = this.size.width) }
            }
            val (w, h) = size.width to size.height
            val px = { day: Int -> xOf(day) * w }
            val py = { p: Double -> yOf(p) * h }
            a.exams.forEach { exam ->
                val x0 = px(exam.firstDay)
                val x1 = (px(exam.lastDay) + 4.dp.toPx()).coerceAtMost(w)
                drawRoundRect(ink.text.copy(alpha = 0.1f), Offset(x0, -6.dp.toPx()), Size(x1 - x0, h + 6.dp.toPx()), CornerRadius(8.dp.toPx()))
                text(measurer, cat(exam.name), axis, Offset((x0 + x1) / 2, -20.dp.toPx()), center = true)
            }
            val y75 = py(75.0)
            drawLine(ink.text, Offset(0f, y75), Offset(w, y75), 2.dp.toPx(), StrokeCap.Round, PathEffect.dashPathEffect(floatArrayOf(1f, 7.dp.toPx())))
            text(measurer, "75%", axis, Offset(w, y75 + 4.dp.toPx()), end = true)
            val points = a.points.map { Offset(px(it.day), py(it.percent)) }
            val stroke = Stroke(7.dp.toPx(), cap = StrokeCap.Round)
            if (a.shape == WrappedArcShape.COMEBACK && lowAt > 0) {
                drawPath(smooth(points.take(lowAt + 1)), ink.shape.copy(alpha = 0.35f), style = stroke)
                drawPath(smooth(points.drop(lowAt)), ink.shape, style = stroke)
            } else drawPath(smooth(points), ink.shape, style = stroke)
            // The fall, dashed over the line; left out when it is the whole line, which the
            // sentence above already says.
            a.drop?.takeUnless { it.from.day == a.points.first().day && it.to.day == a.points.last().day }?.let { d ->
                val from = Offset(px(d.from.day), py(d.from.percent))
                val to = Offset(px(d.to.day), py(d.to.percent))
                val bend = Path().apply {
                    moveTo(from.x, from.y - 14.dp.toPx())
                    quadraticTo((from.x + to.x) / 2 + 30.dp.toPx(), minOf(from.y, to.y) - 30.dp.toPx(), to.x + 6.dp.toPx(), to.y - 18.dp.toPx())
                }
                drawPath(bend, danger, style = Stroke(2.5.dp.toPx(), cap = StrokeCap.Round, pathEffect = PathEffect.dashPathEffect(floatArrayOf(2.dp.toPx(), 5.dp.toPx()))))
                val fall = (d.from.percent - d.to.percent).roundToInt()
                clear(
                    "−$fall in ${count(d.to.day - d.from.day, "day")}", bold.copy(color = danger),
                    Offset(to.x + 14.dp.toPx(), to.y + 10.dp.toPx()) to false,
                    Offset(to.x - 14.dp.toPx(), to.y + 10.dp.toPx()) to true,
                )
            }
            // A peak at the end is the end's shape (a climb).
            if (a.peak.day != end.day) {
                val peak = Offset(px(a.peak.day), py(a.peak.percent))
                drawCircle(ink.text, 6.dp.toPx(), peak)
                clear(
                    "${shown(a.peak.percent)}%", bold,
                    peak + Offset(10.dp.toPx(), -22.dp.toPx()) to false,
                    peak + Offset(-10.dp.toPx(), -22.dp.toPx()) to true,
                )
            }
            val lowPoint = Offset(px(a.low.day), py(a.low.percent))
            // A low at the end is the end's shape.
            if (a.low.day != a.peak.day && a.low.day != end.day) {
                drawCircle(ink.bg, 9.dp.toPx(), lowPoint)
                drawCircle(ink.shape, 9.dp.toPx(), lowPoint, style = Stroke(4.dp.toPx()))
                clear(
                    "${shown(a.low.percent)}%\n${day(a.low.day)}", bold,
                    lowPoint + Offset(-14.dp.toPx(), 8.dp.toPx()) to true,
                    lowPoint + Offset(14.dp.toPx(), 8.dp.toPx()) to false,
                )
            }
            var month = LocalDate.ofEpochDay(r.firstDay.toLong()).withDayOfMonth(1)
            while (month.toEpochDay() <= r.windowEnd) {
                val x = px(month.toEpochDay().toInt().coerceAtLeast(r.firstDay))
                if (x <= w - 24.dp.toPx()) text(measurer, month.month.getDisplayName(java.time.format.TextStyle.SHORT, java.util.Locale.US), axis, Offset(x, h + 8.dp.toPx()))
                month = month.plusMonths(1)
            }
        }
        Box(
            Modifier.offset(endX, endY)
                .size(s).background(ink.shape, MaterialShapes.Clover8Leaf.toShape()),
            Alignment.Center,
        ) { Text("${shown(end.percent)}%", color = ink.onShape, style = type(if (s > 70.dp) 22f else 19f, 800, -0.05f, 1f)) }
    }
}

/** Draws [text] at [at]: its left edge there, or its centre ([center]) or right edge ([end]); kept inside [clampTo] when given. */
private fun DrawScope.text(
    measurer: androidx.compose.ui.text.TextMeasurer,
    text: String,
    style: TextStyle,
    at: Offset,
    center: Boolean = false,
    end: Boolean = false,
    clampTo: Float? = null,
) {
    val layout = measurer.measure(text, style.copy(textAlign = if (end) TextAlign.End else TextAlign.Start))
    var x = when {
        center -> at.x - layout.size.width / 2f
        end -> at.x - layout.size.width
        else -> at.x
    }
    if (clampTo != null) x = x.coerceIn(0f, (clampTo - layout.size.width).coerceAtLeast(0f))
    drawText(layout, topLeft = Offset(x, at.y))
}

/** A smooth line through [points] (Catmull-Rom as cubic curves). */
private fun smooth(points: List<Offset>): Path = Path().apply {
    if (points.isEmpty()) return@apply
    moveTo(points[0].x, points[0].y)
    for (i in 0 until points.lastIndex) {
        val p0 = points[(i - 1).coerceAtLeast(0)]
        val p1 = points[i]
        val p2 = points[i + 1]
        val p3 = points[(i + 2).coerceAtMost(points.lastIndex)]
        cubicTo(
            p1.x + (p2.x - p0.x) / 6, p1.y + (p2.y - p0.y) / 6,
            p2.x - (p3.x - p1.x) / 6, p2.y - (p3.y - p1.y) / 6,
            p2.x, p2.y,
        )
    }
}
