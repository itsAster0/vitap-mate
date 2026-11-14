package vitapmate

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.google.android.gms.tasks.Task
import com.google.android.play.core.appupdate.AppUpdateInfo
import com.google.android.play.core.appupdate.AppUpdateManager
import com.google.android.play.core.appupdate.AppUpdateManagerFactory
import com.google.android.play.core.appupdate.AppUpdateOptions
import com.google.android.play.core.install.InstallState
import com.google.android.play.core.install.InstallStateUpdatedListener
import com.google.android.play.core.install.model.AppUpdateType
import com.google.android.play.core.install.model.InstallStatus
import com.google.android.play.core.install.model.UpdateAvailability
import kotlin.coroutines.cancellation.CancellationException
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.observability.Log
import vitapmate.observability.flow

/** What the update pill above the nav bar shows. */
sealed interface UpdatePill {
    data object Available : UpdatePill
    data class Downloading(val fraction: Float) : UpdatePill
    data object Ready : UpdatePill
}

/**
 * Play's in-app updates, once per process: asks Play (5 s at most) whether a newer version is out
 * and acts on the release's update priority (set when it's uploaded, see `fastlane/Fastfile`):
 * 0–1 nothing; 2 the [pill] ("Update" starts the flexible flow); 3 Play's flexible dialog at once;
 * 4 Play's immediate flow at once, which can be closed; 5 [blocked], the app's own "update to keep
 * using" screen. A flexible download shows in the pill and ends in "Update ready". Installs not
 * from Play (debug, sideloaded) get an error or "not available" and stop quietly. Flow
 * `app.update_check`; log src `update`.
 */
object PlayUpdate {
    private val scope = MainScope()
    private var checked = false
    private var manager: AppUpdateManager? = null
    /** An immediate flow was started: on resume, one Play left half done is resumed. */
    private var immediate = false
    /** Developer's preview: Play isn't asked; "Update" walks the pill through a fake download. */
    private var preview = false

    var pill by mutableStateOf<UpdatePill?>(null)
        private set
    var blocked by mutableStateOf(false)
        private set

    private enum class Level { None, Ready, Resume, Pill, Flexible, Immediate, Block }

    fun check(activity: Activity) {
        if (checked) return
        checked = true
        val manager = AppUpdateManagerFactory.create(activity.applicationContext).also { manager = it }
        scope.launch {
            flow("app.update_check") { f ->
                val info = fresh(manager, f)
                val available = info?.updateAvailability() == UpdateAvailability.UPDATE_AVAILABLE
                val priority = info?.updatePriority() ?: 0
                val flexible = info?.isUpdateTypeAllowed(AppUpdateType.FLEXIBLE) == true
                val immediateOk = info?.isUpdateTypeAllowed(AppUpdateType.IMMEDIATE) == true
                val level = when {
                    info == null -> Level.None
                    info.installStatus() == InstallStatus.DOWNLOADED -> Level.Ready
                    info.updateAvailability() == UpdateAvailability.DEVELOPER_TRIGGERED_UPDATE_IN_PROGRESS -> Level.Resume
                    !available || priority <= 1 -> Level.None
                    priority >= 5 -> Level.Block
                    priority == 4 && immediateOk -> Level.Immediate
                    // A 4 that Play won't run immediately asks as a 3 does.
                    priority >= 3 && flexible -> Level.Flexible
                    priority == 2 && flexible -> Level.Pill
                    else -> Level.None
                }
                Log.info(
                    "update", "checked", f,
                    mapOf(
                        "answered" to "${info != null}", "available" to "$available", "priority" to "$priority", "flexible" to "$flexible",
                        "immediate" to "$immediateOk", "status" to "${info?.installStatus()}", "level" to level.name.lowercase(),
                    ),
                )
                if (info == null || activity.isFinishing) return@flow
                when (level) {
                    Level.None -> {}
                    Level.Ready -> pill = UpdatePill.Ready
                    Level.Pill -> pill = UpdatePill.Available
                    Level.Block -> blocked = true
                    Level.Flexible -> startFlexible(activity, manager, info, f)
                    Level.Resume, Level.Immediate -> startImmediate(activity, manager, info, f)
                }
            }
        }
    }

    /** From onResume: Play's guidance is to resume an immediate update it reports as left in progress. */
    fun resume(activity: Activity) {
        val manager = manager?.takeIf { immediate } ?: return
        manager.appUpdateInfo.addOnSuccessListener { info ->
            if (info.updateAvailability() == UpdateAvailability.DEVELOPER_TRIGGERED_UPDATE_IN_PROGRESS && !activity.isFinishing) {
                scope.launch { flow("app.update_resume") { f -> startImmediate(activity, manager, info, f) } }
            }
        }
    }

    /** The pill's "Update": Play's flexible flow, which asks first. */
    fun update(activity: Activity) = scope.launch {
        flow("app.update") { f ->
            if (preview) {
                for (i in 0..20) { pill = UpdatePill.Downloading(i / 20f); delay(150) }
                pill = UpdatePill.Ready
                return@flow
            }
            val manager = manager ?: return@flow
            fresh(manager, f)?.let { startFlexible(activity, manager, it, f) }
        }
    }

    /** The pill's "Restart": installs the downloaded update. */
    fun restart() = scope.launch {
        flow("app.update_restart") { f ->
            Log.info("update", "restart", f, mapOf("preview" to "$preview"))
            if (preview) { pill = null; preview = false } else manager?.completeUpdate()
        }
    }

    /** The pill's close: gone until the next app start. */
    fun dismiss() {
        pill = null
    }

    /** The blocking screen's "Update": Play's immediate flow, else the app's Play Store page. */
    fun updateNow(activity: Activity) = scope.launch {
        flow("app.update_required") { f ->
            val manager = manager?.takeIf { !preview }
            val info = manager?.let { fresh(it, f) }
            if (manager != null && info?.isUpdateTypeAllowed(AppUpdateType.IMMEDIATE) == true) return@flow startImmediate(activity, manager, info, f)
            Log.info("update", "open store", f, mapOf("preview" to "$preview"))
            val id = activity.packageName
            try {
                activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=$id")))
            } catch (_: ActivityNotFoundException) {
                activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://play.google.com/store/apps/details?id=$id")))
            }
            if (preview) { blocked = false; preview = false }
        }
    }

    /** Developer's preview of a level without Play: [pill] or the blocking screen. */
    fun preview(pill: UpdatePill?, blocked: Boolean = false) {
        preview = pill != null || blocked
        this.pill = pill
        this.blocked = blocked
        Log.info("update", "preview", fields = mapOf("pill" to "${pill?.let { it::class.simpleName }}", "blocked" to "$blocked"))
    }

    /** A new [AppUpdateInfo] (each starts one flow only); null when Play doesn't answer. */
    private suspend fun fresh(manager: AppUpdateManager, f: String): AppUpdateInfo? = try {
        withTimeoutOrNull(5_000) { manager.appUpdateInfo.await() }
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Log.info("update", "play unavailable", f, mapOf("error" to "${e::class.simpleName}"))
        null
    }

    private fun startImmediate(activity: Activity, manager: AppUpdateManager, info: AppUpdateInfo, f: String) {
        immediate = true
        manager.startUpdateFlow(info, activity, AppUpdateOptions.defaultOptions(AppUpdateType.IMMEDIATE))
            .addOnSuccessListener { Log.info("update", "flow answered", f, mapOf("type" to "immediate", "accepted" to "${it == Activity.RESULT_OK}")) }
            .addOnFailureListener { Log.info("update", "flow failed", f, mapOf("type" to "immediate", "error" to "${it::class.simpleName}")) }
    }

    private fun startFlexible(activity: Activity, manager: AppUpdateManager, info: AppUpdateInfo, f: String) {
        val listener = object : InstallStateUpdatedListener {
            override fun onStateUpdate(state: InstallState) {
                when (val status = state.installStatus()) {
                    InstallStatus.PENDING, InstallStatus.DOWNLOADING ->
                        pill = UpdatePill.Downloading(state.bytesDownloaded().toFloat() / state.totalBytesToDownload().coerceAtLeast(1))
                    InstallStatus.DOWNLOADED, InstallStatus.FAILED, InstallStatus.CANCELED -> {
                        manager.unregisterListener(this)
                        Log.info("update", "download ended", f, mapOf("status" to "$status"))
                        pill = if (status == InstallStatus.DOWNLOADED) UpdatePill.Ready else null
                    }
                    else -> {}
                }
            }
        }
        manager.registerListener(listener)
        manager.startUpdateFlow(info, activity, AppUpdateOptions.defaultOptions(AppUpdateType.FLEXIBLE))
            .addOnSuccessListener { result ->
                Log.info("update", "flow answered", f, mapOf("type" to "flexible", "accepted" to "${result == Activity.RESULT_OK}"))
                if (result != Activity.RESULT_OK) manager.unregisterListener(listener)
            }
            .addOnFailureListener {
                Log.info("update", "flow failed", f, mapOf("type" to "flexible", "error" to "${it::class.simpleName}"))
                manager.unregisterListener(listener)
            }
    }
}

private suspend fun <T> Task<T>.await(): T = suspendCancellableCoroutine { c ->
    addOnSuccessListener { c.resume(it) }
    addOnFailureListener { c.resumeWithException(it) }
}
