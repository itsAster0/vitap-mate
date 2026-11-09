package vitapmate.ui

import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow

/**
 * A pager over [pages] that opens on [selected]; a swipe that settles on another page calls
 * [onSwipe] with it (never with one page only). Reads its arguments through rememberUpdatedState,
 * so a recreated state is never written stale.
 */
@Composable
fun <T> rememberSyncedPager(pages: List<T>, selected: T, onSwipe: suspend (T) -> Unit): PagerState {
    val pager = rememberPagerState(pages.indexOf(selected).coerceAtLeast(0)) { pages.size }
    val current by rememberUpdatedState(selected)
    val shown by rememberUpdatedState(pages)
    val swiped by rememberUpdatedState(onSwipe)
    LaunchedEffect(pager) {
        snapshotFlow { pager.settledPage }.collect { settled ->
            val page = shown.getOrNull(settled) ?: return@collect
            if (page != current && shown.size > 1) swiped(page)
        }
    }
    return pager
}
