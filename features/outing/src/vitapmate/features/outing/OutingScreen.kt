package vitapmate.features.outing

import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.MapPin
import com.composables.icons.lucide.Lucide
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.launch
import vitapmate.core.OutingKind
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.Space

/**
 * Outing: General and Weekend (connected buttons), each its own page (`outing` / item, log src
 * `ui.outing`): the saved copy first, then VTOP. Each shows its rules panel with New request,
 * then the requests, newest first.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun OutingScreen() {
    var tab by rememberSaveable { mutableStateOf(OutingKind.GENERAL) }
    val scope = rememberCoroutineScope()
    Column(Modifier.fillMaxSize()) {
        ConnectedChoice(
            listOf(OutingKind.GENERAL, OutingKind.WEEKEND),
            tab,
            onSelect = { kind ->
                tab = kind
                scope.logAction("outing.tab", "ui.outing", "tab ${kind.key}", mapOf("tab" to kind.key))
            },
            tag = "outing.tab",
            key = { it.key },
            label = { if (it == OutingKind.GENERAL) "General" else "Weekend" },
            icon = { if (it == OutingKind.GENERAL) Lucide.MapPin else Lucide.CalendarDays },
            labelOnSelected = true,
            modifier = Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.sm),
        )
        val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
        AnimatedContent(tab, Modifier.weight(1f), transitionSpec = { fadeIn(effects) togetherWith fadeOut(effects) }, label = "outing-tab") { kind ->
            OutingList(kind)
        }
    }
}

/** "general" / "weekend": test tags, page items, pass details. */
internal val OutingKind.key get() = name.lowercase()
