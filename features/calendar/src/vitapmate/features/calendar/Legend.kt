package vitapmate.features.calendar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarMark
import vitapmate.ui.Space

private val SHOWN = listOf(CalendarMark.HOLIDAY, CalendarMark.EXAM, CalendarMark.LAB_FAT, CalendarMark.NO_CLASSES, CalendarMark.SPECIAL)

/** What each band colour on the grid means: a swatch per mark, outlined only for days off. */
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
                val shape = RoundedCornerShape(5.dp)
                Box(
                    Modifier.size(16.dp, 10.dp)
                        .background(if (hollow(mark)) Color.Transparent else tone.subtle, shape)
                        .border(if (hollow(mark)) 1.5.dp else 1.dp, tone.base, shape),
                )
                Text(markLabel(mark), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            }
        }
    }
}
