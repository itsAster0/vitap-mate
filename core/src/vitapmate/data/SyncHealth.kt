package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import vitapmate.core.SyncHealth

/**
 * How background sync stands, as Rust last told it ([vitapmate.core.AppEvents.onSyncHealth]):
 * Rust keeps it with the settings and changes it (`Core.setSyncHealth`, `syncHealthAfterOtp`,
 * `syncHealthResumeIf`, and after each background run).
 */
object SyncHealthState {
    var health by mutableStateOf<SyncHealth>(SyncHealth.Active(0u))
        private set

    /** Why it is paused, as shown ("Paused: …"); null while active. */
    var pausedText by mutableStateOf<String?>(null)
        private set

    internal fun mirror(now: SyncHealth, paused: String?) {
        health = now
        pausedText = paused
    }
}
