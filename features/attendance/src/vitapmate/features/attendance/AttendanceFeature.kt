package vitapmate.features.attendance

import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.UserCheck
import vitapmate.ui.Feature

/**
 * The attendance tab: every course against the 75% line; a course opens its sheet with every
 * class. Its detail is [ATTENDANCE_GUIDE] (from Settings) or a course's [CourseDetail] by its key.
 */
val attendanceFeature = Feature(
    "attendance",
    "Attendance",
    Lucide.UserCheck,
    detail = { item -> if (item == ATTENDANCE_GUIDE) AttendanceGuide() else CourseDetail(item) },
) { AttendanceScreen() }
