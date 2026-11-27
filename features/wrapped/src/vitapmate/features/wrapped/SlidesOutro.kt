package vitapmate.features.wrapped

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
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
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.Share
import vitapmate.core.WrappedReport

// The last screen: what to do with the story now.

/** "Win 25-26" for "Winter Semester 2025-26". */
internal fun semesterChip(name: String): String {
    val season = shortName(name)
    val short = when (season.lowercase()) {
        "winter" -> "Win"
        "summer" -> "Sum"
        else -> season.take(4)
    }
    val years = name.trim().substringAfterLast(' ').removePrefix("20")
    return if (years == name.trim()) short else "$short $years"
}

/**
 * The outro: that's a wrap, the story's cards fanned out when there's room, and what to do
 * next: share a card, switch semester (with more than one), save to Docs, add campus times
 * while class days lack them, or replay. Never shared. Flexible.
 */
@OptIn(ExperimentalLayoutApi::class)
internal fun outroSlide(r: WrappedReport) = Slide("outro", null, fixed = false, outro = true, shareable = { false }) { c ->
    val o = c.outro ?: return@Slide
    val ink = LocalInks.current
    Flow {
        H(c.name?.let { "That's a wrap,\n$it" } ?: "That's a wrap", fits(42f, 36f))
        FlowRow(Modifier.fillMaxWidth().padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Chip(r.title.label())
            r.attendance?.let { Chip("${it.percent}% attendance") }
        }
        if (LocalFrame.current.height >= 600f) Deck(r, Modifier.fillMaxWidth().weight(1f).padding(vertical = 12.dp).heightIn(min = 96.dp))
        else Spacer(Modifier.weight(1f).heightIn(min = 12.dp))
        val campus = c.campus
        val tiles = listOfNotNull<@Composable (Modifier) -> Unit>(
            { m -> Hub(m.testTag("wrapped.outro.share"), Ink.Accent, Lucide.Share, "Share a card", o.share) { HubLine("Story, wallpaper or receipt") } },
            o.semesters.takeIf { it.isNotEmpty() }?.let { list ->
                { m ->
                    Hub(m, Ink.Warning, Lucide.CalendarRange, "Switch semester", null) {
                        FlowRow(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            list.take(3).forEach { (id, name) ->
                                Text(
                                    semesterChip(name),
                                    Modifier.background(MaterialTheme.colorScheme.surface.copy(alpha = 0.45f), RoundedCornerShape(14.dp))
                                        .clickable(role = Role.Button, onClickLabel = "Switch to $name") { o.pick(id) }
                                        .testTag("wrapped.outro.semester")
                                        .padding(horizontal = 10.dp, vertical = 8.dp),
                                    color = LocalInks.current.text, style = type(11f, 700, 0f, 1.2f), maxLines = 1,
                                )
                            }
                        }
                    }
                }
            },
            { m -> Hub(m.testTag("wrapped.outro.save"), Ink.Lab, Lucide.Download, "Save to Docs", o.save) { HubLine("Keep your summary for later") } },
            if (campus.missing.isNotEmpty() || campus.running) {
                { m ->
                    Hub(m.testTag("wrapped.outro.campus"), Ink.Success, Lucide.Clock, "Campus times", campus.start.takeIf { !campus.running }) {
                        if (campus.running) {
                            HubLine("Adding your class days, ${campus.done} of ${campus.total}")
                            val part = if (campus.total > 0) campus.done.toFloat() / campus.total else 0f
                            Box(Modifier.padding(top = 6.dp).fillMaxWidth().height(6.dp).background(LocalInks.current.text.copy(alpha = 0.2f), RoundedCornerShape(3.dp))) {
                                Box(Modifier.fillMaxWidth(part.coerceIn(0.03f, 1f)).height(6.dp).background(LocalInks.current.shape, RoundedCornerShape(3.dp)))
                            }
                        } else HubLine("Gate and hostel times for ${count(campus.missing.size, "class day")}")
                    }
                }
            } else null,
        )
        Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            tiles.chunked(2).forEach { pair ->
                Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    pair.forEach { tile -> tile(Modifier.weight(1f).fillMaxHeight()) }
                    if (pair.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
        Row(
            Modifier.fillMaxWidth().padding(top = 8.dp).height(48.dp).clickable(onClick = o.replay).testTag("wrapped.outro.replay")
                .semantics { role = Role.Button },
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(Lucide.RefreshCw, null, Modifier.size(18.dp), tint = ink.text)
            Text("Replay from the start", Modifier.padding(start = 8.dp), color = ink.text, style = type(15f, 700, -0.01f, 1.2f))
        }
    }
}

/** A tile of the outro in [tone]: its icon on a shape, a title, and [content]; [onClick] makes it a button. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hub(modifier: Modifier, tone: Ink, icon: ImageVector, title: String, onClick: (() -> Unit)?, content: @Composable ColumnScope.() -> Unit) {
    val t = inks(tone)
    val compact = LocalFrame.current.compact
    var m = modifier.heightIn(min = if (compact) 104.dp else 124.dp).background(t.bg, RoundedCornerShape(24.dp))
    if (onClick != null) m = m.clickable(onClick = onClick).semantics { role = Role.Button; contentDescription = title }
    androidx.compose.runtime.CompositionLocalProvider(LocalInks provides t) {
        Column(m.padding(start = 14.dp, top = 12.dp, end = 14.dp, bottom = 14.dp)) {
            Box(Modifier.size(40.dp).background(t.shape, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Icon(icon, null, Modifier.size(19.dp), tint = t.onShape)
            }
            Spacer(Modifier.weight(1f).heightIn(min = 8.dp))
            Text(title, color = t.shape, style = type(17f, 800, -0.03f, 1.1f), maxLines = 2)
            content()
        }
    }
}

@Composable
private fun HubLine(text: String) {
    Text(text, Modifier.padding(top = 3.dp), color = LocalInks.current.text.copy(alpha = 0.85f), style = type(12f, 600, -0.01f, 1.3f), maxLines = 3)
}

/** Three of the story's cards fanned out: its attendance, streak and hours, each in its tone. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Deck(r: WrappedReport, modifier: Modifier) {
    val cards = listOfNotNull(
        r.attendance?.let { Triple(Ink.Lab, -12f, "${it.percent}%") },
        r.streak?.let { Triple(Ink.Warning, 9f, "${it.length}") },
        r.time?.takeIf { it.minutes > 0 }?.let { Triple(Ink.Accent, 0f, "${(it.minutes + 30) / 60}h") },
    )
    if (cards.isEmpty()) {
        Spacer(modifier)
        return
    }
    val surface = MaterialTheme.colorScheme.surface
    BoxWithConstraints(modifier, Alignment.Center) {
        val k = minOf(maxWidth / 312.dp, maxHeight / 190.dp)
        Box(Modifier.size(312.dp * k, 190.dp * k)) {
            val xs = listOf(40, 172, 106)
            cards.forEachIndexed { i, (tone, turn, value) ->
                val t = inks(tone)
                Column(
                    Modifier.offset(xs[i].dp * k, 14.dp * k).size(92.dp * k, 160.dp * k).rotate(turn)
                        .background(surface, RoundedCornerShape(18.dp * k)).padding(3.dp * k)
                        .background(t.bg, RoundedCornerShape(16.dp * k)).padding(14.dp * k),
                ) {
                    Box(Modifier.size(64.dp * k).background(t.shape, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                        Text(value, color = t.onShape, style = type(19f * k, 800, -0.05f, 1f), maxLines = 1)
                    }
                    Box(Modifier.padding(top = 18.dp * k).size(54.dp * k, 8.dp * k).background(t.text.copy(alpha = 0.8f), RoundedCornerShape(4.dp)))
                    Box(Modifier.padding(top = 8.dp * k).size(38.dp * k, 6.dp * k).background(t.text.copy(alpha = 0.45f), RoundedCornerShape(3.dp)))
                }
            }
        }
    }
}
