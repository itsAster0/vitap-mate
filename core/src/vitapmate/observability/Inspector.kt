package vitapmate.observability

import android.content.Context
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.map
import kotlin.time.Duration.Companion.milliseconds

/**
 * The Inspector's data: the app's log file (`cache/logs/app.jsonl` + `.1`, Kotlin and Rust
 * lines) as [LogLine]s and [FlowSummary]s, tailed while collected. Filter with [LogFilter].
 */
object Inspector {
    private lateinit var cache: File

    /** When this process attached (epoch ms): flows older without an end were cut off. */
    var startedAt = 0L
        private set

    /** Called by `AppCore.start`. */
    internal fun attach(context: Context) {
        cache = context.cacheDir
        startedAt = System.currentTimeMillis()
        ActivityHistory.attach(cache)
    }

    private val logs get() = File(cache, "logs")

    /** The last [max] lines, oldest first; re-emitted (polling every 500 ms on IO) when lines are appended. */
    fun lines(max: Int = 2000): Flow<List<LogLine>> = flow {
        val tail = LogTail(logs, max)
        while (true) {
            if (tail.poll()) emit(tail.lines())
            delay(500.milliseconds)
        }
    }.flowOn(Dispatchers.IO)

    /** Flows in the last [max] lines, newest first, tailed like [lines]. */
    fun flows(max: Int = 2000): Flow<List<FlowSummary>> = lines(max).map(::groupFlows).flowOn(Dispatchers.Default)

    /** Runs ([runsOf]) in the last [max] lines, newest first, tailed like [lines]. */
    fun runs(max: Int = 2000): Flow<List<Run>> = flows(max).map { runsOf(it, startedAt) }.flowOn(Dispatchers.Default)

    // Kept a minute after the last viewer leaves, so the sheet opens again at once.
    private val shared = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** The Activity sheet's runs: the last 2000 lines; null until first read. */
    val recentRuns: StateFlow<List<Run>?> by lazy { runs().stateIn(shared, SharingStarted.WhileSubscribed(60_000), null) }

    /**
     * Every run both log files still hold (the current and the rolled one, across app restarts).
     * The rolled file is parsed once per roll-over; the current one is tailed.
     */
    val allRuns: StateFlow<List<Run>?> by lazy { runs(Int.MAX_VALUE).stateIn(shared, SharingStarted.WhileSubscribed(60_000), null) }

    /** The last [max] lines, read once (blocking: call off the main thread). */
    fun read(max: Int = 2000): List<LogLine> = LogTail(logs, max).apply { poll() }.lines()

    /**
     * Both log files in one, oldest first, at `<cacheDir>/share/vitapmate-log.jsonl`, for a share
     * sheet (FileProvider path `cache/share`). Blocking: call off the main thread.
     */
    fun exportFile(): File {
        val out = File(cache, "share/vitapmate-log.jsonl")
        out.parentFile?.mkdirs()
        out.outputStream().use { o ->
            for (name in listOf("app.jsonl.1", "app.jsonl")) File(logs, name).takeIf { it.exists() }?.inputStream()?.use { it.copyTo(o) }
        }
        Log.info("inspector", "log exported", fields = mapOf("bytes" to "${out.length()}"))
        return out
    }
}
