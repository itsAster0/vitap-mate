package vitapmate.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
import vitapmate.data.AccountState
import vitapmate.data.LocalInspector
import vitapmate.data.Problem
import vitapmate.data.ProblemKind
import androidx.compose.material3.IconButton as M3IconButton

/**
 * The last refresh failed but cached data is shown: a tinted banner with what happened, "Try
 * again" (or "Sign in again"), "Details" (Inspector at the flow) and a close button.
 */
@Composable
fun ProblemBanner(
    problem: Problem,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    onRetry: (() -> Unit)? = null,
    onSignIn: (() -> Unit)? = LocalSignIn.current,
) {
    val (icon, tone) = problem.kind.look()
    val inspector = LocalInspector.current
    val signIn = onSignIn.takeIf { problem.kind == ProblemKind.SignInNeeded }
    val action = signIn ?: onRetry?.takeIf { problem.retryable }
    Surface(modifier.fillMaxWidth().testTag("problem.banner"), MaterialTheme.shapes.large, tone.subtle, tone.onSubtle) {
        Row(Modifier.padding(start = Space.lg, top = Space.md), verticalAlignment = Alignment.Top) {
            Icon(icon, null, Modifier.padding(top = 2.dp).size(20.dp), tint = tone.base)
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(problem.title, style = MaterialTheme.typography.titleSmallEmphasized)
                Text(problem.message, style = MaterialTheme.typography.bodyMedium, maxLines = 3, overflow = TextOverflow.Ellipsis)
                // Text buttons pad 12 dp; pull them left so their labels line up with the message.
                Row(Modifier.offset(x = (-12).dp).padding(vertical = Space.xs), verticalAlignment = Alignment.CenterVertically) {
                    val pad = ButtonDefaults.TextButtonContentPadding
                    action?.let {
                        TextButton(it, Modifier.testTag("problem.action"), contentPadding = pad) {
                            Text(if (signIn == null) "Try again" else if (AccountState.rejected) "Update password" else "Sign in again", color = tone.onSubtle)
                        }
                    }
                    TextButton({ inspector.open(problem.flow) }, Modifier.testTag("problem.details"), contentPadding = pad) {
                        Text("Details", color = tone.onSubtle.copy(alpha = 0.8f))
                    }
                }
            }
            M3IconButton(onDismiss, Modifier.testTag("problem.dismiss")) { Icon(Lucide.X, "Dismiss", Modifier.size(18.dp)) }
        }
    }
}
