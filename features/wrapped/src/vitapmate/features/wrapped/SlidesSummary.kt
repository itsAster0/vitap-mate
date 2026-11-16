package vitapmate.features.wrapped

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.layout.layout
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedTitle

// Slides 18 and 19: the title and the summary card.

/** Why [t] fits, from the numbers; null when there's nothing to add. */
internal fun reason(t: WrappedTitle, r: WrappedReport): String? = when (t) {
    WrappedTitle.COMEBACK -> r.comeback?.let { "From ${it.lowPercent}% in ${month(it.lowDay)} to ${it.finalPercent}% at the end" }
    WrappedTitle.IRON_STREAK -> r.streak?.let { s -> "${s.length} classes in a row without a miss" + (s.total.takeIf { it >= s.length }?.let { ", ${(s.length * 100f / it).roundToInt()}% of all $it" } ?: "") }
    WrappedTitle.EARLY_REGULAR -> r.habits?.earlyPercent?.let { "$it% of your 8 AM classes, attended" }
    WrappedTitle.TOPPER -> r.grades?.let { "A ${gpa(it.gpa)} GPA this semester" }
    WrappedTitle.EXPLORER -> r.outings?.let { "${it.leaves + it.weekends} trips off campus" }
    WrappedTitle.LAB_RAT -> r.time?.let { "${(it.labMinutes + 30) / 60} of your ${(it.minutes + 30) / 60} class hours in labs" }
    WrappedTitle.REGULAR -> r.attendance?.let { "${it.percent}% attendance, all semester" }
    WrappedTitle.FREE_SPIRIT -> "You did this semester your own way"
}

/** The summary's short line for [t]. */
private fun short(t: WrappedTitle, r: WrappedReport): String = when (t) {
    WrappedTitle.COMEBACK -> r.comeback?.let { "${it.lowPercent}% to ${it.finalPercent}%" }
    WrappedTitle.IRON_STREAK -> r.streak?.let { "${it.length} in a row" }
    WrappedTitle.EARLY_REGULAR -> r.habits?.earlyPercent?.let { "8 AM: $it%" }
    WrappedTitle.TOPPER -> r.grades?.let { "GPA ${gpa(it.gpa)}" }
    WrappedTitle.EXPLORER -> r.outings?.let { count(it.leaves + it.weekends, "trip") }
    WrappedTitle.LAB_RAT -> r.time?.let { "${(it.labMinutes + 30) / 60} h of labs" }
    WrappedTitle.REGULAR -> r.attendance?.let { "${it.percent}% attendance" }
    WrappedTitle.FREE_SPIRIT -> null
}.orEmpty()

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun titleSlide(r: WrappedReport) = Slide("title", Ink.Accent, grades = r.title != r.titleWithoutGrades) { c ->
    val t = if (c.hide) r.titleWithoutGrades else r.title
    Shaped(MaterialShapes.Cookie9Sided.toShape(), 150, -60, 300)
    Shaped(MaterialShapes.Clover8Leaf.toShape(), 104, 96, 120, color = Ink.Success.tone().base)
    Shaped(RoundedCornerShape(27.dp), 206, 214, 140, 54, color = Ink.Warning.tone().base, rotate = -18f)
    Txt(292) {
        P("This semester, you were", 18f, top = 0)
        H(t.label().replaceFirst(" ", "\n"), 66f, Modifier.padding(top = 10.dp))
        reason(t, r)?.let { P(it, top = 16) }
    }
}

/** One tile per slide above it, under the title; Hide grades leaves out the grade tiles. */
internal fun summarySlide(story: List<Slide>) = Slide("summary", null, grades = story.any { s -> s.tiles.any { it.grades } } || story.any { it.name == "title" && it.grades }, semesterLine = false) { c ->
    val r = c.report
    val ink = LocalInks.current
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val t = if (c.hide) r.titleWithoutGrades else r.title
    val tiles = story.flatMap { it.tiles }.filter { !(c.hide && it.grades) }
    // One column, so a title that takes two lines pushes the tiles down; scaled down, centred,
    // when taller than the space above the footer (or the Share button on screen).
    Column(Modifier.offset(14.dp, 28.dp).width(332.dp).fitHeight(SUMMARY_HEIGHT.dp), verticalArrangement = Arrangement.spacedBy(5.dp)) {
      Column(Modifier.padding(start = 10.dp, end = 10.dp, bottom = 5.dp)) {
        Text("${c.name?.let { "$it's" } ?: "Your"} ${r.semesterName}", color = ink.text, style = type(23f, 800, -0.04f, 1.1f), maxLines = 2, overflow = TextOverflow.Ellipsis)
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
            Text(short(t, r), color = accent.onShape, style = type(10f, 500, 0f, 1.25f), textAlign = TextAlign.End)
        }
        rows(tiles).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                row.forEach { tile -> TileView(tile, Modifier.weight(tile.span.toFloat())) }
            }
        }
    }
}

/** The summary block's room: the frame less its top offset (28) and the footer or Share row (76). */
private const val SUMMARY_HEIGHT = 640 - 28 - 76

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
        Text(tile.value, color = value, style = type(if (tile.small) 15f else 22f, 800, -0.04f, 1.1f), maxLines = if (tile.small) 3 else 2, overflow = TextOverflow.Ellipsis)
        Text(tile.label, Modifier.padding(top = 2.dp), color = text, style = type(10f, 500, 0f, 1.25f), maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}
