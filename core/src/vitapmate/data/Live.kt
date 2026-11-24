package vitapmate.data

import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.conflate
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.onEmpty
import kotlinx.coroutines.flow.runningFold
import kotlinx.coroutines.Dispatchers
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
     * One [Unit] when [topic] is first read and one each time the server's copy changes, while
     * collected (cancelling unsubscribes). Ends at once, empty, when it can't follow (no identity
     * it may use unasked, forwarding off); the caller then reads once. Flow `live.<topic>`.
     */
    fun changes(topic: LiveTopic): Flow<Unit> = callbackFlow {
        val name = "live.${topic.name.lowercase()}"
        val watch = try {
            flow(name) { id ->
                AppCore.await().liveWatch(id, topic, object : LiveListener {
                    override fun onChange() {
                        failing.update { it - topic }
                        trySend(Unit)
                    }

                    override fun onError(code: String, message: String) {
                        // A refused JWT is renewed in Rust and the next change follows; with no
                        // identity left (signed out, this phone removed) the flow ends.
                        Log.warn("data.live", "$name failed: $code", id, mapOf("message" to message))
                        when (code) {
                            "unauthenticated" -> {}
                            "no_identity" -> close()
                            else -> failing.update {
                                it + (topic to Problem(ProblemKind.Unexpected, "Live Updates Stopped", message, id, retryable = true, userInitiated = false))
                            }
                        }
                    }
                })
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // flow() logged it; an empty flow means "read once".
            null
        }
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
 * without), and that first read counts no more, since the screen has it. Followed only while the
 * app is on screen: the socket can then close in the background, and coming back reads it again.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@Composable
fun rememberLive(topic: LiveTopic, loaded: Boolean): State<Int> =
    remember(topic, loaded) {
        Foreground.shownFlow
            .flatMapLatest { shown -> if (shown) Live.changes(topic).onEmpty { emit(Unit) } else emptyFlow() }
            .drop(if (loaded) 1 else 0)
            .runningFold(0) { n, _ -> n + 1 }
    }.collectAsState(0)

/**
 * A poll loop's wait, woken by [topic]'s changes until [stop]: [next] returns at the next change
 * (at most [BACKSTOP_MS] later, so the loop can check its deadline), or after its interval once
 * [topic] can't be followed. While followed, the loop's read is answered from the subscription,
 * so the backstop costs no trip to the server.
 */
class LiveWake(scope: CoroutineScope, topic: LiveTopic) {
    private val woken = Channel<Unit>(Channel.CONFLATED)

    @Volatile
    private var live = true

    private val job = scope.launch {
        Live.changes(topic).collect { woken.trySend(Unit) }
        live = false
        woken.trySend(Unit)
    }

    fun stop() = job.cancel()

    suspend fun next(intervalMs: Long) {
        if (live) withTimeoutOrNull(BACKSTOP_MS) { woken.receive() } else delay(intervalMs)
    }

    private companion object {
        const val BACKSTOP_MS = 30_000L
    }
}
