package vitapmate.observability

import vitapmate.core.logEvent

/**
 * App logs. Lines go through Rust (`logEvent`), so Kotlin and Rust share one file
 * (`files/logs/app.jsonl`), one logcat tag (`vitapmate`) and one order. Never pass cookies,
 * passwords, OTPs, CSRF tokens or keys.
 */
object Log {
    fun debug(src: String, msg: String, flow: String? = null, fields: Map<String, String> = emptyMap()) = emit("debug", src, msg, flow, fields)
    fun info(src: String, msg: String, flow: String? = null, fields: Map<String, String> = emptyMap()) = emit("info", src, msg, flow, fields)
    fun warn(src: String, msg: String, flow: String? = null, fields: Map<String, String> = emptyMap()) = emit("warn", src, msg, flow, fields)
    fun error(src: String, msg: String, flow: String? = null, fields: Map<String, String> = emptyMap()) = emit("error", src, msg, flow, fields)

    private fun emit(level: String, src: String, msg: String, flow: String?, fields: Map<String, String>) {
        try {
            logEvent(level, src, flow, msg, fields)
        } catch (e: Throwable) {
            // The native library isn't loaded (or failed to): logcat only.
            android.util.Log.println(android.util.Log.WARN, "vitapmate", "$level $src ${flow.orEmpty()} $msg $fields ($e)")
        }
    }

    /** Logs an uncaught exception, with its stack, before the default handler ends the process. */
    fun catchUncaught() {
        val default = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, e ->
            error("crash", "uncaught on ${thread.name}: $e", fields = mapOf("stack" to e.stackTraceToString()))
            default?.uncaughtException(thread, e)
        }
    }
}
