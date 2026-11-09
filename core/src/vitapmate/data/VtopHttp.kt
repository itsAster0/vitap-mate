package vitapmate.data

import java.io.File
import java.io.IOException
import java.io.InterruptedIOException
import java.net.InetSocketAddress
import java.net.NoRouteToHostException
import java.net.Proxy
import java.net.SocketException
import java.net.UnknownHostException
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLException
import javax.net.ssl.TrustManagerFactory
import javax.net.ssl.X509TrustManager
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.Call
import okhttp3.Callback
import okhttp3.ConnectionPool
import okhttp3.Dispatcher
import okhttp3.Headers
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import vitapmate.core.Header
import vitapmate.core.HttpRequest
import vitapmate.core.HttpResponse
import vitapmate.core.HttpTransport
import vitapmate.core.TransportException
import vitapmate.core.vtopIntermediateCa

/**
 * VTOP's HTTP hops over OkHttp, for Rust (`set_http_transport`). Rust adds and stores the
 * cookies (in [VtopCookies]), follows redirects and decides retries, so this client has no
 * cookie jar, follows nothing and never retries. A download's body goes straight to the file
 * Rust names (`downloadTo`), never whole into memory or across to Rust. TLS trusts the system CAs plus the intermediate VTOP leaves out of its
 * chain; with Developer → Inspect Traffic on, also [inspectProxy] and [inspectCas] (read once,
 * as Rust's). Logs nothing: Rust logs each request without secrets.
 */
class VtopHttp(private val inspectProxy: String?, private val inspectCas: List<String>) : HttpTransport {
    private val client by lazy {
        val trust = trustManager(listOf(vtopIntermediateCa()) + inspectCas)
        val tls = SSLContext.getInstance("TLS").apply { init(null, arrayOf(trust), null) }
        OkHttpClient.Builder()
            .followRedirects(false)
            .followSslRedirects(false)
            .retryOnConnectionFailure(false)
            // VTOP drops a connection idle for 60 s; OkHttp only finds out when it reuses one.
            .connectionPool(ConnectionPool(32, 50, TimeUnit.SECONDS))
            .dispatcher(Dispatcher().apply { maxRequests = 64; maxRequestsPerHost = 32 })
            // Only the whole-hop and connect timeouts, as reqwest had.
            .readTimeout(0, TimeUnit.MILLISECONDS)
            .writeTimeout(0, TimeUnit.MILLISECONDS)
            .sslSocketFactory(tls.socketFactory, trust)
            .proxy(inspectProxy?.let(::httpProxy) ?: Proxy.NO_PROXY)
            .build()
    }

    override suspend fun send(request: HttpRequest): HttpResponse {
        val headers = Headers.Builder().apply { request.headers.forEach { addUnsafeNonAscii(it.name, it.value) } }.build()
        val body = request.body.takeUnless { request.method == "GET" || request.method == "HEAD" }?.toRequestBody()
        val call = client.newBuilder()
            .callTimeout(request.timeoutMs.toLong(), TimeUnit.MILLISECONDS)
            .connectTimeout(request.connectTimeoutMs.toLong(), TimeUnit.MILLISECONDS)
            .build()
            .newCall(Request.Builder().url(request.url).headers(headers).method(request.method, body).build())
        return suspendCancellableCoroutine { continuation ->
            continuation.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) = continuation.resumeWithException(failure(e))

                // OkHttp asked for gzip itself, so the body is already inflated and
                // Content-Encoding / Content-Length are gone from the headers.
                override fun onResponse(call: Call, response: Response) {
                    val answer = runCatching {
                        response.use {
                            val all = it.headers
                            val headers = List(all.size) { i -> Header(all.name(i), all.value(i)) }
                            val saved = request.downloadTo?.let { path -> File(path).outputStream().use { file -> it.body.byteStream().copyTo(file) } }
                            HttpResponse(it.code.toUShort(), headers, request.url, if (saved == null) it.body.bytes() else ByteArray(0), saved?.toULong())
                        }
                    }
                    answer.fold(continuation::resume) { e -> continuation.resumeWithException(if (e is IOException) failure(e) else e) }
                }
            })
        }
    }

    private companion object {
        /**
         * As reqwest's `is_timeout` / `is_connect`; OkHttp's messages carry no URL query. A
         * SocketException (a pooled connection the phone dropped while the app was in the
         * background: "connection abort", "reset") is a connect failure, so Rust retries it once
         * unless the request is sent at most once.
         */
        fun failure(e: IOException): TransportException {
            val reason = "${e::class.java.simpleName}: ${e.message}"
            return when (e) {
                is InterruptedIOException -> TransportException.Timeout(reason)
                is SocketException, is UnknownHostException, is NoRouteToHostException, is SSLException -> TransportException.Connect(reason)
                else -> TransportException.Other(reason)
            }
        }

        fun httpProxy(hostPort: String): Proxy? {
            val host = hostPort.substringBeforeLast(':')
            val port = hostPort.substringAfterLast(':').toIntOrNull() ?: return null
            return Proxy(Proxy.Type.HTTP, InetSocketAddress.createUnresolved(host, port))
        }

        /** The system CAs plus [pems] as extra anchors. */
        fun trustManager(pems: List<String>): X509TrustManager {
            fun manager(store: KeyStore?) = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm())
                .apply { init(store) }.trustManagers.filterIsInstance<X509TrustManager>().first()
            val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply { load(null) }
            manager(null).acceptedIssuers.forEachIndexed { i, ca -> store.setCertificateEntry("system$i", ca) }
            val x509 = CertificateFactory.getInstance("X.509")
            pems.flatMap { x509.generateCertificates(it.byteInputStream()) }.forEachIndexed { i, ca -> store.setCertificateEntry("extra$i", ca) }
            return manager(store)
        }
    }
}
