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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import java.util.Locale
import vitapmate.core.WrappedCampus
import vitapmate.core.WrappedReport

// Outings and campus times.

internal fun outingsSlide(r: WrappedReport): Slide? {
    val o = r.outings?.takeIf { it.leaves + it.weekends > 0 } ?: return null
    val trips = o.leaves + o.weekends
    return Slide("outings", Ink.Warning, tiles = listOf(Tile(Ink.Warning, count(trips, "trip"), if (o.daysAway > 0) "${count(o.daysAway, "day")} away" else "off campus", span = 3))) {
        val ink = LocalInks.current
        Txt(52) {
            BigBeside("$trips", 130f, if (trips == 1) "trip off\ncampus" else "trips off\ncampus", headingSize = 30f)
        }
        if (o.leaves > 0) Shaped(RoundedCornerShape(30.dp), 24, 206, 190, 60, text = count(o.leaves, "leave"), size = 22f, rotate = -5f, track = -0.03f)
        if (o.weekends > 0) Shaped(
            RoundedCornerShape(30.dp), 118, 268, 220, 60, color = ink.mid,
            text = count(o.weekends, "weekend outing"), size = 20f, textColor = ink.text, rotate = 4f, track = -0.03f,
        )
        Minis(
            356,
            listOfNotNull(
                o.topPlace?.takeIf { it.isNotBlank() }?.let { titled(it) to "most visited" },
                o.longestLeaveDays?.takeIf { it > 0 }?.let { count(it, "day") to "longest leave" },
                o.daysAway.takeIf { it > 0 }?.let { count(it, "day") to "away in total" },
                o.longestStayDays.takeIf { it > 0 }?.let { count(it, "day") to "longest stretch on campus" },
            ),
        )
    }
}

/**
 * Campus times from the cached biometric days; with none cached, only the offer to fetch them
 * (no Share then). While days are missing, the Fetch row, or the worker's progress.
 */
internal fun campusSlide(r: WrappedReport, hasMissing: Boolean): Slide? {
    val c = r.campus ?: return null
    if (c.daysCached == 0 && !hasMissing) return null
    val tiles = if (c.daysCached == 0) emptyList()
    else listOf(Tile(Ink.Lab, count(c.daysOnCampus, "day"), "on campus" + (c.earliestEntry?.let { ", from $it" } ?: ""), span = 3))
    return Slide("campus", Ink.Lab, tiles = tiles, shareable = { c.daysCached > 0 }) { ctx ->
        val ink = LocalInks.current
        val fetch = ctx.campus
        val days = fetch.missing.size
        if (c.daysCached == 0) {
            Arc(96, null)
            Txt(300) {
                H("Add campus times")
                P("Your earliest entry, latest return and days on campus. ${count(days, "day")} to fetch from VTOP, a few days at a time.")
            }
            if (!ctx.share) FetchRow(422, fetch, days, offer = true)
            return@Slide
        }
        Arc(76, c.daysOnCampus.toFloat() / c.daysCached.coerceAtLeast(1))
        Txt(150) {
            Box(Modifier.fillMaxWidth(), Alignment.Center) { Big("${c.daysOnCampus}", 84f) }
            Text("days on campus", Modifier.fillMaxWidth().padding(top = 2.dp), color = ink.text, style = type(16f, 700, -0.01f, 1.2f), textAlign = TextAlign.Center)
        }
        Txt(292) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                c.earliestEntry?.let { Clock(it, "earliest entry", Alignment.Start) } ?: Box(Modifier)
                c.latestReturn?.let { Clock(it, "latest return", Alignment.End) }
            }
        }
        if (c.daysCached != c.daysTotal) Txt(378) { Chip("from ${c.daysCached} of ${c.daysTotal} days") }
        if (!ctx.share && (days > 0 || fetch.running)) FetchRow(432, fetch, days, offer = false)
    }
}

/** Out of the hostel on a class day: hours, the day's out and in runs, and naps. */
internal fun hostelSlide(r: WrappedReport): Slide? {
    val h = r.hostel ?: return null
    val hours = "${h.outTenths / 10}.${h.outTenths % 10}"
    val naps = h.naps.takeIf { it > 0 }
    return Slide("hostel", Ink.Success, tiles = listOf(Tile(Ink.Success, "$hours h", "out of the hostel a day" + (naps?.let { ", ${count(it, "nap")}" } ?: ""), span = 3))) {
        val ink = LocalInks.current
        Txt(58) { Label("Hostel life") }
        Txt(84) {
            BigBeside(hours, 112f, "h", gap = 10)
        }
        Txt(206) { H("out of the hostel on a class day", 26f) }
        // 6 AM to midnight in half hours: runs out in the tone, runs in faint.
        val runs = mutableListOf<Pair<Boolean, Int>>()
        h.outByHalfHour.forEach { out -> if (runs.lastOrNull()?.first == out) runs[runs.lastIndex] = out to runs.last().second + 1 else runs += out to 1 }
        SplitBar(290, 44, runs.map { (out, n) -> Triple("", n.toFloat(), (if (out) ink.shape else ink.text.copy(alpha = 0.16f)) to ink.text) })
        Txt(342) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                listOf("6 AM", "noon", "6 PM", "midnight").forEach { Text(it, color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f)) }
            }
        }
        if (naps != null) Panel(378) {
            Row(Modifier.padding(horizontal = 20.dp, vertical = 14.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Big("$naps", 44f)
                Text(
                    (if (naps == 1) "trip" else "trips") + " back to your room between classes" + (h.napWeekday?.let { ", most on ${WEEKDAYS[it]}" } ?: ""),
                    color = ink.text,
                    style = type(15f, 600, -0.01f, 1.3f),
                )
            }
        }
        BiometricNote(478, hostel = true, r.campus)
    }
}

/** Floors climbed between classes, rooms punched in, hops between blocks. */
internal fun movesSlide(r: WrappedReport): Slide? {
    val m = r.moves ?: return null
    return Slide("moves", Ink.Lab, tiles = listOf(Tile(Ink.Lab, "${m.floors}", "floors climbed, ${count(m.rooms, "room")}", span = 3))) {
        val ink = LocalInks.current
        // A staircase at the top right.
        Shaped(RoundedCornerShape(14.dp), 262, 196, 84, 44, color = lerp(ink.bg, ink.shape, 0.35f))
        Shaped(RoundedCornerShape(14.dp), 290, 152, 56, 44, color = lerp(ink.bg, ink.shape, 0.6f))
        Shaped(RoundedCornerShape(14.dp), 318, 108, 28, 44)
        Txt(58) { Label("On the move") }
        Txt(86) { Big("${m.floors}", if (m.floors >= 1000) 96f else 120f) }
        Txt(212) {
            H(if (m.floors == 1) "floor climbed" else "floors climbed")
            val burj = m.floors / 163f
            P("Between classes" + if (burj >= 0.1f) ", about ${String.format(Locale.US, "%.1f", burj)} Burj Khalifas" else "")
        }
        Minis(
            316,
            listOfNotNull(
                count(m.rooms, "room") to "you punched in",
                m.mostHops.takeIf { it > 0 }?.let { count(it, "hop") to "between blocks on ${day(m.mostHopsDay)}" },
            ),
        )
        Txt(414) {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                m.blocks.forEachIndexed { i, b -> Chip("${b.block} · " + if (i == 0) count(b.rooms, "room") else "${b.rooms}") }
            }
        }
        BiometricNote(478, hostel = false, r.campus)
    }
}

/** The biometric slides' note, on screen and on the share image. */
@Composable
private fun BiometricNote(y: Int, hostel: Boolean, campus: WrappedCampus?) {
    val color = LocalInks.current.text.copy(alpha = 0.78f)
    Txt(y) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(Lucide.Info, null, Modifier.padding(top = 1.dp).size(16.dp), tint = color)
            Text(
                "Worked out from your ${if (hostel) "hostel and " else ""}class punches" +
                    (campus?.let { " on ${it.daysCached} of ${it.daysTotal} days" } ?: "") + ", so it can be off.",
                color = color,
                style = type(12f, 500, 0f, 1.35f),
            )
        }
    }
}

/** The gauge of days on campus ([ratio] filled; null: a dotted track only). */
@Composable
private fun Arc(y: Int, ratio: Float?) {
    val ink = LocalInks.current
    At(24, y, 312, 176) {
        Canvas(Modifier.size(312.dp, 176.dp)) {
            scale(size.width / 312f, Offset.Zero) {
                val track = Stroke(20f, cap = StrokeCap.Round, pathEffect = if (ratio == null) PathEffect.dashPathEffect(floatArrayOf(2f, 34f)) else null)
                drawArc(ink.shape.copy(alpha = 0.25f), 180f, 180f, false, Offset(16f, 20f), Size(280f, 280f), style = track)
                if (ratio != null && ratio > 0f) drawArc(ink.shape, 180f, 180f * ratio.coerceIn(0f, 1f), false, Offset(16f, 20f), Size(280f, 280f), style = Stroke(20f, cap = StrokeCap.Round))
            }
        }
    }
}

@Composable
private fun Clock(time: String, label: String, align: Alignment.Horizontal) {
    val ink = LocalInks.current
    Column(horizontalAlignment = align) {
        Text(time, color = ink.text, style = type(30f, 800, -0.04f, 1.1f))
        Text(label, color = ink.text.copy(alpha = 0.8f), style = type(13f, 600, -0.01f, 1.3f))
    }
}

/** "<k> days missing · Fetch", or the worker's progress while it runs; [offer] is the nothing-cached state's lone button. */
@Composable
private fun FetchRow(y: Int, fetch: CampusFetch, days: Int, offer: Boolean) {
    val ink = LocalInks.current
    if (fetch.running) {
        Panel(y) {
            Column(Modifier.padding(horizontal = 20.dp, vertical = 16.dp).testTag("wrapped.fetch.progress")) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("Fetching campus times", color = ink.text, style = type(15f, 700, -0.01f, 1.3f))
                    if (fetch.total > 0) Text("${fetch.done} of ${fetch.total} days", color = ink.text, style = type(15f, 700, -0.01f, 1.3f))
                }
                val part = if (fetch.total > 0) fetch.done.toFloat() / fetch.total else 0f
                Box(Modifier.padding(top = 12.dp).fillMaxWidth().height(10.dp).background(ink.text.copy(alpha = 0.2f), RoundedCornerShape(5.dp))) {
                    Box(Modifier.fillMaxWidth(part.coerceIn(0.03f, 1f)).height(10.dp).background(ink.shape, RoundedCornerShape(5.dp)))
                }
                Text("Keeps going if you leave. Stop it from the notification.", Modifier.padding(top = 10.dp), color = ink.text.copy(alpha = 0.75f), style = type(12f, 500, 0f, 1.3f))
            }
        }
        return
    }
    val button = @Composable { modifier: Modifier ->
        Button(
            fetch.start,
            modifier = modifier.height(if (offer) 56.dp else 44.dp).testTag("wrapped.fetch").semantics { contentDescription = "Fetch ${count(days, "day")} of campus times from VTOP" },
            colors = ButtonDefaults.buttonColors(containerColor = ink.shape, contentColor = ink.onShape),
            shapes = ButtonDefaults.shapes(),
        ) { Text("Fetch", style = type(if (offer) 16f else 15f, 700, -0.01f, 1f)) }
    }
    if (offer) {
        Box(Modifier.offset(24.dp, (y + 140).dp)) { button(Modifier) }
        return
    }
    Panel(y) {
        Row(Modifier.padding(start = 20.dp, top = 14.dp, end = 14.dp, bottom = 14.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("${count(days, "day")} missing", color = ink.text, style = type(15f, 700, -0.01f, 1.3f))
                Text("Fetched from VTOP, a few days at a time", Modifier.padding(top = 2.dp), color = ink.text.copy(alpha = 0.75f), style = type(12f, 500, 0f, 1.3f))
            }
            button(Modifier)
        }
    }
}
