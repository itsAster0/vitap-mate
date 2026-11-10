package vitapmate.data

import android.content.Context
import android.util.Base64
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.IOException
import java.net.ConnectException
import java.net.InetAddress
import java.net.ServerSocket
import java.net.SocketTimeoutException
import java.net.URLDecoder
import java.net.URLEncoder
import java.net.UnknownHostException
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import okhttp3.FormBody
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import vitapmate.observability.Log

/**
 * VTOP's sign-in OTP read from the student's Gmail with their own Google OAuth client (BYOK, the
 * Flutter app's `google_email_oauth_service.dart`): they import a Desktop-app client JSON, sign in
 * in the browser (loopback redirect with PKCE), and the OTP prompt then finds the newest mail
 * from [SENDER], fills in its 6-digit code and trashes it (or marks it read).
 *
 * The account (refresh token, client secret) is sealed with the Keystore ([Secrets]). Lines go to
 * `core.gmail`; tokens, codes, mail and the email address are never logged.
 */
object Gmail {
    const val SENDER = "noreply.sdc@vitap.ac.in"
    private const val FILE = "gmail"
    private const val SRC = "core.gmail"
    private const val SCOPE = "https://www.googleapis.com/auth/gmail.modify"
    private const val AUTH = "https://accounts.google.com/o/oauth2/v2/auth"
    private const val TOKEN = "https://oauth2.googleapis.com/token"
    private const val REVOKE = "https://oauth2.googleapis.com/revoke"
    private const val API = "https://gmail.googleapis.com/gmail/v1/users/me"
    private const val CLIENT_SUFFIX = ".apps.googleusercontent.com"

    /** VTOP's, Gmail's and the phone's clocks differ a little. */
    private const val CLOCK_SLACK_S = 30

    /** How long the browser sign-in may take. */
    private const val SIGN_IN_MS = 10 * 60_000L

    /** A Desktop-app OAuth client from Google Cloud's JSON. No `toString`: it holds the secret. */
    class Client(val id: String, val secret: String?, val project: String) {
        /** `123456…abcd.apps.googleusercontent.com`, for showing. */
        val shortId: String
            get() = id.removeSuffix(CLIENT_SUFFIX).let { if (it.length <= 10) it else "${it.take(6)}…${it.takeLast(4)}" } + CLIENT_SUFFIX
    }

    /** The connected account. No `toString`: it holds the tokens. */
    class Account(
        val email: String,
        val client: Client,
        val refreshToken: String,
        val accessToken: String,
        val expiresAtMs: Long,
        val deleteAfterReading: Boolean,
        /** When Google ends the grant: a project in Testing gets 7 days (`refresh_token_expires_in`); 0 when it didn't say. */
        val grantEndsAtMs: Long,
        /** Google turned the grant down: [reconnect] with the same client. */
        val ended: Boolean = false,
    ) {
        fun copy(accessToken: String = this.accessToken, expiresAtMs: Long = this.expiresAtMs, deleteAfterReading: Boolean = this.deleteAfterReading, ended: Boolean = this.ended) =
            Account(email, client, refreshToken, accessToken, expiresAtMs, deleteAfterReading, grantEndsAtMs, ended)

        /** The grant ended, or its end has passed. */
        val needsReconnect get() = ended || (grantEndsAtMs > 0 && System.currentTimeMillis() >= grantEndsAtMs)

        /** Days until the grant ends (0 on the last day), or null when it doesn't end. */
        val daysLeft get() = grantEndsAtMs.takeIf { it > 0 }?.let { ((it - System.currentTimeMillis()) / 86_400_000).coerceAtLeast(0) }
    }

    /** A code found in the mailbox; [messageId] so a code VTOP turns down isn't tried again. */
    class Otp(val code: String, val messageId: String)

    /** What went wrong, for the student; [code] is stable, for logs. */
    class GmailException(val code: String, message: String) : Exception(message)

    /** The connected account, or null. Compose state. */
    var account by mutableStateOf<Account?>(null)
        private set

    private lateinit var app: Context
    private val lock = Mutex()
    private val http by lazy {
        OkHttpClient.Builder().callTimeout(20, TimeUnit.SECONDS).followRedirects(false).build()
    }

    /** Call once at start, then [load] off the main thread. */
    fun init(context: Context) {
        app = context.applicationContext
    }

    /** Reads the saved account (a Keystore decrypt). */
    fun load() {
        account = Secrets.read(app, FILE)?.let { runCatching { decode(JSONObject(String(it))) }.getOrNull() }
    }

    /** The client in Google Cloud's JSON ([bytes]), or a [GmailException] saying what's wrong. */
    fun parseClient(bytes: ByteArray): Client {
        if (bytes.isEmpty() || bytes.size > 64 * 1024) throw GmailException("bad_file", "This isn't a Google OAuth client file.")
        val json = runCatching { JSONObject(String(bytes)) }.getOrNull() ?: throw GmailException("bad_json", "This isn't a valid Google OAuth JSON file.")
        if (json.has("web")) throw GmailException("wrong_client_type", "Create a Desktop app OAuth client, not a Web application client.")
        val installed = json.optJSONObject("installed")
            ?: throw GmailException("wrong_client_type", "Pick the JSON downloaded for a Desktop app OAuth client.")
        val id = installed.optString("client_id").trim()
        if (!id.endsWith(CLIENT_SUFFIX) || id.length <= CLIENT_SUFFIX.length) {
            throw GmailException("bad_client_id", "The file has no valid Google OAuth client ID.")
        }
        val secret = installed.optString("client_secret").trim().ifEmpty { null }
        return Client(id, secret, installed.optString("project_id").trim().ifEmpty { "Google Cloud project" })
    }

    /**
     * Signs in with [client] in the browser ([open] shows the URL) and saves the account: a
     * loopback listener on 127.0.0.1 gets Google's redirect, the code is exchanged with PKCE,
     * the account must be the student's college Gmail ending in their registration number
     * [regno], and Gmail must answer.
     * Cancelling the coroutine stops the listener.
     */
    suspend fun connect(client: Client, regno: String, flow: String, open: (String) -> Unit): Account = withContext(Dispatchers.IO) {
        try {
            signIn(client, regno, flow, open)
        } finally {
            SignInService.stop(app)
        }
    }

    private suspend fun signIn(client: Client, regno: String, flow: String, open: (String) -> Unit): Account {
        val verifier = random(32)
        val challenge = b64(MessageDigest.getInstance("SHA-256").digest(verifier.toByteArray()))
        val state = random(16)
        val (code, redirect) = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1")).use { server ->
            val redirect = "http://127.0.0.1:${server.localPort}"
            val url = AUTH + "?" + query(
                "client_id" to client.id,
                "redirect_uri" to redirect,
                "response_type" to "code",
                "scope" to "openid email $SCOPE",
                "code_challenge" to challenge,
                "code_challenge_method" to "S256",
                "state" to state,
                "access_type" to "offline",
                "prompt" to "consent",
                "hd" to "vitapstudent.ac.in",
            )
            Log.info(SRC, "sign-in opened", flow, mapOf("port" to "${server.localPort}"))
            // Awake (with its network) while the student is in the browser, until it's all done.
            SignInService.start(app)
            withContext(Dispatchers.Main) { open(url) }
            awaitRedirect(server, state, flow) to redirect
        }
        // Past SignInService's 3 minutes Android may block the network of the app in the background
        // while the student is still in the browser: try until they're back (the code stays valid for minutes).
        val tokens = untilReachable(flow) {
            post(
                TOKEN,
                flow,
                "code" to code,
            "client_id" to client.id,
            "client_secret" to client.secret,
            "redirect_uri" to redirect,
                "grant_type" to "authorization_code",
                "code_verifier" to verifier,
            )
        }
        val access = tokens.optString("access_token")
        val refresh = tokens.optString("refresh_token")
        try {
            if (access.isEmpty() || refresh.isEmpty()) throw GmailException("no_tokens", "Google didn't send the tokens. Try again.")
            if (SCOPE !in tokens.optString("scope").split(' ')) {
                throw GmailException("no_scope", "Tick the Gmail permission on Google's screen, then try again.")
            }
            val email = emailFrom(tokens.optString("id_token")) ?: throw GmailException("no_email", "Couldn't read your Google account's email.")
            collegeAccountProblem(email, regno)?.let { throw GmailException("wrong_account", it) }
            verifyGmail(access, flow)
            val ends = tokens.optLong("refresh_token_expires_in", 0).takeIf { it > 0 }?.let { System.currentTimeMillis() + it * 1000 } ?: 0
            val saved = Account(email, client, refresh, access, expiry(tokens), account?.deleteAfterReading ?: true, ends)
            save(saved)
            Settings.otpSource = OtpSource.Gmail
            Log.info(SRC, "connected", flow, mapOf("grant_days" to "${saved.daysLeft ?: "-"}"))
            return saved
        } catch (e: Throwable) {
            revoke(refresh.ifEmpty { access }, flow)
            throw e
        }
    }

    /** Forgets the account and revokes its grant (best effort). */
    suspend fun disconnect(flow: String) = withContext(Dispatchers.IO) {
        val old = lock.withLock {
            account.also {
                account = null
                Secrets.delete(app, FILE)
            }
        }
        old?.let { revoke(it.refreshToken, flow) }
        Log.info(SRC, "disconnected", flow)
    }

    fun setDeleteAfterReading(on: Boolean) {
        account?.let { save(it.copy(deleteAfterReading = on)) }
    }

    /**
     * One look at the mailbox for VTOP's OTP sent after [since] (unix seconds), skipping the
     * messages in [tried]; null when it hasn't arrived. A token Gmail rejects is refreshed once.
     */
    suspend fun findOtp(since: Long, tried: Set<String>, flow: String): Otp? = withContext(Dispatchers.IO) {
        val a = account ?: return@withContext null
        if (a.needsReconnect) throw ENDED
        var token = token(flow, force = false)
        try {
            newest(token, since - CLOCK_SLACK_S, tried, flow)
        } catch (e: GmailException) {
            if (e.code != "unauthorized") throw e
            token = token(flow, force = true)
            newest(token, since - CLOCK_SLACK_S, tried, flow)
        }
    }

    /** Trashes the OTP mail, or marks it read; best effort. */
    suspend fun tidyUp(otp: Otp, flow: String) = withContext(Dispatchers.IO) {
        val a = account ?: return@withContext
        runCatching {
            val token = token(flow, force = false)
            val (path, body) = if (a.deleteAfterReading) "/messages/${otp.messageId}/trash" to "" else
                "/messages/${otp.messageId}/modify" to """{"removeLabelIds":["UNREAD"]}"""
            send(Request.Builder().url(API + path).header("Authorization", "Bearer $token").post(body.toRequestBody(JSON)).build(), flow)
        }.onFailure { Log.warn(SRC, "tidy up failed: ${it::class.simpleName}", flow) }
    }

    // --- Gmail ---

    private fun newest(token: String, since: Long, tried: Set<String>, flow: String): Otp? {
        val list = get("$API/messages?" + query("maxResults" to "3", "q" to "from:$SENDER after:$since"), token, flow)
        val ids = list.optJSONArray("messages").objects().map { it.optString("id") }.filter { it.isNotEmpty() && it !in tried }
        for (id in ids) {
            val message = get("$API/messages/$id?format=full", token, flow)
            if ((message.optString("internalDate").toLongOrNull() ?: 0) < since * 1000 || !fromVtop(message)) continue
            sixDigits(text(message))?.let { return Otp(it, id) }
        }
        return null
    }

    private fun fromVtop(message: JSONObject) = message.optJSONObject("payload")?.optJSONArray("headers").objects()
        .any { it.optString("name").equals("from", true) && SENDER in it.optString("value").lowercase() }

    /** The snippet and every decoded body part, quoted-printable decoded. */
    private fun text(message: JSONObject): String {
        val out = StringBuilder(message.optString("snippet")).append('\n')
        fun append(part: JSONObject) {
            part.optJSONObject("body")?.optString("data")?.takeIf { it.isNotEmpty() }?.let { data ->
                runCatching { Base64.decode(data.trimEnd('='), Base64.URL_SAFE or Base64.NO_PADDING or Base64.NO_WRAP) }
                    .getOrNull()?.let { out.append(String(it)).append('\n') }
            }
            part.optJSONArray("parts").objects().forEach(::append)
        }
        message.optJSONObject("payload")?.let(::append)
        return quotedPrintable(out.toString())
    }

    private fun quotedPrintable(input: String): String {
        val text = input.replace("=\r\n", "").replace("=\n", "")
        val out = java.io.ByteArrayOutputStream()
        var i = 0
        while (i < text.length) {
            val hex = if (text[i] == '=' && i + 2 < text.length) text.substring(i + 1, i + 3).toIntOrNull(16) else null
            if (hex != null) {
                out.write(hex)
                i += 3
            } else {
                out.write(text.substring(i, i + 1).toByteArray())
                i++
            }
        }
        return out.toString(Charsets.UTF_8.name())
    }

    private fun sixDigits(text: String) = Regex("(?<!\\d)\\d{6}(?!\\d)").find(text)?.value

    private fun verifyGmail(token: String, flow: String) {
        try {
            get("$API/profile", token, flow)
        } catch (e: GmailException) {
            if (e.code == "forbidden") throw GmailException("gmail_off", "Gmail didn't let the app in. Turn on the Gmail API in your project (step 2), wait a minute and connect again.")
            throw e
        }
    }

    private fun get(url: String, token: String, flow: String): JSONObject =
        send(Request.Builder().url(url).header("Authorization", "Bearer $token").build(), flow)

    // --- OAuth ---

    /** A live access token, refreshed when it expires within a minute (or always, with [force]). */
    private suspend fun token(flow: String, force: Boolean): String = lock.withLock {
        val a = account ?: throw GmailException("not_connected", "Gmail isn't connected.")
        if (!force && a.expiresAtMs > System.currentTimeMillis() + 60_000) return@withLock a.accessToken
        val tokens = try {
            post(TOKEN, flow, "refresh_token" to a.refreshToken, "client_id" to a.client.id, "client_secret" to a.client.secret, "grant_type" to "refresh_token")
        } catch (e: GmailException) {
            if (e.code == "invalid_grant") {
                save(a.copy(ended = true))
                Log.info(SRC, "grant ended", flow)
                throw ENDED
            }
            throw e
        }
        val fresh = a.copy(accessToken = tokens.optString("access_token"), expiresAtMs = expiry(tokens))
        save(fresh)
        fresh.accessToken
    }

    /** Waits for Google's redirect to the listener and answers it; the authorization code. */
    private suspend fun awaitRedirect(server: ServerSocket, state: String, flow: String): String {
        server.soTimeout = 1_000
        val deadline = System.currentTimeMillis() + SIGN_IN_MS
        while (System.currentTimeMillis() < deadline) {
            kotlin.coroutines.coroutineContext.ensureActive()
            val socket = try {
                server.accept()
            } catch (_: SocketTimeoutException) {
                continue
            }
            socket.use {
                it.soTimeout = 5_000
                val line = runCatching { it.getInputStream().bufferedReader().readLine() }.getOrNull().orEmpty()
                val target = line.split(' ').getOrNull(1).orEmpty()
                val params = target.substringAfter('?', "").split('&').filter { p -> '=' in p }
                    .associate { p -> p.substringBefore('=') to URLDecoder.decode(p.substringAfter('='), "UTF-8") }
                if (params.isEmpty()) {
                    reply(it, 404, "Not found")
                    return@use
                }
                val error = params["error"]
                val code = params["code"]
                val done = code != null && params["state"] == state
                reply(it, 200, if (done) "Almost done: go back to Vitap Mate to finish connecting Gmail." else "Google sign-in didn't finish. Go back to Vitap Mate and try again.")
                if (error != null) {
                    Log.info(SRC, "sign-in refused", flow, mapOf("error" to error))
                    throw GmailException(error, if (error == "access_denied") "Google denied access. Is your college email a test user (step 4)?" else "Google sign-in failed: $error.")
                }
                if (!done) throw GmailException("bad_state", "Google's answer didn't match this sign-in. Try again.")
                return code
            }
        }
        throw GmailException("timeout", "Google sign-in took too long. Try again.")
    }

    private fun reply(socket: java.net.Socket, status: Int, text: String) {
        val html = "<!doctype html><meta name=viewport content=\"width=device-width\"><body style=\"font:18px sans-serif;padding:24px\">$text</body>"
        val bytes = html.toByteArray()
        runCatching {
            socket.getOutputStream().apply {
                write("HTTP/1.1 $status ${if (status == 200) "OK" else "Not Found"}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: ${bytes.size}\r\nConnection: close\r\n\r\n".toByteArray())
                write(bytes)
                flush()
            }
        }
    }

    /** Runs [call] again every 2 s for up to 3 min while it fails before reaching Google. */
    private suspend fun <T> untilReachable(flow: String, call: () -> T): T {
        val deadline = System.currentTimeMillis() + 3 * 60_000
        var tries = 1
        while (true) {
            try {
                return call().also { if (tries > 1) Log.info(SRC, "reached Google", flow, mapOf("tries" to "$tries")) }
            } catch (e: GmailException) {
                if (e.code != "unreachable" || System.currentTimeMillis() > deadline) throw e
            }
            tries++
            kotlinx.coroutines.delay(2_000)
        }
    }

    private fun post(url: String, flow: String, vararg fields: Pair<String, String?>): JSONObject {
        val body = FormBody.Builder().apply { fields.forEach { (k, v) -> if (v != null) add(k, v) } }.build()
        return send(Request.Builder().url(url).post(body).build(), flow)
    }

    private fun revoke(token: String, flow: String) {
        if (token.isEmpty()) return
        runCatching { send(Request.Builder().url(REVOKE).post(FormBody.Builder().add("token", token).build()).build(), flow) }
    }

    /** One call; logs path, status and time. Errors become a [GmailException]. */
    private fun send(request: Request, flow: String): JSONObject {
        val started = System.currentTimeMillis()
        val path = request.url.encodedPath
        val response = try {
            http.newCall(request.newBuilder().header("x-request-id", flow).build()).execute()
        } catch (e: IOException) {
            Log.warn(SRC, "${request.method} $path failed: ${e::class.simpleName}", flow)
            // Nothing reached Google: safe to send again, even the one-time sign-in code.
            val unsent = e is UnknownHostException || e is ConnectException
            throw GmailException(if (unsent) "unreachable" else "offline", "Couldn't reach Google. Check your connection.")
        }
        return response.use {
            val text = it.body.string()
            Log.info(SRC, "${request.method} $path ${it.code} ${System.currentTimeMillis() - started}ms", flow)
            val json = runCatching { JSONObject(text) }.getOrDefault(JSONObject())
            when {
                it.isSuccessful -> json
                it.code == 401 -> throw GmailException("unauthorized", "Gmail turned down the sign-in. Connect Gmail again.")
                it.code == 403 -> throw GmailException("forbidden", "Gmail refused access (${json.optJSONObject("error")?.optString("status") ?: "403"}).")
                json.optString("error") == "invalid_grant" -> throw GmailException("invalid_grant", "Google ended the connection.")
                json.optString("error") == "invalid_client" -> throw GmailException("invalid_client", "Google doesn't know this client. Import a new client JSON.")
                else -> throw GmailException("http_${it.code}", "Google answered HTTP ${it.code}.")
            }
        }
    }

    private val ENDED get() = GmailException("ended", "Google ended the Gmail connection (a project in Testing does this every 7 days). Reconnect it in Settings → OTP auto-fill.")

    // --- helpers ---

    private val JSON = "application/json".toMediaType()

    private fun expiry(tokens: JSONObject) = System.currentTimeMillis() + tokens.optLong("expires_in", 3600) * 1000

    /** Flutter's `_validateCollegeAccount`: a VIT-AP student address holding [regno]. */
    private fun collegeAccountProblem(email: String, regno: String): String? {
        if (!email.lowercase().endsWith("@vitapstudent.ac.in")) return "Use your @vitapstudent.ac.in Google account."
        val fromEmail = email.substringBefore('@').split('.').lastOrNull { it.isNotEmpty() }.orEmpty().lowercase().filter { it.isLetterOrDigit() }
        val expected = regno.lowercase().filter { it.isLetterOrDigit() }
        if (expected.isEmpty() || !fromEmail.endsWith(expected)) return "Use the college Google account with your registration number ($regno)."
        return null
    }

    private fun emailFrom(idToken: String): String? = runCatching {
        val payload = idToken.split('.')[1]
        JSONObject(String(Base64.decode(payload, Base64.URL_SAFE or Base64.NO_PADDING or Base64.NO_WRAP))).optString("email").trim().ifEmpty { null }
    }.getOrNull()

    private fun save(a: Account) {
        account = a
        Secrets.write(app, FILE, encode(a).toString().toByteArray())
    }

    private fun encode(a: Account) = JSONObject()
        .put("email", a.email).put("client_id", a.client.id).put("client_secret", a.client.secret).put("project", a.client.project)
        .put("refresh_token", a.refreshToken).put("access_token", a.accessToken).put("expires_at_ms", a.expiresAtMs)
        .put("delete_after_reading", a.deleteAfterReading).put("grant_ends_at_ms", a.grantEndsAtMs).put("ended", a.ended)

    private fun decode(j: JSONObject) = Account(
        j.getString("email"),
        Client(j.getString("client_id"), j.optString("client_secret").ifEmpty { null }, j.optString("project")),
        j.getString("refresh_token"),
        j.optString("access_token"),
        j.optLong("expires_at_ms"),
        j.optBoolean("delete_after_reading", true),
        j.optLong("grant_ends_at_ms"),
        j.optBoolean("ended"),
    )

    private fun random(bytes: Int) = b64(ByteArray(bytes).also(SecureRandom()::nextBytes))

    private fun b64(bytes: ByteArray) = Base64.encodeToString(bytes, Base64.URL_SAFE or Base64.NO_PADDING or Base64.NO_WRAP)

    private fun query(vararg pairs: Pair<String, String>) = pairs.joinToString("&") { (k, v) -> "$k=" + URLEncoder.encode(v, "UTF-8").replace("+", "%20") }

    private fun JSONArray?.objects(): List<JSONObject> = if (this == null) emptyList() else List(length()) { optJSONObject(it) }.filterNotNull()
}
