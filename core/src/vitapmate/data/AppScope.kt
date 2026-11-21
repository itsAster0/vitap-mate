package vitapmate.data

import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import vitapmate.observability.Log

/**
 * Process-wide scope on the main dispatcher for work that must outlive a screen or activity
 * (writes, page fetches, broadcast receivers, exit refreshes). Uses [SupervisorJob] so one
 * failure never cancels siblings, and logs uncaught errors.
 */
val AppScope: CoroutineScope = CoroutineScope(
    SupervisorJob() + Dispatchers.Main.immediate + CoroutineExceptionHandler { _, e ->
        Log.warn("app", "uncaught coroutine error: ${e::class.simpleName}", fields = mapOf("error" to e.toString().take(200)))
    },
)
