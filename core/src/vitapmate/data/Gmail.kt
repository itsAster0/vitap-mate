package vitapmate.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.GmailAccount
import vitapmate.core.GmailClient
import vitapmate.core.GmailException as CoreGmailException
import vitapmate.core.GmailOtp
import vitapmate.core.SyncResume
import vitapmate.core.gmailParseClient
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Thin Kotlin wrapper over Rust's Gmail BYOK OTP implementation (`gmail.rs`): keeps the Compose
 * mirror of [account] and wraps [connect] in [SignInService] so Android keeps the loopback
 * redirect alive while the student signs in in the browser.
 */
object Gmail {
    const val SENDER = "noreply.sdc@vitap.ac.in"
    private const val CLIENT_SUFFIX = ".apps.googleusercontent.com"

    /** A Desktop-app OAuth client from Google Cloud's JSON. No `toString`: it holds the secret. */
    class Client(internal val raw: GmailClient) {
        val id: String get() = raw.id
        val secret: String? get() = raw.secret
        val project: String get() = raw.project

        /** `123456…abcd.apps.googleusercontent.com`, for showing. */
        val shortId: String
            get() = id.removeSuffix(CLIENT_SUFFIX).let { if (it.length <= 10) it else "${it.take(6)}…${it.takeLast(4)}" } + CLIENT_SUFFIX
    }

    /** The connected account. No `toString`: it holds the tokens. */
    class Account(private val raw: GmailAccount) {
        val email: String get() = raw.email
        val client: Client = Client(raw.client)
        val refreshToken: String get() = raw.refreshToken
        val deleteAfterReading: Boolean get() = raw.deleteAfterReading
        val grantEndsAtMs: Long get() = raw.grantEndsAtMs
        val ended: Boolean get() = raw.ended

        /** The grant ended, or its end has passed. */
        val needsReconnect get() = ended || (grantEndsAtMs > 0 && System.currentTimeMillis() >= grantEndsAtMs)

        /** Days until the grant ends (0 on the last day), or null when it doesn't end. */
        val daysLeft get() = grantEndsAtMs.takeIf { it > 0 }?.let { ((it - System.currentTimeMillis()) / 86_400_000).coerceAtLeast(0) }
    }

    /** A code found in the mailbox; [messageId] so a code VTOP turns down isn't tried again. */
    class Otp(internal val raw: GmailOtp) {
        val code: String get() = raw.code
        val messageId: String get() = raw.messageId
    }

    /** What went wrong: [message] for the student, [id] stable, for logs. */
    sealed class GmailError(val id: String, val message: String) {
        /** Google wasn't reached, or its answer didn't come back: worth another try. */
        sealed class Network(id: String, message: String) : GmailError(id, message)

        class Unreachable(message: String) : Network("unreachable", message)
        class Offline(message: String) : Network("offline", message)
        class Other(id: String, message: String) : GmailError(id, message)
    }

    class GmailException(val error: GmailError, cause: Throwable? = null) : Exception(error.message, cause)

    /** The connected account, or null. Compose mirror of Rust's saved account. */
    var account by mutableStateOf<Account?>(null)
        private set

    private lateinit var app: Context
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /** Call once at start so [connect] can start [SignInService]. */
    fun init(context: Context) {
        app = context.applicationContext
    }

    /** Reads the saved account from Rust (`SecretBox`). */
    internal fun load(core: Core, flow: String) {
        account = core.gmailAccount(flow)?.let(::Account)
    }

    /** The client in Google Cloud's JSON ([bytes]), or a [GmailException] saying what's wrong. */
    fun parseClient(bytes: ByteArray): Client = mapErrors { Client(gmailParseClient(bytes)) }

    /**
     * Signs in with [client] in the browser (Rust opens the URL through `UrlOpener` and runs the
     * loopback listener + PKCE exchange), wrapped in [SignInService].
     */
    suspend fun connect(client: Client, regno: String, flow: String): Account = withContext(Dispatchers.IO) {
        SignInService.start(app)
        try {
            val saved = mapErrors { AppCore.await().gmailConnect(flow, client.raw, regno) }
            Account(saved).also {
                account = it
                Settings.otpSource = OtpSource.Gmail
                AppCore.await().syncHealthResumeIf(flow, SyncResume.GMAIL)
            }
        } finally {
            SignInService.stop(app)
        }
    }

    /** Forgets the account and revokes its grant (best effort). */
    suspend fun disconnect(flow: String) = withContext(Dispatchers.IO) {
        account = null
        AppCore.await().gmailDisconnect(flow)
    }

    /** Updates whether OTP mails are trashed after reading, persisted by Rust. */
    fun setDeleteAfterReading(on: Boolean) {
        scope.launch {
            runCatching {
                flow("gmail.delete_after") { f ->
                    account = AppCore.await().gmailSetDeleteAfterReading(f, on)?.let(::Account)
                }
            }.onFailure { Log.warn("core.gmail", "save failed: ${it::class.simpleName}") }
        }
    }

    /**
     * One look at the mailbox for VTOP's OTP sent after [since] (unix seconds), skipping the
     * messages in [tried]; null when it hasn't arrived.
     */
    suspend fun findOtp(since: Long, tried: Set<String>, flow: String): Otp? = withContext(Dispatchers.IO) {
        val core = AppCore.await()
        try {
            mapErrors { core.gmailFindOtp(flow, since * 1000L, tried.toList())?.let(::Otp) }
        } catch (e: GmailException) {
            if (e.error.id == "ended") account = core.gmailAccount(flow)?.let(::Account)
            throw e
        }
    }

    /** Trashes the OTP mail, or marks it read; best effort. */
    suspend fun tidyUp(otp: Otp, flow: String) = withContext(Dispatchers.IO) {
        attempt { AppCore.await().gmailTidyUp(flow, otp.raw) }
            .onFailure { Log.warn("core.gmail", "tidy up failed: ${it::class.simpleName}", flow) }
    }

    private inline fun <T> mapErrors(block: () -> T): T = try {
        block()
    } catch (e: CoreGmailException) {
        throw GmailException(e.toGmailError(), e)
    }

    private fun CoreGmailException.toGmailError(): GmailError = when (this) {
        is CoreGmailException.Unreachable -> GmailError.Unreachable(reason)
        is CoreGmailException.Offline -> GmailError.Offline(reason)
        is CoreGmailException.BadFile -> GmailError.Other("bad_file", reason)
        is CoreGmailException.BadJson -> GmailError.Other("bad_json", reason)
        is CoreGmailException.WrongClientType -> GmailError.Other("wrong_client_type", reason)
        is CoreGmailException.BadClientId -> GmailError.Other("bad_client_id", reason)
        is CoreGmailException.NoTokens -> GmailError.Other("no_tokens", reason)
        is CoreGmailException.NoScope -> GmailError.Other("no_scope", reason)
        is CoreGmailException.NoEmail -> GmailError.Other("no_email", reason)
        is CoreGmailException.WrongAccount -> GmailError.Other("wrong_account", reason)
        is CoreGmailException.GmailOff -> GmailError.Other("gmail_off", reason)
        is CoreGmailException.NotConnected -> GmailError.Other("not_connected", reason)
        is CoreGmailException.SignInRefused -> GmailError.Other(code, reason)
        is CoreGmailException.BadState -> GmailError.Other("bad_state", reason)
        is CoreGmailException.Timeout -> GmailError.Other("timeout", reason)
        is CoreGmailException.Unauthorized -> GmailError.Other("unauthorized", reason)
        is CoreGmailException.Forbidden -> GmailError.Other("forbidden", reason)
        is CoreGmailException.InvalidGrant -> GmailError.Other("invalid_grant", reason)
        is CoreGmailException.InvalidClient -> GmailError.Other("invalid_client", reason)
        is CoreGmailException.Http -> GmailError.Other("http_$status", reason)
        is CoreGmailException.Ended -> GmailError.Other("ended", reason)
    }
}
