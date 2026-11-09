package vitapmate.data

import android.content.Context
import android.net.ConnectivityManager
import android.os.Build
import android.util.Base64
import java.io.File
import java.security.KeyStore
import kotlin.concurrent.thread
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.DevConfig
import vitapmate.core.devConfigure
import vitapmate.core.obsInit
import vitapmate.core.setDeviceLabel
import vitapmate.core.setHttpTransport
import vitapmate.core.setSharedCookies
import vitapmate.core.setUdpNetwork
import vitapmate.observability.Inspector
import vitapmate.observability.Log
import vitapmate.observability.StateDump
import vitapmate.observability.ActivityHistory
import vitapmate.observability.flow

/** [block] with the [Core] on [Dispatchers.IO], in flow [name] (its id is [block]'s second argument). */
suspend fun <T> coreFlow(name: String, block: suspend (Core, String) -> T): T =
    flow(name) { id -> withContext(Dispatchers.IO) { block(AppCore.await(), id) } }

/** The one process-wide [Core], created off the main thread at app start (loads the .so, starts tokio, hands Rust OkHttp and the cookie store). */
object AppCore {
    private val core = CompletableDeferred<Core>()

    private var started = false

    /**
     * Starts creating the core, attaches the Inspector to the log files and, in debuggable
     * builds, registers the state dump (`vitapmate.DUMP`); later calls do nothing.
     */
    fun start(context: Context) {
        if (started) return
        started = true
        Log.catchUncaught()
        val app = context.applicationContext
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
                devConfigure(DevConfig(Settings.tunable(Tunable.SessionReuse).toUInt(), Settings.tunable(Tunable.VtopTimeout).toUInt()))
                val engine = Settings.httpEngine && !Settings.inspectTraffic && Build.VERSION.SDK_INT >= 34
                setHttpTransport(if (engine) VtopHttpEngine(app) else inspectedHttp(app))
                Log.info("core", "http transport", fields = mapOf("stack" to if (engine) "httpengine" else "okhttp"))
                setUdpNetwork(VtopUdp(app))
                setSharedCookies(VtopCookies)
                setDeviceLabel(if (Build.MODEL.startsWith(Build.MANUFACTURER, ignoreCase = true)) Build.MODEL else "${Build.MANUFACTURER} ${Build.MODEL}")
                runBlocking { flow("core_init") { Core(it, dir, app.cacheDir.path) } }
            }.fold(core::complete) { e ->
                Log.error("core", "init failed: $e")
                core.completeExceptionally(e)
            }
            // History kept past the log roll-over, then the 20 MB cap; never in the way of start.
            runCatching {
                ActivityHistory.save()
                runBlocking { flow("store.start") { StoreCap.enforce(app, it) } }
            }.onFailure { Log.warn("store", "start housekeeping failed: ${it::class.simpleName}") }
        }
    }

    /**
     * OkHttp for VTOP; with Developer → Inspect Traffic on (from the next launch), through the
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
