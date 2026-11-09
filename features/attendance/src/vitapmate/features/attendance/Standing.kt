package vitapmate.features.attendance

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import kotlin.math.roundToInt
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/** Where a course stands against 75%; Rust works out `safe`, `canSkip` and `mustAttend` from VTOP's percentage. */
internal enum class Standing(val label: String) {
    NoClasses("No classes yet"),
    Debarred("Debarred"),
    Below("Below 75%"),
    Edge("On the line"),
    Safe("Safe"),
}

internal val AttendanceCourse.standing: Standing
    get() = when {
        debarred -> Standing.Debarred
        total == 0 -> Standing.NoClasses
        !safe -> Standing.Below
        canSkip == 0 -> Standing.Edge
        else -> Standing.Safe
    }

@Composable
internal fun Standing.tone(): Tone = AppTheme.tones.let {
    when (this) {
        Standing.NoClasses -> MaterialTheme.colorScheme.run { Tone(outline, surfaceContainerHigh, onSurfaceVariant) }
        Standing.Debarred, Standing.Below -> it.danger
        Standing.Edge -> it.warning
        Standing.Safe -> it.success
    }
}

internal val AttendanceCourse.isLab get() = kind == ClassKind.LAB

/** VTOP's percentage as it shows it: "79%", "–" before the first class. */
internal val AttendanceCourse.percentLabel get() = if (total == 0) "–" else "${percent.roundToInt()}%"

/**
 * Attended and held, as VTOP's percentage counts them (`counted` leaves out classes not posted
 * yet); labs in two-slot sessions, which VTOP counts twice (an odd total rounds up, an odd
 * attended count down), like Rust's standing.
 */
internal val AttendanceCourse.sessions: Pair<Int, Int>
    get() = if (isLab) attended / 2 to (counted + 1) / 2 else attended to counted

/**
 * The short advice, or null when there is none to give. "Can skip" and "Don't skip" show only
 * with [showSkip] (Settings → skip advice); "Attend N" always shows.
 */
internal fun AttendanceCourse.advice(showSkip: Boolean): String? = when {
    total == 0 -> null
    !safe -> "Attend $mustAttend${labs(mustAttend)}"
    !showSkip -> null
    canSkip == 0 -> "Don't skip"
    else -> "Can skip $canSkip${labs(canSkip)}"
}

/** " lab" / " labs" after a count for labs, which count in sessions; nothing for theory. */
internal fun AttendanceCourse.labs(n: Int) = if (!isLab) "" else if (n == 1) " lab" else " labs"

/**
 * One line for a debarment, which holds for one exam: "Debarred for CAT-2 · Permitted · Clear
 * for FAT", or "… · Clear for FAT at 3 more" while below 75%. Null when not debarred.
 */
internal fun AttendanceCourse.debarLine(): String? {
    val debar = debar ?: return null
    return listOfNotNull(
        if (debar.exam.isBlank()) "Debarred" else "Debarred for ${debar.exam}",
        "Permitted".takeIf { debar.permitted },
        debar.nextExam?.let { next -> if (safe) "Clear for $next" else "Clear for $next at $mustAttend${labs(mustAttend)} more" },
    ).joinToString(" · ")
}

/**
 * The percentage VTOP would show (it rounds up: 31 of 34 is 92%) after attending [attend] and
 * missing [skip] more classes (sessions for labs), with [od] absences turned to present by on-duty
 * approvals still to come, and whether the exact figure is still at 75% or above.
 */
internal fun AttendanceCourse.projected(attend: Int, skip: Int, od: Int = 0): Pair<Int, Boolean> {
    val (a, t) = sessions
    val total = t + attend + skip
    if (total == 0) return 0 to true
    val attended = a + attend + od
    return (attended * 100 + total - 1) / total to (attended * 4 >= total * 3)
}
