package vitapmate.data

/** What happens to a page on a screen ([PageHandle]); [next] is the state after each. */
internal sealed interface PageEvent<out T> {
    /** The saved copy, read before any fetch of the handle starts. */
    data class CacheLoaded<T>(val data: T, val updatedAt: Long) : PageEvent<T>

    data object FetchStarted : PageEvent<Nothing>

    /** The fetch waits for a lane behind [text] ("Waiting for Attendance to finish"), or no longer (null). */
    data class WaitingChanged(val text: String?) : PageEvent<Nothing>

    data class FetchSucceeded<T>(val data: T, val updatedAt: Long) : PageEvent<T>

    data class FetchFailed(val problem: Problem) : PageEvent<Nothing>

    /** Waited off screen past [Tunable.QueueDrop] and never ran ([Dropped]), in [flow]. */
    data class FetchDropped(val flow: String) : PageEvent<Nothing>

    /** Cancelled before it answered (sign-out, an account switch), in [flow]. */
    data class FetchCancelled(val flow: String) : PageEvent<Nothing>
}

/**
 * The page after [event]. A [PageState.Loading] page always has its cache read or a fetch going,
 * so a fetch that ends without an answer and nothing to show fails rather than spinning on.
 * Fresh data always wins; the cache only fills a page that has none.
 */
internal fun <T> PageState<T>.next(event: PageEvent<T>): PageState<T> = when (event) {
    is PageEvent.CacheLoaded -> when (this) {
        // The cache is read before the handle fetches; a copy already shown (fresh) stays.
        is PageState.Loading -> PageState.Ready(event.data, event.updatedAt)
        is PageState.Failed -> PageState.Ready(event.data, event.updatedAt, PageActivity.Stale(problem))
        is PageState.Ready -> this
    }
    PageEvent.FetchStarted -> when (this) {
        is PageState.Ready -> copy(activity = PageActivity.Refreshing())
        is PageState.Loading, is PageState.Failed -> PageState.Loading()
    }
    is PageEvent.WaitingChanged -> when (this) {
        is PageState.Loading -> copy(waiting = event.text)
        is PageState.Ready -> copy(activity = activity.waiting(event.text))
        is PageState.Failed -> this
    }
    is PageEvent.FetchSucceeded -> PageState.Ready(event.data, event.updatedAt)
    is PageEvent.FetchFailed -> when (this) {
        is PageState.Ready -> copy(activity = PageActivity.Stale(event.problem))
        is PageState.Loading, is PageState.Failed -> PageState.Failed(event.problem)
    }
    is PageEvent.FetchDropped -> settled(notLoaded(event.flow))
    is PageEvent.FetchCancelled -> settled(notLoaded(event.flow))
}

/** A fetch ended without an answer: a ready page keeps its data, one with nothing shows [problem]. */
private fun <T> PageState<T>.settled(problem: Problem): PageState<T> = when (this) {
    is PageState.Ready -> copy(activity = activity.settled())
    is PageState.Loading -> PageState.Failed(problem)
    is PageState.Failed -> this
}

/** Not the student's doing (sign-out or an account switch stopped it), so no toast by default. */
private fun notLoaded(flow: String) =
    Problem(ProblemKind.Unexpected, "Not Loaded", "This page stopped loading before it finished. Try again.", flow, retryable = true, userInitiated = false)

/** Whether a fetch of the page runs: [PageState.Loading], or a ready page refreshing. */
internal val PageState<*>.fetching: Boolean
    get() = this is PageState.Loading || (this as? PageState.Ready)?.activity is PageActivity.Refreshing
