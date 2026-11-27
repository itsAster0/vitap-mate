package vitapmate.features.wrapped

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Play
import vitapmate.core.WrappedMixtape
import vitapmate.core.WrappedMixtapeTone
import vitapmate.core.WrappedReport

// The semester as a five-track tape.

private fun WrappedMixtapeTone.ink() = when (this) {
    WrappedMixtapeTone.WARNING -> Ink.Warning
    WrappedMixtapeTone.ACCENT -> Ink.Accent
    WrappedMixtapeTone.SUCCESS -> Ink.Success
    WrappedMixtapeTone.LAB -> Ink.Lab
    WrappedMixtapeTone.DANGER -> Ink.Danger
}

/** The album cover, its tracks and runtime. Flexible; with Hide grades the tape without the grade tracks. */
internal fun mixtapeSlide(r: WrappedReport): Slide? {
    val tape = r.mixtape?.takeIf { it.tracks.isNotEmpty() } ?: return null
    /** The tape [p] leaves: without the grade tracks, the hostel and outing ones, or both. */
    fun of(p: Privacy) = when {
        p.grades && p.hostel -> r.mixtapeWithoutEither
        p.grades -> r.mixtapeWithoutGrades
        p.hostel -> r.mixtapeWithoutHostel
        else -> tape
    }
    val grades = tape != r.mixtapeWithoutGrades
    val opener = tape.tracks.first()
    val tiles = listOf(
        Tile(
            Ink.Warning, opener.title, "your opening track", span = 3, small = true,
            grades = grades && r.mixtapeWithoutGrades?.tracks?.firstOrNull()?.title != opener.title,
            hostel = r.mixtapeWithoutHostel?.tracks?.firstOrNull()?.title != opener.title,
        ),
    )
    return Slide("mixtape", Ink.Warning, grades = grades, tiles = tiles, shareable = { of(it) != null }, fixed = false) { c ->
        val t = of(c.privacy) ?: tape
        val ink = LocalInks.current
        Flow {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                val cover = minOf(fits(168f, 128f), LocalFrame.current.width * 0.46f).dp
                Cover(t, cover, Modifier.padding(top = 4.dp))
                Column(Modifier.weight(1f)) {
                    Label("Your semester mixtape", 13f)
                    H(t.albumTitle, fits(24f, 21f), Modifier.padding(top = 4.dp))
                    P(t.genre, 13f, top = 4)
                    Row(Modifier.padding(top = 10.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Box(Modifier.size(44.dp).background(ink.shape, CircleShape), Alignment.Center) {
                            Icon(Lucide.Play, null, Modifier.size(18.dp), tint = ink.onShape)
                        }
                        Column {
                            Text(t.totalDuration, color = ink.text, style = type(15f, 800, -0.02f, 1.1f))
                            Text(if (t.noSkips) "No skips" else count(t.tracks.size, "track"), color = ink.text.copy(alpha = 0.8f), style = type(12f, 600, 0f, 1.2f))
                        }
                    }
                }
            }
            Column(Modifier.fillMaxWidth().weight(1f).padding(top = fits(12f, 8f).dp), verticalArrangement = Arrangement.SpaceEvenly) {
                t.tracks.forEachIndexed { i, track ->
                    val now = i == 0
                    Row(
                        Modifier.fillMaxWidth().then(if (now) Modifier.background(ink.panel, RoundedCornerShape(20.dp)) else Modifier)
                            .padding(start = 6.dp, end = 12.dp, top = fits(8f, 5f).dp, bottom = fits(8f, 5f).dp),
                        verticalAlignment = Alignment.Top,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Box(Modifier.width(26.dp).padding(top = 2.dp), Alignment.Center) {
                            if (now) Equalizer(animate = !c.share, color = ink.shape)
                            else Text("${track.number}", color = ink.text.copy(alpha = 0.8f), style = type(15f, 700, 0f, 1.1f))
                        }
                        Column(Modifier.weight(1f)) {
                            Text(track.title, color = if (now) ink.shape else ink.text, style = type(15.5f, 800, -0.025f, 1.12f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                            Text(track.statLine, Modifier.padding(top = 1.dp), color = ink.text.copy(alpha = 0.78f), style = type(12f, 600, 0f, 1.25f), maxLines = 2, overflow = TextOverflow.Ellipsis)
                        }
                        // On the title's first line.
                        Text(track.duration, Modifier.padding(top = 1.dp), color = ink.text.copy(alpha = 0.85f), style = type(13f, 700, 0f, 1.2f), maxLines = 1)
                    }
                }
            }
        }
    }
}

/** Three bars; moving on screen, still in a share image. */
@Composable
private fun Equalizer(animate: Boolean, color: Color) {
    val levels = if (animate) {
        val wave = rememberInfiniteTransition(label = "eq")
        listOf(0, 300, 600).map { delay ->
            wave.animateFloat(0.25f, 1f, infiniteRepeatable(tween(450, delay), RepeatMode.Reverse), label = "bar").value
        }
    } else listOf(0.5f, 1f, 0.7f)
    Row(Modifier.height(16.dp), horizontalArrangement = Arrangement.spacedBy(2.dp), verticalAlignment = Alignment.Bottom) {
        levels.forEach { Box(Modifier.width(4.dp).fillMaxHeight(it).background(color, RoundedCornerShape(2.dp))) }
    }
}

/**
 * The cover: the stamp and four theme shapes whose places and sizes come from the tape's seed, so
 * each student's cover differs but stays the same; the lead shape in the cover's tone.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Cover(t: WrappedMixtape, size: Dp, modifier: Modifier) {
    val seed = t.coverSeed
    /** A fraction from 0 to 1 out of the seed's bits from [at]. */
    fun part(at: Int) = ((seed shr at) and 0xFFu).toInt() / 255f
    val lead = t.coverTone.ink()
    val others = listOf(Ink.Success, Ink.Warning, Ink.Danger, Ink.Lab, Ink.Accent).filter { it != lead }
    val surface = MaterialTheme.colorScheme.surface
    val onSurface = MaterialTheme.colorScheme.onSurface
    Box(modifier.size(size).rotate(-3f).clip(RoundedCornerShape(28.dp)).background(surface)) {
        val shapes = listOf(MaterialShapes.Cookie12Sided, MaterialShapes.Clover8Leaf, MaterialShapes.Cookie7Sided, MaterialShapes.SoftBurst)
        Box(Modifier.offset(size * (0.38f + 0.12f * part(0)), -size * (0.22f - 0.1f * part(8))).size(size * (0.8f + 0.15f * part(16))).background(lead.tone().base, shapes[(seed % 4u).toInt()].toShape()))
        Box(Modifier.offset(-size * (0.2f - 0.1f * part(24)), size * (0.45f + 0.1f * part(32))).size(size * 0.62f).background(others[0].tone().base, MaterialShapes.Clover8Leaf.toShape()))
        Box(Modifier.offset(size * (0.38f + 0.08f * part(40)), size * 0.6f).rotate(-10f - 20f * part(48)).size(size * 0.62f, size * 0.24f).background(others[1].tone().base, RoundedCornerShape(50)))
        Box(Modifier.offset(size * (0.66f + 0.08f * part(56)), size * 0.64f).size(size * 0.24f).background(others[2].tone().base, MaterialShapes.SoftBurst.toShape()))
        Text(t.stamp, Modifier.padding(start = 14.dp, top = 14.dp), color = onSurface, style = type(14f, 800, -0.02f, 1f))
    }
}
