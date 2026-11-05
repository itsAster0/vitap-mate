package vitapmate.features.coursepage

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import vitapmate.core.CourseSection
import vitapmate.ui.Space

/** Every section of the course in a modal sheet; picking one shows its plan. */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun SectionSheet(sections: List<CourseSection>, selected: String?, onPick: (String) -> Unit, onDismiss: () -> Unit) {
    val sheet = rememberBottomSheetState(
        initialValue = SheetValue.Hidden,
        enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded),
    )
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        // A sheet is its own window: its tags need their own resource ids.
        Modifier.semantics { testTagsAsResourceId = true }.testTag("coursepage.sheet"),
        sheet,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        LazyColumn(
            Modifier.fillMaxWidth(),
            contentPadding = PaddingValues(start = Space.lg, end = Space.lg, bottom = Space.lg),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            item("title") { Text("Sections", style = MaterialTheme.typography.headlineSmallEmphasized) }
            items(sections, key = { it.classId }) { section ->
                SectionRow(section, section.classId == selected, { onPick(section.classId) })
            }
        }
    }
}
