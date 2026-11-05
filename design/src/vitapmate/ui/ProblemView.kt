package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.LogIn
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RotateCw
import vitapmate.data.AccountState
import vitapmate.data.LocalInspector
import vitapmate.data.Problem
import vitapmate.data.ProblemKind
import androidx.compose.material3.Button as M3Button

/**
 * Full-screen problem (nothing cached): icon, what happened, what to do, one main action
 * ("Sign in again" for [ProblemKind.SignInNeeded], else "Try again" when retryable; [busy] while
 * it runs) and "Details" that opens the Inspector at the problem's flow.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ProblemView(
    problem: Problem,
    onRetry: (() -> Unit)?,
    modifier: Modifier = Modifier,
    onSignIn: (() -> Unit)? = LocalSignIn.current,
    busy: Boolean = false,
) {
    val (icon, tone) = problem.kind.look()
    val inspector = LocalInspector.current
    val signIn = problem.kind == ProblemKind.SignInNeeded && onSignIn != null
    Column(
        modifier.padding(horizontal = Space.xl, vertical = Space.xxl).widthIn(max = 420.dp).testTag("problem.view"),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.size(88.dp).background(tone.subtle, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
            Icon(icon, null, Modifier.size(36.dp), tint = tone.base)
        }
        Spacer(Modifier.height(Space.xl))
        Text(problem.title, style = MaterialTheme.typography.headlineSmallEmphasized, textAlign = TextAlign.Center)
        Spacer(Modifier.height(Space.sm))
        Text(
            problem.message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(Space.xl))
        val action = if (signIn) onSignIn else onRetry?.takeIf { problem.retryable }
        if (action != null) {
            M3Button(action, modifier = Modifier.testTag("problem.action"), enabled = !busy, shapes = ButtonDefaults.shapes()) {
                if (busy) LoadingIndicator(Modifier.size(ButtonDefaults.IconSize))
                else Icon(if (signIn) Lucide.LogIn else Lucide.RotateCw, null, Modifier.size(ButtonDefaults.IconSize))
                Spacer(Modifier.size(ButtonDefaults.IconSpacing))
                Text(if (!signIn) "Try again" else if (AccountState.rejected) "Update password" else "Sign in again")
            }
        }
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(0.dp)) {
            TextButton({ inspector.open(problem.flow) }, Modifier.padding(top = Space.sm).testTag("problem.details")) { Text("Details") }
            Text(
                problem.flow,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            )
        }
    }
}
