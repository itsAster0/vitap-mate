package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.layout
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedRoiVerdict
import vitapmate.core.WrappedScoreSource

// Grades over time: did showing up pay off, and the degree journey.

/** "Operating Systems" as "OS", "DBMS Lab" as "DBMS Lab"; a lone word stays. */
private fun initials(name: String): String {
    val words = titled(name).split(' ').filter { it.isNotBlank() && it.lowercase() !in setOf("and", "of", "the", "to", "for", "in", "&") }
    if (words.size <= 1) return words.firstOrNull()?.take(8).orEmpty()
    if (words.last() == "Lab") return initials(words.dropLast(1).joinToString(" ")) + " Lab"
    return words.joinToString("") { it.take(1).uppercase() }.take(5)
}

/** Each course's attendance against its score, the best return and the hardest grind called out. Flexible; grades. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun roiSlide(r: WrappedReport): Slide? {
    val roi = r.roi?.takeIf { it.points.size >= 3 } ?: return null
    val best = roi.points.getOrNull(roi.best) ?: return null
    val grind = roi.grind?.let { roi.points.getOrNull(it) }
    val scoreAxis = if (roi.source == WrappedScoreSource.TOTAL) "score ↑" else "marks ↑"
    val tile = Tile(Ink.Lab, titled(best.course), "best ROI, ${best.attendance}% → ${best.score}", span = 3, small = true, grades = true)
    return Slide("roi", Ink.Lab, grades = true, shareable = { !it.grades }, tiles = listOf(tile), fixed = false) {
        val ink = LocalInks.current
        val danger = Ink.Danger.tone().base
        val measurer = rememberTextMeasurer()
        Flow {
            H("Did showing up\npay off?", fits(30f, 26f))
            Chip(
                when (roi.verdict) {
                    WrappedRoiVerdict.SHOWED_UP_PAID_OFF -> "Verdict: showing up paid off"
                    WrappedRoiVerdict.BUNKED_AND_ACED -> "Verdict: bunked and aced it"
                    WrappedRoiVerdict.NO_PATTERN -> "Verdict: no pattern, just vibes"
                },
                Modifier.padding(top = 10.dp),
            )
            val minX = minOf(70, (roi.points.minOf { it.attendance } - 5) / 5 * 5).coerceAtLeast(0)
            val minY = minOf(50, (roi.points.minOf { it.score } - 5) / 5 * 5).coerceAtLeast(0)
            BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(top = 14.dp).heightIn(min = 120.dp).background(ink.panel, RoundedCornerShape(24.dp))) {
                val pad = 18.dp
                val w = maxWidth - pad * 2
                val h = maxHeight - pad * 2
                val xOf = { att: Int -> pad + w * ((att - minX) / (100f - minX)).coerceIn(0f, 1f) }
                val yOf = { s: Int -> pad + h * (1 - ((s - minY) / (100f - minY)).coerceIn(0f, 1f)) }
                val axis = type(11.5f, 600, 0f, 1.1f).copy(color = ink.text.copy(alpha = 0.85f))
                Canvas(Modifier.fillMaxSize()) {
                    val x75 = xOf(75).toPx()
                    drawLine(ink.text.copy(alpha = 0.7f), Offset(x75, 10.dp.toPx()), Offset(x75, size.height - 10.dp.toPx()), 2.dp.toPx(), StrokeCap.Round, PathEffect.dashPathEffect(floatArrayOf(1f, 7.dp.toPx())))
                    label(measurer, "75%", axis, Offset(x75 + 6.dp.toPx(), size.height - 26.dp.toPx()))
                    label(measurer, scoreAxis, axis, Offset(12.dp.toPx(), 10.dp.toPx()))
                    val dots = roi.points.map { Offset(xOf(it.attendance).toPx(), yOf(it.score).toPx()) }
                    // Each initial above its dot, else below, right or left: the first spot clear
                    // of every dot (the callouts' larger) and of the initials already placed.
                    val taken = mutableListOf<Rect>()
                    roi.points.forEachIndexed { i, p ->
                        if (i == roi.best || i == roi.grind) return@forEachIndexed
                        val at = dots[i]
                        drawCircle(ink.text.copy(alpha = 0.38f), 7.dp.toPx(), at)
                        if (roi.points.size > 9) return@forEachIndexed
                        val text = measurer.measure(initials(p.course), axis)
                        val (tw, th) = text.size.width.toFloat() to text.size.height.toFloat()
                        val gap = 10.dp.toPx()
                        val spots = listOf(
                            Offset(at.x - tw / 2, at.y - gap - th),
                            Offset(at.x - tw / 2, at.y + gap),
                            Offset(at.x + gap, at.y - th / 2),
                            Offset(at.x - gap - tw, at.y - th / 2),
                        ).map { Rect(it.copy(x = it.x.coerceIn(0f, (size.width - tw).coerceAtLeast(0f))), Size(tw, th)) }
                        val clear = { r: Rect ->
                            dots.indices.none { j ->
                                val reach = if (j == roi.best || j == roi.grind) 24.dp.toPx() else 9.dp.toPx()
                                j != i && r.inflate(reach).contains(dots[j])
                            } && taken.none { it.overlaps(r) }
                        }
                        val spot = spots.firstOrNull(clear) ?: spots.first()
                        taken += spot
                        drawText(text, topLeft = spot.topLeft)
                    }
                }
                /** Beside the point, on the side with fewer points near its height. */
                fun leftOf(att: Int, s: Int): Boolean {
                    val x = xOf(att)
                    val y = yOf(s)
                    val near = roi.points.filter { (yOf(it.score) - y).value in -30f..30f && it.attendance != att }
                    return (near.count { it.attendance > att } > near.count { it.attendance < att } || x > maxWidth - 110.dp) && x > 110.dp
                }
                val bestLeft = leftOf(best.attendance, best.score)
                // Two callouts at about the same height take opposite sides, so their tags don't overlap.
                val grindLeft = grind?.let { g ->
                    val close = (yOf(g.score) - yOf(best.score)).value in -26f..26f
                    if (close) !bestLeft && xOf(g.attendance) > 110.dp else leftOf(g.attendance, g.score)
                } ?: false
                val callout = @androidx.compose.runtime.Composable { att: Int, s: Int, text: String, color: androidx.compose.ui.graphics.Color, left: Boolean ->
                    val x = xOf(att)
                    val y = yOf(s)
                    Box(Modifier.offset(x - 18.dp, y - 18.dp).size(36.dp).background(color, MaterialShapes.Cookie9Sided.toShape()))
                    Box(
                        Modifier.offset(x + if (left) (-22).dp else 22.dp, y - 11.dp)
                            .layout { m, c ->
                                val p = m.measure(c.copy(minWidth = 0))
                                layout(p.width, p.height) { p.place(if (left) -p.width else 0, 0) }
                            }
                            .background(color, RoundedCornerShape(10.dp)).padding(horizontal = 8.dp, vertical = 3.dp),
                    ) { Text(text, color = ink.onShape, style = type(11.5f, 800, 0f, 1.1f), maxLines = 1) }
                }
                if (grind != null) callout(grind.attendance, grind.score, "Hardest grind", danger, grindLeft)
                callout(best.attendance, best.score, "Best ROI", ink.shape, bestLeft)
            }
            Row(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                listOf("$minX%", "attendance →", "100%").forEach { Text(it, color = ink.text.copy(alpha = 0.8f), style = type(11.5f, 600, 0f, 1.2f)) }
            }
            MiniRow(
                listOfNotNull(
                    "${best.attendance}% → ${best.score}" to "${titled(best.course)}\nBest ROI",
                    grind?.let { "${it.attendance}% → ${it.score}" to "${titled(it.course)}\nHardest grind" },
                ),
                Modifier.padding(top = 10.dp),
                columns = 2,
            )
        }
    }
}

/** This semester's GPA among every graded one: its rank, the GPA line, and credits by kind. Flexible; grades. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun degreeSlide(r: WrappedReport): Slide? {
    val d = r.degree?.takeIf { it.semesters.size >= 2 } ?: return null
    val now = d.semesters.firstOrNull { it.current } ?: return null
    val n = d.semesters.size
    val tile = Tile(Ink.Accent, "#${d.rank}", (if (d.rank == 1) "best GPA yet" else "GPA of $n") + ", ${credits(d.credits)} credits", grades = true, span = 3)
    // The CGPA's move this semester, from the one before it.
    val cgpa = r.cgpa?.let { c ->
        val change = c.before?.let { Math.round(c.after * 100) - Math.round(it * 100) }
        "CGPA ${gpa(c.after)}" + when {
            change == null -> ""
            change > 0 -> ", up ${gpa(change / 100.0)}"
            change < 0 -> ", down ${gpa(-change / 100.0)}"
            else -> ", holding steady"
        }
    } ?: d.cgpa?.let { "CGPA ${gpa(it)}" }
    return Slide("degree", Ink.Accent, grades = true, shareable = { !it.grades }, tiles = listOf(tile), fixed = false) {
        val ink = LocalInks.current
        val measurer = rememberTextMeasurer()
        Flow {
            Label("Where this semester ranks")
            Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                val s = fits(140f, 112f)
                Box(Modifier.size(s.dp).background(ink.shape, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                    Text("#${d.rank}", color = ink.onShape, style = type(s * 0.48f, 800, -0.05f, 1f), maxLines = 1)
                }
                Column(Modifier.weight(1f)) {
                    H(if (d.rank == 1) "your best\nGPA yet" else "highest\nGPA yet", fits(30f, 26f))
                    P(
                        "${gpa(now.gpa)} this semester, ${count(n, "semester")} graded" + (cgpa?.let { "\n$it" } ?: ""),
                        13f, top = 8,
                    )
                }
            }
            // The GPA line, oldest first; this semester on a shape, the best marked.
            BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(top = 36.dp, bottom = 28.dp).heightIn(min = 60.dp)) {
                val values = d.semesters.map { it.gpa }
                val lo = (values.min() - 0.2).coerceAtLeast(0.0)
                val hi = (values.max() + 0.15).coerceAtMost(10.0).coerceAtLeast(lo + 0.3)
                val inset = 14.dp
                val xOf = { i: Int -> inset + (maxWidth - inset * 2) * (i.toFloat() / (n - 1)) }
                val yOf = { g: Double -> maxHeight * (1 - ((g - lo) / (hi - lo))).toFloat() }
                val top = values.max()
                val bold = type(12f, 800, -0.01f, 1.1f).copy(color = ink.text)
                val axis = type(11.5f, 600, 0f, 1.1f).copy(color = ink.text.copy(alpha = 0.85f))
                Canvas(Modifier.fillMaxSize()) {
                    val pts = d.semesters.indices.map { Offset(xOf(it).toPx(), yOf(values[it]).toPx()) }
                    val line = Path().apply {
                        moveTo(pts[0].x, pts[0].y)
                        pts.drop(1).forEach { lineTo(it.x, it.y) }
                    }
                    val area = Path().apply {
                        addPath(line)
                        lineTo(pts.last().x, size.height + 8.dp.toPx())
                        lineTo(pts[0].x, size.height + 8.dp.toPx())
                        close()
                    }
                    drawPath(area, ink.shape.copy(alpha = 0.12f))
                    drawPath(line, ink.text.copy(alpha = 0.35f), style = Stroke(4.dp.toPx(), cap = StrokeCap.Round))
                    d.semesters.forEachIndexed { i, sem ->
                        val p = pts[i]
                        val bottom = Offset(p.x, size.height + 10.dp.toPx())
                        label(measurer, sem.short, if (sem.current) bold.copy(color = ink.shape) else axis, bottom, center = true, clampTo = size.width)
                        if (sem.current) return@forEachIndexed
                        val best = sem.gpa == top
                        drawCircle(if (best) ink.text else ink.mid, (if (best) 7f else 5.5f).dp.toPx(), p)
                        label(measurer, gpa(sem.gpa), bold, p + Offset(0f, -24.dp.toPx()), center = true, clampTo = size.width)
                        if (best && d.rank != 1) label(measurer, "#1", axis, p + Offset(0f, -40.dp.toPx()), center = true, clampTo = size.width)
                    }
                }
                val at = d.semesters.indexOfFirst { it.current }
                val s = 52.dp
                Box(
                    Modifier.offset((xOf(at) - s / 2).coerceIn(0.dp, maxWidth - s), (yOf(now.gpa) - s / 2).coerceAtLeast(-s / 2))
                        .size(s).background(ink.shape, MaterialShapes.Clover8Leaf.toShape()),
                    Alignment.Center,
                ) { Text(gpa(now.gpa), color = ink.onShape, style = type(13f, 800, -0.04f, 1f)) }
            }
            BigBeside(credits(d.credits), fits(64f, 52f), "credits\nearned", headingSize = 22f)
            val parts = d.byDistribution.take(4)
            // Wide enough for its label however few credits it holds.
            val least = (parts.sumOf { it.credits } * 0.26).toFloat()
            if (parts.isNotEmpty()) Box(Modifier.padding(top = 10.dp)) {
                Split(
                    40,
                    parts.mapIndexed { i, c ->
                        val colors = when (i) {
                            0 -> ink.shape to ink.onShape
                            1 -> ink.mid to ink.text
                            else -> ink.text.copy(alpha = 0.16f) to ink.text
                        }
                        Triple("${c.label} ${credits(c.credits)}", maxOf(c.credits.toFloat(), least), colors)
                    },
                )
            }
        }
    }
}

/** Draws [text] with its left edge at [at] (its centre with [center]), kept inside [clampTo]. */
internal fun DrawScope.label(measurer: TextMeasurer, text: String, style: TextStyle, at: Offset, center: Boolean = false, clampTo: Float? = null) {
    val layout = measurer.measure(text, style)
    var x = if (center) at.x - layout.size.width / 2f else at.x
    if (clampTo != null) x = x.coerceIn(0f, (clampTo - layout.size.width).coerceAtLeast(0f))
    drawText(layout, topLeft = Offset(x, at.y))
}
