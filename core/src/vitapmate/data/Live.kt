package vitapmate.data

import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.conflate
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.onEmpty
import kotlinx.coroutines.flow.runningFold
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.LiveError
import vitapmate.core.LiveListener
import vitapmate.core.LiveTopic
import vitapmate.observability.Log
import vitapmate.observability.flow

/** The server's live queries (Rust's `Core::live_watch`), as flows. */
object Live {
    /** Each followed topic's failing watch, until its next change or until it stops. */
    private val failing = MutableStateFlow<Map<LiveTopic, Problem>>(emptyMap())

    /** Why [topic]'s watch stopped answering, while it does; null while it follows. */
    fun problem(topic: LiveTopic): Flow<Problem?> = failing.map { it[topic] }

    /**
     * Emits Rust's change count ([vitapmate.core.Core.liveChanges]) when [topic] is first read and
     * each time the server's copy changes, while collected (cancelling unsubscribes). Rust follows
     * only while `app_shown` is true, pausing in the background and re-subscribing on resume.
     * Ends at once, empty, when it can't follow (no identity it may use unasked, forwarding off);
     * the caller then reads once. Flow `live.<topic>`.
     */
    fun changes(topic: LiveTopic): Flow<Int> = callbackFlow {
        val name = "live.${topic.name.lowercase()}"
        val watch = attempt {
            flow(name) { id ->
                val core = AppCore.await()
                core.liveWatch(id, topic, object : LiveListener {
                    override fun onChange() {
                        failing.update { it - topic }
                        trySend(core.liveChanges(id, topic).toInt())
                    }

                    override fun onError(error: LiveError) {
                        // Rust's log has the server's code; Transient is a refused JWT renewed in
                        // Rust (the next change follows) or our own fault the student can't act on.
                        Log.warn("data.live", "$name failed: ${error::class.simpleName}", id)
                        when (error) {
                            LiveError.Transient -> {}
                            is LiveError.NoIdentity -> close()
                            is LiveError.Stopped -> failing.update {
                                it + (topic to Problem(ProblemKind.UNEXPECTED, "Live Updates Stopped", error.message, id, retryable = true, userInitiated = false))
                            }
                        }
                    }
                })
            }
        }.getOrNull() // flow() logged a failure; an empty flow means "read once".
        if (watch == null) close() else awaitClose {
            watch.stop()
            failing.update { it - topic }
        }
    }.conflate().flowOn(Dispatchers.IO)
}

/**
 * How many times [topic] changed while on screen, starting at 1 once it is first read (or at once
 * when it can't be followed): `LaunchedEffect(change) { if (change > 0) load() }`. Once [loaded]
 * turns true it is followed again (that load may have proved the account it couldn't be followed
 * without), and that first read counts no more, since the screen has it. Rust pauses and resumes
 * the watch with `Core.appShown`, so coming back to the app reads it again.
 */
@Composable
fun rememberLive(topic: LiveTopic, loaded: Boolean): State<Int> =
    remember(topic, loaded) {
        Live.changes(topic)
            .onEmpty { emit(1) }
            .drop(if (loaded) 1 else 0)
            .runningFold(0) { n, _ -> n + 1 }
    }.collectAsState(0)

/**
 * A poll loop's wait, woken by [topic]'s changes until [stop]: [next] returns at the next change
 * (at most [BACKSTOP_MS] later, so the loop can check its deadline), or after its interval once
 * [topic] can't be followed (or while off screen, when Rust pauses live watches).
 */
class LiveWake(scope: CoroutineScope, topic: LiveTopic) {
    private val woken = Channel<Int>(Channel.CONFLATED)

    private val job = scope.launch {
        Live.changes(topic).collect { woken.trySend(it) }
        woken.close()
    }

    fun stop() {
        job.cancel()
        woken.close()
    }

    suspend fun next(intervalMs: Long) {
        if (!Foreground.shown) {
            delay(intervalMs)
            return
        }
        val got = withTimeoutOrNull(BACKSTOP_MS) { woken.receiveCatching() }
        if (got != null && got.isClosed) delay(intervalMs)
    }

    private companion object {
        const val BACKSTOP_MS = 30_000L
    }
}
