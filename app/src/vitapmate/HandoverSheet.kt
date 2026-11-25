package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Bot
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Globe
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Puzzle
import com.composables.icons.lucide.TimerOff
import com.composables.icons.lucide.TriangleAlert
import kotlinx.coroutines.delay
import vitapmate.core.HandoverAsk
import vitapmate.data.HandoverConsent
import vitapmate.data.HandoverConsent.Sheet
import vitapmate.data.byAiApp
import vitapmate.data.who
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.rememberHaptics
import vitapmate.ui.rememberNow

/** How long the "signed in" sheet stays before it closes itself. */
private const val SENT_CLOSE_MS = 2_500L

/**
 * [HandoverConsent.sheet] as the approve sheet: the key that asked and the address it asked from,
 * Deny or Approve, then sending until the server took the session (or why not). Swiping it away
 * is Deny. The server's host sits at the bottom. Test tags `handover.*`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
fun HandoverSheet() {
    val current by HandoverConsent.sheet.collectAsState()
    val shown = current ?: return
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    val haptics = rememberHaptics()
    val scheme = MaterialTheme.colorScheme
    if (shown is Sheet.Sent) LaunchedEffect(shown) {
        haptics.confirm()
        delay(SENT_CLOSE_MS)
        HandoverConsent.close()
    }
    ModalBottomSheet(
        { HandoverConsent.close() },
        Modifier.semantics { testTagsAsResourceId = true }.testTag("handover.sheet"),
        sheet,
        containerColor = scheme.surfaceContainerLow,
    ) {
        // No taps through an overlay another app draws over the sheet (tapjacking).
        val view = LocalView.current
        DisposableEffect(view) { view.filterTouchesWhenObscured = true; onDispose {} }
        Column(
            Modifier.fillMaxWidth().padding(horizontal = Space.xl).padding(bottom = Space.lg),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            val ask = shown.ask
            Hero(shown)
            Text(
                title(shown),
                Modifier.fillMaxWidth().semantics { heading() }.testTag("handover.title"),
                style = MaterialTheme.typography.headlineSmall,
                color = scheme.onSurface,
                textAlign = TextAlign.Center,
            )
            Text(
                subtitle(shown),
                Modifier.padding(top = 8.dp),
                style = MaterialTheme.typography.bodyMedium,
                color = scheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Details(ask, Modifier.padding(top = 20.dp))
            if (shown is Sheet.Asking) Countdown(ask, Modifier.padding(top = 14.dp))
            Actions(shown, Modifier.padding(top = 16.dp)) { approve ->
                if (approve) haptics.confirm()
                HandoverConsent.answer(approve)
            }
            ServerFooter(ask.server, Modifier.padding(top = 14.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(shown: Sheet) {
    val tones = AppTheme.tones
    val look = when (shown) {
        is Sheet.Sent -> HeroLook(MaterialShapes.Sunny, tones.success.subtle, tones.success.onSubtle, Lucide.Check)
        is Sheet.Failed -> HeroLook(MaterialShapes.Cookie7Sided, tones.danger.subtle, tones.danger.onSubtle, Lucide.TriangleAlert)
        is Sheet.Expired -> HeroLook(MaterialShapes.Cookie7Sided, MaterialTheme.colorScheme.surfaceContainerHighest, MaterialTheme.colorScheme.onSurfaceVariant, Lucide.TimerOff)
        else -> HeroLook(MaterialShapes.Cookie7Sided, tones.accent, tones.onAccent, if (shown.ask.byAiApp) Lucide.Bot else Lucide.Puzzle)
    }
    Box(Modifier.padding(bottom = 14.dp).size(64.dp).background(look.background, look.shape.toShape()), Alignment.Center) {
        Icon(look.icon, null, Modifier.size(28.dp), tint = look.tint)
    }
}

private data class HeroLook(val shape: androidx.graphics.shapes.RoundedPolygon, val background: Color, val tint: Color, val icon: ImageVector)

private fun title(shown: Sheet) = when (shown) {
    is Sheet.Asking, is Sheet.Sending -> "Sign in ${shown.ask.who}?"
    is Sheet.Sent -> "${shown.ask.who} is signed in"
    is Sheet.Failed -> "Couldn't sign in ${shown.ask.who}"
    is Sheet.Expired -> "This request expired"
}

private fun subtitle(shown: Sheet) = when (shown) {
    is Sheet.Asking -> if (shown.ask.byAiApp) {
        "An AI app wants a live VTOP session for a page its tools don't cover."
    } else {
        "It wants a live VTOP session for this browser."
    }
    is Sheet.Sending -> "Getting your VTOP session and sending it."
    is Sheet.Sent -> "VTOP is opening there. This closes in a moment."
    is Sheet.Failed -> shown.reason
    is Sheet.Expired -> "It waited too long. Ask again from ${shown.ask.who}."
}

/** The key, then the address it asked from (none for an AI app). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Details(ask: HandoverAsk, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    val now by rememberNow(1_000)
    Column(modifier.fillMaxWidth().clip(MaterialTheme.shapes.largeIncreased), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
        DetailRow(
            Lucide.KeyRound,
            "Key",
            ask.who,
            tag = "handover.key",
            chip = DetailChip(if (ask.byAiApp) "AI app" else "Extension", tones.accentTone.subtle, tones.accentTone.onSubtle),
        )
        ask.asker?.ip?.let { ip ->
            DetailRow(
                Lucide.Globe,
                "Asked from",
                ip.address,
                tag = "handover.ip",
                mono = true,
                note = ask.askedAt?.let { agoText(now - it) },
                chip = if (ip.seen) DetailChip("Seen before", tones.success.subtle, tones.success.onSubtle) else DetailChip("New IP", tones.warning.subtle, tones.warning.onSubtle),
                warn = !ip.seen,
            )
        }
    }
}

private data class DetailChip(val text: String, val background: Color, val content: Color)

@Composable
private fun DetailRow(icon: ImageVector, label: String, value: String, tag: String, mono: Boolean = false, note: String? = null, chip: DetailChip? = null, warn: Boolean = false) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Row(
        Modifier.fillMaxWidth().background(scheme.surfaceContainerHigh, MaterialTheme.shapes.extraSmall).padding(horizontal = 14.dp, vertical = 12.dp).testTag(tag),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            Modifier.size(36.dp).background(if (warn) tones.warning.subtle else scheme.surfaceContainerHighest, RoundedCornerShape(12.dp)),
            Alignment.Center,
        ) { Icon(icon, null, Modifier.size(18.dp), tint = if (warn) tones.warning.onSubtle else tones.accentTone.base) }
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            Text(
                value,
                style = MaterialTheme.typography.titleMedium.let { if (mono) it.copy(fontFamily = FontFamily.Monospace) else it },
                color = scheme.onSurface,
            )
            note?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant) }
        }
        chip?.let {
            Text(
                it.text,
                Modifier.background(it.background, RoundedCornerShape(50)).padding(horizontal = 10.dp, vertical = 4.dp),
                style = MaterialTheme.typography.labelMedium,
                color = it.content,
                maxLines = 1,
            )
        }
    }
}

/** Time left to answer, as a wavy line that empties. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Countdown(ask: HandoverAsk, modifier: Modifier = Modifier) {
    val expiresAt = ask.expiresAt ?: return
    val scheme = MaterialTheme.colorScheme
    val now by rememberNow(1_000)
    val total = remember(ask) { (expiresAt - (ask.askedAt ?: System.currentTimeMillis())).coerceAtLeast(1) }
    val left = (expiresAt - now).coerceAtLeast(0)
    Row(modifier.fillMaxWidth().testTag("handover.countdown"), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        LinearWavyProgressIndicator(
            progress = { (left.toFloat() / total).coerceIn(0f, 1f) },
            modifier = Modifier.weight(1f),
            color = AppTheme.tones.accentTone.base,
            trackColor = scheme.surfaceContainerHighest,
        )
        Text("%d:%02d left".format(left / 60_000, left / 1000 % 60), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Actions(shown: Sheet, modifier: Modifier = Modifier, onAnswer: (Boolean) -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        val tall = Modifier.heightIn(min = 56.dp)
        when (shown) {
            is Sheet.Asking, is Sheet.Sending -> {
                val sending = shown is Sheet.Sending
                FilledTonalButton(
                    onClick = { onAnswer(false) },
                    modifier = tall.weight(1f).testTag("handover.deny"),
                    enabled = !sending,
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(28.dp, 10.dp, 10.dp, 28.dp)),
                    colors = ButtonDefaults.filledTonalButtonColors(scheme.surfaceContainerHighest, scheme.onSurface),
                ) { Text("Deny", maxLines = 1) }
                Button(
                    onClick = { onAnswer(true) },
                    modifier = tall.weight(1.4f).testTag("handover.approve"),
                    enabled = !sending,
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(10.dp, 28.dp, 28.dp, 10.dp)),
                ) {
                    if (sending) LoadingIndicator(Modifier.size(28.dp), color = scheme.onSurfaceVariant) else Text("Approve", maxLines = 1)
                }
            }
            else -> FilledTonalButton(
                onClick = { HandoverConsent.close() },
                modifier = tall.weight(1f).testTag("handover.close"),
                colors = ButtonDefaults.filledTonalButtonColors(scheme.surfaceContainerHighest, scheme.onSurface),
            ) { Text(if (shown is Sheet.Sent) "Done" else "Close", maxLines = 1) }
        }
    }
}

/** "Sent through vmate.aster0.dev", by a small flower. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ServerFooter(server: String, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Row(modifier.testTag("handover.server"), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Box(Modifier.size(14.dp).background(scheme.onSurfaceVariant, MaterialShapes.Flower.toShape()))
        Text("Sent through ", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
        Text(server, Modifier.padding(start = 0.dp), style = MaterialTheme.typography.labelMedium, color = scheme.onSurface)
    }
}

private fun agoText(ms: Long): String {
    val s = (ms / 1000).coerceAtLeast(0)
    return if (s < 60) "Asked $s s ago" else "Asked ${s / 60} min ago"
}
