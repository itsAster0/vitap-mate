package vitapmate.features.login

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.CoreException
import vitapmate.data.AppCore
import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.observability.Log

/**
 * One OTP auto-fill at a time: the student picks mail forwarding or Gmail ([Settings.otpSource])
 * in the OTP auto-fill sheet, and only that one fills in the code. The other keeps its setup
 * (switching back needs nothing redone) but is never read.
 */
object OtpAutofill {
    /** The pick; before there is one, whichever is set up (forwarding first). */
    fun selected(): OtpSource = Settings.otpSource ?: if (!MailForwarding.isOn && Gmail.account != null) OtpSource.Gmail else OtpSource.Forwarding

    fun ready(source: OtpSource) = when (source) {
        OtpSource.Forwarding -> MailForwarding.isOn
        OtpSource.Gmail -> Gmail.account?.needsReconnect == false
    }

    /** The one the OTP prompt reads, or null when the pick isn't set up (or needs reconnecting). [MailForwarding.load] first. */
    fun inUse(): OtpSource? = selected().takeIf(::ready)

    /**
     * Gets VTOP's pending OTP accepted with nobody watching (background sync): pulls the code
     * from the auto-fill in use every few seconds for up to [waitMs] and submits it; a code VTOP
     * turns down is skipped. Null when it worked, else why not ("Gmail needs reconnecting"), so
     * the caller can stop quietly and show it in Settings. Never logs the code.
     */
    suspend fun solve(flow: String, waitMs: Long = 120_000): String? {
        MailForwarding.load("sync")
        val source = inUse() ?: return if (selected() == OtpSource.Gmail) "Gmail needs reconnecting" else "OTP auto-fill isn't set up"
        val core = AppCore.await()
        // VTOP mailed the code just now; a minute's slack for the phone's clock.
        val since = System.currentTimeMillis() / 1000 - 60
        // Codes already submitted, and Gmail messages already read.
        val tried = mutableSetOf<String>()
        val read = mutableSetOf<String>()
        var gmail: Gmail.Otp? = null
        var why: String? = "No code came in time"
        withTimeoutOrNull(waitMs) {
            while (true) {
                val code = try {
                    withContext(Dispatchers.IO) {
                        when (source) {
                            OtpSource.Forwarding -> core.fetchForwardedOtp(flow, since)
                            OtpSource.Gmail -> Gmail.findOtp(since, read, flow)?.also { gmail = it; read += it.messageId }?.code
                        }
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Gmail.GmailException) {
                    if (e.code != "offline" && e.code != "unreachable") {
                        why = if (Gmail.account?.needsReconnect == true) "Gmail needs reconnecting" else "Gmail refused: ${e.code}"
                        return@withTimeoutOrNull
                    }
                    null
                } catch (e: Throwable) {
                    Log.warn("sync", "otp pull failed: ${e::class.simpleName}", flow)
                    null
                }
                if (code != null && code !in tried) {
                    tried += code
                    try {
                        withContext(Dispatchers.IO) { core.submitOtp(flow, code) }
                        Log.info("sync", "otp accepted", flow, mapOf("source" to source.name.lowercase()))
                        gmail?.let { Gmail.tidyUp(it, flow) }
                        why = null
                        return@withTimeoutOrNull
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: CoreException.OtpRejected) {
                        Log.info("sync", "otp turned down", flow)
                    } catch (e: Throwable) {
                        why = "VTOP didn't take the code"
                        Log.warn("sync", "otp submit failed: ${e::class.simpleName}", flow)
                        return@withTimeoutOrNull
                    }
                }
                delay(3_000)
            }
        }
        why?.let { Log.info("sync", "otp not solved: $it", flow) }
        return why
    }
}
