package vitapmate.features.exams

import com.composables.icons.lucide.CalendarClock
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** The exams tab: each exam's papers by day, the next one first; finished papers fold away. */
val examsFeature = Feature("exams", "Exams", Lucide.CalendarClock) { ExamsScreen() }
