package vitapmate.features.grades

import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** Grades: a semester's grades and GPA, the grade history with CGPA, and the GPA planner. */
val gradesFeature = Feature("grades", "Grades", Lucide.GraduationCap, pdf = ::gradesPdf) { GradesScreen() }
