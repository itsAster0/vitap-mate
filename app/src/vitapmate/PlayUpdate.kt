package vitapmate

import android.app.Activity
import com.google.android.gms.tasks.Task
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
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * Play's flexible in-app update: once per process, asks Play (5 s at most) whether a newer
 * version is out and, if so, lets Play download it in the background; once downloaded, a toast
 * offers "Restart". Installs not from Play (debug, sideloaded) get an error or "not available"
 * and stop quietly. Flow `app.update_check`; log src `update`.
 */
object PlayUpdate {
    private var checked = false

    fun check(activity: Activity) {
        if (checked) return
        checked = true
        val manager = AppUpdateManagerFactory.create(activity.applicationContext)
        MainScope().launch {
            flow("app.update_check") { f ->
                val info = try {
                    withTimeoutOrNull(5_000) { manager.appUpdateInfo.await() }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.info("update", "play unavailable", f, mapOf("error" to "${e::class.simpleName}"))
                    return@flow
                }
                val available = info?.updateAvailability() == UpdateAvailability.UPDATE_AVAILABLE
                val flexible = info?.isUpdateTypeAllowed(AppUpdateType.FLEXIBLE) == true
                Log.info("update", "checked", f, mapOf("answered" to "${info != null}", "available" to "$available", "flexible" to "$flexible", "status" to "${info?.installStatus()}"))
                when {
                    info == null -> {}
                    info.installStatus() == InstallStatus.DOWNLOADED -> ready(manager)
                    available && flexible && !activity.isFinishing -> {
                        val listener = object : InstallStateUpdatedListener {
                            override fun onStateUpdate(state: InstallState) {
                                val status = state.installStatus()
                                if (status == InstallStatus.DOWNLOADED || status == InstallStatus.FAILED || status == InstallStatus.CANCELED) {
                                    manager.unregisterListener(this)
                                    Log.info("update", "download ended", f, mapOf("status" to "$status"))
                                    if (status == InstallStatus.DOWNLOADED) ready(manager)
                                }
                            }
                        }
                        manager.registerListener(listener)
                        manager.startUpdateFlow(info, activity, AppUpdateOptions.defaultOptions(AppUpdateType.FLEXIBLE))
                            .addOnSuccessListener { result ->
                                Log.info("update", "flow answered", f, mapOf("accepted" to "${result == Activity.RESULT_OK}"))
                                if (result != Activity.RESULT_OK) manager.unregisterListener(listener)
                            }
                            .addOnFailureListener {
                                Log.info("update", "flow failed", f, mapOf("error" to "${it::class.simpleName}"))
                                manager.unregisterListener(listener)
                            }
                    }
                }
            }
        }
    }

    private fun ready(manager: AppUpdateManager) =
        showToast("Update ready", "Restart to use the new version.", "Restart" to { manager.completeUpdate() })
}

private suspend fun <T> Task<T>.await(): T = suspendCancellableCoroutine { c ->
    addOnSuccessListener { c.resume(it) }
    addOnFailureListener { c.resumeWithException(it) }
}
