package vitapmate.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import kotlinx.coroutines.delay

private const val PROVING_AFTER_MS = 2_000L

/**
 * What a call to our server is likely doing, by time alone: Rust reports no progress for the
 * account proof, so the text only moves to the slow step after a while and claims no percentage.
 */
@Composable
fun proofStage(): String {
    var proving by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        delay(PROVING_AFTER_MS)
        proving = true
    }
    return if (proving) "Proving your account to the server… (takes a few seconds)" else "Checking your VTOP sign-in…"
}

/** A loading indicator over [proofStage]'s text, for a page waiting on the server. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ProofProgress(modifier: Modifier = Modifier) {
    Column(
        modifier.fillMaxWidth().padding(Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.md),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        LoadingIndicator()
        Text(proofStage(), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
    }
}
