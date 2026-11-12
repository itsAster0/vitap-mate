package vitapmate.features.login

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.composables.icons.lucide.ArrowLeft
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.X
import login.generated.resources.Res
import login.generated.resources.forwarding_guide_1_add
import login.generated.resources.forwarding_guide_2_verify
import login.generated.resources.forwarding_guide_3_from
import login.generated.resources.forwarding_guide_4_noreload
import login.generated.resources.forwarding_guide_5_forward
import login.generated.resources.forwarding_guide_6_done
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource
import vitapmate.ui.AppTheme
import vitapmate.ui.AppTopBar
import vitapmate.ui.Space

/** One guide step: what to do, then Gmail's screen for it (the student's address blurred). */
private class GuideStep(val title: String, val body: String, val shot: DrawableResource, val caption: String? = null)

/**
 * Mail forwarding, step by step with Gmail's screens: a full-screen page over the OTP auto-fill
 * sheet. [sender] is VTOP's OTP sender for the filter. Test tag `forwarding.guide`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun ForwardingGuide(sender: String, onClose: () -> Unit) {
    val steps = listOf(
        GuideStep(
            "Add your address in Gmail",
            "Copy your address from the app. In the Gmail where VTOP's mail arrives, open Settings → See all settings → Forwarding and POP/IMAP, " +
                "tap Add a forwarding address, paste it, then Next → Proceed → OK. On a phone, use Chrome with ⋮ → Desktop site.",
            Res.drawable.forwarding_guide_1_add,
        ),
        GuideStep(
            "Confirm it",
            "Gmail now shows Verify next to your address. Its confirmation mail comes to the app, not your inbox: tap Check, then Confirm in Gmail, " +
                "or type the code into Gmail's Verify box. Leave \"Disable forwarding\" selected, so only the filter forwards mail.",
            Res.drawable.forwarding_guide_2_verify,
        ),
        GuideStep(
            "Reload Gmail",
            "After confirming, reload the Gmail page before you make the filter. Without a reload, the filter has no address to forward to and shows only Add forwarding address, like this.",
            Res.drawable.forwarding_guide_4_noreload,
            "Before reloading: no address to pick",
        ),
        GuideStep(
            "Create the filter",
            "Open Filters and Blocked Addresses → Create a new filter. Put $sender in From, then tap Create filter.",
            Res.drawable.forwarding_guide_3_from,
        ),
        GuideStep(
            "Forward it to your address",
            "Tick Forward it to and pick your address. Mark as read keeps the OTP mails out of your unread count. Then tap Create filter.",
            Res.drawable.forwarding_guide_5_forward,
        ),
        GuideStep(
            "Done",
            "The filter shows in the list. Back in the app, tap Check: the next VTOP OTP fills in by itself. Mail sent before the filter isn't forwarded.",
            Res.drawable.forwarding_guide_6_done,
        ),
    )
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
        Scaffold(
            Modifier.fillMaxSize().nestedScroll(scroll.nestedScrollConnection).testTag("forwarding.guide"),
            topBar = {
                AppTopBar(
                    "Set up forwarding",
                    subtitle = "About 2 minutes in Gmail, once",
                    scrollBehavior = scroll,
                    navigationIcon = { IconButton(onClose, Modifier.testTag("forwarding.guide.close")) { Icon(Lucide.ArrowLeft, "Back") } },
                )
            },
            containerColor = MaterialTheme.colorScheme.background,
        ) { padding ->
            LazyColumn(
                contentPadding = PaddingValues(
                    start = Space.xl,
                    end = Space.xl,
                    top = padding.calculateTopPadding() + Space.sm,
                    bottom = padding.calculateBottomPadding() + Space.xl,
                ),
                verticalArrangement = Arrangement.spacedBy(Space.lg),
            ) {
                item {
                    Callout(Lucide.MailCheck, AppTheme.tones.lab, "Do this in Gmail on a computer, or in Chrome's desktop site. The Gmail app has no forwarding settings.")
                }
                steps.forEachIndexed { i, step ->
                    item(key = i) {
                        Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
                            Step(i + 1, step.title, null, null, tag = "forwarding.guide", last = true) { step.body }
                            if (i == 2) Callout(Lucide.RefreshCw, AppTheme.tones.warning, "Reload first, or Gmail won't list your address in the filter.")
                            Screenshot(step.shot, step.caption)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun Screenshot(shot: DrawableResource, caption: String?) {
    val scheme = MaterialTheme.colorScheme
    val shape = MaterialTheme.shapes.large
    var open by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(start = 42.dp), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Image(
            painterResource(shot),
            caption ?: "Gmail screen for this step",
            Modifier.fillMaxWidth().clip(shape).border(1.dp, scheme.outlineVariant, shape)
                .clickable(onClickLabel = "Open the screenshot") { open = true }.testTag("forwarding.guide.shot"),
            contentScale = ContentScale.FillWidth,
        )
        Text(caption ?: "Tap to open", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
    }
    if (open) ShotViewer(shot) { open = false }
}

/** A screenshot full screen: pinch to zoom and drag to pan, double tap to zoom in or back out. */
@Composable
private fun ShotViewer(shot: DrawableResource, onClose: () -> Unit) {
    var scale by remember { mutableFloatStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.scrim).testTag("forwarding.guide.viewer")) {
            Image(
                painterResource(shot),
                "Screenshot",
                Modifier.fillMaxSize()
                    .pointerInput(Unit) {
                        detectTransformGestures { _, pan, zoom, _ ->
                            scale = (scale * zoom).coerceIn(1f, 5f)
                            offset = if (scale == 1f) Offset.Zero else offset + pan
                        }
                    }
                    .pointerInput(Unit) {
                        detectTapGestures(onDoubleTap = {
                            scale = if (scale > 1f) 1f else 2.5f
                            offset = Offset.Zero
                        })
                    }
                    .graphicsLayer {
                        scaleX = scale
                        scaleY = scale
                        translationX = offset.x
                        translationY = offset.y
                    },
                contentScale = ContentScale.Fit,
            )
            FilledTonalIconButton(
                onClose,
                Modifier.align(Alignment.TopEnd).windowInsetsPadding(WindowInsets.safeDrawing).padding(Space.md).testTag("forwarding.guide.viewer.close"),
            ) { Icon(Lucide.X, "Close") }
        }
    }
}
