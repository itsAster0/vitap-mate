package vitapmate.features.profile

import vitapmate.ui.rememberHaptics
import android.content.ClipData
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.launch
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/** A section's title in the primary colour, like Settings'. */
@Composable
internal fun SectionTitle(text: String, modifier: Modifier = Modifier) = Text(
    text,
    modifier.padding(start = Space.lg, top = Space.md, bottom = Space.sm).semantics { heading() },
    style = MaterialTheme.typography.titleSmall,
    color = MaterialTheme.colorScheme.primary,
)

/** A rounded card on the card colour holding [content]. */
@Composable
internal fun Card(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) = Column(
    modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(AppTheme.colors.card).padding(horizontal = Space.lg, vertical = Space.sm),
    verticalArrangement = Arrangement.spacedBy(Space.xs),
    content = content,
)

/** Long press copies [value] (with a toast); a tap runs [onTap]. Values are never logged. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun Modifier.copyable(value: String, onTap: (() -> Unit)? = null): Modifier {
    val clipboard = LocalClipboard.current
    val haptics = rememberHaptics()
    val scope = rememberCoroutineScope()
    return combinedClickable(onClick = { onTap?.invoke() }, onLongClick = {
        haptics.longPress()
        scope.launch {
            clipboard.setClipEntry(ClipEntry(ClipData.newPlainText("vitapmate", value)))
            showToast("Copied", value, tone = ToastTone.Success)
        }
    })
}
