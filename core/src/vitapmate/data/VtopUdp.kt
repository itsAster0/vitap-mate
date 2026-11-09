package vitapmate.data

import android.content.Context
import android.net.ConnectivityManager
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.Inet4Address
import java.net.SocketTimeoutException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.Datagram
import vitapmate.core.TransportException
import vitapmate.core.UdpNetwork
import vitapmate.core.UdpSocket

/**
 * The UDP route's sockets (Rust's vtop-wire client runs on them): [java.net.DatagramSocket]s and
 * DNS on the network the phone uses at the time, blocking calls on [Dispatchers.IO].
 */
class VtopUdp(context: Context) : UdpNetwork {
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)
    private val lookups = CoroutineScope(Dispatchers.IO)

    override fun open(): UdpSocket {
        val socket = DatagramSocket()
        runCatching { connectivity?.activeNetwork?.bindSocket(socket) }
        return Socket(socket)
    }

    override suspend fun resolve(host: String, timeoutMs: UInt): List<String> {
        val network = connectivity?.activeNetwork
        // DNS can't be interrupted: give up on it without waiting.
        val lookup = lookups.async { runCatching { network?.getAllByName(host) ?: InetAddress.getAllByName(host) }.getOrNull() }
        val found = withTimeoutOrNull(timeoutMs.toLong()) { lookup.await() } ?: return emptyList()
        return found.sortedBy { it !is Inet4Address }.map { address(it) }
    }

    private class Socket(private val socket: DatagramSocket) : UdpSocket {
        override suspend fun send(ip: String, port: UShort, data: ByteArray) = withContext(Dispatchers.IO) {
            try {
                socket.send(DatagramPacket(data, data.size, InetAddress.getByName(ip), port.toInt()))
            } catch (e: Exception) {
                throw TransportException.Other("udp send: ${e::class.simpleName}")
            }
        }

        override suspend fun receive(timeoutMs: UInt): Datagram? = withContext(Dispatchers.IO) {
            val packet = DatagramPacket(ByteArray(2048), 2048)
            try {
                socket.soTimeout = timeoutMs.coerceIn(1u, Int.MAX_VALUE.toUInt()).toInt()
                socket.receive(packet)
                Datagram(address(packet.address), packet.port.toUShort(), packet.data.copyOf(packet.length))
            } catch (_: SocketTimeoutException) {
                null
            } catch (e: Exception) {
                throw TransportException.Other("udp receive: ${e::class.simpleName}")
            }
        }

        override fun shutdown() = socket.close()
    }
}

/** Without an IPv6 scope (`%wlan0`), as Rust parses it. */
private fun address(address: InetAddress): String = address.hostAddress.orEmpty().substringBefore('%')
