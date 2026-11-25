package vitapmate

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.TriangleAlert
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.data.AppCore
import vitapmate.data.Problem
import vitapmate.data.ServerFailure
import vitapmate.data.attempt
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppDialog
import vitapmate.ui.Haptics
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** One server read on a screen: loading, failed with what to show, or loaded (kept while it loads again). */
internal sealed interface ServerLoad<out T> {
    data object Loading : ServerLoad<Nothing>

    data class Failed(val problem: Problem) : ServerLoad<Nothing>

    data class Ready<T>(val value: T) : ServerLoad<T>
}

/** The phase after [result]: a failure replaces only a phase with nothing loaded yet. */
internal fun <T> ServerLoad<T>.after(result: Result<T>): ServerLoad<T> = result.fold(
    { ServerLoad.Ready(it) },
    { e -> if (this is ServerLoad.Ready) this else ServerLoad.Failed((e as ServerFailure).problem) },
)

/** The loaded value, if any. */
internal val <T> ServerLoad<T>.ready: T? get() = (this as? ServerLoad.Ready)?.value

/**
 * One screen's calls to our server (Account, Connected apps, Developer): each runs as flow `<name>.<call>`,
 * off the main thread; a failure is logged (src `ui.<name>`), shown (VTOP's OTP prompt, or a
 * toast titled [title]) and returned as a [ServerFailure], so the screen's problem view's
 * Details opens that flow.
 */
internal class ServerCalls(val name: String, val title: String) {
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

/**
 * A server change waiting for its confirm dialog: [title] and [text] say what goes, [done] is
 * the toast after it. [call] answers false when the server doesn't offer it yet.
 */
internal class ServerConfirm(
    val title: String,
    val text: String,
    val confirm: String,
    val name: String,
    val done: String,
    val danger: Boolean = true,
    val call: suspend (Core, String) -> Boolean,
)

/**
 * One screen's server changes, one at a time ([busy]): [ask] shows a [ServerConfirm] in
 * [Dialog] first; [run] goes straight away (a switch, whose own label says what it does).
 */
@Stable
internal class ServerActions(private val calls: ServerCalls, private val scope: CoroutineScope, private val haptics: Haptics) {
    var asking by mutableStateOf<ServerConfirm?>(null)
        private set
    var busy by mutableStateOf(false)
        private set

    fun ask(confirm: ServerConfirm) {
        // A second tap while one runs would ask the server twice.
        if (!busy) asking = confirm
    }

    /** [call] as flow `<screen>.<name>`; [onDone] after it worked. */
    fun run(name: String, done: String?, call: suspend (Core, String) -> Boolean, onDone: () -> Unit = {}) {
        if (busy) return
        scope.launch {
            busy = true
            calls(name, call).onSuccess { worked ->
                if (!worked) {
                    showToast(calls.title, "The server doesn't offer this yet.")
                    return@onSuccess
                }
                haptics.confirm()
                done?.let { showToast(calls.title, it) }
                onDone()
            }.onFailure { haptics.reject() }
            busy = false
        }
    }

    /** The dialog of [asking]; confirming runs it, then [onDone] with it. */
    @Composable
    fun Dialog(onDone: (ServerConfirm) -> Unit = {}) {
        val confirm = asking ?: return
        AppDialog(
            { asking = null },
            confirm.title,
            confirm.confirm,
            {
                asking = null
                run(confirm.name, confirm.done, confirm.call) { onDone(confirm) }
            },
            icon = if (confirm.danger) Lucide.TriangleAlert else null,
            danger = confirm.danger,
            confirmTag = "${calls.name}.confirm",
        ) { Text(confirm.text) }
    }
}

@Composable
internal fun rememberServerActions(calls: ServerCalls): ServerActions {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    return remember(calls, scope, haptics) { ServerActions(calls, scope, haptics) }
}
