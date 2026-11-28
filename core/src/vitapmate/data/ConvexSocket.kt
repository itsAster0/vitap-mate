package vitapmate.data

import android.security.NetworkSecurityPolicy
import java.io.EOFException
import java.net.ConnectException
import java.net.NoRouteToHostException
import java.net.SocketException
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import java.security.KeyStore
import javax.net.ssl.SSLException
import javax.net.ssl.SSLHandshakeException
import javax.net.ssl.SSLSocketFactory
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.ConvexNetwork
import vitapmate.core.ConvexSocket
import vitapmate.core.ConvexSocketListener
import vitapmate.core.Header
import vitapmate.core.ProofFrame
import vitapmate.core.ProofNetwork
import vitapmate.core.ProofOpened
import vitapmate.observability.Log
import vitapmate.core.ProofSocket
import vitapmate.core.TransportException

/** The server's sockets run here: each one's reader and writer, on the IO pool. */
private val sockets = CoroutineScope(SupervisorJob() + Dispatchers.IO)

private val platformTls get() = SSLSocketFactory.getDefault() as SSLSocketFactory
private val platformCleartext: (String) -> Boolean get() = NetworkSecurityPolicy.getInstance()::isCleartextTrafficPermitted

/**
 * The server's WebSocket for Rust's Convex client, over [WebSocket]. The listener is called from
 * the socket's own coroutines, never from inside [open], `send` or `disconnect`. TLS trusts what
 * the platform does (the network security config). Rust reconnects with backoff and drops a
 * silent socket, so this never retries.
 */
class SocketConvexNetwork internal constructor(
    private val scope: CoroutineScope,
    private val tls: SSLSocketFactory,
    private val cleartext: (String) -> Boolean,
) : ConvexNetwork {
    constructor() : this(sockets, platformTls, platformCleartext)

    override fun open(url: String, listener: ConvexSocketListener): ConvexSocket {
        val events = object : SocketEvents {
            override fun opened() = listener.onOpen()
            override fun text(text: String) = listener.onText(text)
            override fun closed(reason: String, refused: Refused?) = listener.onClosed(reason)
        }
        return WebSocket(url, events, scope, tls, cleartext).also { it.start() }
    }
}

/**
 * The account proof's WebSocket (vtop-tlsn's prover talks binary to the engine through it).
 * [open] waits for the upgrade: a refusal comes back as its status and body, a failure to reach
 * the server as [TransportException]. Never logs the URL: it carries the ticket.
 */
class SocketProofNetwork internal constructor(
    private val scope: CoroutineScope,
    private val tls: SSLSocketFactory,
    private val cleartext: (String) -> Boolean,
) : ProofNetwork {
    constructor() : this(sockets, platformTls, platformCleartext)

    // System CAs only: a user-added CA (a debugging proxy) must not vouch for VTOP in a proof.
    override fun trustRoots(): List<ByteArray> = runCatching {
        val store = KeyStore.getInstance("AndroidCAStore").apply { load(null) }
        store.aliases().toList().filter { it.startsWith("system:") }.mapNotNull { store.getCertificate(it)?.encoded }
    }.getOrElse { e ->
        // The bundled Sectigo intermediate still lets the proof check VTOP's certificate.
        Log.warn("proof", "system roots unreadable: ${e::class.simpleName}")
        emptyList()
    }

    override suspend fun open(url: String, headers: List<Header>, timeoutMs: UInt): ProofOpened {
        val inbox = Channel<ProofFrame>(Channel.UNLIMITED)
        // The upgrade's status: 101 opened, 0 never answered (see failure), else refused.
        val upgrade = CompletableDeferred<Refused>()
        var failure = ""
        val events = object : SocketEvents {
            override fun opened() {
                upgrade.complete(Refused(SWITCHING))
            }
            override fun text(text: String) {
                inbox.trySend(ProofFrame.Text(text))
            }
            override fun binary(data: ByteArray) {
                inbox.trySend(ProofFrame.Binary(data))
            }
            override fun closed(reason: String, refused: Refused?) {
                failure = reason
                upgrade.complete(refused ?: Refused(0))
                inbox.close()
            }
        }
        val socket = WebSocket(url, events, scope, tls, cleartext, headers.map { it.name to it.value }, PROOF_IDLE_MS)
        socket.start()
        val answer = withTimeoutOrNull(timeoutMs.toLong()) { upgrade.await() }
            ?: run {
                socket.disconnect()
                throw TransportException.Timeout("upgrade")
            }
        return when {
            answer.code == SWITCHING -> ProofOpened.Open(ProofSocketOver(socket, inbox))
            answer.code != 0 -> ProofOpened.Refused(answer.code.toUShort(), answer.body)
            failure in UNREACHABLE -> throw TransportException.Connect(failure)
            else -> throw TransportException.Other(failure)
        }
    }

    private class ProofSocketOver(private val socket: WebSocket, private val inbox: Channel<ProofFrame>) : ProofSocket {
        override suspend fun send(data: ByteArray) {
            if (!socket.sendBinary(data)) throw TransportException.Other("closed")
        }

        override suspend fun receive(): ProofFrame? = inbox.receiveCatching().getOrNull()

        override fun shutdown() {
            socket.disconnect()
            inbox.close()
        }
    }

    private companion object {
        const val SWITCHING = 101
        /** Failures that mean the network (a dropped link, a captive portal's TLS), shown as offline. */
        val UNREACHABLE = setOf(
            ConnectException::class, UnknownHostException::class, NoRouteToHostException::class, SocketException::class,
            SocketTimeoutException::class, EOFException::class, SSLException::class, SSLHandshakeException::class,
        ).map { it.java.simpleName }.toSet()
    }
}
