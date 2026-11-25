package vitapmate.features.login

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Smartphone
import com.composables.icons.lucide.Trash2
import kotlinx.coroutines.launch
import vitapmate.core.MailPhone
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.Pips
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** The most phones one address forwards to (the server's limit). */
private const val MAX_PHONES = 5

/** The phones on the address as the sheet reads them: loading, unreadable (offline), or the list. */
private sealed interface PhonesPhase {
    data object Loading : PhonesPhase

    data object Unavailable : PhonesPhase

    data class Ready(val phones: List<MailPhone>) : PhonesPhase
}

/** A change waiting for its confirm dialog: taking [phone] off, or deleting the address ([phone] null). */
private class PhonesConfirm(val phone: MailPhone?)

/**
 * The phones on this phone's address ("N of 5"), each other one with Remove, and Delete the
 * address everywhere (asks first; [MailForwarding.deleteEverywhere]). Flows
 * `forwarding.phones` and `forwarding.remove_phone`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingPhones() {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val scheme = MaterialTheme.colorScheme
    var phase by remember { mutableStateOf<PhonesPhase>(PhonesPhase.Loading) }
    var confirm by remember { mutableStateOf<PhonesConfirm?>(null) }
    var removing by remember { mutableStateOf(false) }
    suspend fun load() {
        val phones = orNull { coreFlow("forwarding.phones") { c, f -> c.forwardingPhones(f) } }
        phase = if (phones != null) PhonesPhase.Ready(phones) else if (phase is PhonesPhase.Ready) phase else PhonesPhase.Unavailable
    }
    LaunchedEffect(Unit) { load() }

    Column(
        Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(scheme.surfaceContainerHigh).padding(Space.lg).testTag("forwarding.phones"),
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        val p = phase
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Phones on this address", Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
            if (p is PhonesPhase.Ready) Text("${p.phones.size} of $MAX_PHONES", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        }
        when (p) {
            PhonesPhase.Loading -> LoadingIndicator(Modifier.size(28.dp))
            PhonesPhase.Unavailable -> Text("Couldn't load them; check your connection.", style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
            is PhonesPhase.Ready -> {
                Pips(p.phones.size, MAX_PHONES)
                p.phones.forEach { phone ->
                    Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(32.dp).background(scheme.secondaryContainer, CircleShape), Alignment.Center) {
                            Icon(Lucide.Smartphone, null, Modifier.size(16.dp), tint = scheme.onSecondaryContainer)
                        }
                        Column(Modifier.weight(1f)) {
                            Text(phone.label ?: "Phone", style = MaterialTheme.typography.bodyLarge)
                            Text("seen ${ago(phone.seenAt)} · added ${ago(phone.addedAt)}", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                        }
                        if (phone.thisPhone) {
                            StatusChip("This phone", AppTheme.tones.accentTone, solid = true)
                        } else {
                            IconButton({ confirm = PhonesConfirm(phone) }, Modifier.testTag("forwarding.phone.${phone.id.take(8)}.remove"), enabled = !removing) {
                                Icon(Lucide.Trash2, "Remove")
                            }
                        }
                    }
                }
            }
        }
        val danger = AppTheme.tones.danger
        androidx.compose.material3.Button(
            onClick = { confirm = PhonesConfirm(null) },
            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 44.dp).testTag("forwarding.delete"),
            enabled = MailForwarding.busy == null,
            shapes = ButtonDefaults.shapes(),
            colors = ButtonDefaults.buttonColors(containerColor = danger.subtle, contentColor = danger.onSubtle),
        ) {
            Text(if (MailForwarding.busy == ForwardingOp.Delete) "Deleting…" else "Delete address everywhere")
        }
    }

    confirm?.let { c ->
        val phone = c.phone
        val count = (phase as? PhonesPhase.Ready)?.phones?.size
        AppDialog(
            { confirm = null },
            if (phone != null) "Take ${phone.label ?: "this phone"} off the address?" else "Delete the address everywhere?",
            if (phone != null) "Remove" else "Delete",
            {
                confirm = null
                if (phone == null) {
                    MailForwarding.deleteEverywhere()
                } else {
                    scope.launch {
                        removing = true
                        val done = orNull { coreFlow("forwarding.remove_phone") { co, f -> co.removeForwardingPhone(f, phone.id) } } != null
                        if (done) {
                            haptics.confirm()
                            showToast("Mail forwarding", "It no longer gets forwarded OTPs.")
                            load()
                        } else {
                            haptics.reject()
                            showToast("Mail forwarding", "Couldn't remove it; try again.")
                        }
                        removing = false
                    }
                }
            },
            icon = if (phone != null) Lucide.Smartphone else Lucide.Trash2,
            danger = true,
            confirmTag = "forwarding.phones.confirm",
        ) {
            Text(
                if (phone != null) "It stops getting forwarded OTPs. The address keeps working on the other phones."
                else "The address is deleted for ${count?.let { if (it == 1) "this phone" else "all $it phones" } ?: "every phone on it"}; mail Gmail forwards to it is rejected. Remove the forwarding in Gmail too.",
            )
        }
    }
}
