package vitapmate.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue

/**
 * How far the floating nav bar's top edge is above the window's bottom, in px, as it slides with
 * scrolling and in and out with pushes; 0 when no bar is shown. The shell keeps it current, so
 * things floating over every screen (the OTP pill) can sit above the bar. Read it in a layout or
 * offset lambda so following the bar only relayouts.
 */
object NavBarSpace {
    var px by mutableIntStateOf(0)
}
