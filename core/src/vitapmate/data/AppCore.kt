package vitapmate.data

import android.content.Context
import android.content.pm.ApplicationInfo
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import android.util.Base64
import java.io.File
import java.security.KeyStore
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.DevConfig
import vitapmate.core.Platform
import vitapmate.core.RegnoAsk
import vitapmate.core.obsInit
import vitapmate.observability.Inspector
import vitapmate.observability.Log
import vitapmate.observability.StateDump
import vitapmate.observability.ActivityHistory
import vitapmate.observability.flow

/** [block] with the [Core] on [Dispatchers.IO], in flow [name] (its id is [block]'s second argument). */
suspend fun <T> coreFlow(name: String, block: suspend (Core, String) -> T): T =
    flow(name) { id -> withContext(Dispatchers.IO) { block(AppCore.await(), id) } }

/** The one process-wide [Core], created off the main thread at app start (loads the .so, starts tokio, hands Rust the HTTP transport and the cookie store). */
object AppCore {
    private val core = CompletableDeferred<Core>()

    private val started = AtomicBoolean(false)
    @Volatile private var app: Context? = null
    private var proving: Job? = null

    /**
     * Starts creating the core, attaches the Inspector to the log files and, in debuggable
     * builds, registers the state dump (`vitapmate.DUMP`); later calls do nothing.
     */
    fun start(context: Context) {
        // The activity, FCM, WorkManager and widgets each call this, on their own threads.
        if (!started.compareAndSet(false, true)) return
        Settings.load(context)
        Log.catchUncaught()
        val app = context.applicationContext
        this.app = app
        Inspector.attach(app)
        StateDump.register(app, Settings::snapshot)
        Docs.init(app)
        Gmail.init(app)
        val dir = context.filesDir.path
        thread(name = "core-init") {
            runCatching {
                // Logs, the change inbox and Activity history live in the cache folder, which
                // the student can clear; older versions kept them in files/.
                File(app.filesDir, "logs").deleteRecursively()
                for (name in listOf("changes.json", "activity.jsonl")) {
                    val old = File(app.filesDir, name)
                    if (old.exists() && !File(app.cacheDir, name).exists()) old.renameTo(File(app.cacheDir, name)) else old.delete()
                }
                obsInit(app.cacheDir.path)
                runCatching(Gmail::load).onFailure { Log.warn("core.gmail", "load failed: ${it::class.simpleName}") }
                // Debuggable: Developer → Chat's servers may apply (never in a release build).
                val debuggable = app.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
                val engine = Settings.httpEngine && !Settings.inspectTraffic && Build.VERSION.SDK_INT >= 34
                Log.info("core", "http transport", fields = mapOf("stack" to if (engine) "httpengine" else "urlconnection"))
                val platform = Platform(
                    http = if (engine) VtopHttpEngine(app) else inspectedHttp(app),
                    cookies = VtopCookies,
                    udp = VtopUdp(app),
                    convex = SocketConvexNetwork(),
                    proof = SocketProofNetwork(),
                    cookieGate = CookieConsent,
                    cipher = AccountPassword,
                    otp = OtpPrompt,
                    dev = DevConfig(Settings.tunable(Tunable.SessionReuse).toUInt(), Settings.tunable(Tunable.VtopTimeout).toUInt(), debuggable),
                    deviceLabel = if (Build.MODEL.startsWith(Build.MANUFACTURER, ignoreCase = true)) Build.MODEL else "${Build.MANUFACTURER} ${Build.MODEL}",
                )
                runBlocking { flow("core_init") { Core(it, dir, app.cacheDir.path, platform) } }
            }.fold(core::complete) { e ->
                Log.error("core", "init failed: $e")
                core.completeExceptionally(e)
            }
            // History kept past the log roll-over, then the 20 MB cap; never in the way of start.
            runCatching {
                ActivityHistory.save()
                runBlocking { flow("store.start") { StoreCap.enforce(app, it) } }
            }.onFailure { Log.warn("store", "start housekeeping failed: ${it::class.simpleName}") }
            leaveOldForwarding()
        }
    }

    /**
     * Rejoins a forwarding address kept from before backend v3, so forwarded OTPs keep coming
     * after the update. Rust proves the student once (no cookie leaves the phone) and does
     * nothing after that, so it runs whenever the app is signed in.
     */
    fun keepForwarding(then: suspend () -> Unit = {}) {
        AppScope.launch {
            runCatching { flow("forwarding.keep") { f -> withContext(Dispatchers.IO) { await().keepForwarding(f) } } }
                .onFailure { Log.warn("forwarding", "keep failed: ${it::class.simpleName}") }
            then()
        }
    }

    /**
     * Proves who the student is to the server in the background (Rust's `proveInBackground`),
     * so Connected apps, forwarding and the server routes find the identity ready: called after
     * a sign-in and on each open signed in, it does nothing once there is one. Only on an
     * unmetered network, or a validated one that isn't mobile data (unless
     * [Tunable.ProofOnCellular]); gives up after [Tunable.ProofTimeout]; one at a time. First the
     * registration number's sheet, while the student was never asked: Rust proves only after a
     * Share.
     */
    @Synchronized
    fun proveInBackground() {
        val context = app ?: return
        if (proving?.isActive == true) return
        proving = AppScope.launch {
            runCatching { flow("account.regno_ask") { f -> withContext(Dispatchers.IO) { await().askRegnoSharing(f, RegnoAsk.FIRST_TIME) } } }
                .onFailure { Log.info("account", "regno sheet not shown: ${it::class.simpleName}") }
            if (!proofNetworkOk(context)) return@launch
            runCatching {
                flow("account.prove_background", unattended = true) { f ->
                    val ms = Settings.tunable(Tunable.ProofTimeout) * 1000
                    withContext(Dispatchers.IO) { await().proveInBackground(f, ms.toUInt()) }
                }
            }.onFailure { Log.info("account", "background proof not done: ${it::class.simpleName}") }
        }
    }

    private fun proofNetworkOk(context: Context): Boolean {
        val manager = context.getSystemService(ConnectivityManager::class.java) ?: return false
        val caps = manager.getNetworkCapabilities(manager.activeNetwork) ?: return false
        val validated = caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)
        val cellular = caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
        return caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) ||
            validated && (!cellular || Settings.tunable(Tunable.ProofOnCellular) == 1)
    }

    /**
     * Takes this phone off the forwarding addresses a sign-out (or another student signing in) left
     * behind, in the background; Rust keeps the list until the edge confirms each.
     */
    fun leaveOldForwarding() {
        AppScope.launch {
            runCatching { flow("forwarding.leave", unattended = true) { f -> withContext(Dispatchers.IO) { await().leaveOldForwarding(f) } } }
                .onFailure { Log.warn("forwarding", "leave failed: ${it::class.simpleName}") }
        }
    }

    /**
     * [VtopHttp] for VTOP; with Developer → Inspect Traffic on (from the next launch), through the
     * Wi-Fi proxy and trusting the user-installed CAs (the Flutter app's `trafficInspection`).
     * [Settings] must be loaded.
     */
    private fun inspectedHttp(context: Context): VtopHttp {
        if (!Settings.inspectTraffic) return VtopHttp(null, emptyList())
        val proxy = context.getSystemService(ConnectivityManager::class.java)?.defaultProxy
            ?.takeIf { !it.host.isNullOrEmpty() }?.let { "${it.host}:${it.port}" }
        val cas = runCatching {
            val store = KeyStore.getInstance("AndroidCAStore").apply { load(null) }
            store.aliases().toList().filter { it.startsWith("user:") }.mapNotNull { alias ->
                store.getCertificate(alias)?.encoded?.let { "-----BEGIN CERTIFICATE-----\n" + Base64.encodeToString(it, Base64.DEFAULT) + "-----END CERTIFICATE-----\n" }
            }
        }.getOrElse { e ->
            Log.warn("core", "user CAs not read: ${e::class.simpleName}")
            emptyList()
        }
        Log.warn("core", "traffic inspection on", fields = mapOf("proxy" to (proxy ?: "none"), "cas" to "${cas.size}"))
        return VtopHttp(proxy, cas)
    }

    /** The core, once created; throws what creating it threw. */
    suspend fun await(): Core = core.await()
}
