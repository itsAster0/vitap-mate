package vitapmate.features.wrapped

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.basicMarquee
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
import java.time.LocalDate
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.core.WrappedGuess
import vitapmate.core.WrappedReport
import vitapmate.ui.rememberHaptics

// The opening and the attendance guess.

/** From this frame height the opening's heading is a step larger. */
private const val TALL_FRAME = 640f

/** "F26": the season's first letter and the year classes started. */
private fun badge(r: WrappedReport) = shortName(r.semesterName).take(1).uppercase() + LocalDate.ofEpochDay(r.firstDay.toLong()).year % 100

/**
 * The end of the semester: shapes spring in around its badge and the course codes run past.
 * Flexible, the hero taking the spare height; still in a share image.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun openingSlide(r: WrappedReport) = Slide("opening", Ink.Accent, fixed = false) { c ->
    val ink = LocalInks.current
    Flow {
        val facts = listOfNotNull(
            r.spanDays.takeIf { it > 0 }?.let { count(it, "day") },
            r.courses.takeIf { it > 0 }?.let { count(it, "course") },
        ).joinToString(" · ")
        if (facts.isNotEmpty()) Row(
            Modifier.background(ink.text.copy(alpha = 0.14f), RoundedCornerShape(16.dp)).padding(horizontal = 14.dp, vertical = 7.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Box(Modifier.size(8.dp).background(ink.shape, CircleShape))
            Text(facts, color = ink.text, style = type(13f, 600, -0.01f, 1.2f), maxLines = 2)
        }
        Hero(badge(r), animate = !c.share, Modifier.fillMaxWidth().weight(1f).padding(top = 10.dp, bottom = 6.dp).heightIn(min = 96.dp))
        if (r.courseCodes.isNotEmpty()) Ticker(r.courseCodes, animate = !c.share)
        // A tall frame's spare height goes to the hero (its weight) and a larger heading.
        val tall = LocalFrame.current.height >= TALL_FRAME
        H("${shortName(r.semesterName)},\nwrapped", if (tall) 58f else fits(48f, 40f), Modifier.padding(top = if (tall) 18.dp else 14.dp))
        P(c.shownName?.let { "$it, let's check the receipts." } ?: "Let's check the receipts.", 15f, top = 8)
    }
}

/** The opening's shapes in a 312 × 224 box, fitted to the space; each springs in, one after another, on screen. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(badge: String, animate: Boolean, modifier: Modifier) {
    val ink = LocalInks.current
    val parts = remember { List(4) { Animatable(if (animate) 0f else 1f) } }
    LaunchedEffect(animate) {
        if (!animate) return@LaunchedEffect
        parts.forEachIndexed { i, part ->
            launch {
                delay(60L + 90L * i)
                part.animateTo(1f, spring(dampingRatio = 0.55f, stiffness = 260f))
            }
        }
    }
    val drift = if (animate) {
        val wave = rememberInfiniteTransition(label = "drift")
        wave.animateFloat(-3f, 3f, infiniteRepeatable(tween(4200), RepeatMode.Reverse), label = "turn").value
    } else 0f
    BoxWithConstraints(modifier, Alignment.Center) {
        val k = minOf(maxWidth / 312.dp, maxHeight / 224.dp)
        Box(Modifier.size(312.dp * k, 224.dp * k)) {
            @Composable
            fun Part(at: Int, x: Int, y: Int, w: Int, h: Int, shape: Shape, color: Color, turn: Float, content: @Composable () -> Unit = {}) {
                Box(
                    Modifier.offset(x.dp * k, y.dp * k).size(w.dp * k, h.dp * k).scale(parts[at].value).rotate(turn).background(color, shape),
                    Alignment.Center,
                ) { content() }
            }
            Part(1, 8, 176, 40, 40, MaterialShapes.Cookie6Sided.toShape(), ink.mid, drift)
            Part(2, 212, 4, 92, 92, MaterialShapes.Clover8Leaf.toShape(), ink.text, 12f - drift)
            Part(3, 222, 150, 70, 70, MaterialShapes.SoftBurst.toShape(), ink.mid, drift * 2)
            Part(0, 34, 20, 176, 176, MaterialShapes.Cookie9Sided.toShape(), ink.shape, -8f + drift) {
                Text(badge, color = ink.onShape, style = type(54f * k, 800, -0.05f, 1f), maxLines = 1)
            }
        }
    }
}

/** The course codes as pills running past, faded out at the right; still in a share image. */
@Composable
private fun Ticker(codes: List<String>, animate: Boolean) {
    val ink = LocalInks.current
    Box(
        Modifier.fillMaxWidth().clipToBounds()
            .graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }
            .drawWithContent {
                drawContent()
                drawRect(Brush.horizontalGradient(0.72f to Color.Black, 1f to Color.Transparent), blendMode = BlendMode.DstIn)
            },
    ) {
        Row(
            (if (animate) Modifier.basicMarquee(iterations = Int.MAX_VALUE, initialDelayMillis = 600, velocity = 36.dp) else Modifier),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            codes.forEach { code ->
                Box(Modifier.height(30.dp).background(ink.panel, RoundedCornerShape(15.dp)).padding(horizontal = 12.dp), Alignment.Center) {
                    Text(code, color = ink.text, style = type(12.5f, 700, -0.01f, 1f), maxLines = 1)
                }
            }
            // A gap before the codes come round again.
            Spacer(Modifier.width(24.dp))
        }
    }
}

/** "75–79%", or "90%+" for the open top range. */
private fun WrappedGuess.label() = to?.let { "$from–$it%" } ?: "$from%+"

/**
 * Attendance, after a guess: four ranges around the real percent; picking one (or Skip) reveals
 * it, with how far off the guess was. Classes on OD and where most went. Flexible; a share image
 * draws it revealed.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun attendanceSlide(r: WrappedReport): Slide? {
    val a = r.attendance?.takeIf { it.counted > 0 } ?: return null
    val od = a.onDuty.takeIf { it > 0 }
    val o = r.onDuty
    val pending = Pending("Skip", { !it.guessed }) {
        it.guess = null
        it.guessed = true
    }
    return Slide("attendance", Ink.Success, tiles = listOf(Tile(Ink.Success, "${a.percent}%", "attendance" + (od?.let { ", $it on OD" } ?: ""))), fixed = false, pending = pending) { c ->
        val ink = LocalInks.current
        val haptics = rememberHaptics()
        val revealed = c.guessed
        val picked = c.state.guess?.let { a.guesses.getOrNull(it) }
        Flow {
            when {
                !revealed -> {
                    Label("Before the big number")
                    H("Guess your\nattendance", fits(38f, 32f), Modifier.padding(top = 6.dp))
                    P(if (r.courses > 1) "All ${r.courses} courses together, OD counts as present." else "OD counts as present.", 14f, top = 8)
                }
                picked == null -> {
                    Label("Your attendance")
                    H("You showed up.", fits(38f, 32f), Modifier.padding(top = 6.dp))
                    P("${a.attended} of ${a.counted} classes attended" + if (r.courses > 1) ", across all ${r.courses} courses" else "", 14f, top = 8)
                }
                else -> {
                    Label("You guessed ${picked.label()}")
                    H(
                        when {
                            picked.off == 0 -> "Nailed it"
                            picked.off > 0 -> "You undersold\nyourself"
                            else -> "A little\noptimistic"
                        },
                        fits(38f, 32f), Modifier.padding(top = 6.dp),
                    )
                    P(
                        when {
                            picked.off == 0 -> "Spot on. You know your attendance."
                            picked.off > 0 -> "Off by ${picked.off}. Modest, and a little wrong."
                            else -> "Off by ${-picked.off}. Optimism is free."
                        },
                        14f, top = 8,
                    )
                }
            }
            AnimatedContent(
                revealed,
                Modifier.fillMaxWidth().weight(1f).padding(vertical = 10.dp).heightIn(min = 110.dp),
                transitionSpec = { (fadeIn() + scaleIn(initialScale = 0.6f)) togetherWith fadeOut() },
                label = "reveal",
            ) { open -> GuessHero(open, a.percent, animate = !c.share) }
            if (!revealed || picked != null) Pills(a.guesses, c.state.guess.takeIf { revealed }, revealed) { i ->
                c.state.guess = i
                c.state.guessed = true
                if (a.guesses[i].off == 0) haptics.confirm() else haptics.tick()
            }
            val line = when {
                !revealed -> "Tap one. VTOP already knows."
                picked != null -> "${a.attended} of ${a.counted} classes" + (od?.let { ", $it of them on OD" } ?: "")
                else -> null
            }
            if (line != null) Text(line, Modifier.fillMaxWidth().padding(top = 12.dp), color = ink.text.copy(alpha = 0.85f), style = type(13f, 600, -0.01f, 1.3f), textAlign = TextAlign.Center)
            if (revealed && od != null) {
                val top = o?.top?.firstOrNull()
                Chip(
                    if (o != null) listOfNotNull("${o.classes} on OD", count(o.days, "day"), top?.let { "most in ${titled(it.name)}" }).joinToString(" · ") else "$od of them on OD",
                    Modifier.padding(top = 10.dp).align(Alignment.CenterHorizontally),
                )
            }
        }
    }
}

/** A question mark on a shape before the reveal; after it, the percent rolling up on another, with confetti shapes. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun GuessHero(open: Boolean, percent: Int, animate: Boolean) {
    val ink = LocalInks.current
    BoxWithConstraints(Modifier.fillMaxSize(), Alignment.Center) {
        val s = minOf(maxWidth * 0.6f, maxHeight)
        if (!open) {
            Decor(MaterialShapes.SoftBurst.toShape(), -s * 0.62f, -s * 0.12f, s * 0.3f, ink.mid, 12f)
            Box(Modifier.size(s).background(ink.shape, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Text("?", color = ink.onShape, style = type(s.value * 0.58f, 800, -0.05f, 1f))
            }
            Decor(RoundedCornerShape(50), s * 0.55f, s * 0.36f, s * 0.42f, ink.mid, -14f, h = s * 0.19f)
            return@BoxWithConstraints
        }
        val shown = remember { Animatable(if (animate) (percent - 12).coerceAtLeast(0).toFloat() else percent.toFloat()) }
        LaunchedEffect(Unit) { if (animate) shown.animateTo(percent.toFloat(), tween(650)) }
        listOf(
            Triple(MaterialShapes.Cookie6Sided, -0.7f to -0.45f, 0.14f),
            Triple(MaterialShapes.SoftBurst, 0.66f to -0.5f, 0.16f),
            Triple(MaterialShapes.Clover4Leaf, 0.72f to 0.3f, 0.12f),
            Triple(MaterialShapes.Cookie9Sided, -0.74f to 0.36f, 0.1f),
        ).forEach { (shape, at, size) -> Decor(shape.toShape(), s * at.first, s * at.second, s * size, ink.text, 0f) }
        Box(Modifier.size(s).rotate(-6f).background(ink.shape, MaterialShapes.Clover8Leaf.toShape()), Alignment.Center) {
            Text("${shown.value.toInt()}%", color = ink.onShape, style = type(s.value * 0.29f, 800, -0.05f, 1f), maxLines = 1)
        }
    }
}

/** A shape at ([x], [y]) from the centre. */
@Composable
private fun Decor(shape: Shape, x: Dp, y: Dp, w: Dp, color: Color, turn: Float, h: Dp = w) {
    Box(Modifier.offset(x, y).size(w, h).rotate(turn).background(color, shape))
}

/** The four ranges, two to a row: tappable before the reveal; after it the right one filled, the guess dashed, the rest faint. */
@Composable
private fun Pills(guesses: List<WrappedGuess>, picked: Int?, revealed: Boolean, pick: (Int) -> Unit) {
    val ink = LocalInks.current
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        guesses.chunked(2).forEachIndexed { row, pair ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                pair.forEachIndexed { col, g ->
                    val i = row * 2 + col
                    val hit = revealed && g.off == 0
                    val miss = revealed && i == picked && !hit
                    val shape = RoundedCornerShape(if (hit) 18.dp else 29.dp)
                    var m = Modifier.weight(1f).height(fits(56f, 48f).dp)
                    m = when {
                        hit -> m.background(ink.shape, shape)
                        miss -> m.border(2.dp, ink.text.copy(alpha = 0.55f), shape)
                        else -> m.background(ink.panel, shape)
                    }
                    if (!revealed) m = m.clickable { pick(i) }
                    Box(
                        m.testTag("wrapped.guess.$i").semantics {
                            role = Role.Button
                            contentDescription = "Guess ${g.to?.let { "${g.from} to $it percent" } ?: "${g.from} percent or more"}"
                        }.graphicsLayer { alpha = if (revealed && !hit && !miss) 0.32f else 1f },
                        Alignment.Center,
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                if (hit) Icon(Lucide.Check, null, Modifier.size(18.dp), tint = ink.onShape)
                                if (miss) Icon(Lucide.X, null, Modifier.size(16.dp), tint = ink.text)
                                Text(g.label(), color = if (hit) ink.onShape else ink.text, style = type(fits(19f, 17f), 800, -0.03f, 1f), maxLines = 1)
                            }
                            if (miss) Text("your guess", color = ink.text.copy(alpha = 0.8f), style = type(11f, 700, 0f, 1.1f), maxLines = 1)
                        }
                    }
                }
            }
        }
    }
}
