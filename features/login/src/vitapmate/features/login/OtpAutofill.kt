package vitapmate.features.login

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.CoreException
import vitapmate.core.GmailGrant
import vitapmate.data.AppCore
import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.observability.flow

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

    /** Gmail access for the server's saved password while Gmail is in use; null means forwarding. */
    fun serverGrant(): GmailGrant? = Gmail.account?.takeIf { inUse() == OtpSource.Gmail }
        ?.let { GmailGrant(it.refreshToken, it.client.id, it.client.secret, it.deleteAfterReading) }

    /** What the server would need to hear about: the auto-fill in use and Gmail's grant. Holds the token; never log it. */
    internal fun serverKey(): Any? = when (inUse()) {
        null -> null
        OtpSource.Forwarding -> OtpSource.Forwarding
        OtpSource.Gmail -> Gmail.account?.let { it.refreshToken to it.deleteAfterReading }
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /**
     * Tells the server the auto-fill now in use (Gmail with its grant, or forwarding), so its saved
     * password signs in the same way. Only when the server keeps a password and the pick is set up.
     */
    internal fun tellServer() {
        if (inUse() == null) return
        val grant = serverGrant()
        scope.launch {
            try {
                flow("otp_autofill.server") { f ->
                    val sent = AppCore.await().resendServerPassword(f, grant)
                    Log.info("ui.otp_autofill", if (sent) "server told the otp method" else "server keeps no password", f)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Throwable) {
                // Logged by flow(); the server keeps the old method until the next change.
            }
        }
    }

    /**
     * How long a code stays good, in seconds. Assumed: VTOP's mail and pages don't say; the OTP
     * prompt counts down from it too.
     */
    const val LIFETIME_S = 180

    /** [solve]'s answer when no code came, even after a resend. */
    const val NO_CODE = "No code came in time"

    /**
     * Gets VTOP's pending OTP accepted with nobody watching (background sync): pulls the code
     * from the auto-fill in use every few seconds until it expires ([LIFETIME_S] after
     * [issuedAt], VTOP's unix seconds; unknown: from now) and submits it; a code VTOP turns down
     * is skipped. VTOP mails late at times: when none came, asks once for a new code and waits
     * its full lifetime too. Null when it worked, else why not ("Gmail needs reconnecting",
     * [NO_CODE]), so the caller can stop quietly and show it. Never logs the code.
     */
    suspend fun solve(flow: String, issuedAt: Long?): String? {
        MailForwarding.load("sync")
        val source = inUse() ?: return if (selected() == OtpSource.Gmail) "Gmail needs reconnecting" else "OTP auto-fill isn't set up"
        val core = AppCore.await()
        var issued = issuedAt ?: (System.currentTimeMillis() / 1000)
        // Codes already submitted, and Gmail messages already read.
        val tried = mutableSetOf<String>()
        val read = mutableSetOf<String>()
        var gmail: Gmail.Otp? = null
        var why: String? = NO_CODE
        for (round in 0..1) {
            if (round == 1) {
                try {
                    withContext(Dispatchers.IO) { core.resendOtp(flow) }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    Log.warn("sync", "otp resend failed: ${e::class.simpleName}", flow)
                    break
                }
                Log.info("sync", "otp resent", flow)
                issued = System.currentTimeMillis() / 1000
            }
            // Mail since the code was issued; with the source's own slack, about a minute for the clocks.
            val since = issued - 30
            withTimeoutOrNull(((issued + LIFETIME_S) * 1000 - System.currentTimeMillis()).coerceAtLeast(0)) {
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
            if (why != NO_CODE) break
        }
        why?.let { Log.info("sync", "otp not solved: $it", flow) }
        return why
    }
}
