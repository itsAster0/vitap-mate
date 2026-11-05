package vitapmate.features.calendar

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarMark
import vitapmate.ui.Dot
import vitapmate.ui.Space

private val SHOWN = listOf(CalendarMark.HOLIDAY, CalendarMark.EXAM, CalendarMark.LAB_FAT, CalendarMark.NO_CLASSES, CalendarMark.SPECIAL)

/** What each dot colour on the grid means. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun Legend(modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    FlowRow(
        modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(Space.md, Alignment.CenterHorizontally),
        verticalArrangement = Arrangement.spacedBy(Space.xs),
    ) {
        SHOWN.forEach { mark ->
            val tone = markTone(mark) ?: return@forEach
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                Dot(size = 6.dp, color = if (hollow(mark)) Color.Transparent else tone.base, ring = if (hollow(mark)) tone.base else null)
                Text(markLabel(mark), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            }
        }
    }
}
