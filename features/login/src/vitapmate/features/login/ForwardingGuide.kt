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
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
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
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.composables.icons.lucide.ArrowLeft
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.TriangleAlert
import com.composables.icons.lucide.X
import vitapmate.ui.AppTheme
import org.json.JSONObject
import vitapmate.ui.AppTopBar
import vitapmate.ui.ProblemView
import vitapmate.ui.Tone
import vitapmate.ui.Tones
import vitapmate.ui.rememberGuide
import vitapmate.ui.rememberGuideImage
import vitapmate.ui.Space

/** A note in a [tone] ("lab", "warning", ...) with an [icon] ("mail-check", ...), as the edge names them. */
private class Note(val icon: String, val tone: String, val text: String)

/** One guide step: what to do, maybe a [note], then Gmail's screen for it (the student's address blurred). */
private class GuideStep(val title: String, val body: String, val note: Note?, val shot: String?, val caption: String?)

/** `guides/forwarding.json` on the edge: notes and steps in order; `{sender}` stands for VTOP's OTP sender. */
private class Forwarding(val title: String, val subtitle: String, val blocks: List<Any>)

private fun JSONObject.note() = Note(getString("icon"), getString("tone"), getString("text"))

private fun JSONObject.string(key: String) = optString(key).takeIf { has(key) && !isNull(key) }

private fun forwarding(json: JSONObject, sender: String): Forwarding {
    val blocks = json.getJSONArray("blocks")
    // Block types this app doesn't know are skipped (API.md).
    return Forwarding(json.getString("title"), json.getString("subtitle"), (0 until blocks.length()).map(blocks::getJSONObject).mapNotNull { b ->
        when (b.getString("type")) {
            "callout" -> b.note()
            "step" -> GuideStep(b.getString("title"), b.getString("body").replace("{sender}", sender), b.optJSONObject("callout")?.note(), b.string("image"), b.string("caption"))
            else -> null
        }
    })
}

private fun icon(name: String): ImageVector = when (name) {
    "mail-check" -> Lucide.MailCheck
    "refresh-cw" -> Lucide.RefreshCw
    "triangle-alert" -> Lucide.TriangleAlert
    else -> Lucide.Info
}

private fun Tones.named(name: String): Tone = when (name) {
    "lab" -> lab
    "warning" -> warning
    "success" -> success
    "danger" -> danger
    else -> accentTone
}

/**
 * Mail forwarding, step by step with Gmail's screens: a full-screen page over the OTP auto-fill
 * sheet, its text and screenshots read from the edge ([rememberGuide]). [sender] is VTOP's OTP
 * sender for the filter. Test tag `forwarding.guide`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingGuide(sender: String, onClose: () -> Unit) {
    val guide = rememberGuide("forwarding.json") { forwarding(it, sender) }
    val tones = AppTheme.tones
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
        Scaffold(
            Modifier.fillMaxSize().nestedScroll(scroll.nestedScrollConnection).testTag("forwarding.guide"),
            topBar = {
                AppTopBar(
                    guide.value?.title ?: "Set up forwarding",
                    subtitle = guide.value?.subtitle,
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
                val value = guide.value
                if (value == null) {
                    item {
                        Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) {
                            guide.problem?.let { ProblemView(it, guide.retry.takeIf { _ -> it.retryable }) } ?: LoadingIndicator()
                        }
                    }
                    return@LazyColumn
                }
                val numbers = value.blocks.runningFold(0) { n, b -> if (b is GuideStep) n + 1 else n }.drop(1)
                value.blocks.forEachIndexed { i, block ->
                    item(key = i) {
                        if (block is Note) Callout(icon(block.icon), tones.named(block.tone), block.text)
                        if (block is GuideStep) {
                            Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
                                Step(numbers[i], block.title, null, null, tag = "forwarding.guide", last = true) { block.body }
                                block.note?.let { Callout(icon(it.icon), tones.named(it.tone), it.text) }
                                block.shot?.let { Screenshot(it, block.caption) }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun Screenshot(path: String, caption: String?) {
    val shot = rememberGuideImage(path) ?: return
    val scheme = MaterialTheme.colorScheme
    val shape = MaterialTheme.shapes.large
    var open by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(start = 42.dp), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Image(
            shot,
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
private fun ShotViewer(shot: ImageBitmap, onClose: () -> Unit) {
    var scale by remember { mutableFloatStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.scrim).testTag("forwarding.guide.viewer")) {
            Image(
                shot,
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
