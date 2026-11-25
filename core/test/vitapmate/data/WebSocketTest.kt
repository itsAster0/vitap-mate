package vitapmate.data

import java.io.BufferedInputStream
import java.io.ByteArrayInputStream
import java.io.OutputStream
import java.net.InetAddress
import java.net.ProtocolException
import java.net.ServerSocket
import java.net.Socket
import java.net.URI
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLSocketFactory
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.cancel
import vitapmate.core.ConvexSocket
import vitapmate.core.ConvexSocketListener
import vitapmate.core.Header
import vitapmate.core.ProofFrame
import vitapmate.core.ProofOpened
import vitapmate.core.NoHandle

class WebSocketTest {
    private val mask = byteArrayOf(0x37, 0xfa.toByte(), 0x21, 0x3d)

    @Test
    fun `a short masked frame matches RFC 6455's example`() {
        // RFC 6455 §5.7: a masked "Hello".
        val expected = byteArrayOf(0x81.toByte(), 0x85.toByte(), 0x37, 0xfa.toByte(), 0x21, 0x3d, 0x7f, 0x9f.toByte(), 0x4d, 0x51, 0x58)
        assertContentEquals(expected, frame(OP_TEXT, "Hello".encodeToByteArray(), mask))
        val read = readFrame(ByteArrayInputStream(expected))
        assertTrue(read.fin)
        assertEquals(OP_TEXT, read.opcode)
        assertEquals("Hello", read.payload.decodeToString())
    }

    @Test
    fun `lengths over 125 take 16 bits and over 65535 take 64`() {
        for ((size, marker, header) in listOf(Triple(300, 126, 4), Triple(70_000, 127, 10))) {
            val payload = ByteArray(size) { it.toByte() }
            val bytes = frame(OP_BINARY, payload, mask)
            assertEquals(0x80 or marker, bytes[1].toInt() and 0xff)
            assertEquals(header + 4 + size, bytes.size)
            val length = (2 until header).fold(0L) { n, i -> (n shl 8) or (bytes[i].toLong() and 0xff) }
            assertEquals(size.toLong(), length)
            assertFalse(bytes.copyOfRange(header + 4, bytes.size).contentEquals(payload), "masked on the wire")
            assertContentEquals(payload, readFrame(ByteArrayInputStream(bytes)).payload)
            // A server's frame: unmasked.
            assertContentEquals(payload, readFrame(ByteArrayInputStream(frame(OP_BINARY, payload))).payload)
        }
    }

    @Test
    fun `bad frames are refused`() {
        val rsv = byteArrayOf(0xc1.toByte(), 0)
        assertFailsWith<ProtocolException> { readFrame(ByteArrayInputStream(rsv)) }
        val longPing = frame(OP_PING, ByteArray(126))
        assertFailsWith<ProtocolException> { readFrame(ByteArrayInputStream(longPing)) }
        val fragmentedClose = frame(OP_CLOSE, ByteArray(2), fin = false)
        assertFailsWith<ProtocolException> { readFrame(ByteArrayInputStream(fragmentedClose)) }
    }

    @Test
    fun `the handshake checks Sec-WebSocket-Accept`() {
        // RFC 6455 §1.3's example key and answer.
        val key = "dGhlIHNhbXBsZSBub25jZQ=="
        assertEquals("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", accept(key))
        val ok = listOf("HTTP/1.1 101 Switching Protocols", "Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Accept: ${accept(key)}")
        checkUpgrade(ok, key)
        assertFailsWith<ProtocolException> { checkUpgrade(ok.dropLast(1) + "Sec-WebSocket-Accept: AAAA", key) }
        assertFailsWith<ProtocolException> { checkUpgrade(ok - "Upgrade: websocket", key) }
        assertEquals(401, assertFailsWith<Refused> { checkUpgrade(listOf("HTTP/1.1 401 Unauthorized"), key) }.code)
        val request = upgradeRequest(URI("ws://127.0.0.1:3210/api/sync?x=1"), key)
        assertTrue(request.startsWith("GET /api/sync?x=1 HTTP/1.1\r\nHost: 127.0.0.1:3210\r\n"))
        assertTrue(request.endsWith("Sec-WebSocket-Version: 13\r\n\r\n"))
    }

    // Loopback: a tiny server on this machine and the real client.

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val server = ServerSocket(0, 1, InetAddress.getLoopbackAddress())
    private val events = LinkedBlockingQueue<String>()
    private val listener = object : ConvexSocketListener(NoHandle) {
        override fun onOpen() {
            events += "open"
        }

        override fun onText(text: String) {
            events += "text $text"
        }

        override fun onClosed(reason: String) {
            events += "closed: $reason"
        }
    }
    private lateinit var peer: Socket
    private lateinit var input: BufferedInputStream
    private lateinit var output: OutputStream

    @AfterTest
    fun stop() {
        if (::peer.isInitialized) peer.close()
        scope.cancel()
        server.close()
    }

    private fun next(): String = events.poll(5, TimeUnit.SECONDS) ?: error("no event")

    private fun open(cleartext: Boolean = true): ConvexSocket =
        SocketConvexNetwork(scope, SSLSocketFactory.getDefault() as SSLSocketFactory) { cleartext }
            .open("ws://127.0.0.1:${server.localPort}/api/sync", listener)

    /** Takes the client's upgrade and answers it with [answer] (by default, the right 101). */
    private fun handshake(answer: (key: String) -> String = { "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept(it)}\r\n\r\n" }): List<String> {
        peer = server.accept().apply { soTimeout = 5_000 }
        input = BufferedInputStream(peer.getInputStream())
        output = peer.getOutputStream()
        val head = readHead(input)
        val key = head.first { it.startsWith("Sec-WebSocket-Key:") }.substringAfter(':').trim()
        write(answer(key).encodeToByteArray())
        return head
    }

    private fun write(bytes: ByteArray) {
        output.write(bytes)
        output.flush()
    }

    @Test
    fun `texts go masked and come back whole from fragments, pings get pongs`() {
        val socket = open()
        val head = handshake()
        assertEquals("GET /api/sync HTTP/1.1", head.first())
        assertEquals("open", next())

        assertTrue(socket.send("hi"))
        val sent = readFrame(input)
        assertEquals(OP_TEXT, sent.opcode)
        assertEquals("hi", sent.payload.decodeToString())

        write(frame(OP_TEXT, "hel".encodeToByteArray(), fin = false))
        write(frame(OP_PING, "p".encodeToByteArray()))
        write(frame(OP_CONTINUATION, "lo".encodeToByteArray(), fin = false))
        write(frame(OP_CONTINUATION, " there".encodeToByteArray()))
        val pong = readFrame(input)
        assertEquals(OP_PONG, pong.opcode)
        assertEquals("p", pong.payload.decodeToString())
        assertEquals("text hello there", next())
        // Binary messages are dropped.
        write(frame(OP_BINARY, byteArrayOf(1)))
        write(frame(OP_TEXT, "after".encodeToByteArray()))
        assertEquals("text after", next())
        socket.disconnect()
        assertEquals(OP_CLOSE, readFrame(input).opcode)
    }

    @Test
    fun `the server's close is answered and reported with its code`() {
        open()
        handshake()
        assertEquals("open", next())
        write(frame(OP_CLOSE, byteArrayOf(0x03, 0xe9.toByte()))) // 1001
        val answer = readFrame(input)
        assertEquals(OP_CLOSE, answer.opcode)
        assertContentEquals(byteArrayOf(0x03, 0xe8.toByte()), answer.payload) // 1000
        assertEquals("closed: closed 1001", next())
    }

    @Test
    fun `disconnect sends close, refuses sends and reports the server's answer`() {
        val socket = open()
        handshake()
        assertEquals("open", next())
        socket.disconnect()
        assertFalse(socket.send("late"))
        val close = readFrame(input)
        assertEquals(OP_CLOSE, close.opcode)
        assertContentEquals(byteArrayOf(0x03, 0xe8.toByte()), close.payload)
        write(frame(OP_CLOSE, close.payload))
        assertEquals("closed: closed 1000", next())
        assertTrue(events.isEmpty())
    }

    @Test
    fun `a wrong accept, a refusal, a dropped connection and cleartext are failures`() {
        open()
        handshake { "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: AAAA\r\n\r\n" }
        assertEquals("closed: ProtocolException", next())

        open()
        handshake { "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n" }
        assertEquals("closed: ProtocolException HTTP 401", next())

        open()
        handshake()
        assertEquals("open", next())
        peer.close()
        assertEquals("closed: EOFException", next())

        open(cleartext = false)
        assertEquals("closed: UnknownServiceException", next())
    }

    private fun proof(timeoutMs: UInt = 5_000u) = scope.async {
        SocketProofNetwork(scope, SSLSocketFactory.getDefault() as SSLSocketFactory) { true }
            .open("ws://127.0.0.1:${server.localPort}/proof?ticket=t", listOf(Header("X-Test", "1")), timeoutMs)
    }

    @Test
    fun `the proof socket carries binary both ways and texts in order`() = runBlocking {
        val opened = proof()
        val head = handshake()
        assertTrue("X-Test: 1" in head)
        val socket = assertIs<ProofOpened.Open>(opened.await()).socket
        socket.send(byteArrayOf(1, 2))
        val sent = readFrame(input)
        assertEquals(OP_BINARY, sent.opcode)
        assertContentEquals(byteArrayOf(1, 2), sent.payload)
        write(frame(OP_BINARY, byteArrayOf(3)))
        write(frame(OP_TEXT, "done".encodeToByteArray()))
        write(frame(OP_CLOSE, byteArrayOf(0x03, 0xe8.toByte())))
        assertContentEquals(byteArrayOf(3), (socket.receive() as ProofFrame.Binary).data)
        assertEquals("done", (socket.receive() as ProofFrame.Text).text)
        assertNull(socket.receive())
    }

    @Test
    fun `a refused proof upgrade comes back with its status and body`() = runBlocking {
        val opened = proof()
        handshake { "HTTP/1.1 403 Forbidden\r\nContent-Length: 5\r\n\r\nnope!" }
        val answer = opened.await()
        val refused = assertIs<ProofOpened.Refused>(answer)
        assertEquals(403.toUShort(), refused.status)
        assertEquals("nope!", refused.body.decodeToString())
    }
}
