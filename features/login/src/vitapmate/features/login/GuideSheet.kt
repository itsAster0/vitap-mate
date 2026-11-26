package vitapmate.features.login

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.SheetValue
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.composables.icons.lucide.ChevronLeft
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.TriangleAlert
import com.composables.icons.lucide.X
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.ProblemView
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.Tones
import vitapmate.ui.rememberGuide
import vitapmate.ui.rememberGuideImage

/**
 * "Show me" for setup step [step]: its pages from `guides/forwarding.json` (Gmail's screens, the
 * student's address blurred), one at a time with Back and Next, and copy chips for the filter's
 * From ([sender]) and Forward to ([address]). [status] is the sheet's live status line. Test tags
 * `forwarding.guide`, `forwarding.guide.close`, `forwarding.guide.shot` (`.shot_missing` before it
 * loads), `forwarding.guide.dots`, `forwarding.guide.back`, `forwarding.guide.next`,
 * `forwarding.copy_sender`, `forwarding.copy_address`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun GuideSheet(step: Int, title: String, address: String, sender: String, status: String, onClose: () -> Unit) {
    val guide = rememberGuide("forwarding.json") { guidePages(it, sender) }
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    var index by rememberSaveable(step) { mutableIntStateOf(0) }
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    ModalBottomSheet(onClose, Modifier.semantics { testTagsAsResourceId = true }.testTag("forwarding.guide"), sheet) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(start = Space.xl, end = Space.xl, bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            val all = guide.value
            val pages = all?.let { pagesOrAll(step, it) }.orEmpty()
            // Coerced once: a refreshed guide can have fewer pages than the one the index was on.
            val at = shownPage(index, pages.size)
            val page = pages.getOrNull(at)
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    Text(
                        status,
                        Modifier.background(scheme.surfaceContainerHighest, CircleShape).padding(horizontal = Space.sm, vertical = 3.dp),
                        style = MaterialTheme.typography.labelMedium,
                        color = scheme.onSurfaceVariant,
                    )
                    Text("Step $step · $title", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                    page?.let { Text(it.title, style = MaterialTheme.typography.titleMedium) }
                }
                IconButton(onClose, Modifier.testTag("forwarding.guide.close")) {
                    Icon(Lucide.X, "Close", Modifier.size(18.dp))
                }
            }
            when {
                all == null -> Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) {
                    guide.problem?.let { ProblemView(it, guide.retry.takeIf { _ -> it.retryable }) } ?: LoadingIndicator()
                }
                page == null -> Text("No screens for this step yet.", style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                else -> {
                    Text(page.body, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                    page.shot?.let { Screenshot(it, page.caption) }
                    page.note?.let { Callout(noteIcon(it.icon), tones.named(it.tone), it.text) }
                    CopyChip("From", sender, "forwarding.copy_sender")
                    CopyChip("Forward to", address, "forwarding.copy_address")
                    if (pages.size > 1) {
                        Row(
                            Modifier.fillMaxWidth().clearAndSetSemantics { contentDescription = "Screen ${at + 1} of ${pages.size}" }.testTag("forwarding.guide.dots"),
                            horizontalArrangement = Arrangement.spacedBy(Space.xs, Alignment.CenterHorizontally),
                        ) {
                            pages.indices.forEach { i ->
                                Box(
                                    Modifier.size(width = if (i == at) 18.dp else 6.dp, height = 6.dp)
                                        .background(if (i == at) scheme.primary else scheme.outlineVariant, CircleShape),
                                )
                            }
                        }
                        Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                            Button(if (at > 0) ({ index = at - 1 }) else null, Modifier.weight(1f).testTag("forwarding.guide.back"), ButtonVariant.Outline, prefix = Lucide.ChevronLeft) { Text("Back") }
                            Button(if (at < pages.lastIndex) ({ index = at + 1 }) else null, Modifier.weight(1f).testTag("forwarding.guide.next")) {
                                Text("Next")
                                Icon(Lucide.ChevronRight, null, Modifier.size(18.dp))
                            }
                        }
                    }
                }
            }
        }
    }
}

/** A labelled value with a copy button, on the card colour. */
@Composable
private fun CopyChip(label: String, value: String, copyTag: String? = null) {
    val context = LocalContext.current
    val scheme = MaterialTheme.colorScheme
    Row(
        Modifier.fillMaxWidth().background(scheme.surfaceContainerHigh, MaterialTheme.shapes.medium).padding(start = Space.md),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        Text(
            value,
            Modifier.weight(1f),
            style = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.SemiBold),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        IconButton(
            { copy(context, label, value) },
            if (copyTag != null) Modifier.testTag(copyTag) else Modifier,
        ) { Icon(Lucide.Copy, "Copy $label", Modifier.size(18.dp)) }
    }
}

/** The icon a guide note names ("mail-check", ...). */
internal fun noteIcon(name: String): ImageVector = when (name) {
    "mail-check" -> Lucide.MailCheck
    "refresh-cw" -> Lucide.RefreshCw
    "triangle-alert" -> Lucide.TriangleAlert
    else -> Lucide.Info
}

/** The tone a guide note names ("lab", "warning", ...). */
internal fun Tones.named(name: String): Tone = when (name) {
    "lab" -> lab
    "warning" -> warning
    "success" -> success
    "danger" -> danger
    else -> accentTone
}

@Composable
private fun Screenshot(path: String, caption: String?) {
    val shot = rememberGuideImage(path)
    val scheme = MaterialTheme.colorScheme
    val shape = MaterialTheme.shapes.large
    var open by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        if (shot != null) {
            Image(
                shot,
                caption ?: "Gmail screen for this step",
                Modifier.fillMaxWidth().clip(shape).border(1.dp, scheme.outlineVariant, shape)
                    .clickable(onClickLabel = "Open the screenshot") { open = true }.testTag("forwarding.guide.shot"),
                contentScale = ContentScale.FillWidth,
            )
        } else {
            // Loading, or not saved yet and offline: the caption still says what to look for.
            Box(
                Modifier.fillMaxWidth().height(160.dp).clip(shape).background(scheme.surfaceContainerHigh).testTag("forwarding.guide.shot_missing"),
                Alignment.Center,
            ) {
                Text("Screenshot not loaded yet", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            }
        }
        Text(caption ?: if (shot != null) "Tap to open" else "Gmail's screen for this step", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
    }
    if (open && shot != null) ShotViewer(shot) { open = false }
}

/** A screenshot full screen: pinch to zoom and drag to pan, double tap to zoom in or back out. */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun ShotViewer(shot: ImageBitmap, onClose: () -> Unit) {
    var scale by remember { mutableFloatStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }
    var size by remember { mutableStateOf(IntSize.Zero) }
    Dialog(onClose, DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.scrim).semantics { testTagsAsResourceId = true }.testTag("forwarding.guide.viewer")) {
            Image(
                shot,
                "Screenshot",
                Modifier.fillMaxSize()
                    .onSizeChanged { size = it }
                    .pointerInput(Unit) {
                        detectTransformGestures { _, pan, zoom, _ ->
                            scale = (scale * zoom).coerceIn(1f, 5f)
                            offset = panWithin(offset + pan, scale, size.width.toFloat(), size.height.toFloat())
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

/** [offset] kept so a [scale]d image of [width] × [height] never pans off its edges. */
internal fun panWithin(offset: Offset, scale: Float, width: Float, height: Float): Offset {
    if (scale <= 1f || width <= 0f || height <= 0f) return Offset.Zero
    val maxX = width * (scale - 1f) / 2f
    val maxY = height * (scale - 1f) / 2f
    return Offset(offset.x.coerceIn(-maxX, maxX), offset.y.coerceIn(-maxY, maxY))
}
