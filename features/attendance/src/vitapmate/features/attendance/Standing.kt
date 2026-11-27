package vitapmate.features.attendance

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import kotlin.math.floor
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

/** VTOP's verbatim percentage (never computed when VTOP omits it): "79%", else "–". */
internal val AttendanceCourse.reportedPercentLabel get() = reportedPercent?.let { "${it.roundToInt()}%" } ?: "–"

/** VTOP's verbatim debar status text, or null when blank / "-". */
internal val AttendanceCourse.verbatimDebar get() = rawDebarStatus.ifBlank { null }

/**
 * The short advice, or null when there is none to give. "Can skip" and "Don't skip" show only
 * with [showSkip] (Settings → skip advice); "Attend N" always shows.
 */
internal fun AttendanceCourse.advice(showSkip: Boolean): String? = when {
    total == 0 -> null
    !safe -> advice
    !showSkip -> null
    else -> advice
}

/** " lab" / " labs" after a count for labs, which count in sessions; nothing for theory. */
internal fun AttendanceCourse.labs(n: Int) = if (!isLab) "" else if (n == 1) " lab" else " labs"

/** [percent] to one decimal, cut rather than rounded so 74.96 never reads 75: "74.9", "75". */
internal fun percentText(percent: Double): String {
    val tenths = floor(percent * 10 + 1e-6).toLong()
    return if (tenths % 10 == 0L) "${tenths / 10}" else "${tenths / 10}.${tenths % 10}"
}
