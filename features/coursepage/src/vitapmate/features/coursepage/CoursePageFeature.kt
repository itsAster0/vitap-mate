package vitapmate.features.coursepage

import com.composables.icons.lucide.BookOpenText
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/**
 * Course page: this semester's courses by code; a part (theory, lab...) opens as a pushed detail
 * (item: the course page id) with its section's lecture plan and VTOP files.
 */
val coursePageFeature = Feature("coursepage", "Course page", Lucide.BookOpenText, detail = { CourseScreen(it) }) { CoursePageScreen() }
