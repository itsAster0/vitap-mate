package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BasicAlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

/**
 * The app's dialog: an optional shaped [icon] badge over a centred [title], the [content] (plain
 * text, [DialogRows], a field), then two joined full-width buttons, [dismiss] and [confirm].
 * [danger] colours the badge and [confirm] as an error (sign out, delete, cancel on VTOP);
 * [extra] is a third, quiet action above the buttons. Tags work as resource ids here: a dialog is
 * its own window.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
fun AppDialog(
    onDismiss: () -> Unit,
    title: String,
    confirm: String,
    onConfirm: () -> Unit,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    danger: Boolean = false,
    confirmEnabled: Boolean = true,
    confirmTag: String = "dialog.confirm",
    dismiss: String = "Cancel",
    dismissTag: String = "dialog.dismiss",
    onDismissButton: () -> Unit = onDismiss,
    extra: (@Composable () -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit = {},
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    BasicAlertDialog(onDismiss, modifier.semantics { testTagsAsResourceId = true }) {
        Surface(Modifier.widthIn(max = 400.dp), MaterialTheme.shapes.extraExtraLarge, scheme.surfaceContainerHigh) {
            Column(Modifier.padding(start = 20.dp, top = 24.dp, end = 20.dp, bottom = 20.dp)) {
                if (icon != null) Box(
                    Modifier.align(Alignment.CenterHorizontally).padding(bottom = 16.dp).size(60.dp).background(
                        if (danger) tones.danger.subtle else tones.accent,
                        (if (danger) MaterialShapes.Sunny else MaterialShapes.Cookie7Sided).toShape(),
                    ),
                    Alignment.Center,
                ) { Icon(icon, null, Modifier.size(26.dp), tint = if (danger) tones.danger.base else tones.onAccent) }
                Text(
                    title,
                    Modifier.fillMaxWidth().semantics { heading() },
                    style = MaterialTheme.typography.headlineSmall,
                    color = scheme.onSurface,
                    textAlign = if (icon != null) TextAlign.Center else TextAlign.Start,
                )
                CompositionLocalProvider(LocalTextStyle provides MaterialTheme.typography.bodyMedium, LocalContentColor provides scheme.onSurfaceVariant) {
                    Column(Modifier.weight(1f, fill = false).padding(top = 12.dp).verticalScroll(rememberScrollState()), content = content)
                }
                extra?.let { Box(Modifier.align(Alignment.CenterHorizontally).padding(top = 8.dp)) { it() } }
                Row(Modifier.padding(top = 20.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    val side = Modifier.weight(1f).heightIn(min = 48.dp)
                    FilledTonalButton(
                        onClick = onDismissButton,
                        modifier = side.testTag(dismissTag),
                        shapes = ButtonDefaults.shapes(RoundedCornerShape(24.dp, 10.dp, 10.dp, 24.dp)),
                        colors = ButtonDefaults.filledTonalButtonColors(scheme.surfaceContainerHighest, scheme.onSurface),
                    ) { Text(dismiss, maxLines = 1) }
                    Button(
                        onClick = onConfirm,
                        modifier = side.testTag(confirmTag),
                        enabled = confirmEnabled,
                        shapes = ButtonDefaults.shapes(RoundedCornerShape(10.dp, 24.dp, 24.dp, 10.dp)),
                        colors = if (danger) ButtonDefaults.buttonColors(scheme.error, scheme.onError) else ButtonDefaults.buttonColors(),
                    ) { Text(confirm, maxLines = 1) }
                }
            }
        }
    }
}

/** Label and value pairs as one joined block inside an [AppDialog]; a blank label gives the value the full row. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun DialogRows(rows: List<Pair<String, String>>, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(modifier.clip(MaterialTheme.shapes.largeIncreased), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
        rows.forEach { (label, value) ->
            Row(
                Modifier.fillMaxWidth().background(scheme.surfaceContainerHighest, MaterialTheme.shapes.extraSmall).padding(horizontal = 14.dp, vertical = 11.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (label.isNotBlank()) Text(label, Modifier.width(72.dp), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                Text(value, style = MaterialTheme.typography.titleSmall, color = scheme.onSurface)
            }
        }
    }
}
