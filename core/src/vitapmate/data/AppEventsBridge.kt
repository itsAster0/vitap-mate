package vitapmate.data

import android.os.Handler
import android.os.Looper
import vitapmate.core.ActivityEntry
import vitapmate.core.AppEvents
import vitapmate.core.ConsentAsk
import vitapmate.core.HandoverStatus
import vitapmate.core.SemesterState
import vitapmate.core.SyncHealth
import vitapmate.observability.Unattended

/** Rust's app-wide pushes. They arrive on Rust's threads, so each hops to the main thread first. */
object AppEventsBridge : AppEvents {
    private val main by lazy { Handler(Looper.getMainLooper()) }

    override fun onBackgroundProblem(problem: Problem) {
        main.post { if (problem.toasts) BackgroundProblems.shown.tryEmit(problem) }
    }

    override fun onActivity(entries: List<ActivityEntry>) {
        main.post { Activity.mirror(entries) }
    }

    override fun onConsent(ask: ConsentAsk) {
        main.post {
            CookieConsent.onConsent(ask)
            RegnoConsent.onConsent(ask)
            HandoverConsent.onConsent(ask)
        }
    }

    override fun onHandoverStatus(flow: String, requestId: String, status: HandoverStatus) {
        HandoverConsent.onStatus(requestId, status)
    }

    override fun isFlowUnattended(flow: String): Boolean = flow in Unattended.ids

    // Told under Rust's lock, in order: set here, not posted, so a read right after the call sees it.
    override fun onSemesters(state: SemesterState) = SemesterChoice.mirror(state)

    override fun onSyncHealth(health: SyncHealth, paused: String?) = SyncHealthState.mirror(health, paused)
}
