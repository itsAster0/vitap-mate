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
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Contact
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.RegnoAnswer
import vitapmate.core.RegnoSharing
import vitapmate.data.AppCore
import vitapmate.data.RegnoConsent
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/**
 * [RegnoConsent.pending] as the "Share your registration number?" sheet: Share or Not now;
 * swiping it away is no answer. Test tags `regno.*`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
fun RegnoSheet() {
    val pending by RegnoConsent.pending.collectAsState()
    val request = pending ?: return
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    // The sheet slides away first, then Rust hears the answer.
    val answer = { given: RegnoAnswer ->
        if (given == RegnoAnswer.SHARE) haptics.confirm()
        scope.launch { sheet.hide() }.invokeOnCompletion { RegnoConsent.answer(request, given) }
        Unit
    }
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    ModalBottomSheet(
        { RegnoConsent.answer(request, RegnoAnswer.DISMISSED) },
        Modifier.semantics { testTagsAsResourceId = true }.testTag("regno.sheet"),
        sheet,
        containerColor = scheme.surfaceContainerLow,
    ) {
        // No taps through an overlay another app draws over the sheet (tapjacking).
        val view = LocalView.current
        DisposableEffect(view) { view.filterTouchesWhenObscured = true; onDispose {} }
        Column(
            Modifier.fillMaxWidth().padding(horizontal = Space.xl).padding(bottom = Space.xl),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(
                Modifier.padding(bottom = 16.dp).size(60.dp).background(tones.accent, MaterialShapes.Cookie7Sided.toShape()),
                Alignment.Center,
            ) { Icon(Lucide.Contact, null, Modifier.size(26.dp), tint = tones.onAccent) }
            Text(
                "Share your registration number?",
                Modifier.fillMaxWidth().semantics { heading() },
                style = MaterialTheme.typography.headlineSmall,
                color = scheme.onSurface,
                textAlign = TextAlign.Center,
            )
            Text(
                "It links you to the vitapmate server for connected apps, OTP auto-fill, notifications and server routes. Nothing else is sent.",
                Modifier.padding(top = 12.dp),
                style = MaterialTheme.typography.bodyMedium,
                color = scheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Row(Modifier.padding(top = 24.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                val side = Modifier.weight(1f).heightIn(min = 48.dp)
                FilledTonalButton(
                    onClick = { answer(RegnoAnswer.NOT_NOW) },
                    modifier = side.testTag("regno.not_now"),
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(24.dp, 10.dp, 10.dp, 24.dp)),
                    colors = ButtonDefaults.filledTonalButtonColors(scheme.surfaceContainerHighest, scheme.onSurface),
                ) { Text("Not now", maxLines = 1) }
                Button(
                    onClick = { answer(RegnoAnswer.SHARE) },
                    modifier = side.testTag("regno.share"),
                    shapes = ButtonDefaults.shapes(RoundedCornerShape(10.dp, 24.dp, 24.dp, 10.dp)),
                ) { Text("Share", maxLines = 1) }
            }
        }
    }
}

/** Settings → Connections: whether the server may know the registration number, with a switch. */
@Composable
fun RegnoSharingSetting() {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var version by remember { mutableIntStateOf(0) }
    val pending by RegnoConsent.pending.collectAsState()
    // Read again after each sheet and after a change here.
    val sharing by produceState<RegnoSharing?>(null, version, pending) {
        value = orNull { coreFlow("settings.regno") { c, f -> c.regnoSharing(f) } }
    }
    val current = sharing ?: return
    val on = current == RegnoSharing.SHARED
    val set = { share: Boolean ->
        haptics.toggle(share)
        scope.launch {
            val answer = if (share) RegnoAnswer.SHARE else RegnoAnswer.NOT_NOW
            val now = orNull { coreFlow("settings.regno_set") { c, f -> c.setRegnoSharing(f, answer) } } ?: return@launch
            version++
            if (now == RegnoSharing.SHARED) {
                AppCore.proveInBackground()
            } else {
                showToast("Not shared", "Server features stop on this phone until you share it again.")
            }
        }
        Unit
    }
    GroupItem(
        "Share registration number",
        "settings.regno",
        Lucide.Contact,
        when (current) {
            RegnoSharing.SHARED -> "The server knows who you are, for its features"
            RegnoSharing.DECLINED -> "Not shared: server features are off"
            RegnoSharing.UNASKED -> "Not shared yet"
        },
        onClick = { set(!on) },
        trailing = { ItemSwitch(on, { set(it) }) },
        on = on,
    )
}
