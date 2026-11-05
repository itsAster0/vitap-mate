package vitapmate.features.timetable

import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** The timetable: agenda (today and the week) or the weekly grid, for the semester picked in Settings. */
val timetableFeature = Feature(
    id = "timetable",
    label = "Timetable",
    icon = Lucide.CalendarDays,
    actions = { TimetableViewToggleButton() },
) { TimetableScreen() }
