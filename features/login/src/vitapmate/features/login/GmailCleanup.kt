package vitapmate.features.login

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import vitapmate.core.Forwarding
import vitapmate.ui.AppTheme
import vitapmate.ui.OpenHaptic
import vitapmate.ui.Space

/** The address details [GmailCleanup] needs for its two Gmail steps. */
internal data class CleanupTarget(val sender: String, val address: String, val account: String?)

internal fun Forwarding?.cleanupTarget(): CleanupTarget? = when (this) {
    is Forwarding.Waiting -> CleanupTarget(sender, address, collegeMail)
    is Forwarding.On -> CleanupTarget(sender, address, collegeMail)
    is Forwarding.Off, null -> null
}

/**
 * Two-step Gmail clean-up card shown before turning off or deleting a forwarding address:
 * delete the filter for [sender], then remove [address] from Gmail's forwarding settings.
 * Links open in Chrome under [account] when known. Test tag `forwarding.cleanup`.
 */
@Composable
internal fun GmailCleanup(sender: String, address: String, account: String? = null) {
    val scheme = MaterialTheme.colorScheme
    Column(
        Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(scheme.surfaceContainerHigh).padding(Space.lg).testTag("forwarding.cleanup"),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        Text("Clean up Gmail", style = MaterialTheme.typography.titleSmall, color = scheme.onSurface)
        CleanupStep(
            number = 1,
            title = "Delete the filter",
            body = "Filters → the one for $sender → Delete",
            linkLabel = "Open filters",
            url = gmailUrl(GmailPage.Filters, account),
        )
        CleanupStep(
            number = 2,
            title = "Remove the address",
            body = "Forwarding → pick $address → Remove address",
            linkLabel = "Open forwarding",
            url = gmailUrl(GmailPage.Forwarding, account),
            last = true,
        )
    }
}

@Composable
private fun CleanupStep(number: Int, title: String, body: String, linkLabel: String, url: String, last: Boolean = false) {
    val scheme = MaterialTheme.colorScheme
    val context = LocalContext.current
    val rail = scheme.surfaceContainerHighest
    Row(
        horizontalArrangement = Arrangement.spacedBy(Space.md),
        modifier = Modifier.testTag("forwarding.cleanup.step$number").drawBehind {
            // From under the mark down through the gap to the next step.
            if (!last) {
                val x = 15.dp.toPx()
                drawLine(rail, Offset(x, 32.dp.toPx()), Offset(x, size.height + Space.md.toPx()), 2.dp.toPx())
            }
        },
    ) {
        Box(Modifier.size(30.dp).background(scheme.secondaryContainer, CircleShape), Alignment.Center) {
            Text("$number", style = MaterialTheme.typography.labelLarge, color = scheme.onSecondaryContainer)
        }
        Column(Modifier.weight(1f).padding(top = 4.dp)) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = scheme.onSurface)
            Text(body, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
            TextButton({ openInChrome(context, url) }, Modifier.testTag("forwarding.cleanup.step$number.link")) {
                Icon(Lucide.ExternalLink, null, Modifier.size(16.dp))
                Spacer(Modifier.size(Space.xs))
                Text(linkLabel)
            }
        }
    }
}

/**
 * Confirmation sheet for turning off or deleting the forwarding address: a danger badge, [title],
 * one line of what happens ([body]), the [GmailCleanup] card when [target] is present, and the
 * joined Cancel / danger confirm buttons.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun ForwardingConfirmSheet(
    onDismiss: () -> Unit,
    icon: ImageVector,
    title: String,
    body: String,
    confirmLabel: String,
    confirmTag: String,
    dismissTag: String,
    sheetTag: String,
    target: CleanupTarget?,
    onConfirm: () -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        Modifier.semantics { testTagsAsResourceId = true }.testTag(sheetTag),
        sheet,
        containerColor = scheme.surfaceContainerLow,
    ) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Space.xl).padding(bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.md),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(Modifier.size(60.dp).background(tones.danger.subtle, MaterialShapes.Sunny.toShape()), Alignment.Center) {
                Icon(icon, null, Modifier.size(26.dp), tint = tones.danger.base)
            }
            Text(
                title,
                Modifier.fillMaxWidth().semantics { heading() },
                style = MaterialTheme.typography.headlineSmall,
                color = scheme.onSurface,
                textAlign = TextAlign.Center,
            )
            Text(
                body,
                Modifier.fillMaxWidth(),
                style = MaterialTheme.typography.bodyMedium,
                color = scheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            target?.let { GmailCleanup(it.sender, it.address, it.account) }
            Row(Modifier.fillMaxWidth().padding(top = Space.xs), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                val side = Modifier.weight(1f).heightIn(min = 48.dp)
                FilledTonalButton(
                    onClick = onDismiss,
                    modifier = side.testTag(dismissTag),
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(24.dp, 10.dp, 10.dp, 24.dp)),
                    colors = ButtonDefaults.filledTonalButtonColors(scheme.surfaceContainerHighest, scheme.onSurface),
                ) { Text("Cancel", maxLines = 1) }
                Button(
                    onClick = onConfirm,
                    modifier = side.testTag(confirmTag),
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(10.dp, 24.dp, 24.dp, 10.dp)),
                    colors = ButtonDefaults.buttonColors(scheme.error, scheme.onError),
                ) { Text(confirmLabel, maxLines = 1) }
            }
        }
    }
}
