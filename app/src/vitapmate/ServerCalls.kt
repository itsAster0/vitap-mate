package vitapmate

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.data.AppCore
import vitapmate.data.ServerFailure
import vitapmate.data.attempt
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * One screen's calls to our server (Server, Connected apps): each runs as flow `<name>.<call>`,
 * off the main thread; a failure is logged (src `ui.<name>`), shown (VTOP's OTP prompt, or a
 * toast titled [title]) and returned as a [ServerFailure], so the screen's problem view's
 * Details opens that flow.
 */
internal class ServerCalls(private val name: String, private val title: String) {
    suspend operator fun <T> invoke(call: String, block: suspend (Core, String) -> T): Result<T> {
        var flowId = ""
        return attempt {
            Result.success(flow("$name.$call") { f -> flowId = f; withContext(Dispatchers.IO) { block(AppCore.await(), f) } })
        }.getOrElse { e ->
            Log.warn("ui.$name", "$call failed: ${e::class.simpleName}", flowId)
            when (e) {
                // Rust's prompt asked for the code and the student closed it: nothing to add.
                is CoreException.OtpRequired -> {}
                is CoreException.Other -> showToast(title, e.reason)
                else -> showToast(problemOf(e, flowId, server = true))
            }
            Result.failure(ServerFailure(flowId, e))
        }
    }
}
