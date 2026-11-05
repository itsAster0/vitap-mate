package vitapmate.ui

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

/** Emits when the user taps the nav bar tab that is already showing. */
val LocalTabReselect = staticCompositionLocalOf<Flow<Unit>> { emptyFlow() }

/** Scrolls [state] to the top when the current tab is tapped again. */
@Composable
fun ScrollToTopOnReselect(state: LazyListState) {
    val reselect = LocalTabReselect.current
    LaunchedEffect(reselect, state) { reselect.collect { state.animateScrollToItem(0) } }
}

/** Scrolls [state] to the top when the current tab is tapped again. */
@Composable
fun ScrollToTopOnReselect(state: ScrollState) {
    val reselect = LocalTabReselect.current
    LaunchedEffect(reselect, state) { reselect.collect { state.animateScrollTo(0) } }
}
