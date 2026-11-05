package vitapmate.ui

import android.os.Build
import android.view.HapticFeedbackConstants as H
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView
import vitapmate.data.Settings

/**
 * M3 Expressive haptics on the view's [View.performHapticFeedback] (so the system's touch feedback
 * setting applies), each falling back to an older constant; nothing with Settings → Haptics off.
 */
@Stable
class Haptics(private val view: View) {
    /** A tab, segment, chip or day picked. */
    fun tick() = play(if (Build.VERSION.SDK_INT >= 34) H.SEGMENT_TICK else H.CLOCK_TICK)

    fun toggle(on: Boolean) = play(if (Build.VERSION.SDK_INT >= 34) (if (on) H.TOGGLE_ON else H.TOGGLE_OFF) else H.CLOCK_TICK)

    /** Something worked: a refresh, a save, an accepted code. */
    fun confirm() = play(if (Build.VERSION.SDK_INT >= 30) H.CONFIRM else H.CONTEXT_CLICK)

    /** Something the student asked for failed. */
    fun reject() = play(if (Build.VERSION.SDK_INT >= 30) H.REJECT else H.LONG_PRESS)

    /** A drag crossed its threshold (pull to refresh). */
    fun threshold() = play(if (Build.VERSION.SDK_INT >= 34) H.GESTURE_THRESHOLD_ACTIVATE else H.CONTEXT_CLICK)

    fun longPress() = play(H.LONG_PRESS)

    /** A sheet or dialog opened. */
    fun open() = play(H.CONTEXT_CLICK)

    private fun play(constant: Int) {
        if (Settings.haptics) view.performHapticFeedback(constant)
    }
}

@Composable
fun rememberHaptics(): Haptics {
    val view = LocalView.current
    return remember(view) { Haptics(view) }
}

/** One [Haptics.open] when the caller (a sheet) enters composition. */
@Composable
fun OpenHaptic() {
    val haptics = rememberHaptics()
    LaunchedEffect(Unit) { haptics.open() }
}
