package vitapmate.ui

import vitapmate.data.AccountState
import vitapmate.data.toasts
import kotlinx.coroutines.flow.drop
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.pulltorefresh.PullToRefreshDefaults
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import kotlinx.coroutines.launch
import vitapmate.data.PageState
import vitapmate.data.ProblemKind

/**
 * Every screen's states in one place. [PageState.Loading]: [loading] (a skeleton). [PageState.Ready]:
 * pull to refresh and [content] ([empty] when [isEmpty]); a failed refresh shows as a toast with
 * Retry (the data stays), and the data's age is in [LocalUpdated] for an [UpdatedFooter].
 * [PageState.Failed] (nothing cached): a full-screen [ProblemView]. A fetch queued behind other
 * pages says so in a line on top (`waiting`). [onRefresh] runs for pulls and
 * retries and handles its own errors (they arrive as a new state).
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun <T> PageScaffold(
    state: PageState<T>,
    onRefresh: suspend () -> Unit,
    isEmpty: (T) -> Boolean,
    empty: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    pullToRefresh: Boolean = true,
    loading: @Composable () -> Unit = { SkeletonList(Modifier.padding(Space.page)) },
    content: @Composable (T) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var busy by remember { mutableStateOf(false) }
    val haptics = rememberHaptics()
    val latest by rememberUpdatedState(state)
    val refresh: () -> Unit = {
        if (!busy) scope.launch {
            busy = true
            try { onRefresh() } finally { busy = false }
            // Pulls and retries only: the student asked, so the result is felt (an OTP prompt is not a failure).
            when (val now = latest) {
                is PageState.Ready -> now.problem.let { if (it == null) haptics.confirm() else if (it.toasts) haptics.reject() }
                is PageState.Failed -> if (now.problem.toasts) haptics.reject()
                is PageState.Loading -> {}
            }
        }
    }
    val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
    AnimatedContent(
        state,
        modifier.fillMaxSize(),
        transitionSpec = { fadeIn(effects) togetherWith fadeOut(effects) },
        contentKey = { it::class },
    ) { s ->
        when (s) {
            is PageState.Loading -> Column(Modifier.fillMaxSize().testTag("page.loading")) {
                Waiting(s.waiting)
                loading()
            }
            is PageState.Failed -> ScrollCentered(Modifier.testTag("page.failed")) { ProblemView(s.problem, refresh, busy = busy) }
            is PageState.Ready -> {
                val ready: @Composable () -> Unit = {
                    // Each new failure once, as a toast: "Sign in" when the account needs it, else Retry.
                    // Only for refreshes the student asked for (unless "Show background errors"); an
                    // OTP is part of signing in, not an error: its prompt handles it (Problem.toasts).
                    val signIn = LocalSignIn.current
                    LaunchedEffect(s.problem) {
                        val p = s.problem ?: return@LaunchedEffect
                        val action = when {
                            p.kind == ProblemKind.SignInNeeded && signIn != null -> (if (AccountState.rejected) "Update password" else "Sign in") to signIn
                            p.retryable -> "Retry" to refresh
                            else -> null
                        }
                        showToast(p, action)
                    }
                    CompositionLocalProvider(LocalUpdated provides (s.updatedAt to s.refreshing)) {
                        Column(Modifier.fillMaxSize()) {
                            Waiting(s.waiting)
                            Box(Modifier.weight(1f)) { if (isEmpty(s.data)) ScrollCentered(content = empty) else content(s.data) }
                        }
                    }
                }
                if (pullToRefresh) {
                    val pull = rememberPullToRefreshState()
                    LaunchedEffect(pull) { snapshotFlow { pull.distanceFraction >= 1f }.drop(1).collect { if (it && !busy) haptics.threshold() } }
                    PullToRefreshBox(
                        busy,
                        refresh,
                        Modifier.fillMaxSize().testTag("page.ready"),
                        pull,
                        indicator = { PullToRefreshDefaults.LoadingIndicator(pull, busy, Modifier.align(Alignment.TopCenter)) },
                    ) { ready() }
                } else {
                    // In a sheet a pull would drag the sheet: the screen offers its own refresh.
                    Box(Modifier.fillMaxSize().testTag("page.ready")) { ready() }
                }
            }
        }
    }
}

/** "Waiting for Attendance to finish" above the page while its fetch is queued ([PageState.Loading.waiting]). */
@Composable
private fun Waiting(text: String?) {
    // The last text stays while the line folds away.
    var last by remember { mutableStateOf(text) }
    if (text != null) last = text
    AnimatedVisibility(text != null, enter = expandVertically() + fadeIn(), exit = shrinkVertically() + fadeOut()) {
        Box(Modifier.fillMaxWidth().padding(vertical = Space.sm), Alignment.Center) { UpdatedLine(0, refreshing = true, busy = last.orEmpty()) }
    }
}

/** [content] centred in the space given, scrollable so pull to refresh reaches it. */
@Composable
private fun ScrollCentered(modifier: Modifier = Modifier, content: @Composable () -> Unit) = BoxWithConstraints(modifier.fillMaxSize()) {
    Box(Modifier.verticalScroll(rememberScrollState()).fillMaxWidth().heightIn(min = maxHeight), Alignment.Center) { content() }
}
