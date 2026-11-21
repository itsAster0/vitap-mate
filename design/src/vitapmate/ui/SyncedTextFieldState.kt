package vitapmate.ui

import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.foundation.text.input.setTextAndPlaceCursorAtEnd
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow

/**
 * A [TextFieldState] for a caller that keeps the text as a String: the user's edits go to
 * [onValueChange], and a new [value] from the caller (a reset, an autofilled code) replaces the
 * text without coming back through [onValueChange]. A [value] that is only an edit the caller
 * hasn't recomposed with yet is left alone, so fast typing never loses a character. Not saved
 * across recreation (it may hold a password); the caller's [value] fills it again.
 */
@Composable
fun rememberSyncedTextFieldState(value: String, onValueChange: (String) -> Unit): TextFieldState {
    val state = remember { TextFieldState(value) }
    val sync = remember { Sync(value) }
    val change by rememberUpdatedState(onValueChange)
    LaunchedEffect(state) {
        snapshotFlow { state.text.toString() }.collect {
            if (it != sync.held) {
                sync.held = it
                sync.sent += it
                change(it)
            }
        }
    }
    LaunchedEffect(value) {
        when (value) {
            state.text.toString() -> sync.settle(value)
            in sync.sent -> Unit
            else -> {
                sync.settle(value)
                state.setTextAndPlaceCursorAtEnd(value)
            }
        }
    }
    return state
}

/** What the caller holds, and the edits sent to it since it last caught up. */
private class Sync(var held: String) {
    val sent = mutableListOf<String>()

    fun settle(value: String) {
        held = value
        sent.clear()
    }
}
