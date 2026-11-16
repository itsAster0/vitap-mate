package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.core.WrappedReport

// Slides 10 to 13 and 17: marks, exams, grades, CGPA and the last semester.

/** "CAT-I" as "CAT-1". */
private fun cat(exam: String) = exam.replace("CAT-II", "CAT-2").replace("CAT-I", "CAT-1")

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun marksSlide(r: WrappedReport): Slide? {
    val m = r.marks ?: return null
    val best = m.bestCat
    val course = m.bestCourse?.takeIf { it.isNotBlank() }
    val change = m.catChange?.takeIf { it > 0 }
    if (best == null && course == null && change == null) return null
    val tile = if (best != null) Tile(Ink.Lab, "${best.percent}%", "best CAT", key = true, grades = true)
    else if (course != null) Tile(Ink.Lab, "${m.bestCoursePercent}%", "best course by marks", key = true, grades = true)
    else Tile(Ink.Lab, "+$change", "points, CAT-1 to CAT-2", key = true, grades = true)
    return Slide("marks", Ink.Lab, grades = true, tiles = listOf(tile), shareable = { !it }) {
        if (best != null) {
            Shaped(MaterialShapes.SoftBurst.toShape(), -44, 60, 240, text = "${best.percent}%", size = 68f)
            Column(Modifier.offset(208.dp, 132.dp).width(136.dp)) {
                H("Your best CAT", 24f)
                P("${cat(best.exam)} in ${titled(best.course)}", 13f, top = 6)
            }
        }
        val rows = listOfNotNull(
            course?.let { Triple("${m.bestCoursePercent}%", "Best course by marks", titled(it)) },
            change?.let { Triple("+$it", "points from CAT-1 to CAT-2, on average", m.mostImproved?.takeIf { s -> s.isNotBlank() }?.let { s -> "Most in ${titled(s)}" }) },
        )
        Txt(if (best != null) 332 else 120) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) { rows.forEach { (value, line, name) -> Wide(value, line, name) } }
        }
    }
}

/** A panel with a number beside two lines (the Marks slide's rows). */
@Composable
private fun Wide(value: String, line: String, name: String?) {
    val ink = LocalInks.current
    Row(
        Modifier.fillMaxWidth().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Text(value, color = ink.shape, style = type(34f, 800, -0.04f, 1.1f))
        Column {
            Text(line, color = ink.text, style = type(12f, 600, -0.01f, 1.3f))
            if (name != null) Text(name, color = ink.text, style = type(15f, 700, -0.01f, 1.3f), maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun examsSlide(r: WrappedReport): Slide? {
    val e = r.exams?.takeIf { it.cats + it.fats > 0 } ?: return null
    val total = e.cats + e.fats
    return Slide("exams", Ink.Danger, tiles = listOf(Tile(Ink.Danger, "$total", if (total == 1) "exam sat" else "exams sat"))) {
        val ink = LocalInks.current
        Txt(58) {
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                Big("$total", 140f)
                H(if (total == 1) "exam\nsat" else "exams\nsat", modifier = Modifier.padding(bottom = 18.dp))
            }
        }
        if (e.cats > 0) Shaped(MaterialShapes.Clover8Leaf.toShape(), 14, 226, 176, text = count(e.cats, "CAT"), size = 30f, track = -0.03f)
        if (e.fats > 0) Shaped(MaterialShapes.Cookie6Sided.toShape(), 186, 296, 150, color = ink.mid, text = count(e.fats, "FAT"), size = 28f, textColor = ink.text, track = -0.03f)
        e.topVenue?.takeIf { it.isNotBlank() }?.let { venue ->
            Txt(470) {
                Label("Most of them in")
                H(venue, 30f, Modifier.padding(top = 4.dp))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun gradesSlide(r: WrappedReport): Slide? {
    val g = r.grades ?: return null
    val s = g.sCount.takeIf { it > 0 }
    return Slide("grades", Ink.Success, grades = true, shareable = { !it }, tiles = listOf(Tile(Ink.Success, gpa(g.gpa), "GPA" + (s?.let { ", ${count(it, "S grade")}" } ?: ""), key = true, grades = true))) {
        val ink = LocalInks.current
        Shaped(MaterialShapes.Clover4Leaf.toShape(), 35, 52, 290, text = gpa(g.gpa), size = 88f, shapeRotate = 45f)
        Txt(352) { H("Semester GPA") }
        val mix = g.mix.filter { it.count > 0 }
        if (mix.isNotEmpty()) {
            val shades = listOf(0f, 0.28f, 0.54f, 0.7f, 0.8f, 0.86f, 0.9f)
            SplitBar(
                404, 48,
                mix.mapIndexed { i, m ->
                    val shade = shades[i.coerceAtMost(shades.lastIndex)]
                    Triple("${m.grade} ×${m.count}", m.count.toFloat(), lerp(ink.shape, ink.bg, shade) to if (shade < 0.5f) ink.onShape else ink.text)
                },
            )
        }
        if (s != null) Txt(470) { Chip(if (g.sRecord) "${count(s, "S grade")}, your most in a semester" else count(s, "S grade")) }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun cgpaSlide(r: WrappedReport): Slide? {
    val c = r.cgpa ?: return null
    val before = c.before
    // At hundredths, as drawn: positive up, negative down, 0 held.
    val change = before?.let { Math.round(c.after * 100) - Math.round(it * 100) }
    val moved = when {
        change == null -> ""
        change > 0 -> ", up from ${gpa(before)}"
        change < 0 -> ", down from ${gpa(before)}"
        else -> ", same as before"
    }
    val tile = Tile(Ink.Accent, gpa(c.after), "CGPA$moved", grades = true)
    return Slide("cgpa", Ink.Accent, grades = true, shareable = { !it }, tiles = listOf(tile)) {
        val ink = LocalInks.current
        Txt(62) {
            H(
                when {
                    change == null -> "Your CGPA"
                    change > 0 -> "Your CGPA moved up"
                    change < 0 -> "Your CGPA dipped"
                    else -> "Your CGPA held steady"
                },
            )
        }
        if (before != null) {
            At(24, 190, 96, 96) {
                Box(Modifier.size(96.dp).background(lerp(ink.bg, ink.shape, 0.4f), CircleShape), Alignment.Center) {
                    Text(gpa(before), color = ink.text, style = type(28f, 800, -0.04f, 1f))
                }
            }
            At(36, 294) { Label("before") }
            At(96, 150, 110, 90) {
                Canvas(Modifier.size(110.dp, 90.dp)) {
                    scale(size.width / 110f, Offset.Zero) {
                        val path = Path().apply { moveTo(10f, 70f); cubicTo(30f, 20f, 70f, 10f, 100f, 26f) }
                        drawPath(path, ink.shape, style = Stroke(6f, cap = StrokeCap.Round, pathEffect = PathEffect.dashPathEffect(floatArrayOf(1f, 13f))))
                    }
                }
            }
        }
        Shaped(MaterialShapes.Cookie9Sided.toShape(), 140, 116, 216, text = gpa(c.after), size = 68f)
        if (before != null && change != null) At(228, 340) {
            Label(
                when {
                    change > 0 -> "now, up ${gpa(change / 100.0)}"
                    change < 0 -> "now, down ${gpa(-change / 100.0)}"
                    else -> "now"
                },
            )
        }
        Minis(
            392,
            listOfNotNull(
                c.creditsSemester.takeIf { it > 0 }?.let { credits(it) to "credits earned this semester" },
                c.creditsTotal.takeIf { it > 0 }?.let { credits(it) to "credits in total" },
            ),
        )
    }
}

internal fun previousSlide(r: WrappedReport): Slide? {
    val p = r.previous ?: return null
    val attendance = p.attendanceChange?.let { change -> r.attendance?.let { it.percent to change } }
    val gpaPair = p.gpaChange?.let { change -> r.grades?.let { it.gpa to change } }
    if (attendance == null && gpaPair == null) return null
    val tile = if (attendance != null) Tile(Ink.Success, "+${attendance.second}%", "attendance on last semester")
    else Tile(Ink.Success, "+${gpa(gpaPair!!.second)}", "GPA on last semester", grades = true)
    val then = shortName(p.semesterName)
    val now = shortName(r.semesterName)
    return Slide("last_semester", Ink.Success, grades = gpaPair != null, tiles = listOf(tile), shareable = { hide -> !hide || attendance != null }) { c ->
        Txt(62) { H("Better than ${p.semesterName}") }
        var y = 176
        if (attendance != null) {
            val (percent, change) = attendance
            Compare(y, "Attendance", "$then, ${percent - change}%", "$now, $percent%", "up $change%", (percent - change).toFloat() / percent)
            y = 340
        }
        if (gpaPair != null && !c.hide) {
            val (value, change) = gpaPair
            Compare(y, "GPA", "$then, ${gpa(value - change)}", "$now, ${gpa(value)}", "up ${gpa(change)}", ((value - change) / value).toFloat())
        }
    }
}

/** Last semester's bar over this one's, for one fact. */
@Composable
private fun Compare(y: Int, label: String, before: String, after: String, change: String, ratio: Float) {
    val ink = LocalInks.current
    Txt(y) {
        Label(label)
        Box(
            Modifier.padding(top = 10.dp).fillMaxWidth(ratio.coerceIn(0.45f, 0.9f)).height(34.dp).background(lerp(ink.bg, ink.shape, 0.4f), RoundedCornerShape(17.dp)).padding(start = 14.dp),
            Alignment.CenterStart,
        ) { Text(before, color = ink.text, style = type(13f, 700, -0.01f, 1.2f)) }
        Row(
            Modifier.padding(top = 6.dp).fillMaxWidth().height(52.dp).background(ink.shape, RoundedCornerShape(26.dp)).padding(horizontal = 18.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(after, color = ink.onShape, style = type(18f, 800, -0.02f, 1.2f))
            Text(change, color = ink.onShape, style = type(18f, 800, -0.02f, 1.2f))
        }
    }
}
