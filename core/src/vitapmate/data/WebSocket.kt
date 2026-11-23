package vitapmate.data

import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.EOFException
import java.io.IOException
import java.io.InputStream
import java.net.InetSocketAddress
import java.net.ProtocolException
import java.net.Socket
import java.net.URI
import java.net.UnknownServiceException
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Base64
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SNIHostName
import javax.net.ssl.SSLPeerUnverifiedException
import javax.net.ssl.SSLSocket
import javax.net.ssl.SSLSocketFactory
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.ConvexSocket

internal const val OP_CONTINUATION = 0x0
internal const val OP_TEXT = 0x1
internal const val OP_BINARY = 0x2
internal const val OP_CLOSE = 0x8
internal const val OP_PING = 0x9
internal const val OP_PONG = 0xA

/** Connecting, and the server's answer to the upgrade (the timeouts the chat had before). */
private const val CONNECT_MS = 10_000
private const val HANDSHAKE_READ_MS = 10_000

/** A backstop only: Rust drops a socket silent for 30 s (Convex pings every ~15 s). */
private const val IDLE_MS = 60_000

/** How long a close waits for the other side's close before the socket is shut. */
internal const val CLOSE_WAIT_MS = 5_000L

/** The largest message read; a bigger one fails the socket. */
private const val MAX_MESSAGE = 64 shl 20

/** One frame as read: unmasked [payload]. */
internal class Frame(val fin: Boolean, val opcode: Int, val payload: ByteArray)

/** The server answered the upgrade with [code] instead of 101, and [body] (at most 8 KB). */
internal class Refused(val code: Int, val body: ByteArray = ByteArray(0)) : ProtocolException("HTTP $code")

/** What happens to a [WebSocket], on its reader coroutine. */
internal interface SocketEvents {
    fun opened()
    fun text(text: String)
    fun binary(data: ByteArray) = Unit
    /** Once, last: why it closed. A refused upgrade comes as [Refused] first. */
    fun closed(reason: String, refused: Refused? = null)
}

/**
 * A small RFC 6455 client over a plain [Socket] (wss: TLS with SNI and the platform's hostname
 * check; ws only to hosts the network security config lets in clear), for the server's Convex
 * socket ([SocketConvexNetwork]) and the account proof's ([SocketProofNetwork]). One coroutine on
 * [scope] connects and then reads; another writes the queued frames in order. So [send] and
 * [disconnect] never block or call [events], and [events] hears [SocketEvents.opened] once the
 * upgrade is accepted, each message, and exactly one [SocketEvents.closed]: "closed <code>" after
 * a close frame, "ProtocolException HTTP <code>" when the upgrade was refused, else the failure's
 * class only (a message can quote the URL). Logs nothing: Rust logs the connection.
 */
internal class WebSocket(
    private val url: String,
    private val events: SocketEvents,
    private val scope: CoroutineScope,
    private val tlsFactory: SSLSocketFactory,
    private val cleartext: (String) -> Boolean,
    private val headers: List<Pair<String, String>> = emptyList(),
) : ConvexSocket {
    private val outbox = Channel<ByteArray>(Channel.UNLIMITED)
    private val closing = AtomicBoolean()
    private val socket = Socket()
    private var writer: Job? = null

    fun start() {
        scope.launch { run() }
    }

    override fun send(text: String): Boolean = !closing.get() && outbox.trySend(masked(OP_TEXT, text.encodeToByteArray())).isSuccess

    fun sendBinary(data: ByteArray): Boolean = !closing.get() && outbox.trySend(masked(OP_BINARY, data)).isSuccess

    override fun disconnect() {
        if (startClose()) scope.launch {
            delay(CLOSE_WAIT_MS)
            socket.close()
        }
    }

    /** Queues our close frame, the last one; false when closing already. */
    private fun startClose(): Boolean {
        if (!closing.compareAndSet(false, true)) return false
        outbox.trySend(masked(OP_CLOSE, byteArrayOf((1000 shr 8).toByte(), 1000.toByte())))
        outbox.close()
        return true
    }

    private suspend fun run() {
        var top = socket
        var refused: Refused? = null
        val reason = try {
            val uri = URI(url)
            val secure = uri.scheme == "wss"
            val host = uri.host ?: throw ProtocolException("no host")
            val port = uri.port.takeIf { it != -1 } ?: if (secure) 443 else 80
            if (!secure && !cleartext(host)) throw UnknownServiceException("cleartext not permitted")
            socket.connect(InetSocketAddress(host, port), CONNECT_MS)
            socket.soTimeout = HANDSHAKE_READ_MS
            socket.tcpNoDelay = true
            if (secure) top = tls(host, port)
            val input = BufferedInputStream(top.getInputStream())
            val output = BufferedOutputStream(top.getOutputStream())
            val key = Base64.getEncoder().encodeToString(ByteArray(16).also(random::nextBytes))
            output.write(upgradeRequest(uri, key, headers).encodeToByteArray())
            output.flush()
            checkUpgrade(readHead(input), key, input)
            socket.soTimeout = IDLE_MS
            writer = scope.launch {
                try {
                    for (frame in outbox) {
                        output.write(frame)
                        output.flush()
                    }
                } catch (_: IOException) {
                    socket.close()
                }
            }
            events.opened()
            read(input)
        } catch (e: Refused) {
            refused = e
            "ProtocolException HTTP ${e.code}"
        } catch (e: Exception) {
            e::class.java.simpleName
        }
        closing.set(true)
        outbox.close()
        runCatching { top.close() }
        socket.close()
        events.closed(reason, refused)
    }

    private fun tls(host: String, port: Int): Socket = (tlsFactory.createSocket(socket, host, port, true) as SSLSocket).apply {
        // Not for IP literals, which SNI can't carry.
        if (':' !in host && !host.last().isDigit()) sslParameters = sslParameters.apply { serverNames = listOf(SNIHostName(host)) }
        startHandshake()
        if (!HttpsURLConnection.getDefaultHostnameVerifier().verify(host, session)) throw SSLPeerUnverifiedException("hostname")
    }

    /** Reads frames until a close frame (answered, then waited for) or a failure; the close reason. */
    private suspend fun read(input: InputStream): String {
        val message = ByteArrayOutputStream()
        var type: Int? = null
        while (true) {
            val frame = readFrame(input)
            when (frame.opcode) {
                OP_TEXT, OP_BINARY, OP_CONTINUATION -> {
                    if ((frame.opcode == OP_CONTINUATION) != (type != null)) throw ProtocolException("bad fragment")
                    if (type == null) type = frame.opcode
                    message.write(frame.payload)
                    if (message.size() > MAX_MESSAGE) throw ProtocolException("message too big")
                    if (frame.fin) {
                        if (type == OP_TEXT) events.text(String(message.toByteArray(), Charsets.UTF_8)) else events.binary(message.toByteArray())
                        message.reset()
                        type = null
                    }
                }
                OP_PING -> outbox.trySend(masked(OP_PONG, frame.payload))
                OP_PONG -> Unit
                OP_CLOSE -> {
                    val p = frame.payload
                    val code = if (p.size >= 2) ((p[0].toInt() and 0xff) shl 8) or (p[1].toInt() and 0xff) else 1005
                    startClose()
                    withTimeoutOrNull(CLOSE_WAIT_MS) { writer?.join() }
                    return "closed $code"
                }
                else -> throw ProtocolException("opcode ${frame.opcode}")
            }
        }
    }

    private companion object {
        val random = SecureRandom()

        fun masked(opcode: Int, payload: ByteArray) = frame(opcode, payload, ByteArray(4).also(random::nextBytes))
    }
}

/** One frame: FIN, [opcode], the length in 7, 16 or 64 bits, then [payload] masked with [mask] when given (a client's frames are). */
internal fun frame(opcode: Int, payload: ByteArray, mask: ByteArray? = null, fin: Boolean = true): ByteArray {
    val out = ByteArrayOutputStream(payload.size + 14)
    out.write((if (fin) 0x80 else 0) or opcode)
    val maskBit = if (mask != null) 0x80 else 0
    when {
        payload.size < 126 -> out.write(maskBit or payload.size)
        payload.size <= 0xffff -> {
            out.write(maskBit or 126)
            out.write(payload.size shr 8)
            out.write(payload.size)
        }
        else -> {
            out.write(maskBit or 127)
            for (shift in 56 downTo 0 step 8) out.write((payload.size.toLong() shr shift).toInt())
        }
    }
    if (mask == null) return out.apply { write(payload) }.toByteArray()
    out.write(mask)
    return out.apply { write(payload.xor(mask)) }.toByteArray()
}

/** The next frame from [input], unmasked; checks the reserved bits and control frames' rules. */
internal fun readFrame(input: InputStream): Frame {
    val data = DataInputStream(input)
    val b0 = data.readUnsignedByte()
    val b1 = data.readUnsignedByte()
    if (b0 and 0x70 != 0) throw ProtocolException("reserved bits")
    val opcode = b0 and 0x0f
    val fin = b0 and 0x80 != 0
    val length = when (val n = b1 and 0x7f) {
        126 -> data.readUnsignedShort().toLong()
        127 -> data.readLong()
        else -> n.toLong()
    }
    if (opcode >= OP_CLOSE && (length > 125 || !fin)) throw ProtocolException("bad control frame")
    if (length !in 0..MAX_MESSAGE.toLong()) throw ProtocolException("frame too big")
    val mask = if (b1 and 0x80 != 0) ByteArray(4).also(data::readFully) else null
    val payload = ByteArray(length.toInt()).also(data::readFully)
    return Frame(fin, opcode, mask?.let(payload::xor) ?: payload)
}

private fun ByteArray.xor(mask: ByteArray) = ByteArray(size) { (this[it].toInt() xor mask[it % 4].toInt()).toByte() }

/** The HTTP/1.1 upgrade request for [url] with [key] and [headers]. */
internal fun upgradeRequest(url: URI, key: String, headers: List<Pair<String, String>> = emptyList()): String {
    val path = url.rawPath.ifEmpty { "/" } + (url.rawQuery?.let { "?$it" } ?: "")
    val host = if (url.port == -1) url.host else "${url.host}:${url.port}"
    val extra = headers.joinToString("") { (name, value) -> "$name: $value\r\n" }
    return "GET $path HTTP/1.1\r\nHost: $host\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
        "Sec-WebSocket-Key: $key\r\nSec-WebSocket-Version: 13\r\n$extra\r\n"
}

/** A refusal's body: its Content-Length's worth, at most 8 KB; empty when it says none. */
internal fun readBody(head: List<String>, input: InputStream): ByteArray {
    val length = head.firstOrNull { it.startsWith("content-length:", ignoreCase = true) }
        ?.substringAfter(':')?.trim()?.toIntOrNull()?.coerceIn(0, 8192) ?: return ByteArray(0)
    return runCatching { DataInputStream(input).let { data -> ByteArray(length).also(data::readFully) } }.getOrDefault(ByteArray(0))
}

/** The response's status line and headers, up to the blank line, leaving the frames after it in [input]. */
internal fun readHead(input: InputStream): List<String> {
    val lines = mutableListOf<String>()
    val line = StringBuilder()
    while (true) {
        val b = input.read()
        if (b == -1) throw EOFException()
        if (b != '\n'.code) {
            line.append(b.toChar())
            if (line.length > 8192) throw ProtocolException("header too long")
            continue
        }
        val text = line.trimEnd('\r').toString()
        if (text.isEmpty()) return lines
        lines += text
        line.clear()
    }
}

/** Throws unless [head] is a 101 upgrade to websocket that answers [key]; a refusal carries its body from [input]. */
internal fun checkUpgrade(head: List<String>, key: String, input: InputStream? = null) {
    val code = head.firstOrNull()?.split(' ')?.getOrNull(1)?.toIntOrNull() ?: throw ProtocolException("bad status line")
    if (code != 101) throw Refused(code, input?.let { readBody(head, it) } ?: ByteArray(0))
    val headers = head.drop(1).associate { it.substringBefore(':').trim().lowercase() to it.substringAfter(':').trim() }
    if (!headers["upgrade"].equals("websocket", ignoreCase = true) || headers["connection"]?.contains("upgrade", ignoreCase = true) != true) {
        throw ProtocolException("not an upgrade")
    }
    if (headers["sec-websocket-accept"] != accept(key)) throw ProtocolException("bad Sec-WebSocket-Accept")
}

/** The Sec-WebSocket-Accept a server must answer [key] with. */
internal fun accept(key: String): String =
    Base64.getEncoder().encodeToString(MessageDigest.getInstance("SHA-1").digest((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encodeToByteArray()))
