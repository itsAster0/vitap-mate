package vitapmate.features.login

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.InputTransformation
import androidx.compose.foundation.text.input.TextFieldDecorator
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.placeCursorAtEnd
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.rememberSyncedTextFieldState

/**
 * Six digit cells over one hidden text field (digits only, at most six; SMS/email OTP autofill).
 * The next empty cell is outlined in primary with a blinking caret while focused; all cells
 * turn error-coloured with [isError]. Done runs [onDone]. [readOnly] while the code is being sent.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun OtpField(
    code: String,
    onChange: (String) -> Unit,
    isError: Boolean,
    onDone: () -> Unit,
    focus: FocusRequester,
    modifier: Modifier = Modifier,
    readOnly: Boolean = false,
) {
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val state = rememberSyncedTextFieldState(code, onChange)
    // The caret always sits at the end.
    LaunchedEffect(state) {
        snapshotFlow { state.selection }.collect { if (it != TextRange(state.text.length)) state.edit { placeCursorAtEnd() } }
    }
    BasicTextField(
        state,
        modifier.focusRequester(focus).testTag("otp.code").semantics { contentType = ContentType.SmsOtpCode },
        readOnly = readOnly,
        inputTransformation = DigitsOnly,
        textStyle = MaterialTheme.typography.bodyLarge.copy(color = Color.Transparent),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword, imeAction = ImeAction.Done),
        onKeyboardAction = { onDone() },
        lineLimits = TextFieldLineLimits.SingleLine,
        interactionSource = interaction,
        cursorBrush = SolidColor(Color.Transparent),
        decorator = TextFieldDecorator { input ->
            Box {
                Box(Modifier.matchParentSize().alpha(0f)) { input() }
                Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                    val text = state.text
                    val current = if (focused && !readOnly) text.length.coerceAtMost(5) else -1
                    for (i in 0 until 6) Cell(text.getOrNull(i), focused = i == current, isError = isError)
                }
            }
        },
    )
}

/** Keeps the digits of an edit (typed, pasted or autofilled), at most six. */
private val DigitsOnly = InputTransformation {
    val digits = asCharSequence().filter(Char::isDigit).take(6)
    if (digits.length != length) replace(0, length, digits)
    placeCursorAtEnd()
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Cell(char: Char?, focused: Boolean, isError: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val shape = MaterialTheme.shapes.medium
    val border = when {
        isError -> scheme.error
        focused -> scheme.primary
        else -> Color.Transparent
    }
    Box(
        Modifier.size(width = 46.dp, height = 56.dp).background(scheme.surfaceContainerHigh, shape).border(2.dp, border, shape),
        Alignment.Center,
    ) {
        when {
            char != null -> Text(char.toString(), style = MaterialTheme.typography.headlineSmallEmphasized.nums())
            focused -> Caret()
        }
    }
}

@Composable
private fun Caret() {
    val on by produceState(true) { while (true) { delay(500); value = !value } }
    Box(Modifier.size(2.dp, 24.dp).alpha(if (on) 1f else 0f).background(MaterialTheme.colorScheme.primary))
}
