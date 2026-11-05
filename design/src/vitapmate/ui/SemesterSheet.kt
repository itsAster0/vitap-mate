package vitapmate.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Calendar
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Leaf
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.Snowflake
import com.composables.icons.lucide.Sun
import vitapmate.core.Semester
import vitapmate.data.Problem

/**
 * Picks the semester pages show (Settings → Account → Semester, the season icon in the bar):
 * [SemesterList] in a sheet, with a refresh.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SemesterSheet(
    semesters: List<Semester>,
    selected: String?,
    loading: Boolean,
    problem: Problem?,
    onSelect: (String) -> Unit,
    onRefresh: () -> Unit,
    onDismiss: () -> Unit,
) {
    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.testTag("semester.sheet"), containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
        Row(Modifier.fillMaxWidth().padding(start = 24.dp, end = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("Semester", style = MaterialTheme.typography.titleLarge)
                Text("Pick which semester the app shows.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
                if (loading) LoadingIndicator(Modifier.size(32.dp))
                else IconButton(onRefresh, Modifier.testTag("semester.refresh")) { Icon(Lucide.RefreshCw, "Refresh the list") }
            }
        }
        problem?.let { ProblemBanner(it, onDismiss = {}, Modifier.padding(horizontal = 16.dp, vertical = 8.dp), onRetry = onRefresh) }
        if (semesters.isEmpty() && problem == null && !loading) {
            Text("No semesters yet. Refresh to load them from VTOP.", Modifier.padding(24.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        SemesterList(semesters, selected, onSelect, contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 24.dp))
    }
}

/** VTOP's [semesters] in its order (newest first), grouped by academic year; [selected] is ticked. */
@Composable
fun SemesterList(
    semesters: List<Semester>,
    selected: String?,
    onSelect: (String) -> Unit,
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(0.dp),
) {
    LazyColumn(modifier, contentPadding = contentPadding, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        semesters.groupBy { it.year.ifEmpty { "Other" } }.forEach { (year, items) ->
            item(key = "year.$year") {
                Text(
                    year,
                    Modifier.padding(start = 8.dp, top = 12.dp, bottom = 2.dp),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            items(items, key = { it.id }) { semester ->
                SemesterRow(semester, semester.id == selected) { onSelect(semester.id) }
            }
        }
    }
}

@Composable
private fun SemesterRow(semester: Semester, selected: Boolean, onClick: () -> Unit) {
    val accent = AppTheme.tones.accentTone
    val shape = RoundedCornerShape(16.dp)
    val fill by animateColorAsState(if (selected) accent.subtle else MaterialTheme.colorScheme.surfaceContainerHighest)
    val edge by animateColorAsState(if (selected) accent.base else Color.Transparent)
    Row(
        Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(fill)
            .border(1.5.dp, edge, shape)
            .selectable(selected, role = Role.RadioButton, onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 12.dp)
            .testTag("semester.item.${semester.id}"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            Modifier.size(36.dp).clip(CircleShape).background(if (selected) accent.base else MaterialTheme.colorScheme.surfaceContainerLow),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                seasonIcon(semester.season),
                null,
                Modifier.size(18.dp),
                tint = if (selected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Column(Modifier.weight(1f)) {
            Text(
                semester.season,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Medium,
                color = if (selected) accent.onSubtle else MaterialTheme.colorScheme.onSurface,
            )
            if (semester.note.isNotEmpty()) {
                Text(semester.note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (selected) Icon(Lucide.Check, "Selected", Modifier.size(20.dp), tint = accent.onSubtle)
    }
}

/** Snowflake, leaf or sun for a semester's season; a calendar otherwise. */
fun seasonIcon(season: String): ImageVector = when (season.lowercase()) {
    "fall" -> Lucide.Leaf
    "winter" -> Lucide.Snowflake
    "summer" -> Lucide.Sun
    else -> Lucide.Calendar
}

/** "Fall 2025-26": VTOP's name without "Semester"; the full name when it has another shape. */
fun shortSemesterName(semester: Semester): String =
    if (semester.year.isEmpty()) semester.name else "${semester.season} ${semester.year}"
