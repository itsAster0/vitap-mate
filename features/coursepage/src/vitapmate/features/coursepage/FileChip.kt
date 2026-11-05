package vitapmate.features.coursepage

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.size
import androidx.compose.material3.AssistChip
import androidx.compose.material3.AssistChipDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.LoadingIndicatorDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import vitapmate.ui.AppTheme

/**
 * A VTOP file to open: [icon] and [label], a loading indicator in place of the icon while [busy].
 * [tonal]: a lecture's material on the accent fill ([solid]: the solid accent, for chips on an
 * accent-filled card); else an outlined chip.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun FileChip(icon: ImageVector, label: String, busy: Boolean, onPress: () -> Unit, modifier: Modifier = Modifier, tonal: Boolean = false, solid: Boolean = false) {
    val motion = MaterialTheme.motionScheme
    val tones = AppTheme.tones
    val (fill, onFill) = if (solid) tones.accent to tones.onAccent else tones.accentTone.subtle to tones.accentTone.onSubtle
    AssistChip(
        { if (!busy) onPress() },
        { Text(label, style = MaterialTheme.typography.labelLarge) },
        modifier,
        leadingIcon = {
            AnimatedContent(busy, transitionSpec = { (fadeIn(motion.fastEffectsSpec()) + scaleIn(motion.fastSpatialSpec())) togetherWith fadeOut(motion.fastEffectsSpec()) }, label = "file-chip") { loading ->
                if (loading) LoadingIndicator(Modifier.size(AssistChipDefaults.IconSize), if (solid) onFill else LoadingIndicatorDefaults.indicatorColor)
                else Icon(icon, null, Modifier.size(AssistChipDefaults.IconSize))
            }
        },
        shape = if (tonal) MaterialTheme.shapes.extraLarge else MaterialTheme.shapes.medium,
        colors = if (tonal) AssistChipDefaults.assistChipColors(containerColor = fill, labelColor = onFill, leadingIconContentColor = onFill)
        else AssistChipDefaults.assistChipColors(),
        border = if (tonal) null else AssistChipDefaults.assistChipBorder(enabled = true),
    )
}
