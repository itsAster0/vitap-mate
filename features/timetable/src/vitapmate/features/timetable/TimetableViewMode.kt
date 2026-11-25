package vitapmate.features.timetable

import vitapmate.ui.rememberHaptics
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.EnterExitState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Columns3
import com.composables.icons.lucide.ListTodo
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.Settings
import vitapmate.data.TimetableViewMode
import vitapmate.observability.Log
import vitapmate.observability.flow

/** The saved view ([Settings.timetableView]). */
fun timetableViewMode(): TimetableViewMode = Settings.timetableView

/**
 * Top bar action that flips between the agenda and the weekly grid; the icon shows the view
 * you'll switch to.
 */
@Composable
fun TimetableViewToggleButton() {
    val haptics = rememberHaptics()
    val scope = rememberCoroutineScope()
    val current = timetableViewMode()
    val label = when {
        current == TimetableViewMode.Agenda -> "Show weekly timetable"
        else -> "Show agenda"
    }
    IconButton(
        onClick = {
            haptics.tick()
            scope.launch {
                flow("timetable.toggle") { id ->
                    val next = if (current == TimetableViewMode.Agenda) TimetableViewMode.Weekly else TimetableViewMode.Agenda
                    Settings.timetableView = next
                    Log.info("ui.timetable", "view ${next.name.lowercase()}", id, mapOf("view" to next.name.lowercase()))
                }
            }
        },
        modifier = Modifier.testTag("timetable.toggle").semantics { contentDescription = label },
    ) {
        // The new icon turns in from a quarter turn while it fades in.
        AnimatedContent(current, transitionSpec = { fadeIn(tween(200)) togetherWith fadeOut(tween(200)) }, label = "view-icon") { m ->
            val turn by transition.animateFloat({ tween(200) }, label = "turn") { if (it == EnterExitState.Visible) 0f else -90f }
            Icon(
                if (m == TimetableViewMode.Agenda) Lucide.Columns3 else Lucide.ListTodo,
                null,
                Modifier.size(24.dp).graphicsLayer { rotationZ = turn },
            )
        }
    }
}
