package vitapmate.features.vtop

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.ContainedLoadingIndicator
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import vitapmate.ui.Space

/** While the session is prepared (the Flutter app's `VtopWebviewLoading`); says so after 15 s. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun VtopLoading(reconnecting: Boolean) {
    var slow by remember(reconnecting) { mutableStateOf(false) }
    LaunchedEffect(reconnecting) {
        delay(15_000)
        slow = true
    }
    Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).testTag("vtop.loading"), Alignment.Center) {
        Column(
            Modifier.widthIn(max = 340.dp).padding(Space.xl),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            ContainedLoadingIndicator(Modifier.size(72.dp))
            Text(
                if (reconnecting) "Reconnecting to VTOP" else "Opening VTOP",
                style = MaterialTheme.typography.titleMediumEmphasized,
                modifier = Modifier.padding(top = Space.md),
            )
            Text(
                if (slow) "VTOP is taking longer than usual. Please keep waiting." else "Preparing your login session securely.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}
