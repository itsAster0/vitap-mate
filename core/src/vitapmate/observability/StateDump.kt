package vitapmate.observability

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ApplicationInfo
import android.os.Build
import java.io.File
import kotlin.concurrent.thread
import org.json.JSONArray
import org.json.JSONObject

/**
 * The agent state dump (debuggable builds only): `adb shell am broadcast -a vitapmate.DUMP -p
 * com.vitap_pal.app` writes `files/state.json` (route, settings, every page's state, the last 20
 * flows) and logs `state dumped`. `mise run state` does both and prints it.
 */
internal object StateDump {
    const val ACTION = "vitapmate.DUMP"

    /** Registers the receiver; [settings] gives the settings' values at dump time. */
    fun register(context: Context, settings: () -> Map<String, Any?>) {
        if (context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE == 0) return
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(c: Context, intent: Intent) {
                val pending = goAsync()
                thread(name = "state-dump") {
                    try {
                        write(File(context.filesDir, "state.json"), settings())
                    } catch (e: Throwable) {
                        Log.error("state", "state dump failed: $e")
                    } finally {
                        pending.finish()
                    }
                }
            }
        }
        val filter = IntentFilter(ACTION)
        if (Build.VERSION.SDK_INT >= 33) context.registerReceiver(receiver, filter, Context.RECEIVER_EXPORTED)
        else context.registerReceiver(receiver, filter)
    }

    private fun write(file: File, settings: Map<String, Any?>) {
        val registry = StateRegistry.snapshot()
        val flows = groupFlows(Inspector.read()).take(20)
        val json = JSONObject()
            .put("t", System.currentTimeMillis())
            .put("route", registry["route"]?.let { JSONObject(it) } ?: JSONObject.NULL)
            .put("settings", JSONObject(settings))
            .put("activity", JSONObject(vitapmate.data.Activity.snapshot()))
            .put("pages", JSONObject(registry.filterKeys { it.startsWith("page.") }.mapKeys { it.key.removePrefix("page.") }))
            .put("reported", JSONObject(registry.filterKeys { it != "route" && !it.startsWith("page.") }))
            .put(
                "flows",
                JSONArray(
                    flows.map { f ->
                        val worst = f.lines.lastOrNull { it.level == "error" || it.level == "warn" }
                        JSONObject()
                            .put("id", f.id)
                            .put("name", f.name)
                            .put("startT", f.startT)
                            .put("ms", f.ms ?: JSONObject.NULL)
                            .put("outcome", f.outcome ?: "running")
                            .put("lines", f.lines.size)
                            .put("lastWarning", worst?.let { "${it.src}: ${it.msg}" } ?: JSONObject.NULL)
                    },
                ),
            )
        val text = json.toString(2)
        file.writeText(text)
        Log.info("state", "state dumped", fields = mapOf("path" to file.path, "bytes" to "${text.length}", "pages" to "${json.getJSONObject("pages").length()}"))
    }
}
