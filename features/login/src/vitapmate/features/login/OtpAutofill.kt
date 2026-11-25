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
import vitapmate.core.ForwardingReadiness
import vitapmate.core.GmailGrant
import vitapmate.core.LiveTopic
import vitapmate.data.AppCore
import vitapmate.data.Gmail
import vitapmate.data.LiveWake
import vitapmate.data.OtpPrompt
import vitapmate.data.OtpSource
import vitapmate.data.PauseReason
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
    fun selected(): OtpSource = Settings.otpSource ?: if (MailForwarding.readiness == ForwardingReadiness.OFF && Gmail.account != null) OtpSource.Gmail else OtpSource.Forwarding

    fun ready(source: OtpSource) = when (source) {
        OtpSource.Forwarding -> MailForwarding.readiness != ForwardingReadiness.OFF
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

    /**
     * Gets VTOP's pending OTP accepted with nobody watching (background sync): pulls the code
     * from the auto-fill in use every few seconds until it expires ([LIFETIME_S] after
     * [issuedAt], VTOP's unix seconds; unknown: from now) and submits it; a code VTOP turns down
     * is skipped. VTOP mails late at times: when none came, asks once for a new code and waits
     * its full lifetime too. Null when it worked, else the typed [PauseReason], so the caller can
     * stop quietly and show it. Never logs the code.
     */
    suspend fun solve(flow: String, issuedAt: Long?): PauseReason? {
        MailForwarding.load("sync")
        val source = inUse() ?: return if (selected() == OtpSource.Gmail) PauseReason.GmailReconnect else PauseReason.NotSetUp
        val core = AppCore.await()
        var issued = issuedAt ?: (System.currentTimeMillis() / 1000)
        // Codes already submitted, and Gmail messages already read.
        val tried = mutableSetOf<String>()
        val read = mutableSetOf<String>()
        var gmail: Gmail.Otp? = null
        var why: PauseReason? = PauseReason.NoCode
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
                // Forwarded mail wakes the loop as it lands; Gmail is pulled every few seconds.
                val wake = if (source == OtpSource.Forwarding) LiveWake(this, LiveTopic.MAIL_INBOX) else null
                try {
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
                                why = if (Gmail.account?.needsReconnect == true) PauseReason.GmailReconnect else PauseReason.GmailRefused(e.code)
                                return@withTimeoutOrNull
                            }
                            null
                        } catch (e: CoreException.ForwardingOff) {
                            // The address is gone: nothing will come to it in this wait.
                            why = PauseReason.ForwardingOff(e.kind)
                            return@withTimeoutOrNull
                        } catch (e: Throwable) {
                            Log.warn("sync", "otp pull failed: ${e::class.simpleName}", flow)
                            null
                        }
                        if (code != null && code !in tried) {
                            tried += code
                            try {
                                withContext(Dispatchers.IO) { core.submitOtp(flow, code) }
                                Log.info("sync", "otp accepted", flow, mapOf("source" to source.name.lowercase()))
                                // The app's prompt for the same sign-in, if open, has nothing left to do.
                                if (OtpPrompt.message != null) withContext(Dispatchers.Main) { OtpPrompt.done() }
                                gmail?.let { Gmail.tidyUp(it, flow) }
                                why = null
                                return@withTimeoutOrNull
                            } catch (e: CancellationException) {
                                throw e
                            } catch (_: CoreException.OtpRejected) {
                                Log.info("sync", "otp turned down", flow)
                            } catch (e: Throwable) {
                                why = PauseReason.CodeRefused
                                Log.warn("sync", "otp submit failed: ${e::class.simpleName}", flow)
                                return@withTimeoutOrNull
                            }
                        }
                        if (wake != null) wake.next(3_000) else delay(3_000)
                    }
                } finally {
                    wake?.stop()
                }
            }
            if (why != PauseReason.NoCode) break
        }
        why?.let { Log.info("sync", "otp not solved: ${it.id}", flow) }
        return why
    }
}
