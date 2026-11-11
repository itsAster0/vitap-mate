package vitapmate.ui

import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.TextFieldColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import com.composables.icons.lucide.Eye
import com.composables.icons.lucide.EyeOff
import com.composables.icons.lucide.Lucide

/**
 * A password [OutlinedTextField] with a show/hide eye as its trailing icon (toggle tagged
 * `[tag].toggle`). Hidden by default, and hidden again whenever the field leaves the screen.
 */
@Composable
fun PasswordField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    tag: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    isError: Boolean = false,
    leadingIcon: @Composable (() -> Unit)? = null,
    supportingText: @Composable (() -> Unit)? = null,
    imeAction: ImeAction = ImeAction.Done,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    shape: Shape = OutlinedTextFieldDefaults.shape,
    initiallyShown: Boolean = false,
    colors: TextFieldColors = OutlinedTextFieldDefaults.colors(),
) {
    var shown by remember(initiallyShown) { mutableStateOf(initiallyShown) }
    OutlinedTextField(
        value,
        onValueChange,
        modifier.testTag(tag).semantics { contentType = ContentType.Password },
        enabled = enabled,
        label = { androidx.compose.material3.Text(label) },
        leadingIcon = leadingIcon,
        trailingIcon = {
            IconButton({ shown = !shown }, Modifier.testTag("$tag.toggle")) {
                Icon(if (shown) Lucide.EyeOff else Lucide.Eye, if (shown) "Hide password" else "Show password")
            }
        },
        supportingText = supportingText,
        isError = isError,
        visualTransformation = if (shown) VisualTransformation.None else PasswordVisualTransformation(),
        keyboardOptions = KeyboardOptions(autoCorrectEnabled = false, keyboardType = KeyboardType.Password, imeAction = imeAction),
        keyboardActions = keyboardActions,
        singleLine = true,
        shape = shape,
        colors = colors,
    )
}
