package vitapmate.features.wrapped

import androidx.compose.foundation.Canvas
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
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.ScanFace
import vitapmate.core.WrappedFfcs
import vitapmate.core.WrappedFfcsPart
import vitapmate.core.WrappedGrade
import vitapmate.core.WrappedReport

// The week's timetable graded, and the absences VTOP put down to a missing punch.

/** "4.2" for 252 minutes. */
private fun hours(minutes: Int) = ((minutes * 10 + 30) / 60).let { "${it / 10}.${it % 10}" }

/** What a part says when it went well, and when it didn't. */
private fun WrappedFfcs.good(part: WrappedFfcsPart) = when (part) {
    WrappedFfcsPart.EARLY -> if (early == 0) "no 8 AMs" else "few 8 AMs"
    WrappedFfcsPart.GAPS -> "short waits between classes"
    WrappedFfcsPart.LUNCH -> "lunch kept free"
    WrappedFfcsPart.FREE_DAYS -> freeDays.singleOrNull()?.let { "${WEEKDAYS[it]}s free" } ?: "${freeDays.size} free days"
}

private fun WrappedFfcs.bad(part: WrappedFfcsPart) = when (part) {
    WrappedFfcsPart.EARLY -> "${count(early, "class at 8 AM", "classes at 8 AM")} a week"
    WrappedFfcsPart.GAPS -> "${hours(gapMinutes)} h of waiting a week"
    WrappedFfcsPart.LUNCH -> "lunch squeezed on ${count(classDays - lunchFreeDays, "day")}"
    WrappedFfcsPart.FREE_DAYS -> "no day off"
}

/**
 * FFCS verdict: a grade for the week's timetable from four facts (8 AMs, waiting between
 * classes, lunch kept free, days off), the week drawn with the lunch break, and those facts.
 * Not in a first semester. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
internal fun ffcsSlide(r: WrappedReport): Slide? {
    val f = r.ffcs?.takeIf { it.week.isNotEmpty() } ?: return null
    val letter = f.grade.name
    return Slide("ffcs", Ink.Warning, tiles = listOf(Tile(Ink.Warning, letter, "FFCS verdict, ${f.score} of 100")), fixed = false) {
        val ink = LocalInks.current
        Flow {
            Label("FFCS verdict")
            Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                val s = fits(140f, 112f)
                Box(Modifier.size(s.dp).rotate(-4f).background(ink.shape, MaterialShapes.Cookie12Sided.toShape()), Alignment.Center) {
                    Text(letter, color = ink.onShape, style = type(s * 0.52f, 800, -0.05f, 1f), maxLines = 1)
                }
                Column(Modifier.weight(1f)) {
                    H(
                        when (f.grade) {
                            WrappedGrade.S -> "You won\nslot picking"
                            WrappedGrade.A -> "Slot picking,\nnailed"
                            WrappedGrade.B -> "Pretty good\npicks"
                            WrappedGrade.C -> "FFCS had\nits moments"
                            WrappedGrade.D -> "FFCS was\nnot kind"
                            WrappedGrade.E -> "FFCS fought\nback"
                        },
                        fits(26f, 23f),
                    )
                    val why = if (f.best == f.worst) f.good(f.best) else "${f.good(f.best)}, but ${f.bad(f.worst)}"
                    P("Score ${f.score} of 100. ${why.replaceFirstChar { it.uppercase() }}.", 13f, top = 8)
                }
            }
            Week(f, Modifier.fillMaxWidth().weight(1f).padding(top = 22.dp).heightIn(min = 96.dp, max = 190.dp))
            if (f.week.any { it.lab }) Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                listOf(ink.shape to "Theory", Ink.Lab.tone().base to "Lab").forEach { (color, label) ->
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                        Box(Modifier.size(10.dp).background(color, RoundedCornerShape(3.dp)))
                        Text(label, color = ink.text.copy(alpha = 0.85f), style = type(11f, 700, 0f, 1.2f))
                    }
                }
            }
            FlowRow(Modifier.fillMaxWidth().padding(top = 10.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Chip(
                    when (f.freeDays.size) {
                        0 -> "No day off"
                        1 -> "Free ${WEEKDAYS[f.freeDays[0]]}s"
                        else -> "${f.freeDays.size} free days a week"
                    },
                )
                Chip(if (f.lunchFreeDays >= f.classDays) "Lunch untouched" else "Lunch free ${f.lunchFreeDays} of ${f.classDays} days")
                Chip(if (f.early == 0) "No 8 AMs" else "${count(f.early, "class at 8 AM", "classes at 8 AM")} a week")
            }
            MiniRow(
                listOfNotNull(
                    "${hours(f.gapMinutes)} h" to "waiting between classes a week",
                    if (f.sprints > 0) "${f.sprints}" to listOfNotNull(f.sprintFrom, f.sprintTo).joinToString(" → ").ifEmpty { "block" } + " dashes a week"
                    else "${f.lunchFreeDays} of ${f.classDays}" to "lunches kept free",
                ),
                Modifier.padding(top = 10.dp),
                columns = 2,
            )
        }
    }
}

/** The week: a row per day from Monday, classes on an hour axis, the lunch break dashed, a day off named. */
@Composable
private fun Week(f: WrappedFfcs, modifier: Modifier) {
    val ink = LocalInks.current
    val lab = Ink.Lab.tone().base
    val measurer = rememberTextMeasurer()
    val axis = type(11f, 600, 0f, 1.1f).copy(color = ink.text.copy(alpha = 0.85f))
    val bold = type(11f, 800, 0f, 1.1f).copy(color = ink.text)
    val days = if (f.week.any { it.day == 6 }) 7 else 6
    val h0 = minOf(8, f.week.minOf { it.start } / 60)
    val h1 = maxOf(19, (f.week.maxOf { it.end } + 59) / 60)
    Canvas(modifier.padding(top = 4.dp)) {
        val labelW = 32.dp.toPx()
        val w = size.width - labelW
        val rowH = (size.height - 18.dp.toPx()) / days
        val bar = (rowH * 0.78f).coerceAtMost(22.dp.toPx())
        val xOf = { minute: Int -> labelW + w * ((minute / 60f - h0) / (h1 - h0)) }
        val lunch0 = xOf(12 * 60 + 30)
        val lunch1 = xOf(14 * 60)
        drawRoundRect(
            ink.text.copy(alpha = 0.55f), Offset(lunch0, -4.dp.toPx()), Size(lunch1 - lunch0, rowH * days + 2.dp.toPx()), CornerRadius(7.dp.toPx()),
            style = Stroke(1.4.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 4.dp.toPx()))),
        )
        label(measurer, "lunch", axis, Offset((lunch0 + lunch1) / 2, rowH * days + 3.dp.toPx()), center = true, clampTo = size.width)
        for (d in 0 until days) {
            val y = d * rowH
            label(measurer, WEEKDAYS[d].take(3), bold, Offset(0f, y + (bar - 13.dp.toPx()) / 2))
            drawRoundRect(ink.text.copy(alpha = 0.07f), Offset(labelW, y), Size(w, bar), CornerRadius(7.dp.toPx()))
            val today = f.week.filter { it.day == d }
            today.forEach { slot ->
                val x0 = xOf(slot.start) + 1.dp.toPx()
                val x1 = xOf(slot.end) - 1.dp.toPx()
                drawRoundRect(if (slot.lab) lab else ink.shape, Offset(x0, y), Size((x1 - x0).coerceAtLeast(3.dp.toPx()), bar), CornerRadius(6.dp.toPx()))
            }
            if (today.isEmpty() && d < 6) label(measurer, "free, every week", bold.copy(color = ink.shape), Offset(labelW + 8.dp.toPx(), y + (bar - 13.dp.toPx()) / 2))
        }
    }
}

/**
 * Caught by the machine: absences VTOP put down to a missing punch, a face per absence (filled
 * when a class punch came in time after all), and the courses with the most. Flexible.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
internal fun trapsSlide(r: WrappedReport): Slide? {
    val t = r.traps ?: return null
    return Slide("traps", Ink.Danger, tiles = listOf(Tile(Ink.Danger, "${t.trapped}", "“No punch found” absences")), fixed = false) {
        val ink = LocalInks.current
        Flow {
            Label("Caught by the machine")
            Box(Modifier.padding(top = 4.dp)) { BigBeside("${t.trapped}", fits(110f, 92f), "“No punch\nfound”\nabsences", headingSize = fits(24f, 21f)) }
            P("VTOP put ${t.trapped} of your ${count(t.absences, "absence")} down to a missing face scan.", 14f, top = 6)
            // A face per such absence, as many as fit; those punched in on time filled.
            BoxWithConstraints(Modifier.fillMaxWidth().weight(1f).padding(top = 14.dp).heightIn(min = 34.dp).clipToBounds()) {
                val cell = 30.dp
                val perRow = ((maxWidth + 6.dp) / (cell + 6.dp)).toInt().coerceAtLeast(1)
                val rows = ((maxHeight + 6.dp) / (cell + 6.dp)).toInt().coerceAtLeast(1)
                val room = perRow * rows
                val shown = if (t.trapped > room) room - 1 else t.trapped
                FlowRow(Modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    repeat(shown) { i ->
                        val hit = i < t.punched
                        Box(Modifier.size(cell).background(if (hit) ink.shape else ink.text.copy(alpha = 0.14f), RoundedCornerShape(10.dp)), Alignment.Center) {
                            Icon(Lucide.ScanFace, null, Modifier.size(17.dp), tint = if (hit) ink.onShape else ink.text)
                        }
                    }
                    if (shown < t.trapped) Box(Modifier.size(cell).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(10.dp)), Alignment.Center) {
                        Text("+${t.trapped - shown}", color = ink.text, style = type(11f, 800, 0f, 1f), maxLines = 1)
                    }
                }
            }
            if (t.punched > 0) Text(
                "${count(t.punched, "time")}, you had actually punched in. Rude.",
                Modifier.padding(top = 8.dp), color = ink.text, style = type(13f, 700, -0.01f, 1.3f),
            )
            Spacer(Modifier.heightIn(min = 6.dp))
            t.byCourse.take(if (LocalFrame.current.compact) 2 else 3).forEachIndexed { i, c ->
                Row(
                    Modifier.fillMaxWidth().padding(top = 6.dp).rotate(listOf(-1.2f, 0.8f, -0.5f)[i]).background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 11.dp, vertical = 9.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    val shape = listOf(MaterialShapes.Cookie9Sided, MaterialShapes.Clover4Leaf, MaterialShapes.SoftBurst)[i]
                    Box(Modifier.size(40.dp).background(ink.shape, shape.toShape()), Alignment.Center) {
                        Text("${c.percent}×", color = ink.onShape, style = type(14f, 800, -0.03f, 1f), maxLines = 1)
                    }
                    Text(titled(c.name), Modifier.weight(1f), color = ink.text, style = type(15f, 800, -0.025f, 1.15f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}
