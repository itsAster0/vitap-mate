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
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLException
import javax.net.ssl.TrustManagerFactory
import javax.net.ssl.X509TrustManager
import java.net.HttpURLConnection
import java.net.URL
import javax.net.ssl.HttpsURLConnection
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
import vitapmate.core.vtopIntermediateCa

/**
 * VTOP's HTTP hops over the platform's [HttpURLConnection], for Rust (`set_http_transport`). Rust
 * adds and stores the cookies (in [VtopCookies]), follows redirects and decides retries, so this
 * client has no cookie jar and follows nothing; a body is streamed with a fixed length, so the
 * platform never sends it twice. A download's body goes straight to the file Rust names
 * (`downloadTo`), never whole into memory or across to Rust. TLS trusts the system CAs plus the
 * intermediate VTOP leaves out of its chain; with Developer → Inspect Traffic on, also
 * [inspectProxy] and [inspectCas] (read once, as Rust's). Logs nothing: Rust logs each request
 * without secrets.
 */
class VtopHttp(private val inspectProxy: String?, private val inspectCas: List<String>) : HttpTransport {
    private val tls by lazy {
        // VTOP drops a connection idle for 60 s; the platform's pool keeps them 5 min by default.
        System.setProperty("http.keepAliveDuration", "50000")
        val trust = trustManager(listOf(vtopIntermediateCa()) + inspectCas)
        SSLContext.getInstance("TLS").apply { init(null, arrayOf(trust), null) }.socketFactory
    }
    private val proxy by lazy { inspectProxy?.let(::httpProxy) ?: Proxy.NO_PROXY }

    override suspend fun send(request: HttpRequest): HttpResponse {
        val hop = suspend { exchange(request) }
        if (request.timeoutMs == 0u) return hop()
        return withTimeoutOrNull(request.timeoutMs.toLong()) { hop() }
            ?: throw TransportException.Timeout("timed out after ${request.timeoutMs} ms")
    }

    /**
     * One blocking exchange on the IO pool; cancelling (the hop's timeout) closes the socket. The
     * platform asks for gzip itself, so the body comes inflated and Content-Encoding / Content-Length
     * are gone from the headers.
     */
    private suspend fun exchange(request: HttpRequest): HttpResponse = suspendCancellableCoroutine { continuation ->
        val conn = URL(request.url).openConnection(proxy) as HttpURLConnection
        continuation.invokeOnCancellation { conn.disconnect() }
        Dispatchers.IO.asExecutor().execute {
            runCatching { blocking(conn, request) }
                .fold(continuation::resume) { e -> continuation.resumeWithException(if (e is IOException) failure(e) else e) }
        }
    }

    private fun blocking(conn: HttpURLConnection, request: HttpRequest): HttpResponse {
        (conn as? HttpsURLConnection)?.sslSocketFactory = tls
        conn.instanceFollowRedirects = false
        conn.useCaches = false
        conn.connectTimeout = request.connectTimeoutMs.toInt()
        conn.readTimeout = 0
        conn.requestMethod = request.method
        request.headers.forEach { conn.addRequestProperty(it.name, it.value) }
        request.body.takeUnless { request.method == "GET" || request.method == "HEAD" }?.let { body ->
            conn.doOutput = true
            conn.setFixedLengthStreamingMode(body.size)
            conn.outputStream.use { it.write(body) }
        }
        val code = conn.responseCode
        val headers = conn.headerFields.flatMap { (name, values) -> if (name == null) emptyList() else values.map { Header(name, it) } }
        val stream = (if (code >= 400) conn.errorStream else conn.inputStream) ?: ByteArray(0).inputStream()
        return stream.use { input ->
            val saved = request.downloadTo?.let { path -> File(path).outputStream().use { input.copyTo(it) } }
            HttpResponse(code.toUShort(), headers, request.url, if (saved == null) input.readBytes() else ByteArray(0), saved?.toULong())
        }
    }

    private companion object {
        /**
         * As reqwest's `is_timeout` / `is_connect`; the platform's messages carry no URL query. A
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
