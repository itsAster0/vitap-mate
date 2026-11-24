package vitapmate

import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import vitapmate.core.LiveTopic
import vitapmate.data.Live
import vitapmate.ui.ProblemBanner

/** Over a screen's loaded data while [topic]'s live watch fails: it may be out of date. */
@Composable
fun LiveProblemBanner(topic: LiveTopic, onRetry: () -> Unit) {
    val problem by remember(topic) { Live.problem(topic) }.collectAsState(null)
    var dismissed by remember(problem) { mutableStateOf(false) }
    problem?.takeUnless { dismissed }?.let {
        ProblemBanner(it, { dismissed = true }, Modifier.padding(horizontal = 16.dp, vertical = 8.dp), onRetry = { dismissed = true; onRetry() })
    }
}
