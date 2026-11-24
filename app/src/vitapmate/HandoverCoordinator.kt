package vitapmate

import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

/** Per-request lifecycle for a connected app's VTOP session handover (`vtop_cookie_request`). */
sealed interface HandoverState {
    val expiresAtMs: Long

    /** Notification posted while backgrounded, waiting for a tap or foreground promotion. */
    data class Notified(override val expiresAtMs: Long) : HandoverState

    /** Actively verifying, prompting for consent, or sending the session answer. */
    data class Answering(override val expiresAtMs: Long, val attemptId: Long) : HandoverState

    /** Terminal state: answered, timed out, or expired. */
    data class Done(override val expiresAtMs: Long) : HandoverState
}

/** Side effect chosen by [HandoverCoordinator.onRequest]. */
sealed interface HandoverAction {
    /** Background arrival: post a notification using [notificationId]. */
    data class Notify(val notificationId: Int, val expiresAtMs: Long) : HandoverAction

    /** Foreground arrival, tap, or promotion: cancel [notificationId] and answer with [attemptId]. */
    data class Answer(val attemptId: Long, val notificationId: Int, val expiresAtMs: Long) : HandoverAction

    /** Already past expiry: drop cleanly and cancel any lingering notification [notificationId]. */
    data class Expired(val notificationId: Int) : HandoverAction

    /** Already notified (while still backgrounded), answering, or done: do nothing. */
    data object Ignore : HandoverAction
}

/** One [HandoverState.Notified] request promoted to [HandoverState.Answering] on foreground entry. */
data class PromotedRequest(
    val requestId: String,
    val attemptId: Long,
    val notificationId: Int,
    val expiresAtMs: Long,
)

/** Result of [HandoverCoordinator.promoteNotified]: requests to answer and expired notifications to clear. */
data class Promotion(
    val toAnswer: List<PromotedRequest>,
    val expiredNotificationIds: List<Int>,
)

/**
 * Coordinates connected-app session handover requests across FCM, live `sessions:pending` updates,
 * foreground transitions, and notification taps. Tracks bounded per-request state, filters expired
 * requests via [clock], derives unique notification IDs, and guarantees at most one terminal
 * `sessions:answer` call per request when racing completion against timeout.
 */
class HandoverCoordinator(
    private val clock: () -> Long = System::currentTimeMillis,
    private val defaultTtlMs: Long = DEFAULT_REQUEST_TTL_MS,
    private val pruneGraceMs: Long = DEFAULT_PRUNE_GRACE_MS,
) {
    private val lock = Any()
    private val entries = HashMap<String, HandoverState>()
    private val jobs = HashMap<String, ActiveJob>()
    private var nextAttemptId = 0L

    private data class ActiveJob(val attemptId: Long, val job: Job)

    /**
     * Records a request arriving from FCM, `sessions:pending`, or a notification tap.
     * Drops already-expired requests, avoids duplicate notifications or concurrent answers, and
     * promotes [HandoverState.Notified] entries when [foreground] or [tapped].
     */
    fun onRequest(
        requestId: String,
        expiresAtMs: Long?,
        foreground: Boolean,
        tapped: Boolean = false,
    ): HandoverAction = synchronized(lock) {
        val now = clock()
        // Capture prior state before pruning so an expired Notified entry keeps its deadline
        // even if a late tap arrives without an expiresAt extra.
        val previous = entries[requestId]
        pruneLocked(now)
        val current = entries[requestId]
        val nid = notificationId(requestId)
        val expiry = expiresAtMs ?: previous?.expiresAtMs ?: (now + defaultTtlMs)

        if (previous is HandoverState.Notified && current is HandoverState.Done) {
            return HandoverAction.Expired(nid)
        }

        when (current) {
            is HandoverState.Done, is HandoverState.Answering -> HandoverAction.Ignore
            is HandoverState.Notified -> when {
                expiry <= now -> {
                    markExpiredLocked(requestId, expiry, now)
                    HandoverAction.Expired(nid)
                }
                foreground || tapped -> startAnsweringLocked(requestId, expiry, nid)
                else -> {
                    if (expiry < current.expiresAtMs) entries[requestId] = HandoverState.Notified(expiry)
                    HandoverAction.Ignore
                }
            }
            null -> when {
                expiry <= now -> HandoverAction.Expired(nid)
                foreground || tapped -> startAnsweringLocked(requestId, expiry, nid)
                else -> {
                    entries[requestId] = HandoverState.Notified(expiry)
                    HandoverAction.Notify(nid, expiry)
                }
            }
        }
    }

    /**
     * Promotes every non-expired [HandoverState.Notified] request to [HandoverState.Answering]
     * when the app enters the foreground, and returns expired notification IDs to cancel.
     */
    fun promoteNotified(): Promotion = synchronized(lock) {
        val now = clock()
        val expiredNotifications = pruneLocked(now)
        val promoted = mutableListOf<PromotedRequest>()
        val iterator = entries.entries.iterator()
        while (iterator.hasNext()) {
            val entry = iterator.next()
            val state = entry.value
            if (state is HandoverState.Notified) {
                val attemptId = ++nextAttemptId
                val nid = notificationId(entry.key)
                entry.setValue(HandoverState.Answering(state.expiresAtMs, attemptId))
                promoted += PromotedRequest(entry.key, attemptId, nid, state.expiresAtMs)
            }
        }
        Promotion(promoted, expiredNotifications)
    }

    /**
     * Associates the cancellable answering [job] with an active [HandoverState.Answering] attempt.
     * Cancels [job] immediately and returns false if the attempt is no longer active.
     */
    fun attachJob(requestId: String, attemptId: Long, job: Job): Boolean {
        val attached = synchronized(lock) {
            val current = entries[requestId]
            if (current is HandoverState.Answering && current.attemptId == attemptId) {
                jobs[requestId] = ActiveJob(attemptId, job)
                true
            } else {
                false
            }
        }
        if (!attached) job.cancel()
        return attached
    }

    /** Transitions [HandoverState.Answering] to [HandoverState.Done] if [attemptId] expired before starting. */
    fun expireIfDue(requestId: String, attemptId: Long): Boolean = synchronized(lock) {
        val now = clock()
        val current = entries[requestId]
        if (current !is HandoverState.Answering || current.attemptId != attemptId) return false
        if (now < current.expiresAtMs) return false
        finishLocked(requestId, attemptId, now)
        true
    }

    /**
     * Claims the terminal transition from [HandoverState.Answering] to [HandoverState.Done] after
     * an answer call returns. Returns true only for the single winning terminal transition.
     */
    fun complete(requestId: String, attemptId: Long): Boolean = synchronized(lock) {
        finishLocked(requestId, attemptId, clock())
    }

    /**
     * Claims the terminal transition from [HandoverState.Answering] to [HandoverState.Done] on
     * timeout and cancels any attached answering job. Returns true only if this timeout won.
     */
    fun timeout(requestId: String, attemptId: Long): Boolean {
        val jobToCancel: Job?
        val won = synchronized(lock) {
            val active = jobs[requestId]?.takeIf { it.attemptId == attemptId }?.job
            val transitioned = finishLocked(requestId, attemptId, clock())
            jobToCancel = if (transitioned) active else null
            transitioned
        }
        jobToCancel?.cancel()
        return won
    }

    /** Releases an externally cancelled [HandoverState.Answering] attempt that neither completed nor timed out. */
    fun abandon(requestId: String, attemptId: Long) {
        val jobToCancel = synchronized(lock) {
            val now = clock()
            val current = entries[requestId]
            if (current !is HandoverState.Answering || current.attemptId != attemptId) {
                pruneLocked(now)
                return
            }
            val active = jobs.remove(requestId)?.job
            if (now >= current.expiresAtMs && now < current.expiresAtMs + pruneGraceMs) {
                entries[requestId] = HandoverState.Done(current.expiresAtMs)
            } else {
                entries.remove(requestId)
            }
            pruneLocked(now)
            active
        }
        jobToCancel?.cancel()
    }

    /**
     * Runs one [HandoverState.Answering] attempt with a hard timeout, cancelling and joining the
     * answering job before claiming timeout so at most one `sessions:answer` call is ever sent.
     */
    suspend fun executeAttempt(
        requestId: String,
        attemptId: Long,
        timeoutMs: Long,
        timeoutError: String,
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        waitForWork: suspend (Job) -> Boolean = { job -> withTimeoutOrNull(timeoutMs) { job.join() } != null },
        answer: suspend (error: String?) -> Unit,
    ): Result<Unit> = coroutineScope {
        val outcome = AtomicReference<Result<Unit>?>(null)
        val workJob = launch(dispatcher, start = CoroutineStart.LAZY) {
            if (expireIfDue(requestId, attemptId)) {
                outcome.set(Result.success(Unit))
                return@launch
            }
            val result = try {
                Result.success(answer(null))
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                Result.failure(e)
            }
            // Synchronous claim with no suspension point after answer(null) returns: if it finished
            // right as the timeout fired, complete() wins before cancelAndJoin() returns.
            if (complete(requestId, attemptId)) {
                outcome.set(result)
            }
        }
        if (!attachJob(requestId, attemptId, workJob)) {
            return@coroutineScope Result.success(Unit)
        }
        workJob.start()
        try {
            val finishedInTime = waitForWork(workJob)
            if (!finishedInTime) {
                workJob.cancelAndJoin()
                if (timeout(requestId, attemptId)) {
                    outcome.set(
                        try {
                            Result.success(withContext(dispatcher) { answer(timeoutError) })
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Throwable) {
                            Result.failure(e)
                        },
                    )
                }
            }
            outcome.get() ?: Result.success(Unit)
        } finally {
            workJob.cancel()
            abandon(requestId, attemptId)
        }
    }

    /** Prunes expired entries as of the current clock; returns expired notification IDs to cancel. */
    fun prune(): List<Int> = synchronized(lock) { pruneLocked(clock()) }

    /** Current tracked state of [requestId], or null when absent or pruned. */
    fun stateOf(requestId: String): HandoverState? = synchronized(lock) { entries[requestId] }

    /** Number of currently tracked requests. */
    val size: Int get() = synchronized(lock) { entries.size }

    private fun startAnsweringLocked(requestId: String, expiresAtMs: Long, notificationId: Int): HandoverAction.Answer {
        val attemptId = ++nextAttemptId
        entries[requestId] = HandoverState.Answering(expiresAtMs, attemptId)
        return HandoverAction.Answer(attemptId, notificationId, expiresAtMs)
    }

    private fun markExpiredLocked(requestId: String, expiresAtMs: Long, now: Long) {
        if (now < expiresAtMs + pruneGraceMs) {
            entries[requestId] = HandoverState.Done(expiresAtMs)
        } else {
            entries.remove(requestId)
        }
    }

    private fun finishLocked(requestId: String, attemptId: Long, now: Long): Boolean {
        val current = entries[requestId]
        if (current !is HandoverState.Answering || current.attemptId != attemptId) {
            pruneLocked(now)
            return false
        }
        if (jobs[requestId]?.attemptId == attemptId) jobs.remove(requestId)
        markExpiredLocked(requestId, current.expiresAtMs, now)
        pruneLocked(now)
        return true
    }

    private fun pruneLocked(now: Long): List<Int> {
        val expiredNotifications = mutableListOf<Int>()
        val iterator = entries.entries.iterator()
        while (iterator.hasNext()) {
            val entry = iterator.next()
            when (val state = entry.value) {
                is HandoverState.Done -> if (now >= state.expiresAtMs + pruneGraceMs) iterator.remove()
                is HandoverState.Notified -> if (now >= state.expiresAtMs) {
                    expiredNotifications += notificationId(entry.key)
                    if (now >= state.expiresAtMs + pruneGraceMs) {
                        iterator.remove()
                    } else {
                        entry.setValue(HandoverState.Done(state.expiresAtMs))
                    }
                }
                is HandoverState.Answering -> Unit
            }
        }
        return expiredNotifications
    }

    companion object {
        /** Matches the backend's 120 s session-request lifetime (`REQUEST_TIMEOUT_SECS`). */
        const val DEFAULT_REQUEST_TTL_MS = 120_000L

        /** Keeps terminal entries briefly past expiry to absorb delayed FCM or tap duplicates. */
        const val DEFAULT_PRUNE_GRACE_MS = 30_000L

        // Disjoint positive range so session-request notifications never collide with account or live notifications.
        private const val NOTIFICATION_ID_BASE = 0x4000_0000
        private const val NOTIFICATION_ID_MASK = 0x0fff_ffff
        private const val SECONDS_THRESHOLD = 10_000_000_000L

        /** Stable, per-request Android notification ID and PendingIntent request code. */
        fun notificationId(requestId: String): Int =
            NOTIFICATION_ID_BASE + (requestId.hashCode() and NOTIFICATION_ID_MASK)

        /**
         * Derives a session request's Unix-ms expiry from FCM `data` (`expiresAt` if supplied,
         * else `at` + [DEFAULT_REQUEST_TTL_MS]). Returns null when neither timestamp is present.
         */
        fun parsePushExpiresAtMs(data: Map<String, String>): Long? {
            data["expiresAt"]?.toLongOrNull()?.takeIf { it > 0L }?.let { raw ->
                return if (raw < SECONDS_THRESHOLD) raw * 1000L else raw
            }
            data["at"]?.toLongOrNull()?.takeIf { it > 0L }?.let { raw ->
                val atMs = if (raw < SECONDS_THRESHOLD) raw * 1000L else raw
                return atMs + DEFAULT_REQUEST_TTL_MS
            }
            return null
        }
    }
}
