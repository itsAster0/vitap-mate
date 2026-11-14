package vitapmate

import androidx.activity.compose.LocalActivity
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonSize
import vitapmate.ui.NavBarSpace
import vitapmate.ui.ProgressBar
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * [PlayUpdate.pill] floating just above the nav bar while it shows, following it as it slides (as
 * the OTP pill): "Update available" with Update and close, the download's progress, then "Update
 * ready" with Restart.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatePillHost() {
    val bar by remember { derivedStateOf { NavBarSpace.px > 0 } }
    val insets = WindowInsets.navigationBars
    val motion = MaterialTheme.motionScheme
    AnimatedContent(
        PlayUpdate.pill.takeIf { bar },
        Modifier.fillMaxSize().wrapContentSize(Alignment.BottomCenter).navigationBarsPadding().padding(bottom = Space.lg).offset {
            val rest = insets.getBottom(this) + Space.lg.roundToPx()
            IntOffset(0, -(NavBarSpace.px + Space.md.roundToPx() - rest).coerceAtLeast(0))
        },
        transitionSpec = { (scaleIn(motion.fastSpatialSpec(), 0.9f) + fadeIn()) togetherWith (scaleOut(targetScale = 0.9f) + fadeOut()) using SizeTransform(clip = false) },
        contentAlignment = Alignment.BottomCenter,
        contentKey = { it?.let { p -> p::class } },
        label = "update",
    ) { p -> if (p != null) Pill(p) }
}

@Composable
private fun Pill(p: UpdatePill) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val activity = LocalActivity.current
    val tone = if (p == UpdatePill.Ready) tones.success else tones.accentTone
    Surface(
        Modifier.padding(horizontal = Space.page).widthIn(max = 560.dp).fillMaxWidth().testTag("update.pill"),
        shape = CircleShape,
        color = scheme.surfaceContainerHigh,
        shadowElevation = 6.dp,
    ) {
        Row(Modifier.padding(start = 8.dp, end = 6.dp, top = 6.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(34.dp).background(tone.subtle, CircleShape), Alignment.Center) {
                Icon(if (p == UpdatePill.Ready) Lucide.Check else Lucide.Download, null, Modifier.size(17.dp), tint = tone.onSubtle)
            }
            Column(Modifier.weight(1f).padding(horizontal = 10.dp)) {
                val title = when (p) {
                    UpdatePill.Available -> "Update available"
                    is UpdatePill.Downloading -> "Downloading update"
                    UpdatePill.Ready -> "Update ready"
                }
                Text(title, style = MaterialTheme.typography.labelLargeEmphasized)
                when (p) {
                    is UpdatePill.Downloading -> ProgressBar(p.fraction, tones.accent, Modifier.padding(top = 5.dp), height = 4.dp)
                    UpdatePill.Ready -> Text("Restart to use the new version.", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                    UpdatePill.Available -> {}
                }
            }
            when (p) {
                is UpdatePill.Downloading -> Text(
                    "${(p.fraction * 100).toInt()}%",
                    Modifier.padding(horizontal = 12.dp).testTag("update.progress"),
                    style = MaterialTheme.typography.labelMedium.nums(),
                    color = scheme.onSurfaceVariant,
                )
                UpdatePill.Available -> Button({ activity?.let(PlayUpdate::update) }, Modifier.testTag("update.start"), size = ButtonSize.Sm) { Text("Update") }
                UpdatePill.Ready -> Button(PlayUpdate::restart, Modifier.testTag("update.restart"), size = ButtonSize.Sm) { Text("Restart") }
            }
            if (p !is UpdatePill.Downloading) {
                IconButton(PlayUpdate::dismiss, Modifier.size(40.dp).testTag("update.close")) {
                    Icon(Lucide.X, "Close", Modifier.size(16.dp), tint = scheme.onSurfaceVariant)
                }
            }
        }
    }
}
