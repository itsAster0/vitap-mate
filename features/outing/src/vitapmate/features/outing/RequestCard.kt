package vitapmate.features.outing

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Circle
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Hourglass
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.X
import vitapmate.core.OutingRequest
import vitapmate.core.OutingStanding
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonSize
import vitapmate.ui.ButtonVariant
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone
import vitapmate.ui.ToneBadge
import vitapmate.ui.neutralTone
import vitapmate.ui.nums

/**
 * One request as a trip: a shaped mark for its standing, the place and why, the when lines as one
 * row (Out at the start, the time away in a pill, Back at the end), then its status, how soon, who
 * it waits on, the Pass and Cancel (while VTOP allows it). A trip still ahead gets the accent
 * outline. Busy buttons show progress; the card itself only changes when VTOP's list does.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
@Composable
internal fun RequestCard(
    request: OutingRequest,
    onPass: (() -> Unit)?,
    passLoading: Boolean,
    onCancel: (() -> Unit)?,
    cancelling: Boolean,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val tone = standingTone(request.standing)
    val effects = MaterialTheme.motionScheme.fastEffectsSpec<Float>()
    Card(
        modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.extraLarge,
        colors = CardDefaults.cardColors(containerColor = AppTheme.colors.card),
        border = request.highlight?.let { BorderStroke(1.dp, tones.accentTone.base) },
    ) {
        Row(Modifier.padding(start = Space.lg, top = 14.dp, end = Space.lg, bottom = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(40.dp).background(tone.subtle, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                val icon = when (request.standing) {
                    OutingStanding.APPROVED -> Lucide.Check
                    OutingStanding.PENDING -> Lucide.Hourglass
                    OutingStanding.REJECTED -> Lucide.X
                    OutingStanding.OTHER -> Lucide.Circle
                }
                Icon(icon, null, Modifier.size(18.dp), tint = tone.base)
            }
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(request.place, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (request.purpose.isNotBlank()) {
                    Text(request.purpose, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
        if (request.`when`.isNotEmpty()) {
            // The trip: the first line at the start, the time away as a pill, the next line at the end.
            val away = request.`when`.firstOrNull { it.label == "Away" }
            val awayTone = if (request.standing == OutingStanding.APPROVED) tones.success else neutralTone
            Column(Modifier.padding(horizontal = Space.lg), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                (request.`when` - listOfNotNull(away)).chunked(2).forEachIndexed { index, legs ->
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                        legs.forEachIndexed { side, line ->
                            val label = line.label
                            if (label == null) Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                                Icon(Lucide.Clock, null, Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
                                Spacer(Modifier.width(Space.sm))
                                Text(line.text, style = MaterialTheme.typography.labelLarge.nums(), color = scheme.onSurface)
                            } else Column(Modifier.weight(1f), horizontalAlignment = if (side == 0) Alignment.Start else Alignment.End) {
                                Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                                // The date over the time.
                                Text(
                                    line.text.replace(", ", "\n"),
                                    style = MaterialTheme.typography.labelLarge.nums(),
                                    color = scheme.onSurface,
                                    textAlign = if (side == 0) TextAlign.Start else TextAlign.End,
                                )
                            }
                            if (side == 0 && index == 0 && away != null) {
                                val dash = Modifier.size(12.dp, 2.dp).background(awayTone.base.copy(alpha = 0.6f), CircleShape)
                                if (legs.size == 2) Box(dash)
                                ToneBadge("${away.text} ${away.label.orEmpty().lowercase()}", tone = awayTone)
                                if (legs.size == 2) Box(dash)
                            }
                        }
                    }
                }
            }
        }
        Row(Modifier.padding(start = Space.lg, top = Space.md, end = Space.md, bottom = Space.md), verticalAlignment = Alignment.CenterVertically) {
            FlowRow(
                Modifier.weight(1f),
                horizontalArrangement = Arrangement.spacedBy(Space.xs),
                verticalArrangement = Arrangement.spacedBy(Space.xs),
                itemVerticalAlignment = Alignment.CenterVertically,
            ) {
                StatusChip(request.statusLabel, tone)
                request.highlight?.let { StatusChip(it, tones.accentTone) }
                if (request.standing == OutingStanding.PENDING && request.status.isNotBlank()) {
                    Text(request.status, style = MaterialTheme.typography.labelMedium, color = tone.onSubtle)
                }
            }
            if (onPass != null) {
                Spacer(Modifier.width(Space.sm))
                Button(
                    if (passLoading) null else onPass,
                    Modifier.width(76.dp).testTag("outing.pass"),
                    variant = ButtonVariant.Outline,
                    size = ButtonSize.Sm,
                ) {
                    if (passLoading) LoadingIndicator(Modifier.size(20.dp)) else Text("Pass")
                }
            }
            if (onCancel != null) {
                Spacer(Modifier.width(Space.sm))
                AnimatedContent(cancelling, transitionSpec = { fadeIn(effects) togetherWith fadeOut(effects) }, label = "cancel") { busy ->
                    if (busy) Box(Modifier.size(40.dp), Alignment.Center) { LoadingIndicator(Modifier.size(28.dp).testTag("outing.cancelling")) }
                    else FilledTonalButton(
                        onClick = onCancel,
                        modifier = Modifier.height(40.dp).semantics { contentDescription = "Cancel request" }.testTag("outing.cancel"),
                        shapes = ButtonDefaults.shapes(),
                        colors = ButtonDefaults.filledTonalButtonColors(containerColor = tones.danger.subtle, contentColor = tones.danger.onSubtle),
                        contentPadding = PaddingValues(start = 12.dp, end = 16.dp),
                    ) {
                        Icon(Lucide.Trash2, null, Modifier.size(16.dp), tint = tones.danger.base)
                        Spacer(Modifier.width(6.dp))
                        Text("Cancel")
                    }
                }
            }
        }
    }
}

@Composable
private fun standingTone(standing: OutingStanding): Tone = AppTheme.tones.let {
    when (standing) {
        OutingStanding.APPROVED -> it.success
        OutingStanding.PENDING -> it.warning
        OutingStanding.REJECTED -> it.danger
        OutingStanding.OTHER -> neutralTone
    }
}
