package vitapmate.features.attendance

import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.UserCheck
import vitapmate.ui.Feature

/**
 * The attendance tab: every course against the 75% line; a course opens its sheet with every
 * class. Its detail item [ATTENDANCE_GUIDE] is the guide to the numbers (from Settings).
 */
val attendanceFeature = Feature(
    "attendance",
    "Attendance",
    Lucide.UserCheck,
    detail = { item -> if (item == ATTENDANCE_GUIDE) AttendanceGuide() },
) { AttendanceScreen() }
