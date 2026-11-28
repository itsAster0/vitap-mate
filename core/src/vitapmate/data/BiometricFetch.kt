package vitapmate.data

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkInfo
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.await
import androidx.work.workDataOf
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.BiometricStallNext
import vitapmate.core.BiometricStep
import vitapmate.core.biometricAfterStall
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Wrapped's campus times: a WorkManager job, started only by the student's Fetch, that loops
 * Rust's [vitapmate.core.Core.biometricStep] for a semester's missing biometric days. Unique per
 * semester (KEEP), so a second tap joins the run; days already complete in the cache are skipped
 * in Rust. Before Android's 10-minute limit, or when VTOP drops after some days came in, it
 * appends a fresh run of itself to the unique work and ends, so WorkManager's retry count and
 * backoff only ever count attempts that fetched nothing. Flow `wrapped.biometric`, log src
 * `wrapped`.
 */
object BiometricFetch {
    internal fun work(semester: String) = "wrapped.biometric.$semester"

    /** The Stop action's request code, one per semester. */
    internal fun stopCode(semester: String) = work(semester).hashCode()

    /** Queues the fetch of [days] (days since 1970-01-01, in India) for [semester]. */
    fun start(context: Context, semester: String, days: List<Int>) {
        WorkManager.getInstance(context).enqueueUniqueWork(work(semester), ExistingWorkPolicy.KEEP, request(semester, days.size))
    }

    /** One run; [total] is the initial day count for progress display. */
    internal fun request(semester: String, total: Int, delaySecs: Long = 0): OneTimeWorkRequest =
        OneTimeWorkRequestBuilder<BiometricFetchWorker>()
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, BACKOFF_SECS, TimeUnit.SECONDS)
            .setInitialDelay(delaySecs, TimeUnit.SECONDS)
            .setInputData(workDataOf(BiometricFetchWorker.SEMESTER to semester, BiometricFetchWorker.TOTAL to total))
            .build()

    /** The semester's run (progress `done` and `total`), or null when none was queued: the unfinished one of a hand-over chain first. */
    fun state(context: Context, semester: String): Flow<WorkInfo?> =
        WorkManager.getInstance(context).getWorkInfosForUniqueWorkFlow(work(semester)).map { infos ->
            infos.firstOrNull { it.state == WorkInfo.State.RUNNING } ?: infos.firstOrNull { !it.state.isFinished } ?: infos.firstOrNull()
        }

    /** First retry 30 s after a stalled attempt, then 60, 120, 240. */
    internal const val BACKOFF_SECS = 30L
}

class BiometricFetchWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    private val notifications = NotificationManagerCompat.from(context)
    /** Set when this run ends in a hand-over or retry: the progress notification stays. */
    private var keepNotification = false

    override suspend fun doWork(): Result {
        val app = applicationContext
        Settings.load(app)
        AppCore.start(app)
        val semester = inputData.getString(SEMESTER) ?: return Result.success()
        val total = inputData.getInt(TOTAL, 0).takeIf { it > 0 }
            ?: inputData.getIntArray(DAYS)?.size
            ?: 0
        app.getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(CHANNEL, "Campus times", NotificationManager.IMPORTANCE_LOW))
        try {
            return run(semester, total)
        } finally {
            // Success, failure or Stop: gone. A hand-over or retry keeps the last count up until the next run posts its own.
            if (!keepNotification) notifications.cancel(ID)
        }
    }

    private suspend fun run(semester: String, initialTotal: Int): Result {
        val started = System.currentTimeMillis()
        // A sign-in or couldn't-reach notice from an earlier run is out of date now.
        notifications.cancel(END_ID)
        return flow("wrapped.biometric") { f ->
            var fetched = 0
            var total = initialTotal
            fun ended(result: Result, why: String) = result.also {
                Log.info("wrapped", "campus times $why", f, mapOf("days" to "$total", "fetched" to "$fetched", "attempt" to "$runAttemptCount"))
                if (it !is Result.Success) Log.warn("wrapped", "campus times ended: $why", f, mapOf("result" to "$it"))
            }
            fun unreachable(why: String, done: Int): Result = when (biometricAfterStall(fetched.toUInt(), runAttemptCount.toUInt())) {
                BiometricStallNext.RERUN -> ended(rerun(semester, total, BiometricFetch.BACKOFF_SECS, done), "resumes after $why")
                BiometricStallNext.RETRY -> {
                    keepNotification = true
                    show(done, total, semester, waiting = true)
                    ended(Result.retry(), "retry after $why")
                }
                BiometricStallNext.GIVE_UP -> {
                    post(END_ID, notification("Couldn't reach VTOP. Open Wrapped to try again.").setAutoCancel(true).build())
                    ended(Result.failure(), "gave up after $why")
                }
            }
            withContext(Dispatchers.IO) {
                val core = AppCore.await()
                val initialMissing = core.wrappedMissingDays(f).size
                if (total <= 0) total = initialMissing
                var done = (total - initialMissing).coerceIn(0, total)
                show(done, total, semester)

                while (true) {
                    val remaining = RUN_LIMIT_MS - (System.currentTimeMillis() - started)
                    if (remaining <= 0) {
                        return@withContext ended(rerun(semester, total, 0, done), "handed to a rerun")
                    }
                    val stepBudget = remaining.coerceAtMost(STEP_BUDGET_MS)
                    val stepStart = System.currentTimeMillis()
                    when (val step = core.biometricStep(f, semester, stepBudget.toULong())) {
                        is BiometricStep.Done -> {
                            fetched += step.fetched.toInt()
                            done = total
                            setProgress(workDataOf(DONE to done, TOTAL to total))
                            show(done, total, semester)
                            return@withContext ended(Result.success(), "fetched")
                        }
                        is BiometricStep.More -> {
                            fetched += step.fetched.toInt()
                            val missingNow = core.wrappedMissingDays(f).size
                            done = (total - missingNow).coerceIn(0, total)
                            setProgress(workDataOf(DONE to done, TOTAL to total))
                            show(done, total, semester)
                            val elapsed = System.currentTimeMillis() - started
                            val stepElapsed = System.currentTimeMillis() - stepStart
                            if (elapsed >= RUN_LIMIT_MS) {
                                return@withContext ended(rerun(semester, total, 0, done), "handed to a rerun")
                            }
                            if (stepElapsed < stepBudget) {
                                return@withContext ended(rerun(semester, total, BiometricFetch.BACKOFF_SECS, done), "resumes after VTOP unreachable")
                            }
                        }
                        is BiometricStep.Failed -> {
                            fetched += step.fetched.toInt()
                            val kind = step.problem.kind
                            if (kind == ProblemKind.SIGN_IN_NEEDED || kind == ProblemKind.OTP_NEEDED) {
                                post(END_ID, notification("Open vitapmate to sign in and continue").setAutoCancel(true).build())
                                return@withContext ended(Result.failure(), "need a sign-in: ${kind.name}")
                            }
                            return@withContext unreachable(kind.name, done)
                        }
                    }
                }
                @Suppress("UNREACHABLE_CODE")
                Result.success()
            }
        }
    }

    /**
     * Appends a fresh run to this semester's unique work and ends this one with success, so the
     * new run starts at attempt 0. The notification stays (Stop cancels the whole unique work).
     */
    private fun rerun(semester: String, total: Int, delaySecs: Long, done: Int): Result {
        val next = BiometricFetch.request(semester, total, delaySecs)
        WorkManager.getInstance(applicationContext).enqueueUniqueWork(BiometricFetch.work(semester), ExistingWorkPolicy.APPEND_OR_REPLACE, next)
        show(done, total, semester, waiting = true)
        keepNotification = true
        return Result.success()
    }

    /**
     * The progress notification. Ongoing only while this run fetches; [waiting] (for a retry or the
     * next run) it can be swiped away, as no worker runs to clear it if the work is cancelled.
     * Stop goes to [BiometricFetchStop]: WorkManager's own cancel intent needs SystemForegroundService,
     * which the manifest removes.
     */
    private fun show(done: Int, total: Int, semester: String?, waiting: Boolean = false) {
        // Stopped mid-batch: the receiver or the finally clears it; don't post it again.
        if (isStopped) return
        val n = notification("Fetching campus times · $done of $total days")
            .setOngoing(!waiting)
            .setOnlyAlertOnce(true)
            .setProgress(total, done, false)
        if (semester != null) n.addAction(0, "Stop", BiometricFetchStop.intent(applicationContext, semester))
        post(ID, n.build())
    }

    /** Not permitted: the fetch just runs without one. */
    private fun post(id: Int, n: android.app.Notification) {
        if (notifications.areNotificationsEnabled()) runCatching { notifications.notify(id, n) }
    }

    /** Tapping opens the app on Wrapped (`open = "wrapped"`, as the shell's other notifications do). */
    private fun notification(text: String): NotificationCompat.Builder {
        val app = applicationContext
        val icon = app.resources.getIdentifier("ic_notification", "drawable", app.packageName).takeIf { it != 0 } ?: app.applicationInfo.icon
        val open = app.packageManager.getLaunchIntentForPackage(app.packageName)?.putExtra("open", "wrapped")?.let {
            PendingIntent.getActivity(app, ID, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        return NotificationCompat.Builder(app, CHANNEL).setSmallIcon(icon).setContentTitle("Campus times").setContentText(text).setContentIntent(open)
    }

    companion object {
        const val DAYS = "days"
        const val DONE = "done"
        const val TOTAL = "total"
        internal const val SEMESTER = "semester"
        private const val CHANNEL = "campus_fetch"
        internal const val ID = 0x5168
        /** The end notices: sign in, or couldn't reach VTOP. */
        private const val END_ID = ID + 1
        /** Under the 10 minutes Android gives an ordinary job, with room for the days in flight. */
        private const val RUN_LIMIT_MS = 8 * 60_000L
        /** Slice length per [vitapmate.core.Core.biometricStep] call so progress updates smoothly. */
        private const val STEP_BUDGET_MS = 15_000L
    }
}

/**
 * The progress notification's Stop: cancels the semester's unique work, the running run and any
 * queued retry or hand-over with it, then the notification, which no worker clears while a run
 * waits. Not exported; flow `wrapped.biometric.stop`.
 */
class BiometricFetchStop : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val semester = intent.getStringExtra(BiometricFetchWorker.SEMESTER) ?: return
        val pending = goAsync()
        val app = context.applicationContext
        AppScope.launch {
            try {
                Settings.load(app)
                AppCore.start(app)
                flow("wrapped.biometric.stop") { f ->
                    WorkManager.getInstance(app).cancelUniqueWork(BiometricFetch.work(semester)).await()
                    Log.info("wrapped", "campus times stopped", f)
                }
            } finally {
                NotificationManagerCompat.from(app).cancel(BiometricFetchWorker.ID)
                pending.finish()
            }
        }
    }

    companion object {
        internal fun intent(context: Context, semester: String): PendingIntent = PendingIntent.getBroadcast(
            context,
            BiometricFetch.stopCode(semester),
            Intent(context, BiometricFetchStop::class.java).putExtra(BiometricFetchWorker.SEMESTER, semester),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
    }
}
