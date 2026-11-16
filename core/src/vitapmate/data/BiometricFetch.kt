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
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Wrapped's campus times: a WorkManager job, started only by the student's Fetch, that fetches a
 * semester's missing biometric days a few VTOP requests at a time. Unique per semester (KEEP), so a
 * second tap joins the run; days already complete in the cache are skipped. An ordinary (not
 * foreground) job: before Android's 10-minute limit, or when VTOP drops after some days came in,
 * it appends a fresh run of itself to the unique work and ends, so WorkManager's retry count and
 * backoff only ever count attempts that fetched nothing. Flow `wrapped.biometric`, log src `wrapped`.
 */
object BiometricFetch {
    internal fun work(semester: String) = "wrapped.biometric.$semester"

    /** The Stop action's request code, one per semester. */
    internal fun stopCode(semester: String) = work(semester).hashCode()

    /** Queues the fetch of [days] (days since 1970-01-01, in India) for [semester]. */
    fun start(context: Context, semester: String, days: List<Int>) {
        WorkManager.getInstance(context).enqueueUniqueWork(work(semester), ExistingWorkPolicy.KEEP, request(semester, days.toIntArray(), IntArray(0)))
    }

    /** One run; [skipped] are days that failed in an earlier run of this job, not asked for again. */
    internal fun request(semester: String, days: IntArray, skipped: IntArray, delaySecs: Long = 0): OneTimeWorkRequest =
        OneTimeWorkRequestBuilder<BiometricFetchWorker>()
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, BACKOFF_SECS, TimeUnit.SECONDS)
            .setInitialDelay(delaySecs, TimeUnit.SECONDS)
            .setInputData(workDataOf(BiometricFetchWorker.SEMESTER to semester, BiometricFetchWorker.DAYS to days, BiometricFetchWorker.SKIPPED to skipped))
            .build()

    /** The semester's run (progress `done` and `total`), or null when none was queued: the unfinished one of a hand-over chain first. */
    fun state(context: Context, semester: String): Flow<WorkInfo?> =
        WorkManager.getInstance(context).getWorkInfosForUniqueWorkFlow(work(semester)).map { infos ->
            infos.firstOrNull { it.state == WorkInfo.State.RUNNING } ?: infos.firstOrNull { !it.state.isFinished } ?: infos.firstOrNull()
        }

    /** Windows of at most 14 days (what `cachedBiometricRange` takes) that cover [days]. */
    internal fun windows(days: List<Int>): List<IntRange> {
        val out = mutableListOf<IntRange>()
        for (day in days.distinct().sorted()) if (out.lastOrNull()?.contains(day) != true) out += day..(day + MAX_RANGE_DAYS - 1)
        return out
    }

    /** What an attempt that ended on Offline / VTOP unavailable does next. */
    internal enum class Next { Rerun, Retry, GiveUp }

    /**
     * An attempt that [fetched] some days is progress: a fresh run (attempt count 0). One that
     * fetched nothing retries with backoff; the [MAX_STALLS]th in a row ([attempt] is
     * `runAttemptCount`, from 0) gives up.
     */
    internal fun afterStall(fetched: Int, attempt: Int): Next = when {
        fetched > 0 -> Next.Rerun
        attempt + 1 >= MAX_STALLS -> Next.GiveUp
        else -> Next.Retry
    }

    internal const val MAX_RANGE_DAYS = 14
    internal const val MAX_STALLS = 5
    /** First retry 30 s after a stalled attempt, then 60, 120, 240. */
    internal const val BACKOFF_SECS = 30L
}

class BiometricFetchWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    private val notifications = NotificationManagerCompat.from(context)
    /** Set when this run ends in a hand-over or retry: the progress notification stays. */
    private var keepNotification = false

    /** How a day's fetch ended. */
    private enum class Day { Done, SignIn, Unreachable, Skipped }

    override suspend fun doWork(): Result {
        val app = applicationContext
        Settings.load(app)
        AppCore.start(app)
        val days = inputData.getIntArray(DAYS) ?: return Result.success()
        val semester = inputData.getString(SEMESTER)
        val skipped = (inputData.getIntArray(SKIPPED) ?: IntArray(0)).toMutableSet()
        app.getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(CHANNEL, "Campus times", NotificationManager.IMPORTANCE_LOW))
        try {
            return run(days, semester, skipped)
        } finally {
            // Success, failure or Stop: gone. A hand-over or retry keeps the last count up until the next run posts its own.
            if (!keepNotification) notifications.cancel(ID)
        }
    }

    private suspend fun run(days: IntArray, semester: String?, skipped: MutableSet<Int>): Result {
        val total = days.size
        val started = System.currentTimeMillis()
        // A sign-in or couldn't-reach notice from an earlier run is out of date now.
        notifications.cancel(END_ID)
        return flow("wrapped.biometric") { f ->
            var cached = 0
            var fetched = 0
            fun ended(result: Result, why: String) = result.also {
                Log.info("wrapped", "campus times $why", f, mapOf("days" to "$total", "cached" to "$cached", "fetched" to "$fetched", "skipped" to "${skipped.size}", "attempt" to "$runAttemptCount"))
                if (it !is Result.Success) Log.warn("wrapped", "campus times ended: $why", f, mapOf("result" to "$it"))
            }
            /** Offline or VTOP unavailable: a fresh run after progress, else a backed-off retry, at most [BiometricFetch.MAX_STALLS] in a row. */
            fun unreachable(why: String, done: Int = cached): Result = when (BiometricFetch.afterStall(fetched, runAttemptCount)) {
                BiometricFetch.Next.Rerun -> ended(rerun(semester, days, skipped, BiometricFetch.BACKOFF_SECS, done, total), "resumes after $why")
                BiometricFetch.Next.Retry -> {
                    keepNotification = true
                    show(done, total, semester, waiting = true)
                    ended(Result.retry(), "retry after $why")
                }
                BiometricFetch.Next.GiveUp -> {
                    post(END_ID, notification("Couldn't reach VTOP. Open Wrapped to try again.").setAutoCancel(true).build())
                    ended(Result.failure(), "gave up after $why")
                }
            }
            try {
                withContext(Dispatchers.IO) {
                    val core = AppCore.await()
                    // The cache, read in 14-day windows (one log line each, not one per day).
                    val complete = BiometricFetch.windows(days.toList())
                        .flatMap { core.cachedBiometricRange(f, it.first, it.last) }
                        .filter { it.complete }.map { it.day }.toSet()
                    cached = days.count { it in complete }
                    val missing = days.filter { it !in complete && it !in skipped }
                    fun done() = cached + fetched + skipped.count { it !in complete }
                    show(done(), total, semester)
                    if (missing.isNotEmpty()) {
                        try {
                            // Signs in again only when the session is older than 15 minutes.
                            core.ensureFreshSession(f, 900uL)
                        } catch (e: CoreException) {
                            if (e.signIn()) throw e
                            return@withContext unreachable("sign-in: ${e::class.simpleName}")
                        }
                    }
                    // A few days at once (VTOP takes the app's page fetches in parallel too), then the gap.
                    for (batch in missing.chunked(Settings.tunable(Tunable.CampusFetchAtOnce))) {
                        // Before Android stops an ordinary job (10 minutes): a fresh run skips the done days.
                        if (System.currentTimeMillis() - started > RUN_LIMIT_MS) return@withContext ended(rerun(semester, days, skipped, 0, done(), total), "handed to a rerun")
                        val outcomes = coroutineScope { batch.map { day -> async { day to fetchDay(core, f, day) } }.awaitAll() }
                        fetched += outcomes.count { it.second == Day.Done }
                        outcomes.filter { it.second == Day.Skipped }.forEach { skipped += it.first }
                        if (outcomes.any { it.second == Day.SignIn }) {
                            post(END_ID, notification("Open vitapmate to sign in and continue").setAutoCancel(true).build())
                            return@withContext ended(Result.failure(), "need a sign-in")
                        }
                        if (outcomes.any { it.second == Day.Unreachable }) return@withContext unreachable("VTOP unreachable", done())
                        setProgress(workDataOf(DONE to done(), TOTAL to total))
                        show(done(), total, semester)
                        delay(Settings.tunable(Tunable.CampusFetchGap) * 1000L)
                    }
                    ended(Result.success(), "fetched")
                }
            } catch (e: CoreException) {
                // Only the sign-in errors reach here (from ensureFreshSession).
                post(END_ID, notification("Open vitapmate to sign in and continue").setAutoCancel(true).build())
                ended(Result.failure(), "need a sign-in: ${e::class.simpleName}")
            }
        }
    }

    /** One day's fetch; only the sign-in and reachability errors end the run, any other skips the day. */
    private suspend fun fetchDay(core: Core, f: String, day: Int): Day = try {
        core.fetchBiometricRange(f, day, day)
        Day.Done
    } catch (e: CoreException) {
        when {
            e.signIn() -> Day.SignIn
            e is CoreException.Offline || e is CoreException.VtopUnavailable -> Day.Unreachable
            else -> {
                Log.warn("wrapped", "campus day skipped: ${e::class.simpleName}", f, mapOf("day" to "$day"))
                Day.Skipped
            }
        }
    }

    /** Ends the run with the sign-in notice, never a retry: signed out, OTP needed or a wrong password. */
    private fun CoreException.signIn() = this is CoreException.OtpRequired || this is CoreException.InvalidCredentials || this is CoreException.NotSignedIn

    /**
     * Appends a fresh run to this semester's unique work and ends this one with success, so the
     * new run starts at attempt 0. The notification stays (Stop cancels the whole unique work).
     * A run queued before the semester was in the input falls back to a retry.
     */
    private fun rerun(semester: String?, days: IntArray, skipped: Set<Int>, delaySecs: Long, done: Int, total: Int): Result {
        if (semester == null) return Result.retry().also { keepNotification = true }
        val next = BiometricFetch.request(semester, days, skipped.toIntArray(), delaySecs)
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
        internal const val SKIPPED = "skipped"
        private const val CHANNEL = "campus_fetch"
        internal const val ID = 0x5168
        /** The end notices: sign in, or couldn't reach VTOP. */
        private const val END_ID = ID + 1
        /** Under the 10 minutes Android gives an ordinary job, with room for the days in flight. */
        private const val RUN_LIMIT_MS = 8 * 60_000L
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
        MainScope().launch {
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
