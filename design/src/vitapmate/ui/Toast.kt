package vitapmate.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarData
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarVisuals
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.addOutline
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.CircleAlert
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.TriangleAlert
import kotlin.math.abs
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import vitapmate.data.BackgroundProblems
import vitapmate.data.Problem
import vitapmate.data.ProblemKind
import vitapmate.data.toasts
import vitapmate.observability.Log

/** What a toast is about: its icon and colour. */
enum class ToastTone { Neutral, Success, Warning, Error }

/** A toast: [title], [message] (may be empty), [tone], [icon] (else the tone's), [action] (e.g. Retry). */
class ToastVisuals(
    val title: String,
    override val message: String,
    val action: Pair<String, () -> Unit>? = null,
    val tone: ToastTone = ToastTone.Neutral,
    val icon: ImageVector? = null,
) : SnackbarVisuals {
    override val actionLabel: String? = action?.first
    override val withDismissAction = false
    override val duration = if (action != null) SnackbarDuration.Long else SnackbarDuration.Short
}

/** How long a toast shows, in ms: [ToastHost] dismisses it then and its badge's ring drains over it. */
private val SnackbarVisuals.lifetime get() = if (duration == SnackbarDuration.Long) 8_000 else 4_000

private val toasts = MutableSharedFlow<ToastVisuals>(extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)

/** Shows a toast on the app's [LocalSnackbar] (needs [ToastHost]); a newer one replaces it. */
fun showToast(
    title: String,
    description: String,
    action: Pair<String, () -> Unit>? = null,
    tone: ToastTone = ToastTone.Neutral,
    icon: ImageVector? = null,
) {
    Log.info("ui.toast", title, fields = mapOf("tone" to tone.name))
    toasts.tryEmit(ToastVisuals(title, description, action, tone, icon))
}

/** [problem] as a toast, when it [toasts] (automatic work only with "Show background errors"). */
fun showToast(problem: Problem, action: Pair<String, () -> Unit>? = null) {
    if (!problem.toasts) return
    val tone = when (problem.kind) {
        ProblemKind.Offline, ProblemKind.VtopDown -> ToastTone.Warning
        ProblemKind.SignInNeeded, ProblemKind.SemesterNeeded, ProblemKind.OtpNeeded -> ToastTone.Neutral
        ProblemKind.OtpRejected, ProblemKind.Unexpected -> ToastTone.Error
    }
    showToast(problem.title, problem.message, action, tone, problem.kind.icon)
}

/**
 * The app's toasts, last in the root box: [showToast] messages, failures of background work
 * ([BackgroundProblems]) and anything posted on [LocalSnackbar]. They sit just above the floating
 * nav bar and follow it as it slides ([NavBarSpace]), else at the bottom inset; spring in, swipe
 * sideways or time out.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ToastHost() {
    val host = LocalSnackbar.current
    LaunchedEffect(host) { toasts.collectLatest { host.showSnackbar(it) } }
    LaunchedEffect(Unit) { BackgroundProblems.shown.collect { showToast(it) } }
    val current = host.currentSnackbarData
    LaunchedEffect(current) {
        val data = current ?: return@LaunchedEffect
        delay(data.visuals.lifetime.toLong())
        data.dismiss()
    }
    val insets = WindowInsets.safeDrawing
    val motion = MaterialTheme.motionScheme
    Box(
        Modifier.fillMaxSize().wrapContentSize(Alignment.BottomCenter)
            .windowInsetsPadding(insets.only(WindowInsetsSides.Horizontal + WindowInsetsSides.Bottom)).padding(bottom = Space.lg)
            .offset {
                // As the OTP pill: Space.md above the bar's top while it shows.
                val rest = insets.getBottom(this) + Space.lg.roundToPx()
                IntOffset(0, -(NavBarSpace.px + Space.md.roundToPx() - rest).coerceAtLeast(0))
            },
    ) {
        AnimatedContent(
            current,
            transitionSpec = {
                (slideInVertically(motion.defaultSpatialSpec()) { it } + scaleIn(motion.defaultSpatialSpec(), 0.8f) + fadeIn(motion.defaultEffectsSpec())) togetherWith
                    (scaleOut(motion.fastSpatialSpec(), 0.9f) + slideOutVertically(motion.fastSpatialSpec()) { it / 2 } + fadeOut(motion.fastEffectsSpec())) using
                    SizeTransform(clip = false)
            },
            contentAlignment = Alignment.BottomCenter,
            contentKey = { it?.visuals },
            label = "toast",
        ) { data -> if (data != null) ToastCard(data) }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ToastCard(data: SnackbarData) {
    val scheme = MaterialTheme.colorScheme
    val toast = data.visuals as? ToastVisuals
    val tones = AppTheme.tones
    val kind = toast?.tone ?: ToastTone.Neutral
    val tone = when (kind) {
        ToastTone.Neutral -> tones.accentTone
        ToastTone.Success -> tones.success
        ToastTone.Warning -> tones.warning
        ToastTone.Error -> tones.danger
    }
    val icon = toast?.icon ?: when (kind) {
        ToastTone.Neutral -> Lucide.Info
        ToastTone.Success -> Lucide.Check
        ToastTone.Warning -> Lucide.TriangleAlert
        ToastTone.Error -> Lucide.CircleAlert
    }
    // Swiped far enough sideways it goes; else it springs back.
    val drag = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val spring = MaterialTheme.motionScheme.fastSpatialSpec<Float>()
    // The badge springs in; its ring drains over the toast's lifetime.
    val pop = remember(data) { Animatable(0f) }
    val left = remember(data) { Animatable(1f) }
    LaunchedEffect(data) {
        launch { pop.animateTo(1f, spring) }
        left.animateTo(0f, tween(data.visuals.lifetime, easing = LinearEasing))
    }
    val shape = when (kind) {
        ToastTone.Neutral -> MaterialShapes.Circle
        ToastTone.Success -> MaterialShapes.Cookie9Sided
        ToastTone.Warning -> MaterialShapes.Pentagon
        ToastTone.Error -> MaterialShapes.SoftBurst
    }.toShape()
    Surface(
        Modifier.padding(horizontal = Space.page).widthIn(max = 560.dp).testTag("toast")
            .graphicsLayer {
                translationX = drag.value
                alpha = 1f - (abs(drag.value) / size.width).coerceIn(0f, 1f)
            }
            .pointerInput(data) {
                detectHorizontalDragGestures(
                    onDragEnd = { scope.launch { if (abs(drag.value) > size.width / 3f) data.dismiss() else drag.animateTo(0f, spring) } },
                    onDragCancel = { scope.launch { drag.animateTo(0f, spring) } },
                ) { _, dx -> scope.launch { drag.snapTo(drag.value + dx) } }
            }
            .semantics { liveRegion = LiveRegionMode.Polite },
        shape = MaterialTheme.shapes.extraLargeIncreased,
        color = scheme.surfaceContainerHighest,
        contentColor = scheme.onSurface,
        shadowElevation = 6.dp,
    ) {
        Row(
            Modifier.padding(start = 8.dp, end = if (toast?.action != null) 10.dp else Space.lg, top = 8.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(Space.md),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                Modifier.size(48.dp).drawWithCache {
                    // The badge's outline around it, drawn from its top clockwise for the time left.
                    val width = 2.dp.toPx()
                    val ring = Path().apply { addOutline(shape.createOutline(Size(size.width - width, size.height - width), layoutDirection, this@drawWithCache)) }
                    val measure = PathMeasure().apply { setPath(ring, true) }
                    val length = measure.length
                    val top = (0 until 72).minBy { measure.getPosition(length * it / 72).y } * length / 72
                    val clockwise = measure.getPosition((top + length / 36) % length).x > measure.getPosition(top).x
                    val stroke = Stroke(width, cap = StrokeCap.Round)
                    val rest = Path()
                    onDrawBehind {
                        translate(width / 2, width / 2) {
                            drawPath(ring, tone.base.copy(alpha = 0.18f), style = stroke)
                            val l = length * left.value
                            val from = if (clockwise) top else (top - l + length) % length
                            rest.reset()
                            measure.getSegment(from, minOf(from + l, length), rest)
                            if (from + l > length) measure.getSegment(0f, from + l - length, rest, false)
                            drawPath(rest, tone.base, style = stroke)
                        }
                    }
                },
                Alignment.Center,
            ) {
                Box(
                    Modifier.size(40.dp)
                        .graphicsLayer {
                            scaleX = 0.3f + 0.7f * pop.value
                            scaleY = scaleX
                            rotationZ = -90f * (1f - pop.value)
                        }
                        .background(tone.base, shape),
                    Alignment.Center,
                ) { Icon(icon, null, Modifier.size(20.dp), tint = scheme.surfaceContainerHighest) }
            }
            Column(Modifier.weight(1f, fill = false), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                toast?.let {
                    Text(it.title, Modifier.testTag("toast.title"), style = MaterialTheme.typography.titleMediumEmphasized, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                if (data.visuals.message.isNotBlank()) {
                    Text(
                        data.visuals.message,
                        Modifier.testTag("toast.description"),
                        style = MaterialTheme.typography.bodyMedium,
                        color = scheme.onSurfaceVariant,
                        maxLines = 3,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            toast?.action?.let { (label, run) ->
                FilledTonalButton(
                    { data.dismiss(); run() },
                    Modifier.height(40.dp).testTag("toast.action"),
                    shape = CircleShape,
                    colors = ButtonDefaults.filledTonalButtonColors(containerColor = tone.subtle, contentColor = tone.onSubtle),
                    contentPadding = PaddingValues(horizontal = 18.dp),
                ) { Text(label, style = MaterialTheme.typography.labelLargeEmphasized) }
            }
        }
    }
}
