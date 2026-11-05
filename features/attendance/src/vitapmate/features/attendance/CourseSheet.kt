package vitapmate.features.attendance

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import vitapmate.ui.LocalContentPadding

/**
 * A course's [CourseDetail] in a modal sheet nine tenths of the window tall, under its [title]
 * and a slim drag handle; dragging it down, a tap outside or back (predictive) calls [onDismiss].
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun CourseSheet(key: String, title: String, onDismiss: () -> Unit) {
    val sheet = rememberBottomSheetState(
        initialValue = SheetValue.Hidden,
        enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded),
    )
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        // A sheet is its own window: its tags need their own resource ids.
        Modifier.semantics { testTagsAsResourceId = true }.testTag("attendance.sheet"),
        sheet,
        containerColor = MaterialTheme.colorScheme.surface,
        // Slimmer than the default handle (22 dp above and below), so the title sits higher.
        dragHandle = {
            Box(Modifier.fillMaxWidth().padding(top = Space.sm, bottom = Space.xs), Alignment.Center) {
                Box(Modifier.size(32.dp, 4.dp).background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f), CircleShape))
            }
        },
    ) {
        // The sheet keeps clear of the gesture bar itself; no nav bar to clear in here.
        CompositionLocalProvider(LocalContentPadding provides PaddingValues()) {
            Column(Modifier.fillMaxWidth().fillMaxHeight(0.9f)) {
                CourseDetail(key, title)
            }
        }
    }
}
