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
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
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
import vitapmate.data.AppScope
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.Pips
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.rememberHaptics

/** The most phones one address forwards to (the server's limit). */
private const val MAX_PHONES = 5

/** The phones on the address as the sheet reads them: loading, unreadable (offline), or the list. */
private sealed interface PhonesPhase {
    data object Loading : PhonesPhase

    data object Unavailable : PhonesPhase

    data class Ready(val phones: List<MailPhone>) : PhonesPhase
}

/** A change waiting for its confirm dialog: deleting the address, or taking one phone off it. */
internal sealed interface PhonesConfirm {
    data object Delete : PhonesConfirm

    data class Remove(val id: String) : PhonesConfirm

    companion object {
        /** Kept across process death as `delete` or `remove:<id>` (Stored), so the dialog comes back. */
        val Saver: Saver<PhonesConfirm?, String> = Saver(
            save = { c ->
                when (c) {
                    Delete -> "delete"
                    is Remove -> "remove:${c.id}"
                    null -> null
                }
            },
            restore = { s ->
                when {
                    s == "delete" -> Delete
                    s.startsWith("remove:") -> Remove(s.removePrefix("remove:"))
                    else -> null
                }
            },
        )
    }
}

/**
 * The phones on this phone's address ("N of 5"), each other one with Remove, and Delete the
 * address everywhere (asks first; [MailForwarding.deleteEverywhere]). Flows
 * `forwarding.phones` and `forwarding.remove_phone`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingPhones() {
    val haptics = rememberHaptics()
    val scheme = MaterialTheme.colorScheme
    var phase by remember { mutableStateOf<PhonesPhase>(PhonesPhase.Loading) }
    var confirm by rememberSaveable(stateSaver = PhonesConfirm.Saver) { mutableStateOf<PhonesConfirm?>(null) }
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
                            IconButton(
                                { confirm = PhonesConfirm.Remove(phone.id) },
                                Modifier.testTag("forwarding.phone.${phone.id.take(8)}.remove"),
                                enabled = !removing && MailForwarding.busy == null,
                            ) {
                                Icon(Lucide.Trash2, "Remove ${phone.label ?: "this phone"}")
                            }
                        }
                    }
                }
            }
        }
        val danger = AppTheme.tones.danger
        androidx.compose.material3.Button(
            onClick = { confirm = PhonesConfirm.Delete },
            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 44.dp).testTag("forwarding.delete"),
            enabled = MailForwarding.busy == null,
            shapes = ButtonDefaults.shapes(),
            colors = ButtonDefaults.buttonColors(containerColor = danger.subtle, contentColor = danger.onSubtle),
        ) {
            Text(if (MailForwarding.busy == ForwardingOp.Delete) "Deleting…" else "Delete address everywhere")
        }
    }

    val ready = (phase as? PhonesPhase.Ready)?.phones
    when (val c = confirm) {
        null -> Unit
        is PhonesConfirm.Remove -> {
            val label = ready?.firstOrNull { it.id == c.id }?.label ?: "this phone"
            AppDialog(
                { confirm = null },
                "Take $label off the address?",
                "Remove",
                {
                    confirm = null
                    // On the app's scope: closing the sheet mid-way mustn't drop the server's answer.
                    AppScope.launch {
                        removing = true
                        try {
                            val done = orNull { coreFlow("forwarding.remove_phone") { co, f -> co.removeForwardingPhone(f, c.id) } } != null
                            if (done) {
                                haptics.confirm()
                                MailForwarding.note("Phone removed", "It no longer gets forwarded OTPs.")
                                load()
                            } else {
                                haptics.reject()
                                MailForwarding.failed("Couldn't remove the phone", "Check your connection and try again.")
                            }
                        } finally {
                            removing = false
                        }
                    }
                },
                icon = Lucide.Smartphone,
                danger = true,
                confirmTag = "forwarding.phones.confirm",
            ) {
                Text("It stops getting forwarded OTPs. The address keeps working on the other phones.")
            }
        }
        PhonesConfirm.Delete -> {
            val count = ready?.size
            ForwardingConfirmSheet(
                onDismiss = { confirm = null },
                icon = Lucide.Trash2,
                title = "Delete the address everywhere?",
                body = "The address is deleted for ${count?.let { if (it == 1) "this phone" else "all $it phones" } ?: "every phone on it"}, and mail Gmail forwards to it is rejected.",
                confirmLabel = "Delete",
                confirmTag = "forwarding.phones.confirm",
                dismissTag = "forwarding.phones.cancel",
                sheetTag = "forwarding.delete.sheet",
                target = MailForwarding.status.cleanupTarget(),
                onConfirm = {
                    confirm = null
                    MailForwarding.deleteEverywhere()
                },
            )
        }
    }
}
