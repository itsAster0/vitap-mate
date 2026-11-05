package vitapmate.features.calendar

import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** The academic calendar: the semester month by month, with its holidays, exams and Lab FATs. */
val calendarFeature = Feature("calendar", "Calendar", Lucide.CalendarDays) { CalendarScreen() }
