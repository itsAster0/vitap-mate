package vitapmate

import com.composables.icons.lucide.LayoutGrid
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Settings
import com.composables.icons.lucide.SquareActivity
import vitapmate.features.attendance.attendanceFeature
import vitapmate.features.biometric.biometricFeature
import vitapmate.features.calendar.calendarFeature
import vitapmate.features.coursepage.coursePageFeature
import vitapmate.features.docs.docsFeature
import vitapmate.features.exams.examsFeature
import vitapmate.features.grades.gradesFeature
import vitapmate.features.marks.marksFeature
import vitapmate.features.outing.outingFeature
import vitapmate.features.profile.profileFeature
import vitapmate.features.timetable.timetableFeature
import vitapmate.features.vtop.vtopFeature
import vitapmate.ui.Feature

/** More's sections, three features to a row. */
val featureGroups: List<Pair<String, List<Feature>>> = listOf(
    "Classes" to listOf(timetableFeature, attendanceFeature, coursePageFeature),
    "Exams and results" to listOf(examsFeature, marksFeature, gradesFeature),
    "Campus" to listOf(calendarFeature, outingFeature, biometricFeature),
    "Tools" to listOf(docsFeature, profileFeature, vtopFeature),
)

/**
 * Every feature, in the order More and Settings list them. The one place a feature is listed:
 * its id is its route, its settings key and its log src (`ui.<id>`).
 */
val features: List<Feature> = featureGroups.flatMap { it.second }

/** Screens the shell itself provides; they can be tabs too. Their content is drawn by [Shell]. */
const val MORE = "more"
const val SETTINGS = "settings"
const val INSPECTOR = "inspector"

val builtIns: List<Feature> = listOf(
    Feature(MORE, "More", Lucide.LayoutGrid) {},
    Feature(SETTINGS, "Settings", Lucide.Settings) {},
    Feature(INSPECTOR, "Inspector", Lucide.SquareActivity) {},
)

/** What can be a tab: features first, then the built-ins. */
val tabChoices: List<Feature> = features.filterNot { it.fullScreen } + builtIns.filter { it.id != INSPECTOR }

fun featureById(id: String): Feature? = (features + builtIns).firstOrNull { it.id == id }
