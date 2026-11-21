package vitapmate.features.vtop

import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import java.io.File
import java.io.FileInputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.asExecutor
import org.json.JSONObject
import vitapmate.observability.Log

/**
 * VTOP's public static files for the web view, kept in `cache/vtop-assets` (VTOP sends them all
 * `no-store`, so the web view downloads them again on every open). Fetched without cookies:
 * nothing here is a page or needs the login. `/vtop/webjars/` has the version in its path, so a
 * saved copy is served at once and checked at most daily; `/vtop/assets/` images and CSS are
 * served at once and checked in the background (a change shows on the next open); its JS is
 * checked first (old JS on a new page can break it). A check is `If-Modified-Since`, a 304 when
 * nothing changed. Anything that fails returns null, so the web view loads the file itself.
 */
internal class VtopAssets(cacheDir: File) {
    private val dir = File(cacheDir, "vtop-assets")
    private val background = Dispatchers.IO.asExecutor()
    private val checking = ConcurrentHashMap.newKeySet<String>()
    private val served = AtomicInteger()
    private val fetched = AtomicInteger()
    private val updated = AtomicInteger()
    private val failed = AtomicInteger()

    private enum class Kind(val recheckMs: Long) { Versioned(DAY_MS), Static(MINUTE_MS), Script(0) }

    private class Saved(val body: File, val meta: File, val lastModified: String?, val type: String, val checked: Long)

    /** From `shouldInterceptRequest` (a web view thread): the saved file, or null to load it normally. */
    fun serve(request: WebResourceRequest, userAgent: String): WebResourceResponse? {
        val uri = request.url
        if (request.method != "GET" || request.isForMainFrame || !VtopBrowser.isVtop(uri) || uri.query != null) return null
        val path = uri.path ?: return null
        if (".." in path) return null
        val kind = when {
            path.startsWith("/vtop/webjars/") -> Kind.Versioned
            path.startsWith("/vtop/assets/") -> if (path.endsWith(".js", ignoreCase = true)) Kind.Script else Kind.Static
            else -> return null
        }
        return try {
            val saved = read(path)
            when {
                saved == null -> fetch(path, null, userAgent)?.let(::response)
                // Offline or VTOP failing: the saved copy beats none.
                kind == Kind.Script -> response(runCatching { fetch(path, saved, userAgent) }.getOrNull() ?: saved)
                else -> {
                    if (System.currentTimeMillis() - saved.checked >= kind.recheckMs) checkLater(path, saved, userAgent)
                    response(saved)
                }
            }
        } catch (e: Exception) {
            failed.incrementAndGet()
            Log.warn(VtopBrowser.SRC, "asset failed: ${e::class.simpleName}", null, mapOf("path" to path))
            null
        }
    }

    /** What this page load served, fetched and updated; then starts counting again. */
    fun summary(): Map<String, String> = mapOf(
        "served" to served.getAndSet(0), "fetched" to fetched.getAndSet(0),
        "updated" to updated.getAndSet(0), "failed" to failed.getAndSet(0),
    ).mapValues { it.value.toString() }

    fun bytes(): Long = dir.listFiles().orEmpty().sumOf { it.length() }

    fun clear() {
        dir.deleteRecursively()
    }

    private fun checkLater(path: String, saved: Saved, userAgent: String) {
        if (!checking.add(path)) return
        background.execute {
            try {
                fetch(path, saved, userAgent)
            } catch (e: Exception) {
                Log.warn(VtopBrowser.SRC, "asset check failed: ${e::class.simpleName}", null, mapOf("path" to path))
            } finally {
                checking.remove(path)
            }
        }
    }

    /** The saved copy, or null (a body whose size doesn't match what was saved is dropped). */
    private fun read(path: String): Saved? {
        val key = key(path)
        val body = File(dir, key)
        val meta = File(dir, "$key.json")
        if (!body.exists() || !meta.exists()) return null
        val m = runCatching { JSONObject(meta.readText()) }.getOrNull()
        if (m == null || m.optLong("length", -1) != body.length()) {
            body.delete()
            meta.delete()
            return null
        }
        return Saved(body, meta, m.optString("lastModified").ifEmpty { null }, m.optString("type"), m.optLong("checked"))
    }

    /**
     * Asks VTOP for [path] (since [saved]'s date when there is one). The saved copy on a 304, a new
     * one on a 200, null when VTOP answered anything else or the body came short.
     */
    private fun fetch(path: String, saved: Saved?, userAgent: String): Saved? {
        val connection = URL(VtopScripts.ORIGIN + path).openConnection() as HttpURLConnection
        try {
            connection.instanceFollowRedirects = false
            connection.useCaches = false
            connection.connectTimeout = TIMEOUT_MS
            connection.readTimeout = TIMEOUT_MS
            connection.setRequestProperty("User-Agent", userAgent)
            saved?.lastModified?.let { connection.setRequestProperty("If-Modified-Since", it) }
            when (connection.responseCode) {
                HttpURLConnection.HTTP_NOT_MODIFIED -> if (saved != null) {
                    writeMeta(saved.meta, saved.lastModified, saved.type, saved.body.length())
                    return saved
                }
                HttpURLConnection.HTTP_OK -> {
                    dir.mkdirs()
                    val key = key(path)
                    val temp = File.createTempFile(key, ".part", dir)
                    try {
                        val written = connection.inputStream.use { input -> temp.outputStream().use { input.copyTo(it) } }
                        // Cut off on the way: keep the old copy (or none) rather than half a file.
                        if (connection.contentLengthLong >= 0 && written != connection.contentLengthLong) return null
                        val body = File(dir, key)
                        val meta = File(dir, "$key.json")
                        val type = connection.contentType ?: "application/octet-stream"
                        val lastModified = connection.getHeaderField("Last-Modified")
                        if (!temp.renameTo(body)) return null
                        writeMeta(meta, lastModified, type, written)
                        (if (saved == null) fetched else updated).incrementAndGet()
                        trim()
                        return Saved(body, meta, lastModified, type, System.currentTimeMillis())
                    } finally {
                        temp.delete()
                    }
                }
            }
            return null
        } finally {
            connection.disconnect()
        }
    }

    private fun writeMeta(meta: File, lastModified: String?, type: String, length: Long) {
        val json = JSONObject().put("lastModified", lastModified.orEmpty()).put("type", type)
            .put("length", length).put("checked", System.currentTimeMillis())
        val temp = File(meta.path + ".part")
        temp.writeText(json.toString())
        temp.renameTo(meta)
    }

    private fun response(saved: Saved): WebResourceResponse {
        served.incrementAndGet()
        // For the cap: the files served last are kept longest.
        saved.body.setLastModified(System.currentTimeMillis())
        val parts = saved.type.split(';').map { it.trim() }
        val charset = parts.drop(1).firstOrNull { it.startsWith("charset=", ignoreCase = true) }?.substringAfter('=')?.trim('"')
        return WebResourceResponse(parts.first(), charset, 200, "OK", mapOf("Cache-Control" to "no-store"), FileInputStream(saved.body))
    }

    /** Past [MAX_BYTES], removes the files served longest ago until under it. */
    private fun trim() {
        val bodies = dir.listFiles { f -> !f.name.endsWith(".json") && !f.name.endsWith(".part") }.orEmpty()
        var total = dir.listFiles().orEmpty().sumOf { it.length() }
        if (total <= MAX_BYTES) return
        for (body in bodies.sortedBy { it.lastModified() }) {
            if (total <= MAX_BYTES) break
            val meta = File(dir, "${body.name}.json")
            total -= body.length() + meta.length()
            body.delete()
            meta.delete()
        }
    }

    private fun key(path: String): String =
        MessageDigest.getInstance("SHA-256").digest(path.toByteArray()).joinToString("") { "%02x".format(it) }.take(32)

    private companion object {
        const val MAX_BYTES = 20L * 1024 * 1024
        const val TIMEOUT_MS = 10_000
        const val MINUTE_MS = 60_000L
        const val DAY_MS = 24 * 60 * MINUTE_MS
    }
}
