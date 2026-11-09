package vitapmate.data

import android.content.Context
import android.net.http.HttpEngine
import android.net.http.HttpException
import android.net.http.NetworkException
import android.net.http.UploadDataProvider
import android.net.http.UploadDataSink
import android.net.http.UrlRequest
import android.net.http.UrlResponseInfo
import androidx.annotation.RequiresApi
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.asExecutor
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.Header
import vitapmate.core.HttpRequest
import vitapmate.core.HttpResponse
import vitapmate.core.HttpTransport
import vitapmate.core.TransportException
import vitapmate.observability.Log

/**
 * [VtopHttp]'s job on Android 14+'s [HttpEngine] (the platform's Cronet: HTTP/3 where the
 * server offers it, else HTTP/2 or 1.1), behind Developer → HttpEngine. Same contract: one hop,
 * a 3xx comes back as it is (the request is cancelled at the redirect, so its body is dropped),
 * no cookies of its own, no cache, a download goes straight to its file. TLS is the platform's,
 * so VTOP's missing intermediate comes from `res/xml/network_security_config.xml`; no proxy or
 * user CAs, so Inspect Traffic keeps [VtopHttp]. No connect timeout (HttpEngine has none to set):
 * the whole-hop timeout covers it. Logs each host's protocol once.
 */
@RequiresApi(34)
class VtopHttpEngine(context: Context) : HttpTransport {
    private val engine by lazy {
        HttpEngine.Builder(context)
            .setEnableQuic(true)
            .setEnableHttp2(true)
            .setEnableBrotli(true)
            .setEnableHttpCache(HttpEngine.Builder.HTTP_CACHE_DISABLED, 0)
            .build()
    }
    private val executor = Dispatchers.IO.asExecutor()
    private val protocols = ConcurrentHashMap<String, String>()

    override suspend fun send(request: HttpRequest): HttpResponse {
        val hop = suspend { exchange(request) }
        if (request.timeoutMs == 0u) return hop()
        return withTimeoutOrNull(request.timeoutMs.toLong()) { hop() }
            ?: throw TransportException.Timeout("timed out after ${request.timeoutMs} ms")
    }

    private suspend fun exchange(request: HttpRequest): HttpResponse = suspendCancellableCoroutine { continuation ->
        val body = ByteArrayOutputStream()
        val file = request.downloadTo?.let { FileOutputStream(File(it)) }
        var saved = 0L
        var redirect: UrlResponseInfo? = null

        fun answer(info: UrlResponseInfo, whole: Boolean) {
            file?.close()
            val host = request.url.substringAfter("://").substringBefore('/')
            if (protocols.put(host, info.negotiatedProtocol) != info.negotiatedProtocol) {
                Log.info("http", "engine protocol", fields = mapOf("host" to host, "protocol" to info.negotiatedProtocol))
            }
            // As OkHttp: the body is already decoded, so its encoding and length go.
            val decoded = whole && info.headers.asList.any { it.key.equals("content-encoding", true) }
            val headers = info.headers.asList
                .filterNot { decoded && (it.key.equals("content-encoding", true) || it.key.equals("content-length", true)) }
                .map { Header(it.key, it.value) }
            val saving = file != null && whole
            continuation.resume(HttpResponse(info.httpStatusCode.toUShort(), headers, request.url, if (saving) ByteArray(0) else body.toByteArray(), if (saving) saved.toULong() else null))
        }

        val callback = object : UrlRequest.Callback {
            override fun onRedirectReceived(r: UrlRequest, info: UrlResponseInfo, location: String) {
                redirect = info
                r.cancel()
            }

            override fun onResponseStarted(r: UrlRequest, info: UrlResponseInfo) = r.read(ByteBuffer.allocateDirect(64 * 1024))

            override fun onReadCompleted(r: UrlRequest, info: UrlResponseInfo, buffer: ByteBuffer) {
                buffer.flip()
                if (file != null) {
                    saved += buffer.remaining()
                    file.channel.write(buffer)
                } else {
                    body.write(ByteArray(buffer.remaining()).also(buffer::get))
                }
                buffer.clear()
                r.read(buffer)
            }

            override fun onSucceeded(r: UrlRequest, info: UrlResponseInfo) = answer(info, whole = true)

            override fun onFailed(r: UrlRequest, info: UrlResponseInfo?, error: HttpException) {
                file?.close()
                continuation.resumeWithException(failure(error))
            }

            override fun onCanceled(r: UrlRequest, info: UrlResponseInfo?) {
                val at = redirect
                if (at != null) answer(at, whole = false) else file?.close()
            }
        }
        val builder = engine.newUrlRequestBuilder(request.url, executor, callback)
            .setHttpMethod(request.method)
            .setCacheDisabled(true)
        request.headers.forEach { builder.addHeader(it.name, it.value) }
        if (request.body.isNotEmpty() && request.method != "GET" && request.method != "HEAD") builder.setUploadDataProvider(Upload(request.body), executor)
        val call = builder.build()
        continuation.invokeOnCancellation { call.cancel() }
        call.start()
    }

    private class Upload(private val bytes: ByteArray) : UploadDataProvider() {
        private var at = 0

        override fun getLength() = bytes.size.toLong()

        override fun read(sink: UploadDataSink, buffer: ByteBuffer) {
            val n = minOf(buffer.remaining(), bytes.size - at)
            buffer.put(bytes, at, n)
            at += n
            sink.onReadSucceeded(false)
        }

        override fun rewind(sink: UploadDataSink) {
            at = 0
            sink.onRewindSucceeded()
        }
    }

    private companion object {
        /**
         * As [VtopHttp]'s: timeouts, then no connection (Rust retries those once), then the rest.
         * A TLS failure (net::ERR_CERT_*) comes as ERROR_OTHER, so it is not retried.
         */
        fun failure(e: HttpException): TransportException {
            val reason = "${e::class.java.simpleName}: ${e.message}"
            if (e !is NetworkException) return TransportException.Other(reason)
            return when (e.errorCode) {
                NetworkException.ERROR_TIMED_OUT, NetworkException.ERROR_CONNECTION_TIMED_OUT -> TransportException.Timeout(reason)
                NetworkException.ERROR_HOSTNAME_NOT_RESOLVED, NetworkException.ERROR_INTERNET_DISCONNECTED, NetworkException.ERROR_NETWORK_CHANGED,
                NetworkException.ERROR_CONNECTION_CLOSED, NetworkException.ERROR_CONNECTION_REFUSED, NetworkException.ERROR_CONNECTION_RESET,
                NetworkException.ERROR_ADDRESS_UNREACHABLE, NetworkException.ERROR_QUIC_PROTOCOL_FAILED -> TransportException.Connect(reason)
                else -> TransportException.Other(reason)
            }
        }
    }
}
