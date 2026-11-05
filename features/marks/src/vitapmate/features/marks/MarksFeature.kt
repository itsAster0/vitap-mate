package vitapmate.features.marks

import com.composables.icons.lucide.ChartColumn
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** The marks tab: each course's weighted total so far; a card expands to every assessment. */
val marksFeature = Feature("marks", "Marks", Lucide.ChartColumn) { MarksScreen() }
