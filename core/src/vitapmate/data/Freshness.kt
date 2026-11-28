package vitapmate.data

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import java.util.concurrent.Executors
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.runBlocking
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Whether the app is on screen, and how often it came back (open pages then refresh if old).
 * Rust counts the comebacks ([vitapmate.core.Core.appShown]); [resumes] mirrors its count.
 */
object Foreground {
    var resumes by mutableIntStateOf(0)
        private set
    /**
     * Activities started and not stopped: a new MainActivity can start before the old one stops.
     * Kept for synchronous reads in `PushService` (before `AppCore.start` on background FCM wake),
     * `Push.follow`, and `LiveWake`; Rust owns consent cancellation, live-watch pause/resume and
     * the open-app sweep via [vitapmate.core.Core.appShown].
     */
    @Volatile private var started = 0
    val shown get() = started > 0
    private val shownNow = MutableStateFlow(false)

    /** [shown] as it changes: `Push.follow` watches session requests while on screen. */
    val shownFlow: StateFlow<Boolean> = shownNow

    /** One thread, so Rust hears starts and stops in order, after the core is up. */
    private val toRust = Executors.newSingleThreadExecutor { Thread(it, "app-shown").apply { isDaemon = true } }
    private val main by lazy { Handler(Looper.getMainLooper()) }

    /** From the activity's `onStart`. */
    fun started() {
        started++
        shownNow.value = true
        tell(true)
    }

    /** From the activity's `onStop`. */
    fun stopped() {
        started = (started - 1).coerceAtLeast(0)
        shownNow.value = shown
        tell(false)
    }

    private fun tell(on: Boolean) = toRust.execute {
        runCatching { runBlocking { flow("app.shown") { f -> AppCore.await().run { appShown(f, on); resumes() } } } }
            .onSuccess { n -> main.post { resumes = n.toInt() } }
            .onFailure { Log.warn("app", "app_shown not told: ${it::class.simpleName}") }
    }
}
