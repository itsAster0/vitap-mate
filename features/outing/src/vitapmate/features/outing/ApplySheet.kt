package vitapmate.features.outing

import vitapmate.ui.OpenHaptic
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space

/**
 * An apply sheet: title and the room, the fields scrolling, VTOP's refusal ([failure]) and the
 * Review button (progress while [sending]). While sending it stays open: the request is on its
 * way and its answer shows here.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun ApplySheet(
    title: String,
    subtitle: String?,
    failure: String?,
    sending: Boolean,
    tag: String,
    onReview: () -> Unit,
    onDismiss: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    val busy by rememberUpdatedState(sending)
    val sheet = rememberBottomSheetState(
        initialValue = SheetValue.Hidden,
        enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded),
        confirmValueChange = { it != SheetValue.Hidden || !busy },
    )
    OpenHaptic()
    ModalBottomSheet(
        { if (!busy) onDismiss() },
        // A sheet is its own window: its tags need their own resource ids.
        Modifier.semantics { testTagsAsResourceId = true }.testTag(tag),
        sheet,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        CompositionLocalProvider(LocalContentPadding provides PaddingValues()) {
            Column(Modifier.fillMaxWidth().imePadding()) {
                Column(Modifier.padding(horizontal = Space.xl)) {
                    Text(title, style = MaterialTheme.typography.headlineSmallEmphasized)
                    subtitle?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                }
                Column(
                    Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(horizontal = Space.xl, vertical = Space.lg),
                    content = content,
                )
                Column(Modifier.padding(start = Space.xl, end = Space.xl, bottom = Space.lg)) {
                    AnimatedVisibility(failure != null, enter = fadeIn() + expandVertically(), exit = fadeOut() + shrinkVertically()) {
                        Text(
                            failure.orEmpty(),
                            Modifier.fillMaxWidth().padding(bottom = Space.sm)
                                .background(AppTheme.tones.danger.subtle, MaterialTheme.shapes.medium)
                                .padding(Space.md).testTag("outing.failure"),
                            style = MaterialTheme.typography.bodySmall,
                            color = AppTheme.tones.danger.onSubtle,
                        )
                    }
                    Button(if (sending) null else onReview, Modifier.fillMaxWidth().testTag("outing.review")) {
                        if (sending) LoadingIndicator(Modifier.size(24.dp).testTag("outing.sending")) else Text("Review request")
                    }
                }
            }
        }
    }
}
