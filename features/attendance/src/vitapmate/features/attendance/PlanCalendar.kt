package vitapmate.features.attendance

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.formatDay
import vitapmate.ui.isoWeekday
import vitapmate.ui.nums

/**
 * Every month holding [days], Monday first. A class day is a [shape] tile, tagged `<tag>.<day>`:
 * outlined until [toneOf] gives it a tone, then on that tone's fill with a border in its base;
 * [corner] draws in it (a mark, a class count) and tapping it calls [onTap]. Other days are faint
 * and can't be tapped.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun PlanCalendar(
    days: List<Int>,
    tag: String,
    toneOf: (Int) -> Tone?,
    onTap: (Int) -> Unit,
    modifier: Modifier = Modifier,
    shape: Shape = CircleShape,
    corner: @Composable BoxScope.(day: Int, tone: Tone?) -> Unit = { _, _ -> },
) {
    val scheme = MaterialTheme.colorScheme
    val open = days.toSet()
    Column(modifier, verticalArrangement = Arrangement.spacedBy(Space.md)) {
        for (first in days.map { firstOfMonth(it) }.distinct()) {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(formatDay(first, "MMMM"), style = MaterialTheme.typography.labelLarge, color = scheme.onSurfaceVariant)
                Row(Modifier.fillMaxWidth()) {
                    for (d in listOf("M", "T", "W", "T", "F", "S", "S")) {
                        Text(d, Modifier.weight(1f), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant, textAlign = TextAlign.Center)
                    }
                }
                val leading = isoWeekday(first) - 1
                val length = daysInMonth(first)
                for (week in 0 until (leading + length + 6) / 7) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        for (col in 0 until 7) {
                            val n = week * 7 + col - leading
                            val day = first + n
                            val usable = n in 0 until length && day in open
                            val tone = if (usable) toneOf(day) else null
                            Box(
                                Modifier.weight(1f).height(44.dp)
                                    .then(
                                        when {
                                            tone != null -> Modifier.background(tone.subtle, shape).border(1.5.dp, tone.base, shape)
                                            usable -> Modifier.border(1.dp, scheme.outlineVariant, shape)
                                            else -> Modifier
                                        },
                                    )
                                    .then(if (usable) Modifier.clip(shape).clickable { onTap(day) }.testTag("$tag.$day") else Modifier),
                                Alignment.Center,
                            ) {
                                if (n !in 0 until length) return@Box
                                if (usable) corner(day, tone)
                                Text(
                                    "${n + 1}",
                                    style = MaterialTheme.typography.labelLarge.nums(),
                                    fontWeight = if (usable) FontWeight.Bold else FontWeight.Normal,
                                    color = when {
                                        tone != null -> tone.onSubtle
                                        usable -> scheme.onSurface
                                        else -> scheme.onSurfaceVariant.copy(alpha = 0.4f)
                                    },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
