package vitapmate.features.attendance

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minus
import com.composables.icons.lucide.Plus
import vitapmate.ui.nums

/**
 * [label] with tonal − and + buttons around [value], which rolls up or down as it changes
 * (springy). Tagged [tag], its buttons `<tag>.minus` / `<tag>.plus`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Stepper(label: String, value: Int, onChanged: (Int) -> Unit, tag: String, modifier: Modifier = Modifier, min: Int = 0) {
    val slide = MaterialTheme.motionScheme.fastSpatialSpec<IntOffset>()
    val fade = MaterialTheme.motionScheme.fastEffectsSpec<Float>()
    Row(modifier.fillMaxWidth().testTag(tag), verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
        FilledTonalIconButton({ onChanged(value - 1) }, Modifier.testTag("$tag.minus"), enabled = value > min) {
            Icon(Lucide.Minus, "Fewer to $label")
        }
        AnimatedContent(
            value,
            Modifier.width(52.dp),
            transitionSpec = {
                val up = targetState > initialState
                (slideInVertically(slide) { if (up) it else -it } + fadeIn(fade)) togetherWith
                    (slideOutVertically(slide) { if (up) -it else it } + fadeOut(fade)) using SizeTransform(clip = false)
            },
            contentAlignment = Alignment.Center,
            label = "stepper",
        ) {
            Text("$it", style = MaterialTheme.typography.titleLargeEmphasized.nums(), textAlign = TextAlign.Center)
        }
        FilledTonalIconButton({ onChanged(value + 1) }, Modifier.testTag("$tag.plus")) {
            Icon(Lucide.Plus, "More to $label")
        }
    }
}
