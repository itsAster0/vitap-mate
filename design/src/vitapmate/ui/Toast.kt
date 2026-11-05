package vitapmate.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
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
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarData
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarVisuals
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
import com.composables.icons.lucide.CircleAlert
import com.composables.icons.lucide.CircleCheck
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
        delay(if (data.visuals.duration == SnackbarDuration.Long) 8_000 else 4_000)
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
        ToastTone.Success -> Lucide.CircleCheck
        ToastTone.Warning -> Lucide.TriangleAlert
        ToastTone.Error -> Lucide.CircleAlert
    }
    // Swiped far enough sideways it goes; else it springs back.
    val drag = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val spring = MaterialTheme.motionScheme.fastSpatialSpec<Float>()
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
            Modifier.padding(start = Space.md, end = if (toast?.action != null) Space.sm else Space.lg, top = Space.md, bottom = Space.md),
            horizontalArrangement = Arrangement.spacedBy(Space.md),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(36.dp).background(tone.subtle, CircleShape), Alignment.Center) {
                Icon(icon, null, Modifier.size(20.dp), tint = tone.base)
            }
            Column(Modifier.weight(1f, fill = false), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                toast?.let {
                    Text(it.title, Modifier.testTag("toast.title"), style = MaterialTheme.typography.titleSmallEmphasized, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                if (data.visuals.message.isNotBlank()) {
                    Text(
                        data.visuals.message,
                        Modifier.testTag("toast.description"),
                        style = MaterialTheme.typography.bodySmall,
                        color = scheme.onSurfaceVariant,
                        maxLines = 3,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            toast?.action?.let { (label, run) ->
                TextButton({ data.dismiss(); run() }, Modifier.testTag("toast.action")) { Text(label, style = MaterialTheme.typography.labelLargeEmphasized) }
            }
        }
    }
}
