package vitapmate.features.outing

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.OutingResult
import vitapmate.data.AppCore
import vitapmate.data.askedOtp
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Sends an application once in flow `outing.apply` (on [writes], so closing the screen doesn't cut
 * it off). [onResult] gets VTOP's answer; [onError] a failure's words (it may have reached VTOP);
 * an OTP wanted on the way opens the OTP prompt instead. [onDone] runs last either way.
 */
internal fun sendApplication(
    kind: String,
    call: suspend (flow: String, core: Core) -> OutingResult,
    onResult: (OutingResult) -> Unit,
    onError: (String) -> Unit,
    onDone: () -> Unit,
) {
    writes.launch {
        try {
            val result = flow("outing.apply") { id ->
                Log.info("ui.outing", "apply", id, mapOf("kind" to kind))
                withContext(Dispatchers.IO) { call(id, AppCore.await()) }
                    .also { Log.info("ui.outing", if (it.done) "applied" else "not applied", id, mapOf("kind" to kind, "done" to "${it.done}")) }
            }
            onResult(result)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            if (!askedOtp(e)) onError(failureOf(e, ""))
        } finally {
            onDone()
        }
    }
}
