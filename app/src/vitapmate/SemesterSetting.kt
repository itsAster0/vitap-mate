package vitapmate

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.observability.flow
import vitapmate.ui.SemesterSheet
import vitapmate.ui.shortSemesterName

/**
 * Settings → Account → Semester, as in the Flutter app: the semester pages show; tapping opens
 * [SemesterSheet].
 */
@Composable
fun SemesterSetting() {
    var open by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        flow("semesters.open") { SemesterChoice.readCache(it) }
        // As the Flutter app's account card: fresh on every visit with Auto Refresh on.
        if (SemesterChoice.semesters == null || Settings.autoRefresh) SemesterChoice.refresh("settings")
    }
    val list = SemesterChoice.semesters
    val hint = when {
        list == null -> if (SemesterChoice.loading) "Loading…" else "Tap to load from VTOP"
        else -> list.items.firstOrNull { it.id == list.selected }?.let(::shortSemesterName) ?: "Not chosen"
    }
    GroupItem("Semester", "settings.semester", Lucide.CalendarRange, hint, onClick = { open = true }, trailing = { Chevron() })
    if (open) SemesterPickerSheet { open = false }
}

/** [SemesterSheet] wired to [SemesterChoice]: picking closes it and reloads every page. */
@Composable
fun SemesterPickerSheet(onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val list = SemesterChoice.semesters
    SemesterSheet(
        semesters = list?.items.orEmpty(),
        selected = list?.selected,
        loading = SemesterChoice.loading,
        problem = SemesterChoice.problem,
        onSelect = { id ->
            SemesterChoice.pick(id)
            onDismiss()
        },
        onRefresh = { scope.launch { SemesterChoice.refresh("pull") } },
        onDismiss = onDismiss,
    )
}
