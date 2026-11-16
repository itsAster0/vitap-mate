package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.BaselineShift
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import kotlin.math.roundToInt
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedTitle

// Slides 1 to 9: the semester, attendance and class time.

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun openingSlide(r: WrappedReport) = Slide("opening", Ink.Accent, semesterLine = false) { c ->
    Shaped(MaterialShapes.Cookie9Sided.toShape(), 176, 336, 330)
    Shaped(MaterialShapes.Clover8Leaf.toShape(), 88, 400, 130, color = Ink.Success.tone().base)
    Shaped(RoundedCornerShape(29.dp), 196, 300, 150, 58, color = Ink.Warning.tone().base, rotate = -18f)
    Txt(64) {
        if (c.name != null) P("${c.name}, this was", 18f, top = 0)
        val words = "Your ${r.semesterName}, wrapped"
        H(words, if (words.length > 26) 50f else 62f, Modifier.padding(top = 14.dp))
    }
    val lines = listOfNotNull(
        "${day(r.firstDay)} to ${day(r.lastDay)}",
        r.courses.takeIf { it > 0 }?.let { count(it, "course") },
        r.classDayCount.takeIf { it > 0 }?.let { count(it, "class day") },
        r.holidays.takeIf { it > 0 }?.let { count(it, "holiday") },
    )
    Text(lines.joinToString("\n"), Modifier.align(Alignment.BottomStart).padding(start = 24.dp, bottom = if (c.share) 84.dp else 28.dp), color = LocalInks.current.text, style = type(15f, 600, -0.01f, 1.5f))
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun attendanceSlide(r: WrappedReport): Slide? {
    val a = r.attendance?.takeIf { it.counted > 0 } ?: return null
    val od = a.onDuty.takeIf { it > 0 }
    return Slide("attendance", Ink.Success, tiles = listOf(Tile(Ink.Success, "${a.percent}%", "attendance" + (od?.let { ", $it on OD" } ?: "")))) {
        val number = buildAnnotatedString {
            append("${a.percent}")
            withStyle(SpanStyle(fontSize = 0.45.em, baselineShift = BaselineShift(0.7f))) { append("%") }
        }
        At(66, 64, 330, 330) {
            Box(Modifier.size(330.dp).background(LocalInks.current.shape, MaterialShapes.Cookie12Sided.toShape()))
            Text(number, Modifier.align(Alignment.Center), color = LocalInks.current.onShape, style = type(120f, 800, -0.05f, 1f))
        }
        Txt(410) {
            H("You showed up.")
            P("${a.attended} of ${a.counted} classes attended" + if (r.courses > 1) ", across all ${r.courses} courses" else "")
            if (od != null) Chip("$od of them on OD", Modifier.padding(top = 14.dp))
        }
    }
}

internal fun streakSlide(r: WrappedReport): Slide? {
    val s = r.streak?.takeIf { it.length > 1 } ?: return null
    val weeks = s.perfectWeeks.takeIf { it > 0 }
    val total = s.total.takeIf { it >= s.length }
    val share = total?.let { (s.length * 100f / it).roundToInt() }
    return Slide("streak", Ink.Warning, tiles = listOf(Tile(Ink.Warning, "${s.length}", "in a row" + (share?.let { ", $it% of classes" } ?: "")))) {
        val ink = LocalInks.current
        Txt(56) { Big("${s.length}", 160f) }
        Txt(214) {
            H("classes in a row")
            P("Your longest run without a miss, ${day(s.firstDay)} to ${day(s.lastDay)}")
        }
        Txt(340) {
            if (total != null && share != null) {
                // Every posted class of the semester in time order, the run where it fell.
                BoxWithConstraints(Modifier.fillMaxWidth().height(44.dp).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(22.dp))) {
                    val run = (maxWidth * s.length / total).coerceIn(16.dp, maxWidth)
                    val x = (maxWidth * s.offset.coerceAtLeast(0) / total).coerceIn(0.dp, maxWidth - run)
                    Box(Modifier.offset(x = x).width(run).fillMaxHeight().background(ink.shape, RoundedCornerShape(22.dp)))
                }
                Row(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(day(r.firstDay), color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f))
                    Text(day(r.lastDay), color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f))
                }
                Text("${s.length} of $total classes, $share%", Modifier.padding(top = 10.dp), color = ink.text, style = type(15f, 700, -0.01f, 1.3f))
            }
            if (weeks != null) {
                Text(count(weeks, "perfect week"), Modifier.padding(top = 16.dp), color = ink.text, style = type(15f, 700, -0.01f, 1.3f))
                Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    repeat(weeks.coerceAtMost(17)) { Box(Modifier.size(13.dp, 26.dp).background(ink.shape, RoundedCornerShape(7.dp))) }
                }
            }
        }
    }
}

internal fun neverMissedSlide(r: WrappedReport): Slide? {
    val names = r.neverMissed.takeIf { it.isNotEmpty() } ?: return null
    return Slide("never_missed", Ink.Lab, tiles = listOf(Tile(Ink.Lab, "${names.size}", if (names.size == 1) "course never missed" else "courses never missed"))) {
        val ink = LocalInks.current
        Txt(60) { Big("100%", 118f) }
        Txt(186) { H("Never missed, in ${count(names.size, "course")}") }
        val spots = listOf(Triple(20, 312, -5f), Triple(50, 402, 4f), Triple(30, 492, -3f))
        names.take(3).forEachIndexed { i, name ->
            val (x, y, turn) = spots[i]
            val base = i % 2 == 0
            Shaped(
                RoundedCornerShape(38.dp), x, y, 300 - (i % 2) * 10, 76,
                color = if (base) ink.shape else ink.mid, text = titled(name), size = 23f,
                textColor = if (base) ink.onShape else ink.text, rotate = turn, track = -0.03f,
            )
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun comebackSlide(r: WrappedReport): Slide? {
    val c = r.comeback ?: return null
    // The title is this comeback: its tile says it, so no second one.
    val tiles = if (r.title == WrappedTitle.COMEBACK) emptyList() else listOf(Tile(Ink.Accent, "${c.lowPercent} → ${c.finalPercent}%", "back from ${month(c.lowDay)}"))
    return Slide("comeback", Ink.Accent, tiles = tiles) {
        val ink = LocalInks.current
        Txt(62) {
            H("The comeback")
            P("Down to ${c.lowPercent}% in ${month(c.lowDay)}. You finished at ${c.finalPercent}%.")
        }
        At(0, 200, 360, 300) {
            Canvas(Modifier.size(360.dp, 300.dp)) {
                scale(size.width / 360f, Offset.Zero) {
                    val down = Path().apply { moveTo(30f, 90f); cubicTo(80f, 100f, 100f, 215f, 150f, 222f) }
                    val up = Path().apply { moveTo(150f, 222f); cubicTo(200f, 229f, 250f, 110f, 318f, 52f) }
                    drawPath(down, ink.shape.copy(alpha = 0.3f), style = Stroke(16f, cap = StrokeCap.Round))
                    drawPath(up, ink.shape, style = Stroke(16f, cap = StrokeCap.Round))
                    drawCircle(ink.bg, 15f, Offset(150f, 222f))
                    drawCircle(ink.shape, 15f, Offset(150f, 222f), style = Stroke(7f))
                }
            }
        }
        Shaped(MaterialShapes.Clover8Leaf.toShape(), 262, 196, 104, text = "${c.finalPercent}%", size = 28f)
        At(112, 446) {
            Column {
                Text("${c.lowPercent}%", color = ink.text, style = type(26f, 800, -0.04f, 1.1f))
                Text(month(c.lowDay), color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f))
            }
        }
    }
}

internal fun habitsSlide(r: WrappedReport): Slide? {
    val h = r.habits ?: return null
    val worst = h.worstWeekday?.takeIf { h.worstWeekdayMisses > 0 }
    val early = h.earlyPercent?.takeIf { h.earlyClasses > 0 }
    val minis = listOfNotNull(
        h.firstSlotDays.takeIf { it > 0 }?.let { count(it, "day") to "started at the first slot" },
        h.lastSlotDays.takeIf { it > 0 }?.let { count(it, "day") to "ended at the last slot" },
    )
    if (worst == null && early == null && minis.isEmpty()) return null
    val tile = when {
        worst != null -> Tile(Ink.Danger, WEEKDAYS[worst].take(3), "worst day" + (early?.let { ", 8 AM $it%" } ?: ""))
        early != null -> Tile(Ink.Danger, "$early%", "of 8 AM classes attended")
        else -> Tile(Ink.Danger, minis[0].first, minis[0].second)
    }
    return Slide("habits", Ink.Danger, tiles = listOf(tile)) {
        val ink = LocalInks.current
        Txt(58) {
            if (worst != null) {
                H("${WEEKDAYS[worst]} was your worst day", 30f)
                P("${h.worstWeekdayMisses} of your ${count(h.misses, "miss", "misses")}")
            } else H("Your days", 30f)
        }
        // Misses per weekday, the worst day the tallest with its count; Sunday only with misses.
        if (worst != null) Txt(186) {
            Row(Modifier.fillMaxWidth().height(150.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.Bottom) {
                val misses = h.missesByWeekday
                val days = if (worst == 6 || misses.getOrElse(6) { 0 } > 0) 7 else 6
                val most = misses.take(days).maxOrNull()?.coerceAtLeast(1) ?: 1
                repeat(days) { d ->
                    val on = d == worst
                    val n = misses.getOrElse(d) { 0 }
                    val tall = if (on) 122 else if (n == 0) 8 else (122 * n / most).coerceIn(17, 122)
                    Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                        Box(
                            Modifier.fillMaxWidth().height(tall.dp)
                                .background(if (on) ink.shape else ink.text.copy(alpha = 0.22f), RoundedCornerShape(if (n == 0 && !on) 4.dp else 17.dp)),
                            Alignment.TopCenter,
                        ) { if (on) Text("${h.worstWeekdayMisses}", Modifier.padding(top = 8.dp), color = ink.onShape, style = type(22f, 800, -0.03f, 1f)) }
                        Text(WEEKDAYS[d].take(1), Modifier.padding(top = 6.dp), color = if (on) ink.shape else ink.text, style = type(13f, 700, 0f, 1.2f))
                    }
                }
            }
        }
        if (early != null) Panel(if (worst != null) 358 else 150) {
            Row(Modifier.padding(horizontal = 20.dp, vertical = 14.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Big("$early%", 44f)
                Text("of your 8 AM classes attended", color = ink.text, style = type(15f, 600, -0.01f, 1.3f))
            }
        }
        Minis(if (worst != null) 450 else 250, minis)
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun coursesSlide(r: WrappedReport): Slide? {
    val a = r.attendance?.takeIf { it.best.isNotBlank() && it.least.isNotBlank() && it.best != it.least } ?: return null
    val top = a.top.filter { it.name.isNotBlank() && it.name != a.least }.map { it.name to it.percent }.ifEmpty { listOf(a.best to a.bestPercent) }
    return Slide(
        "courses",
        Ink.Success,
        tiles = listOf(
            Tile(Ink.Success, titled(a.best), "most attended, ${a.bestPercent}%", span = 3, small = true),
            Tile(Ink.Lab, titled(a.least), "least attended, ${a.leastPercent}%", span = 3, small = true),
        ),
    ) {
        val ink = LocalInks.current
        val lab = inks(Ink.Lab)
        // The lower part in the lab tone, cut on a slant.
        Canvas(Modifier.size(360.dp, 640.dp)) {
            val path = Path().apply {
                moveTo(0f, size.height * 0.56f); lineTo(size.width, size.height * 0.44f)
                lineTo(size.width, size.height); lineTo(0f, size.height); close()
            }
            drawPath(path, lab.bg)
        }
        Txt(56) { Label(if (top.size > 1) "Most attended, top ${top.size}" else "Most attended") }
        // The best in its shape with the name beside, the next two as pills that narrow above the slant.
        val (first, firstPercent) = top.first()
        Shaped(MaterialShapes.Cookie7Sided.toShape(), 24, 82, 88, text = "$firstPercent%", size = 25f)
        Column(Modifier.offset(126.dp, 82.dp).width(210.dp).height(88.dp), verticalArrangement = Arrangement.Center) {
            Text(titled(first), color = ink.text, style = type(24f), maxLines = 3, overflow = TextOverflow.Ellipsis)
        }
        top.drop(1).forEachIndexed { i, (name, percent) ->
            Row(
                Modifier.offset(24.dp, (182 + 54 * i).dp).width((300 - 32 * i).dp).height(48.dp)
                    .background(ink.mid, RoundedCornerShape(24.dp)).padding(horizontal = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("$percent%", Modifier.width(62.dp), color = ink.text, style = type(20f, 800, -0.04f, 1f), maxLines = 1)
                Text(titled(name), color = ink.text, style = type(15f, 700, -0.01f, 1.2f), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Shaped(RoundedCornerShape(36.dp), 22, 356, 132, 72, color = lab.shape, text = "${a.leastPercent}%", size = 38f, textColor = lab.onShape, rotate = -6f)
        Txt(446) {
            Text("Least attended", color = lab.text.copy(alpha = 0.8f), style = type(14f, 600, -0.01f, 1.3f))
            Text(titled(a.least), Modifier.padding(top = 8.dp), color = lab.text, style = type(32f))
        }
    }
}

internal fun timeSlide(r: WrappedReport): Slide? {
    val t = r.time?.takeIf { it.minutes > 0 } ?: return null
    val hours = (t.minutes + 30) / 60
    val labHours = (t.labMinutes + 30) / 60
    val theoryHours = hours - labHours
    val busiest = WEEKDAYS.getOrNull(t.busiestWeekday)
    return Slide("time", Ink.Accent, tiles = listOf(Tile(Ink.Accent, "$hours h", "in class" + (busiest?.let { ", busiest $it" } ?: "")))) {
        val ink = LocalInks.current
        Shaped(RoundedCornerShape(80.dp), -30, 76, 380, 160, text = "$hours", size = if (hours >= 1000) 96f else 116f, rotate = -9f)
        Txt(270) {
            H(if (hours == 1) "hour in class" else "hours in class")
            P("${count(t.classes, "class", "classes")} sat through")
        }
        if (labHours > 0 && theoryHours > 0) {
            val lab = inks(Ink.Lab)
            SplitBar(
                368, 44,
                listOf(
                    Triple("Theory $theoryHours h", theoryHours.toFloat(), ink.shape to ink.onShape),
                    Triple("Labs $labHours h", labHours.toFloat(), lab.shape to lab.onShape),
                ),
            )
        }
        if (busiest != null) {
            // A circle per weekday sized by its classes, the busiest the largest and named;
            // Sunday only with classes.
            Txt(436) {
                Row(Modifier.fillMaxWidth().height(58.dp), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    val classes = t.classesByWeekday
                    val days = if (t.busiestWeekday == 6 || classes.getOrElse(6) { 0 } > 0) 7 else 6
                    val most = classes.take(days).maxOrNull()?.coerceAtLeast(1) ?: 1
                    // Up to 50 dp, 40 when Sunday shows too, so seven fit in the row.
                    val grow = if (days == 7) 24 else 34
                    repeat(days) { d ->
                        if (d == t.busiestWeekday) Box(Modifier.size(58.dp).background(ink.shape, CircleShape), Alignment.Center) {
                            Text(busiest.take(3), color = ink.onShape, style = type(15f, 800, -0.02f, 1f))
                        } else Box(Modifier.size((16 + grow * classes.getOrElse(d) { 0 } / most).dp).background(ink.text.copy(alpha = 0.24f), CircleShape))
                    }
                }
            }
            Txt(508) { Text("$busiest was your busiest day", color = ink.text, style = type(15f, 600, -0.01f, 1.3f)) }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun placesSlide(r: WrappedReport): Slide? {
    val p = r.places ?: return null
    val room = p.room?.takeIf { it.isNotBlank() && p.roomMinutes > 0 }
    val faculty = p.faculty?.takeIf { it.isNotBlank() && p.facultyClasses > 0 }?.let(::titled)
    if (room == null && faculty == null) return null
    val roomHours = (p.roomMinutes + 30) / 60
    val tile = if (room != null) Tile(Ink.Warning, room, "your room, ${count(roomHours, "hour")}", small = true)
    else Tile(Ink.Warning, faculty!!, "${count(p.facultyClasses, "class", "classes")} together", small = true)
    return Slide("places", Ink.Warning, tiles = listOf(tile)) {
        val ink = LocalInks.current
        val facultyY = if (room != null) 340 else 62
        if (room != null) {
            Txt(62) { Label("Your room") }
            Shaped(RoundedCornerShape(44.dp, 44.dp, 44.dp, 12.dp), 24, 90, 312, 150, text = room, size = if (room.length > 8) 44f else 60f)
            Txt(254) { P("${count(roomHours, "hour")} in this room, more than any other", top = 0) }
        }
        if (faculty != null) {
            Txt(facultyY) { Label("Your faculty") }
            val initials = faculty.split(' ').filter { it.isNotBlank() && !it.endsWith('.') }.take(2).joinToString("") { it.take(1) }
            Shaped(MaterialShapes.Cookie9Sided.toShape(), 24, facultyY + 30, 112, color = ink.mid, text = initials, size = 34f, textColor = ink.text)
            Column(Modifier.offset(152.dp, (facultyY + 48).dp).width(184.dp)) {
                H(faculty, 26f)
                P("${count(p.facultyClasses, "class", "classes")} together", top = 6)
            }
        }
    }
}
