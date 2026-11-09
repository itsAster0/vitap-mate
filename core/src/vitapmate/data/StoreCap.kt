package vitapmate.data

import android.content.Context
import java.io.File
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.observability.ActivityHistory
import vitapmate.observability.Log

/**
 * Keeps the app's history small: the change inbox (Rust `changes.json`), the saved Activity
 * history (`activity.jsonl`) and the log files. Past [MAX_BYTES] together, each is cut to its
 * share of [TARGET_BYTES], oldest first (the rolled log file goes whole). Cached VTOP pages and
 * the student's files are not part of it. Runs at start and after each sync. Log src `store`.
 */
object StoreCap {
    const val MAX_BYTES = 20L * 1024 * 1024
    const val TARGET_BYTES = 10L * 1024 * 1024

    suspend fun enforce(context: Context, flow: String) = withContext(Dispatchers.IO) {
        try {
            val core = AppCore.await()
            val logs = File(context.filesDir, "logs")
            val rolled = File(logs, "app.jsonl.1")
            val logBytes = logs.listFiles().orEmpty().sumOf { it.length() }
            val activity = ActivityHistory.bytes
            val changes = core.changesBytes(flow).toLong()
            val total = logBytes + activity + changes
            if (total <= MAX_BYTES) return@withContext
            val share = TARGET_BYTES.toDouble() / total
            ActivityHistory.trim((activity * share).toLong())
            core.trimChanges(flow, (changes * share).toULong())
            // The current log is at most 1 MB (Rust rolls it); the older one is what can go.
            rolled.delete()
            Log.info("store", "trimmed", flow, mapOf("before" to "$total", "logs" to "$logBytes", "activity" to "$activity", "changes" to "$changes"))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("store", "trim failed: ${e::class.simpleName}", flow)
        }
    }
}
