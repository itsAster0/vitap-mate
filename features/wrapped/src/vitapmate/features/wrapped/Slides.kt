package vitapmate.features.wrapped

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
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
 * The story's slides, in design dp. On screen the frame is the screen's width over a scale of
 * width / 360 kept within 0.94 to 1.2, so a narrow phone gets a narrower frame (340 at 320 dp)
 * rather than smaller text, and a tall phone a taller one; the Share row sits below it. A share
 * image is 1080 × 1920: a 360 × 564 frame over the vitapmate footer. Fixed slides are laid out at
 * the mockups' positions in 360 × 564 and scaled down when the frame is smaller; flexible ones
 * fill the frame (see [Frame]).
 */

/** The frame a slide is laid out in, in design dp: [width] across, [height] above the Share row or the share image's footer. */
@Immutable
internal class Frame(val width: Float, val height: Float) {
    /** A short frame: headings a step smaller, gaps tighter. */
    val compact: Boolean get() = height < COMPACT_BELOW
}

internal val LocalFrame = staticCompositionLocalOf { Frame(DESIGN_WIDTH, CONTENT_HEIGHT) }

/** The share image's frame height, and the footer band under it (640 in all). */
internal const val CONTENT_HEIGHT = 564f
internal const val FOOTER_HEIGHT = 76f
private const val COMPACT_BELOW = 600f

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

/** A summary tile: a slide's headline number. [grades] tiles go with Hide grades, [hostel] ones with Hide hostel and outings. */
internal class Tile(
    val ink: Ink,
    val value: String,
    val label: String,
    val span: Int = 2,
    val key: Boolean = false,
    val small: Boolean = false,
    val grades: Boolean = false,
    val hostel: Boolean = false,
)

/**
 * The outro's campus times: [missing] class days without them, the worker's [done] of [total]
 * while it [running], and [start] to queue it.
 */
internal class CampusFetch(val missing: List<Int>, val running: Boolean, val done: Int, val total: Int, val start: () -> Unit)

/** What a share image leaves out; on screen nothing is. */
@Immutable
internal data class Privacy(val grades: Boolean = false, val name: Boolean = false, val hostel: Boolean = false)

/**
 * The story's moments that wait for the student: the attendance guess ([guess], an index into
 * its ranges, null when skipped; [guessed] once revealed), the sealed title, and the summary as
 * a grid or a receipt. A share image draws every moment revealed.
 */
@Stable
internal class StoryState {
    var guess by mutableStateOf<Int?>(null)
    var guessed by mutableStateOf(false)
    var unsealed by mutableStateOf(false)
    var receipt by mutableStateOf(false)

    /** Back to the start of the story: every moment waiting again. */
    fun reset() {
        guess = null
        guessed = false
        unsealed = false
        receipt = false
    }
}

/** What the outro does: the share sheet, Save to Docs, a replay, and the other semesters to switch to (id to name). */
internal class Outro(
    val share: () -> Unit,
    val save: () -> Unit,
    val replay: () -> Unit,
    val semesters: List<Pair<String, String>>,
    val pick: (String) -> Unit,
)

/**
 * What a slide draws from. [share] is the share image, drawn as [privacy] says ([wallpaper]:
 * without the footer); [state] the story's moments.
 */
internal class SlideContext(
    val report: WrappedReport,
    val name: String?,
    val share: Boolean,
    val privacy: Privacy,
    val campus: CampusFetch,
    val state: StoryState,
    val outro: Outro? = null,
    val wallpaper: Boolean = false,
) {
    /** Hide grades. */
    val hide: Boolean get() = privacy.grades

    /** The first name, unless the share image hides it. */
    val shownName: String? get() = name.takeUnless { share && privacy.name }

    /** Revealed: in a share image, or once the student guessed or skipped. */
    val guessed: Boolean get() = share || state.guessed
    val unsealed: Boolean get() = share || state.unsealed
}

/** A moment that waits for the student before the slide shows its fact: [skip] reveals it, [label] names that. */
internal class Pending(val label: String, val waiting: (StoryState) -> Boolean, val skip: (StoryState) -> Unit)

/**
 * One screen of the story: [name] for its test tag and image file, [ink] its tone (null: the
 * summary's plain surface), [grades] shows grades, [hostel] shows hostel times or outings,
 * [shareable] whether its image has a fact left, [tiles] its summary tiles, [pending] a moment
 * before its fact (no Share until revealed). [fixed] slides place their parts at the mockups'
 * positions in 360 × 564; the others fill the [Frame] with a flexible layout. [outro] is the
 * last screen: never shared, no Share row.
 */
internal class Slide(
    val name: String,
    val ink: Ink?,
    val grades: Boolean = false,
    val semesterLine: Boolean = true,
    val tiles: List<Tile> = emptyList(),
    val shareable: (Privacy) -> Boolean = { true },
    val fixed: Boolean = true,
    val hostel: Boolean = false,
    val pending: Pending? = null,
    val outro: Boolean = false,
    /** The tone at the frame's bottom edge when the lower part is another; the Share row takes it. */
    val foot: Ink? = null,
    val content: @Composable BoxScope.(SlideContext) -> Unit,
) {
    fun withInk(ink: Ink) = Slide(name, ink, grades, semesterLine, tiles, shareable, fixed, hostel, pending, outro, foot, content)

    /** Whether a share image with [privacy] has anything left to show. */
    fun canShare(privacy: Privacy) = !outro && shareable(privacy) && !(hostel && privacy.hostel)
}

/** Every slide with facts, in story order, then the summary and the outro; no two in a row share a tone. */
internal fun slides(r: WrappedReport): List<Slide> {
    val story = listOfNotNull(
        openingSlide(r), attendanceSlide(r), streakSlide(r), mosaicSlide(r), arcSlide(r), coursesSlide(r), closeCallSlide(r),
        rhythmSlide(r), timeSlide(r), trapsSlide(r), ffcsSlide(r), examsSlide(r), marksSlide(r), roiSlide(r), gradesSlide(r),
        degreeSlide(r), previousSlide(r), outingsSlide(r), campusSlide(r), hostelSlide(r), movesSlide(r), mixtapeSlide(r),
        titleSlide(r),
    ).let(::distinctInks)
    return story + summarySlide(story) + outroSlide(r)
}

/** Tones a slide takes when its own matches the one before it, in this order. */
private val SPARE_INKS = listOf(Ink.Accent, Ink.Success, Ink.Warning, Ink.Lab, Ink.Danger)

/** [story] with each slide whose tone repeats the one before it moved to a spare tone, unlike either neighbour. */
private fun distinctInks(story: List<Slide>): List<Slide> = story.foldIndexed(listOf()) { i, done, slide ->
    val before = done.lastOrNull()?.ink
    if (slide.ink == null || slide.ink != before) done + slide
    else done + slide.withInk(SPARE_INKS.first { it != before && it != story.getOrNull(i + 1)?.ink })
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

/** A pill of quiet text; a long one (a course's name) wraps to a second line. */
@Composable
internal fun Chip(text: String, modifier: Modifier = Modifier) {
    val ink = LocalInks.current
    Box(modifier.heightIn(min = 32.dp).background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(16.dp)).padding(horizontal = 14.dp, vertical = 6.dp), Alignment.CenterStart) {
        Text(text, color = ink.text, style = type(13f, 600, -0.01f, 1.2f), maxLines = 2, overflow = TextOverflow.Ellipsis)
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

/** A flexible slide's column: the frame under the semester line, 24 in from each side. */
@Composable
internal fun Flow(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    val compact = LocalFrame.current.compact
    Column(Modifier.fillMaxSize().padding(start = 24.dp, end = 24.dp, top = if (compact) 52.dp else 58.dp, bottom = 12.dp).then(modifier), content = content)
}

/** [compact] in a short frame, else [roomy]. */
@Composable
@ReadOnlyComposable
internal fun fits(roomy: Float, compact: Float): Float = if (LocalFrame.current.compact) compact else roomy

/** Panels of a number over a line, [columns] to a row, in a flexible column. */
@Composable
internal fun MiniRow(items: List<Pair<String, String>>, modifier: Modifier = Modifier, columns: Int = items.size.coerceIn(1, 3)) {
    if (items.isEmpty()) return
    val ink = LocalInks.current
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items.chunked(columns).forEach { row ->
            Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (value, label) ->
                    Column(Modifier.weight(1f).fillMaxHeight().background(ink.panel, RoundedCornerShape(22.dp)).padding(horizontal = 14.dp, vertical = 11.dp)) {
                        Text(value, color = ink.shape, style = type(22f, 800, -0.04f, 1.15f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                        Text(label, color = ink.text, style = type(12f, 600, -0.01f, 1.3f), maxLines = 3, overflow = TextOverflow.Ellipsis)
                    }
                }
                repeat(columns - row.size) { Box(Modifier.weight(1f)) }
            }
        }
    }
}

/** One bar split into parts by weight (theory and labs, the grade mix), [h] tall. */
@Composable
internal fun SplitBar(y: Int, h: Int, parts: List<Triple<String, Float, Pair<Color, Color>>>) {
    Txt(y) { Split(h, parts) }
}

/** [SplitBar]'s bar, across its column. */
@Composable
internal fun Split(h: Int, parts: List<Triple<String, Float, Pair<Color, Color>>>) {
    val r = h / 2
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
    WrappedTitle.TOPPER -> "The Topper"
    WrappedTitle.GHOST_GENIUS -> "The Ghost Genius"
    WrappedTitle.EXAM_CLUTCH -> "The Exam Clutch"
    WrappedTitle.PERFECTIONIST -> "The Perfectionist"
    WrappedTitle.COMEBACK -> "The Comeback"
    WrappedTitle.EDGE_WALKER -> "The 75% Tactician"
    WrappedTitle.IRON_STREAK -> "The Iron Streak"
    WrappedTitle.EARLY_REGULAR -> "The 8 AM Regular"
    WrappedTitle.REGULAR -> "The Regular"
    WrappedTitle.CLUB_CAPTAIN -> "The Club Captain"
    WrappedTitle.LAB_RAT -> "The Lab Rat"
    WrappedTitle.EXPLORER -> "The Explorer"
    WrappedTitle.STAIRMASTER -> "The Stairmaster"
    WrappedTitle.NIGHT_OWL -> "The Night Owl"
    WrappedTitle.STEADY_CRUISER -> "The Steady Cruiser"
}

/** The traits [privacy] leaves, strongest first; the title is the first. */
internal fun WrappedReport.shownTraits(privacy: Privacy) = traits.filter { !(privacy.grades && it.grades) && !(privacy.hostel && it.hostel) }

/** The title [privacy] leaves. */
internal fun WrappedReport.shownTitle(privacy: Privacy): WrappedTitle = shownTraits(privacy).firstOrNull()?.title ?: WrappedTitle.STEADY_CRUISER
