package vitapmate.features.outing

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Trash2
import vitapmate.core.OutingRequest
import vitapmate.core.OutingStanding
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonSize
import vitapmate.ui.ButtonVariant
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone
import vitapmate.ui.neutralTone
import vitapmate.ui.nums

/**
 * One request: the place (cancel at its end while VTOP allows it), why, when, then its status,
 * how soon, who it waits on and the Pass. A trip still ahead gets the accent outline. Busy
 * buttons show progress; the card itself only changes when VTOP's list does.
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
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = AppTheme.colors.card),
        border = request.highlight?.let { BorderStroke(1.dp, tones.accentTone.base) },
    ) {
        Column(Modifier.padding(start = Space.lg, top = Space.md, end = Space.md, bottom = Space.md)) {
            Row(verticalAlignment = Alignment.Top) {
                Text(
                    request.place,
                    Modifier.weight(1f).padding(top = Space.xs),
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                if (onCancel != null) {
                    Spacer(Modifier.width(Space.sm))
                    AnimatedContent(cancelling, transitionSpec = { fadeIn(effects) togetherWith fadeOut(effects) }, label = "cancel") { busy ->
                        Box(Modifier.size(40.dp), Alignment.Center) {
                            if (busy) LoadingIndicator(Modifier.size(28.dp).testTag("outing.cancelling"))
                            else IconButton(
                                onClick = onCancel,
                                modifier = Modifier.testTag("outing.cancel"),
                                shapes = IconButtonDefaults.shapes(),
                            ) { Icon(Lucide.Trash2, "Cancel request", Modifier.size(18.dp), tint = tones.danger.base) }
                        }
                    }
                }
            }
            if (request.purpose.isNotBlank()) {
                Text(request.purpose, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Spacer(Modifier.height(Space.xs))
            request.`when`.forEach { line ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.width(if (line.label == null) 18.dp else 42.dp)) {
                        val label = line.label
                        if (label == null) Icon(Lucide.Clock, null, Modifier.size(12.dp), tint = scheme.onSurfaceVariant)
                        else Text(label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                    }
                    Text(line.text, style = MaterialTheme.typography.labelLarge.nums(), color = scheme.onSurface)
                }
            }
            Spacer(Modifier.height(Space.sm))
            Row(verticalAlignment = Alignment.CenterVertically) {
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
