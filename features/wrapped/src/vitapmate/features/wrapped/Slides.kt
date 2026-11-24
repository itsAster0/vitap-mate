package vitapmate.features.wrapped

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.TextAutoSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.layout.LastBaseline
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale
import vitapmate.core.WrappedReport
import vitapmate.core.WrappedTitle
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/*
 * The story's slides. Each is drawn in a 360 × 640 design frame (the mockups' phone), scaled to
 * the screen in the app and to 1080 × 1920 for a share image, so both look the same. Positions
 * below are the mockups' (docs/superpowers/specs/wrapped-mockups/).
 */

/** The slide's tone, from the theme. */
internal enum class Ink { Accent, Success, Warning, Danger, Lab }

@Composable
@ReadOnlyComposable
internal fun Ink.tone(): Tone = when (this) {
    Ink.Accent -> AppTheme.tones.accentTone
    Ink.Success -> AppTheme.tones.success
    Ink.Warning -> AppTheme.tones.warning
    Ink.Danger -> AppTheme.tones.danger
    Ink.Lab -> AppTheme.tones.lab
}

/** A summary tile: a slide's headline number. [grades] tiles go with Hide grades. */
internal class Tile(
    val ink: Ink,
    val value: String,
    val label: String,
    val span: Int = 2,
    val key: Boolean = false,
    val small: Boolean = false,
    val grades: Boolean = false,
)

/**
 * The Campus slide's fetch: [missing] class days without campus times, the worker's [done] of
 * [total] while it [running], and [start] to queue it.
 */
internal class CampusFetch(val missing: List<Int>, val running: Boolean, val done: Int, val total: Int, val start: () -> Unit)

/** What a slide draws from. [share] is the share image (no fetch row); [hide] is Hide grades, only ever on in it. */
internal class SlideContext(
    val report: WrappedReport,
    val name: String?,
    val share: Boolean,
    val hide: Boolean,
    val campus: CampusFetch,
)

/**
 * One screen of the story: [name] for its test tag and image file, [ink] its tone (null: the
 * summary's plain surface), [grades] shows Hide grades, [shareable] whether its image has a fact
 * left, [tiles] its summary tiles.
 */
internal class Slide(
    val name: String,
    val ink: Ink?,
    val grades: Boolean = false,
    val semesterLine: Boolean = true,
    val tiles: List<Tile> = emptyList(),
    val shareable: (hide: Boolean) -> Boolean = { true },
    val content: @Composable BoxScope.(SlideContext) -> Unit,
)

/** Every slide with facts, in the spec's order, ending with the summary. */
internal fun slides(r: WrappedReport, hasMissing: Boolean): List<Slide> {
    val story = listOfNotNull(
        openingSlide(r), attendanceSlide(r), odSlide(r), streakSlide(r), neverMissedSlide(r), comebackSlide(r), habitsSlide(r),
        dayHalfSlide(r), coursesSlide(r), closeCallSlide(r), timeSlide(r), placesSlide(r), marksSlide(r), fullMarksSlide(r),
        assessmentsSlide(r), examsSlide(r), crunchSlide(r), gradesSlide(r), cgpaSlide(r), outingsSlide(r), campusSlide(r, hasMissing), hostelSlide(r), movesSlide(r),
        previousSlide(r), titleSlide(r),
    )
    return story + summarySlide(story)
}

// Colours of the slide being drawn.

@Immutable
internal class Inks(val bg: Color, val shape: Color, val text: Color, val onShape: Color, val mid: Color, val panel: Color)

internal val LocalInks = staticCompositionLocalOf { Inks(Color.Black, Color.White, Color.White, Color.Black, Color.Gray, Color.DarkGray) }

/** [ink]'s colours: the subtle fill behind, the base for shapes, a surface colour on them. Null: the plain surface. */
@Composable
@ReadOnlyComposable
internal fun inks(ink: Ink?): Inks {
    val s = MaterialTheme.colorScheme
    if (ink == null) return Inks(s.surface, s.primary, s.onSurface, s.surface, s.surfaceContainerHighest, s.surfaceContainer)
    val t = ink.tone()
    return Inks(t.subtle, t.base, t.onSubtle, s.surface, lerp(t.subtle, t.base, 0.55f), s.surface.copy(alpha = 0.6f))
}

// Type: Inter, emphasized, tight like the mockups.

@Composable
@ReadOnlyComposable
internal fun type(size: Float, weight: Int = 800, track: Float = -0.035f, height: Float = 1.05f): TextStyle =
    MaterialTheme.typography.bodyLarge.copy(
        fontSize = size.sp,
        fontWeight = FontWeight(weight),
        letterSpacing = track.em,
        lineHeight = height.em,
        fontFeatureSettings = "tnum",
    )

// Building blocks, in design units (dp of the 360 × 640 frame).

/** A box at ([x], [y]) of the frame. */
@Composable
internal fun At(x: Int, y: Int, w: Int? = null, h: Int? = null, rotate: Float = 0f, content: @Composable BoxScope.() -> Unit) {
    var m = Modifier.offset(x.dp, y.dp)
    if (w != null) m = m.width(w.dp)
    if (h != null) m = m.height(h.dp)
    Box(m.rotate(rotate), content = content)
}

/** A text column across the frame (24 from each side) from [y]. */
@Composable
internal fun Txt(y: Int, modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.offset(24.dp, y.dp).width(312.dp).then(modifier), content = content)
}

/** The slide's heading. */
@Composable
internal fun H(text: String, size: Float = 34f, modifier: Modifier = Modifier) {
    Text(text, modifier, color = LocalInks.current.text, style = type(size))
}

/** A line under a heading. */
@Composable
internal fun P(text: String, size: Float = 16f, top: Int = 10) {
    Text(text, Modifier.padding(top = top.dp), color = LocalInks.current.text.copy(alpha = 0.82f), style = type(size, 500, -0.01f, 1.4f))
}

/** A small label over a fact. */
@Composable
internal fun Label(text: String, size: Float = 14f) {
    Text(text, color = LocalInks.current.text.copy(alpha = 0.8f), style = type(size, 600, -0.01f, 1.3f))
}

/** A large number in the base colour, shrunk to fit its width. */
@Composable
internal fun Big(text: String, size: Float, modifier: Modifier = Modifier) {
    Text(
        text, modifier, color = LocalInks.current.shape, style = type(size, 800, -0.06f, 1f), maxLines = 1,
        autoSize = TextAutoSize.StepBased(minFontSize = (size * 0.3f).sp, maxFontSize = size.sp),
    )
}

/** [number] with [heading] beside it on its last line; the number shrinks so the heading keeps its width. */
@Composable
internal fun BigBeside(number: String, size: Float, heading: String, headingSize: Float = 34f, gap: Int = 14) {
    Row(horizontalArrangement = Arrangement.spacedBy(gap.dp)) {
        Big(number, size, Modifier.weight(1f, fill = false).alignBy(LastBaseline))
        H(heading, headingSize, Modifier.alignBy(LastBaseline))
    }
}

/** A pill of quiet text. */
@Composable
internal fun Chip(text: String, modifier: Modifier = Modifier) {
    val ink = LocalInks.current
    Box(modifier.height(32.dp).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(16.dp)).padding(horizontal = 14.dp), Alignment.Center) {
        Text(text, color = ink.text, style = type(13f, 600, -0.01f, 1.2f), maxLines = 1)
    }
}

/**
 * A [shape] of [w] × [h] at ([x], [y]) filled with [color], holding [text] in [size]. [rotate]
 * turns both; [shapeRotate] the shape only.
 */
@Composable
internal fun Shaped(
    shape: Shape,
    x: Int,
    y: Int,
    w: Int,
    h: Int = w,
    color: Color = LocalInks.current.shape,
    text: String? = null,
    size: Float = 0f,
    textColor: Color = LocalInks.current.onShape,
    rotate: Float = 0f,
    shapeRotate: Float = 0f,
    track: Float = -0.05f,
) {
    At(x, y, w, h, rotate) {
        Box(Modifier.matchParentSize().rotate(shapeRotate).background(color, shape))
        if (text != null) {
            Text(
                text,
                Modifier.align(Alignment.Center).padding(horizontal = 12.dp),
                color = textColor,
                style = type(size, 800, track, 1f),
                textAlign = TextAlign.Center,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/** Panels of a number over a line, [columns] to a row, from [y]. */
@Composable
internal fun Minis(y: Int, items: List<Pair<String, String>>, columns: Int = 2) {
    if (items.isEmpty()) return
    val ink = LocalInks.current
    Txt(y) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items.chunked(columns).forEach { row ->
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    row.forEach { (value, label) ->
                        Column(Modifier.weight(1f).background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 12.dp)) {
                            Text(value, color = ink.shape, style = type(22f, 800, -0.04f, 1.15f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                            Text(label, color = ink.text, style = type(12f, 600, -0.01f, 1.3f))
                        }
                    }
                    repeat(columns - row.size) { Box(Modifier.weight(1f)) }
                }
            }
        }
    }
}

/** A dark or light panel across the frame at [y] (the 8 AM line, the campus fetch). */
@Composable
internal fun Panel(y: Int, modifier: Modifier = Modifier, content: @Composable BoxScope.() -> Unit) {
    Box(
        Modifier.offset(24.dp, y.dp).width(312.dp).background(LocalInks.current.panel, RoundedCornerShape(28.dp)).then(modifier),
        content = content,
    )
}

/** One bar split into parts by weight (theory and labs, the grade mix), [h] tall. */
@Composable
internal fun SplitBar(y: Int, h: Int, parts: List<Triple<String, Float, Pair<Color, Color>>>) {
    val r = h / 2
    Txt(y) {
        Row(Modifier.fillMaxWidth().height(h.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            parts.forEachIndexed { i, (text, weight, colors) ->
                val shape = RoundedCornerShape(
                    topStart = if (i == 0) r.dp else 8.dp, bottomStart = if (i == 0) r.dp else 8.dp,
                    topEnd = if (i == parts.lastIndex) r.dp else 8.dp, bottomEnd = if (i == parts.lastIndex) r.dp else 8.dp,
                )
                Box(Modifier.weight(weight).height(h.dp).background(colors.first, shape), Alignment.Center) {
                    Text(text, color = colors.second, style = type(if (h > 46) 16f else 14f, 800, -0.02f, 1f), maxLines = 1, overflow = TextOverflow.Clip)
                }
            }
        }
    }
}

// Words and numbers.

private val shortDate = DateTimeFormatter.ofPattern("d MMM", Locale.US)

/** "12 Jul" for a day since 1970-01-01. */
internal fun day(d: Int): String = LocalDate.ofEpochDay(d.toLong()).format(shortDate)

/** "September". */
internal fun month(d: Int): String = LocalDate.ofEpochDay(d.toLong()).month.getDisplayName(java.time.format.TextStyle.FULL, Locale.US)

internal val WEEKDAYS = listOf("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")

/** "1 class", "3 classes". */
internal fun count(n: Int, one: String, many: String = one + "s") = "$n ${if (n == 1) one else many}"

internal fun gpa(v: Double) = String.format(Locale.US, "%.2f", v)

/** 24.0 as "24", 24.5 as "24.5". */
internal fun credits(v: Double) = if (v % 1.0 == 0.0) v.toLong().toString() else String.format(Locale.US, "%.1f", v)

/** "GUNTUR" as "Guntur"; text with lower case letters stays. */
internal fun titled(text: String) =
    if (text.any { it.isLowerCase() }) text else text.lowercase().split(' ').joinToString(" ") { w -> w.replaceFirstChar { it.uppercase() } }

/** "Fall Semester 2026-27" as "Fall". */
internal fun shortName(semester: String) = semester.trim().substringBefore(' ')

internal fun WrappedTitle.label() = when (this) {
    WrappedTitle.COMEBACK -> "The Comeback"
    WrappedTitle.IRON_STREAK -> "The Iron Streak"
    WrappedTitle.EARLY_REGULAR -> "The 8 AM Regular"
    WrappedTitle.TOPPER -> "The Topper"
    WrappedTitle.EXPLORER -> "The Explorer"
    WrappedTitle.LAB_RAT -> "The Lab Rat"
    WrappedTitle.REGULAR -> "The Regular"
    WrappedTitle.FREE_SPIRIT -> "The Free Spirit"
}
