package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.TextAutoSize
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.roundToInt
import vitapmate.core.WrappedReport

// Streaks, courses, close calls, rhythm and class time.

internal fun streakSlide(r: WrappedReport): Slide? {
    val s = r.streak ?: return null
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

/**
 * Your class rhythm: the side of the day with most attended classes and classes by start hour,
 * the worst weekday for misses, the quietest weekday, and the 8 AM rate. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun rhythmSlide(r: WrappedReport): Slide? {
    val d = r.dayHalf
    val h = r.habits
    val worst = h?.worstWeekday?.takeIf { h.worstWeekdayMisses > 0 }
    val quiet = h?.quietestWeekday
    val early = h?.earlyPercent?.takeIf { h.earlyClasses > 0 }
    if (d == null && worst == null && quiet == null && early == null) return null
    val tile = when {
        d != null -> Tile(Ink.Lab, "${d.percent}%", (if (d.morning) "before noon" else "after 2 PM") + (early?.let { ", 8 AM $it%" } ?: ""), span = 3)
        worst != null -> Tile(Ink.Lab, WEEKDAYS[worst].take(3), "worst day" + (early?.let { ", 8 AM $it%" } ?: ""))
        early != null -> Tile(Ink.Lab, "$early%", "of 8 AM classes attended")
        else -> Tile(Ink.Lab, WEEKDAYS[quiet ?: 0].take(3), "quietest day")
    }
    return Slide("rhythm", Ink.Lab, tiles = listOf(tile), fixed = false) {
        val ink = LocalInks.current
        Flow {
            if (d != null) {
                Row(verticalAlignment = Alignment.Top) {
                    Big("${d.percent}%", fits(96f, 80f), Modifier.weight(1f))
                    // A sun either way.
                    Box(Modifier.size(fits(88f, 68f).dp).background(Ink.Warning.tone().base, MaterialShapes.Cookie12Sided.toShape()))
                }
                H(if (d.morning) "A morning person" else "An afternoon person", fits(30f, 26f), Modifier.padding(top = 6.dp))
                P("${d.classes} of ${count(d.total, "class", "classes")} started " + if (d.morning) "before noon" else "at 2 PM or later", 14f, top = 6)
                // Classes by start hour, 8 AM to 6 PM; the winning side in the tone.
                Column(Modifier.weight(1f).padding(top = 14.dp).heightIn(min = 64.dp)) {
                    Row(Modifier.fillMaxWidth().weight(1f), horizontalArrangement = Arrangement.spacedBy(5.dp), verticalAlignment = Alignment.Bottom) {
                        val most = d.byHour.maxOrNull()?.coerceAtLeast(1) ?: 1
                        d.byHour.forEachIndexed { i, n ->
                            val on = if (d.morning) i < 4 else i >= 6
                            Box(
                                Modifier.weight(1f).fillMaxHeight((n.toFloat() / most).coerceAtLeast(0.05f))
                                    .background(if (on) ink.shape else ink.text.copy(alpha = 0.16f), RoundedCornerShape(10.dp)),
                            )
                        }
                    }
                    Row(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                        listOf("8 AM", "noon", "2 PM", "6 PM").forEach { Text(it, color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f)) }
                    }
                }
            } else {
                H("Your class rhythm", fits(34f, 28f))
                Spacer(Modifier.weight(1f))
            }
            val minis = listOfNotNull(
                worst?.let { w -> Triple(w, "worst day, ${h.worstWeekdayMisses} of ${count(h.misses, "miss", "misses")}", true) },
                quiet?.let { q -> Triple(q, "quietest, ${tenths(h.classesPerDayTenths.getOrElse(q) { 0 })} classes a day", false) },
            )
            if (h != null && minis.isNotEmpty()) Row(Modifier.fillMaxWidth().padding(top = 14.dp).height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                minis.forEach { (day, line, isWorst) ->
                    val pick = if (isWorst) Ink.Danger.tone().base else ink.shape
                    Column(Modifier.weight(1f).fillMaxHeight().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 11.dp)) {
                        Text(WEEKDAYS[day], color = pick, style = type(21f, 800, -0.04f, 1.15f), maxLines = 1, autoSize = TextAutoSize.StepBased(minFontSize = 14.sp, maxFontSize = 21.sp))
                        Text(line, color = ink.text, style = type(12f, 600, -0.01f, 1.3f), maxLines = 2)
                        // Each weekday's misses (worst) or classes a day (quietest), Monday to Saturday.
                        val values = (if (isWorst) h.missesByWeekday else h.classesPerDayTenths).take(6)
                        val top = values.maxOrNull()?.coerceAtLeast(1) ?: 1
                        Row(Modifier.padding(top = 8.dp).height(22.dp), horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.Bottom) {
                            values.forEachIndexed { i, v ->
                                Box(Modifier.width(6.dp).height((4f + 18f * v / top).dp).background(if (i == day) pick else ink.text.copy(alpha = 0.26f), RoundedCornerShape(3.dp)))
                            }
                        }
                    }
                }
                if (minis.size == 1) Box(Modifier.weight(1f))
            }
            if (early != null) Row(
                Modifier.fillMaxWidth().padding(top = 8.dp).background(ink.panel, RoundedCornerShape(26.dp)).padding(horizontal = 20.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Big("$early%", fits(40f, 34f))
                Text("of your 8 AM classes attended", color = ink.text, style = type(14f, 600, -0.01f, 1.3f))
            }
        }
    }
}

/** 21 as "2.1". */
private fun tenths(n: Int) = "${n / 10}.${n % 10}"

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun coursesSlide(r: WrappedReport): Slide? {
    val a = r.attendance?.takeIf { it.best.isNotBlank() && it.least.isNotBlank() && it.best != it.least } ?: return null
    val top = a.top.filter { it.name.isNotBlank() && it.name != a.least }.map { it.name to it.percent }.ifEmpty { listOf(a.best to a.bestPercent) }
    return Slide(
        "courses",
        Ink.Success,
        foot = Ink.Lab,
        tiles = listOfNotNull(
            Tile(Ink.Success, titled(a.best), "most attended, ${a.bestPercent}%", span = 3, small = true),
            Tile(Ink.Lab, titled(a.least), "least attended, ${a.leastPercent}%", span = 3, small = true),
            r.neverMissed.takeIf { it.isNotEmpty() }?.let { Tile(Ink.Lab, "${it.size}", if (it.size == 1) "course never missed" else "courses never missed") },
        ),
    ) {
        val ink = LocalInks.current
        val lab = inks(Ink.Lab)
        // The lower part in the lab tone, cut on a slant, carried on past the sides and the
        // bottom so a wide or tall screen's margins show it too (a share image clips it).
        Canvas(Modifier.size(360.dp, 640.dp)) {
            val (w, h) = size.width to size.height
            val path = Path().apply {
                moveTo(-w, h * 0.68f); lineTo(w * 2, h * 0.32f)
                lineTo(w * 2, h * 3); lineTo(-w, h * 3); close()
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
                Text(titled(name), color = ink.text, style = type(if (name.length > 22) 13f else 15f, 700, -0.01f, 1.1f), maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
        }
        // Never missed: the 100% courses, one badge in place of their own slide.
        val never = r.neverMissed
        if (never.isNotEmpty()) Box(Modifier.offset(24.dp, (182 + 54 * (top.size - 1)).dp)) {
            Chip(if (never.size == 1) "Never missed: ${titled(never[0])}" else "Never missed in ${never.size} courses")
        }
        Shaped(RoundedCornerShape(36.dp), 22, 356, 132, 72, color = lab.shape, text = "${a.leastPercent}%", size = 38f, textColor = lab.onShape, rotate = -6f)
        Txt(446) {
            Text("Least attended", color = lab.text.copy(alpha = 0.8f), style = type(14f, 600, -0.01f, 1.3f))
            Text(titled(a.least), Modifier.padding(top = 8.dp), color = lab.text, style = type(32f))
        }
    }
}

/** The tightest any course got to 75%: classes to spare then, or how far under it dipped. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun closeCallSlide(r: WrappedReport): Slide? {
    val c = r.closeCall ?: return null
    val course = titled(c.course)
    val spare = c.spare
    val tile = if (spare != null) Tile(Ink.Danger, count(spare, "class", "classes"), "to spare in $course", span = 3)
    else Tile(Ink.Danger, "${c.percent}%", "low in $course", span = 3)
    return Slide("close_call", Ink.Danger, tiles = listOf(tile)) {
        val ink = LocalInks.current
        Txt(58) { Label("Close call") }
        Txt(84) {
            if (spare != null) BigBeside("$spare", 150f, if (spare == 1) "class\nto spare" else "classes\nto spare")
            else BigBeside("${c.percent}%", 120f, "your\nlowest")
        }
        Txt(252) {
            P(
                when (spare) {
                    null -> "You dipped to ${c.percent}% in $course on ${day(c.day)}. You finished at ${c.finalPercent}%."
                    0 -> "$course on ${day(c.day)}: any miss and it was under 75%."
                    else -> "$course on ${day(c.day)}: you could miss just ${count(spare, "more class", "more classes")} and stay at 75%."
                },
                top = 0,
            )
        }
        // 50% to 100%, the 75% line, the moment's percent.
        Txt(332) {
            val at = ((c.percent - 50) / 50f).coerceIn(0f, 1f)
            BoxWithConstraints(Modifier.fillMaxWidth().height(44.dp).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(22.dp))) {
                Box(Modifier.width(maxWidth * at).fillMaxHeight().background(ink.shape, RoundedCornerShape(22.dp)))
                Box(Modifier.offset(maxWidth / 2 - 3.dp, (-10).dp).size(6.dp, 64.dp).background(ink.text, RoundedCornerShape(3.dp)))
                Box(Modifier.offset(x = (maxWidth * at - 22.dp).coerceIn(0.dp, maxWidth - 44.dp)).size(44.dp).background(ink.onShape, MaterialShapes.Cookie9Sided.toShape()))
            }
            Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                listOf("50%", "75%", "100%").forEach {
                    val line = it == "75%"
                    Text(it, color = ink.text.copy(alpha = if (line) 1f else 0.8f), style = type(12f, if (line) 800 else 600, 0f, 1.2f))
                }
            }
        }
        Minis(420, listOf("${c.percent}%" to "that day", "${c.finalPercent}%" to "where it finished"))
    }
}

/**
 * Hours in class on a tilted pill, theory against labs, and where they went: the room and the
 * faculty with the most of them. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun timeSlide(r: WrappedReport): Slide? {
    val t = r.time?.takeIf { it.minutes > 0 } ?: return null
    val hours = (t.minutes + 30) / 60
    val labHours = (t.labMinutes + 30) / 60
    val theoryHours = hours - labHours
    val busiest = WEEKDAYS.getOrNull(t.busiestWeekday)
    val p = r.places
    val room = p?.room?.takeIf { it.isNotBlank() && p.roomMinutes > 0 }
    val faculty = p?.faculty?.takeIf { it.isNotBlank() && p.facultyMinutes > 0 }?.let(::titled)
    val roomHours = (p?.roomMinutes ?: 0).plus(30) / 60
    val facultyHours = (p?.facultyMinutes ?: 0).plus(30) / 60
    val tiles = listOfNotNull(
        Tile(Ink.Accent, "$hours h", "in class" + (busiest?.let { ", busiest ${it.take(3)}" } ?: "")),
        room?.let { Tile(Ink.Warning, it, "your room, ${count(roomHours, "hour")}", small = true) }
            ?: faculty?.let { Tile(Ink.Warning, it, "${count(facultyHours, "hour")} together", small = true) },
    )
    return Slide("time", Ink.Accent, tiles = tiles, fixed = false) {
        val ink = LocalInks.current
        Flow {
            // The hours on a pill that runs off both sides.
            Box(Modifier.fillMaxWidth().height(fits(150f, 118f).dp), Alignment.Center) {
                Box(
                    Modifier.requiredWidth(380.dp).height(fits(150f, 116f).dp).rotate(-9f).background(ink.shape, RoundedCornerShape(80.dp)),
                    Alignment.Center,
                ) { Text("$hours", color = ink.onShape, style = type(fits(if (hours >= 1000) 96f else 116f, 92f), 800, -0.05f, 1f), maxLines = 1) }
            }
            H(if (hours == 1) "hour in class" else "hours in class", fits(34f, 30f), Modifier.padding(top = fits(26f, 18f).dp))
            P("${count(t.classes, "class", "classes")} sat through" + (busiest?.let { ", the most on ${it}s" } ?: ""), 15f, top = 6)
            if (labHours > 0 && theoryHours > 0) {
                val lab = inks(Ink.Lab)
                Box(Modifier.padding(top = 14.dp)) {
                    Split(
                        44,
                        listOf(
                            Triple("Theory $theoryHours h", theoryHours.toFloat(), ink.shape to ink.onShape),
                            Triple("Labs $labHours h", labHours.toFloat(), lab.shape to lab.onShape),
                        ),
                    )
                }
            }
            Spacer(Modifier.weight(1f).heightIn(min = 12.dp))
            if (room != null || faculty != null) Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (room != null) Column(Modifier.weight(1f).fillMaxHeight().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 12.dp)) {
                    Text("Your room", color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, -0.01f, 1.2f))
                    Text(room, Modifier.padding(top = 4.dp), color = ink.shape, style = type(26f, 800, -0.04f, 1.1f), maxLines = 2)
                    Text("${count(roomHours, "hour")} here, more than anywhere", Modifier.padding(top = 4.dp), color = ink.text, style = type(12f, 600, -0.01f, 1.3f), maxLines = 3)
                }
                if (faculty != null) Column(Modifier.weight(1f).fillMaxHeight().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 12.dp)) {
                    Text("Your faculty", color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, -0.01f, 1.2f))
                    Row(Modifier.padding(top = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        val initials = faculty.split(' ').filter { it.isNotBlank() && !it.endsWith('.') }.take(2).joinToString("") { it.take(1) }
                        Box(Modifier.size(40.dp).background(ink.mid, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                            Text(initials, color = ink.text, style = type(15f, 800, -0.03f, 1f), maxLines = 1)
                        }
                        Text(faculty, Modifier.weight(1f), color = ink.text, style = type(15f, 800, -0.02f, 1.15f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                    }
                    Text("${count(facultyHours, "hour")} of class together", Modifier.padding(top = 6.dp), color = ink.text, style = type(12f, 600, -0.01f, 1.3f), maxLines = 2)
                }
            }
        }
    }
}
