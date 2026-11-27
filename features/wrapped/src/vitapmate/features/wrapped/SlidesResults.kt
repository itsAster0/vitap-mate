package vitapmate.features.wrapped

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.time.LocalDate
import vitapmate.core.WrappedReport

// Marks, exams, grades and the last semester.

/** "CAT-I" as "CAT-1". */
private fun cat(exam: String) = exam.replace("CAT-II", "CAT-2").replace("CAT-I", "CAT-1")

/**
 * Marks: the best CAT on a burst, the best course, the CAT-1 to CAT-2 jump, and full marks out
 * of every assessment posted, by kind. Flexible; grades.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
internal fun marksSlide(r: WrappedReport): Slide? {
    val m = r.marks
    val best = m?.bestCat
    val course = m?.bestCourse?.takeIf { it.isNotBlank() }
    val change = m?.catChange?.takeIf { it > 0 }
    val rise = m?.mostImproved
    val a = r.assessments
    val f = r.fullMarks
    if (best == null && course == null && change == null && rise == null && a == null) return null
    val tile = when {
        best != null -> Tile(Ink.Lab, "${best.percent}%", "best CAT", key = true, grades = true)
        course != null -> Tile(Ink.Lab, "${m.bestCoursePercent}%", "best course by marks", key = true, grades = true)
        change != null -> Tile(Ink.Lab, "+$change", "points, CAT-1 to CAT-2", key = true, grades = true)
        rise != null -> Tile(Ink.Lab, "+${rise.points}", "points in ${titled(rise.course)}", key = true, grades = true)
        else -> null
    }
    val tiles = listOfNotNull(tile, a?.let { Tile(Ink.Lab, "${it.total}", if (it.total == 1) "assessment" else "assessments") })
    return Slide("marks", Ink.Lab, grades = true, tiles = tiles, shareable = { !it.grades }, fixed = false) {
        val ink = LocalInks.current
        Flow {
            if (best != null) Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                val s = fits(190f, 150f)
                Box(Modifier.offset(x = (-44).dp).size(s.dp).background(ink.shape, MaterialShapes.SoftBurst.toShape()), Alignment.Center) {
                    Text("${best.percent}%", Modifier.offset(x = 10.dp), color = ink.onShape, style = type(s * 0.3f, 800, -0.05f, 1f), maxLines = 1)
                }
                Column(Modifier.weight(1f).offset(x = (-40).dp)) {
                    H("Your best CAT", fits(26f, 23f))
                    P("${cat(best.exam)} in ${titled(best.course)}", 13f, top = 6)
                }
            } else H("Your marks", fits(34f, 28f))
            Spacer(Modifier.weight(1f).heightIn(min = 10.dp))
            val rows = listOfNotNull(
                course?.let { Triple("${m.bestCoursePercent}%", "Best course by marks", titled(it)) },
                change?.let { Triple("+$it", "points from CAT-1 to CAT-2, on average", rise?.let { s -> "Most in ${titled(s.course)}, up ${s.points}" }) },
                // The biggest jump shows even when the average fell.
                rise?.takeIf { change == null }?.let { Triple("+${it.points}", "points from CAT-1 to CAT-2, your biggest jump", titled(it.course)) },
                a?.let { all ->
                    val full = f?.count ?: 0
                    Triple("$full", (if (full == 1) "full mark" else "full marks") + " of ${count(all.total, "assessment")}", f?.topCourse?.takeIf { it.isNotBlank() && full > 1 }?.let { "${f.topCount} in ${titled(it)}" })
                },
            )
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) { rows.forEach { (value, line, name) -> Wide(value, line, name) } }
            if (a != null && a.byKind.isNotEmpty()) FlowRow(Modifier.fillMaxWidth().padding(top = 10.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                a.byKind.take(if (LocalFrame.current.compact) 2 else 3).forEach { Chip("${it.count} ${it.kind}") }
            }
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

/**
 * Exams: papers sat, CATs and FATs, hours in exam halls, the busiest week as a column per day
 * with a shape per paper, and the venue used most. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun examsSlide(r: WrappedReport): Slide? {
    val e = r.exams?.takeIf { it.cats + it.fats > 0 } ?: return null
    val c = r.crunch
    val total = e.cats + e.fats
    val hours = (e.minutes + 30) / 60
    val venue = e.topVenue?.takeIf { it.isNotBlank() && it != "-" }
    val weekday = { d: Int -> LocalDate.ofEpochDay(d.toLong()).dayOfWeek.value - 1 }
    return Slide("exams", Ink.Danger, tiles = listOf(Tile(Ink.Danger, "$total", (if (total == 1) "exam" else "exams") + if (hours > 0) ", $hours h in halls" else "")), fixed = false) {
        val ink = LocalInks.current
        Flow {
            BigBeside("$total", fits(124f, 100f), if (total == 1) "exam\nsat" else "exams\nsat")
            P(
                listOfNotNull(
                    e.cats.takeIf { it > 0 }?.let { count(it, "CAT") },
                    e.fats.takeIf { it > 0 }?.let { count(it, "FAT") },
                ).joinToString(" and ") + if (hours > 0) ", ${count(hours, "hour")} under the clock" else "",
                14f, top = 4,
            )
            // CATs, FATs and the hours, across the column.
            BoxWithConstraints(Modifier.fillMaxWidth().padding(top = 10.dp).height(fits(104f, 88f).dp)) {
                val w = maxWidth
                val size = minOf(maxHeight, w * 0.32f)
                if (e.cats > 0) Box(Modifier.size(size).background(ink.shape, MaterialShapes.Clover8Leaf.toShape()), Alignment.Center) {
                    Text("${e.cats} CAT", color = ink.onShape, style = type(20f, 800, -0.03f, 1f), maxLines = 1)
                }
                if (e.fats > 0) Box(Modifier.offset(x = size * 0.96f, y = size * 0.08f).size(size * 0.88f).background(ink.mid, MaterialShapes.Cookie6Sided.toShape()), Alignment.Center) {
                    Text("${e.fats} FAT", color = ink.text, style = type(18f, 800, -0.03f, 1f), maxLines = 1)
                }
                if (hours > 0) Column(Modifier.align(Alignment.TopEnd).width(w * 0.34f), horizontalAlignment = Alignment.End) {
                    Box(Modifier.fillMaxWidth().height(size * 0.56f).rotate(-8f).background(ink.shape, RoundedCornerShape(50)), Alignment.Center) {
                        Text("$hours h", color = ink.onShape, style = type(26f, 800, -0.04f, 1f), maxLines = 1)
                    }
                    Text("in exam halls", Modifier.padding(top = 8.dp), color = ink.text.copy(alpha = 0.85f), style = type(12f, 600, 0f, 1.2f))
                }
            }
            if (c != null) {
                Label("Crunch week, ${day(c.firstDay)} to ${day(c.lastDay)}", 14f)
                val most = c.byDay.maxOrNull()?.coerceAtLeast(1) ?: 1
                // The busiest day on a pill, only when it had more than one.
                val busiest = if (most > 1) c.byDay.indexOf(most) else -1
                BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(top = 8.dp).heightIn(min = 72.dp, max = 150.dp)) {
                    val dot = minOf((maxHeight - 34.dp) / most - 6.dp, 30.dp).coerceAtLeast(12.dp)
                    Row(Modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.Bottom) {
                        c.byDay.forEachIndexed { i, n ->
                            val on = i == busiest
                            Column(
                                Modifier.weight(1f).then(if (on) Modifier.background(ink.shape.copy(alpha = 0.22f), RoundedCornerShape(24.dp)).padding(vertical = 8.dp) else Modifier.padding(vertical = 8.dp)),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                repeat(n) { Box(Modifier.size(dot).background(ink.shape, MaterialShapes.Cookie9Sided.toShape())) }
                                Text(
                                    WEEKDAYS[weekday(c.firstDay + i)].take(1),
                                    color = if (on) ink.shape else ink.text.copy(alpha = if (n > 0) 1f else 0.5f),
                                    style = type(13f, 700, 0f, 1.2f),
                                )
                            }
                        }
                    }
                }
            } else Spacer(Modifier.weight(1f))
            MiniRow(
                listOfNotNull(
                    c?.mostInADay?.takeIf { it > 1 }?.let { "$it in a day" to "${WEEKDAYS[weekday(c.mostDay)].take(3)} ${day(c.mostDay)}" },
                    venue?.let { it to "most of your papers" },
                    c?.takeIf { venue == null || it.mostInADay <= 1 }?.let { count(it.examDays, "day") to "with an exam" },
                ).take(2),
                Modifier.padding(top = 10.dp),
                columns = 2,
            )
        }
    }
}

/** The semester's GPA on a clover, the grade mix, S grades, and credits earned in a first semester (no Degree journey then). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
internal fun gradesSlide(r: WrappedReport): Slide? {
    val g = r.grades ?: return null
    val s = g.sCount.takeIf { it > 0 }
    val earned = r.cgpa?.creditsSemester?.takeIf { it > 0 && degreeSlide(r) == null }
    return Slide("grades", Ink.Success, grades = true, shareable = { !it.grades }, tiles = listOf(Tile(Ink.Success, gpa(g.gpa), "GPA" + (s?.let { ", ${count(it, "S grade")}" } ?: ""), key = true, grades = true))) {
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
        if (s != null || earned != null) Txt(470) {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (s != null) Chip(if (g.sRecord) "${count(s, "S grade")}, your most in a semester" else count(s, "S grade"))
                if (earned != null) Chip("${credits(earned)} credits earned")
            }
        }
    }
}

/**
 * Better than last semester: attendance first, its change large over last semester's bar and
 * this one's; the GPA's too, only when the Degree journey doesn't show it. Flexible.
 */
internal fun previousSlide(r: WrappedReport): Slide? {
    val p = r.previous ?: return null
    val attendance = p.attendanceChange?.let { change -> r.attendance?.let { it.percent to change } }
    val gpaPair = p.gpaChange?.takeIf { degreeSlide(r) == null }?.let { change -> r.grades?.let { it.gpa to change } }
    if (attendance == null && gpaPair == null) return null
    val tile = when {
        attendance != null -> Tile(Ink.Success, "+${attendance.second}%", "attendance on last semester")
        gpaPair != null -> Tile(Ink.Success, "+${gpa(gpaPair.second)}", "GPA on last semester", grades = true)
        else -> return null
    }
    val then = shortName(p.semesterName)
    val now = shortName(r.semesterName)
    return Slide("last_semester", Ink.Success, grades = gpaPair != null, tiles = listOf(tile), shareable = { hide -> !hide.grades || attendance != null }, fixed = false) { c ->
        val gpaShown = gpaPair?.takeIf { !c.hide }
        val ink = LocalInks.current
        Flow {
            Label("Better than ${p.semesterName}")
            if (attendance != null) {
                BigBeside("+${attendance.second}", fits(120f, 96f), "points of\nattendance", headingSize = fits(28f, 24f))
            } else if (gpaShown != null) {
                BigBeside("+${gpa(gpaShown.second)}", fits(96f, 80f), "on your\nGPA", headingSize = fits(28f, 24f))
            }
            Spacer(Modifier.weight(1f).heightIn(min = 12.dp))
            if (attendance != null) {
                val (percent, change) = attendance
                Compare("Attendance", "$then, ${percent - change}%", "$now, $percent%", "up $change", (percent - change).toFloat() / percent)
            }
            if (gpaShown != null) {
                val (value, change) = gpaShown
                Box(Modifier.padding(top = if (attendance != null) 18.dp else 0.dp)) {
                    Compare("GPA", "$then, ${gpa(value - change)}", "$now, ${gpa(value)}", "up ${gpa(change)}", ((value - change) / value).toFloat())
                }
            }
            // One fact left: what moved it, so the slide isn't half empty.
            if ((attendance == null) != (gpaShown == null)) {
                val a = r.attendance
                MiniRow(
                    listOfNotNull(
                        a?.let { "${it.attended}" to "classes attended this semester" },
                        r.streak?.let { "${it.length}" to "in a row, your longest run" } ?: r.time?.let { "${(it.minutes + 30) / 60} h" to "in class" },
                    ),
                    Modifier.padding(top = 14.dp),
                    columns = 2,
                )
            }
            Shapes(ink)
        }
    }
}

/** Two small shapes under the facts, so a short slide still has its tone's shapes. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Shapes(ink: Inks) {
    Row(Modifier.fillMaxWidth().padding(top = 14.dp), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.Bottom) {
        Box(Modifier.size(28.dp).background(ink.mid, MaterialShapes.Cookie6Sided.toShape()))
        Box(Modifier.padding(start = 8.dp).size(44.dp).rotate(12f).background(ink.shape, MaterialShapes.SoftBurst.toShape()))
    }
}

/** Last semester's bar over this one's, for one fact. */
@Composable
private fun Compare(label: String, before: String, after: String, change: String, ratio: Float) {
    val ink = LocalInks.current
    Column(Modifier.fillMaxWidth()) {
        Label(label)
        Box(
            Modifier.padding(top = 10.dp).fillMaxWidth(ratio.coerceIn(0.45f, 0.9f)).height(34.dp).background(lerp(ink.bg, ink.shape, 0.4f), RoundedCornerShape(17.dp)).padding(start = 14.dp),
            Alignment.CenterStart,
        ) { Text(before, color = ink.text, style = type(13f, 700, -0.01f, 1.2f), maxLines = 1) }
        Row(
            Modifier.padding(top = 6.dp).fillMaxWidth().height(52.dp).background(ink.shape, RoundedCornerShape(26.dp)).padding(horizontal = 18.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(after, color = ink.onShape, style = type(18f, 800, -0.02f, 1.2f), maxLines = 1)
            Text(change, color = ink.onShape, style = type(18f, 800, -0.02f, 1.2f), maxLines = 1)
        }
    }
}
