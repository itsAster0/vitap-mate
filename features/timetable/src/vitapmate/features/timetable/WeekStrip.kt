package vitapmate.features.timetable

import vitapmate.ui.rememberHaptics
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * The week as a day selector: weekday over its date, the selected day one primary pill (its
 * corners grow as it fills), today ringed, a dot per class (up to five) and an amber dot on
 * [examDays]. Class-free days recede unless selected or today. Days are ISO weekdays (1 = Monday).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun WeekStrip(dates: List<Int>, today: Int, selectedDay: Int, onSelect: (Int) -> Unit, classCounts: List<Int>, examDays: Set<Int>, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val haptics = rememberHaptics()
    val effects = MaterialTheme.motionScheme.fastEffectsSpec<Color>()
    Row(modifier.fillMaxWidth().testTag("timetable.week"), horizontalArrangement = Arrangement.SpaceBetween) {
        dates.forEachIndexed { index, date ->
            val day = index + 1
            val selected = selectedDay == day
            val isToday = today == day
            val count = classCounts[index]
            val exam = day in examDays
            // Class-free days recede to the muted role unless selected or today.
            val free = count == 0 && !exam && !selected && !isToday
            // The selected day is one pill behind weekday, date and dots; today is ringed.
            val corner by animateDpAsState(if (selected || isToday) 22.dp else 12.dp, MaterialTheme.motionScheme.fastSpatialSpec())
            val fill by animateColorAsState(if (selected) scheme.primary else Color.Transparent, effects)
            val number by animateColorAsState(
                when {
                    selected -> scheme.onPrimary
                    isToday -> scheme.primary
                    free -> scheme.onSurfaceVariant
                    else -> scheme.onSurface
                },
                effects,
            )
            val label by animateColorAsState(if (selected) scheme.onPrimary else if (isToday) scheme.primary else scheme.onSurfaceVariant, effects)
            val shape = RoundedCornerShape(corner)
            Column(
                Modifier
                    .weight(1f)
                    .padding(horizontal = 2.dp)
                    .clip(shape)
                    .background(fill)
                    .then(if (isToday && !selected) Modifier.border(1.5.dp, scheme.primary, shape) else Modifier)
                    .selectable(selected, role = Role.Tab, onClick = { if (day != selectedDay) haptics.tick(); onSelect(day) })
                    .semantics { contentDescription = "${formatDay(date, "EEEE, d MMMM")}, ${if (count == 0) "no classes" else "$count classes"}" }
                    .testTag("timetable.day.$day")
                    .padding(vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(formatDay(date, "EEE"), style = MaterialTheme.typography.labelMedium, color = label)
                Spacer(Modifier.height(2.dp))
                Text("${dayOfMonth(date)}", style = MaterialTheme.typography.titleMediumEmphasized.nums(), color = number)
                Spacer(Modifier.height(3.dp))
                // One dot per class, so the week's load reads at a glance.
                Row(Modifier.height(4.dp), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                    repeat(count.coerceAtMost(5)) { Box(Modifier.size(4.dp).background(if (selected) label else scheme.onSurfaceVariant, CircleShape)) }
                    if (exam) Box(Modifier.size(4.dp).background(AppTheme.tones.warning.base, CircleShape))
                }
            }
        }
    }
}

/** Day of the month of an epoch day. */
fun dayOfMonth(day: Int): Int = formatDay(day, "d").toInt()
