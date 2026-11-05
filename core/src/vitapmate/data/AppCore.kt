package vitapmate.data

import android.content.Context
import android.net.ConnectivityManager
import android.util.Base64
import java.security.KeyStore
import kotlin.concurrent.thread
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.runBlocking
import vitapmate.core.Core
import vitapmate.core.DevConfig
import vitapmate.core.devConfigure
import vitapmate.core.obsInit
import vitapmate.observability.Inspector
import vitapmate.observability.Log
import vitapmate.observability.StateDump
import vitapmate.observability.flow

/** The one process-wide [Core], created off the main thread at app start (loads the .so, starts tokio). */
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
        val dir = context.filesDir.path
        thread(name = "core-init") {
            runCatching {
                obsInit(dir)
                devConfigure(devConfig(app))
                runBlocking { flow("core_init") { Core(it, dir) } }
            }.fold(core::complete) { e ->
                Log.error("core", "init failed: $e")
                core.completeExceptionally(e)
            }
        }
    }

    /**
     * What Rust reads once, before the core (Settings → Developer, so from the next launch): the
     * session reuse limit, the VTOP timeout and, with Inspect Traffic on, the Wi-Fi proxy and the
     * user-installed CAs (the Flutter app's `trafficInspection`). [Settings] must be loaded.
     */
    private fun devConfig(context: Context): DevConfig {
        val inspect = Settings.inspectTraffic
        val proxy = context.takeIf { inspect }?.getSystemService(ConnectivityManager::class.java)?.defaultProxy
            ?.takeIf { !it.host.isNullOrEmpty() }?.let { "${it.host}:${it.port}" }
        val cas = if (!inspect) emptyList() else runCatching {
            val store = KeyStore.getInstance("AndroidCAStore").apply { load(null) }
            store.aliases().toList().filter { it.startsWith("user:") }.mapNotNull { alias ->
                store.getCertificate(alias)?.encoded?.let { "-----BEGIN CERTIFICATE-----\n" + Base64.encodeToString(it, Base64.DEFAULT) + "-----END CERTIFICATE-----\n" }
            }
        }.getOrElse { e ->
            Log.warn("core", "user CAs not read: ${e::class.simpleName}")
            emptyList()
        }
        return DevConfig(Settings.tunable(Tunable.SessionReuse).toUInt(), Settings.tunable(Tunable.VtopTimeout).toUInt(), proxy, cas)
    }

    /** The core, once created; throws what creating it threw. */
    suspend fun await(): Core = core.await()
}
