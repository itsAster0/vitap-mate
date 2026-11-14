package vitapmate.observability

import androidx.tracing.traceAsync
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.cancellation.CancellationException
import kotlin.random.Random
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * Runs [block] as one user action or background run named [name]: makes a short flow id (pass it
 * as the first argument of every `Core` call, so Rust's lines carry it too), logs start and end
 * with ms and outcome (`ok`, `cancelled` or the exception's class), and shows as an async
 * section in Perfetto next to Rust's spans. An [unattended] flow (a background run) never sends
 * the VTOP session to our servers, whatever the student ticked.
 */
suspend fun <T> flow(name: String, unattended: Boolean = false, block: suspend (id: String) -> T): T {
    val id = Random.nextInt().toUInt().toString(16).padStart(8, '0')
    val start = System.nanoTime()
    fun ms() = ((System.nanoTime() - start) / 1_000_000).toString()
    Log.info("flow", "$name start", id, mapOf("step" to name, "ev" to "start"))
    if (unattended) Unattended.ids += id
    return traceAsync("flow.$name", id.hashCode()) {
        try {
            block(id).also { Log.info("flow", "$name done", id, mapOf("step" to name, "ev" to "done", "ms" to ms(), "outcome" to "ok")) }
        } catch (e: CancellationException) {
            Log.info("flow", "$name cancelled", id, mapOf("step" to name, "ev" to "cancelled", "ms" to ms(), "outcome" to "cancelled"))
            throw e
        } catch (e: Throwable) {
            Log.warn("flow", "$name failed: ${e::class.simpleName}", id, mapOf("step" to name, "ev" to "failed", "ms" to ms(), "outcome" to (e::class.simpleName ?: "error")))
            throw e
        } finally {
            Unattended.ids -= id
        }
    }
}

/** The ids of the [flow]s running unattended. */
object Unattended {
    val ids: MutableSet<String> = ConcurrentHashMap.newKeySet()
}

/** A tap that only changes the screen (a filter, a tab, a sheet): its own flow [name] and one log line. */
fun CoroutineScope.logAction(name: String, src: String, msg: String, fields: Map<String, String> = emptyMap()) {
    launch { flow(name) { id -> Log.info(src, msg, id, fields) } }
}
