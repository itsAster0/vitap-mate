package vitapmate.data

import android.content.Context
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.observability.Log

/**
 * Help pages the app reads from the edge instead of shipping them (`rust/vtop-edge/guides`, see
 * `/guides/` in `rust/vtop-edge/API.md`): JSON and its images, fetched when a page first opens and
 * kept in the cache folder. Each open shows the kept copy and checks it with its ETag, so a fixed
 * guide needs only an edge deploy, not a release.
 */
object Guides {
    private const val BASE = "https://vmate.aster0.dev/guides/"

    /** The JSON shape this app reads; the edge bumps `schema` only for a change old apps can't read. */
    const val SCHEMA = 1

    private fun file(context: Context, path: String) = File(context.cacheDir, "guides/$path")

    /** The kept copy of [path], or null. */
    fun cached(context: Context, path: String): File? = file(context, path).takeIf { it.isFile }

    /**
     * [path] as the edge has it now: downloaded when the kept copy is missing or out of date, else
     * the kept copy (a 304). Throws [IOException] when the edge can't be reached.
     */
    suspend fun refresh(context: Context, path: String, flow: String): File = withContext(Dispatchers.IO) {
        val file = file(context, path)
        val tag = File(file.path + ".etag")
        val started = System.currentTimeMillis()
        val conn = URL(BASE + path).openConnection() as HttpURLConnection
        try {
            conn.connectTimeout = 10_000
            conn.readTimeout = 15_000
            conn.setRequestProperty("x-request-id", flow)
            if (file.isFile && tag.isFile) conn.setRequestProperty("If-None-Match", tag.readText())
            val code = conn.responseCode
            Log.info("guide", "GET $path $code ${System.currentTimeMillis() - started}ms", flow)
            when (code) {
                HttpURLConnection.HTTP_NOT_MODIFIED -> file
                HttpURLConnection.HTTP_OK -> {
                    file.parentFile?.mkdirs()
                    val part = File(file.path + ".part")
                    conn.inputStream.use { input -> part.outputStream().use { input.copyTo(it) } }
                    if (!part.renameTo(file)) throw IOException("couldn't keep $path")
                    conn.getHeaderField("ETag")?.let(tag::writeText) ?: tag.delete()
                    file
                }
                else -> throw IOException("HTTP $code")
            }
        } catch (e: IOException) {
            Log.warn("guide", "GET $path failed: ${e::class.simpleName}", flow)
            throw e
        } finally {
            conn.disconnect()
        }
    }
}
