package vitapmate.observability

import java.io.File
import java.io.RandomAccessFile

/**
 * Keeps the last [max] lines of `<dir>/app.jsonl` (and its roll-over `.1`), reading only what
 * was appended since the last [poll]. A torn last line waits for the next poll; corrupt lines
 * are skipped.
 */
class LogTail(private val dir: File, private val max: Int = 2000) {
    private val current = File(dir, "app.jsonl")
    private val rolled = File(dir, "app.jsonl.1")
    private val lines = ArrayDeque<LogLine>()
    private var offset = -1L
    private var rolledStamp = 0L to 0L

    /** Reads new lines; true when [lines] changed. */
    fun poll(): Boolean {
        val stamp = rolled.length() to rolled.lastModified()
        val size = current.length()
        if (offset < 0 || size < offset || stamp != rolledStamp) {
            rolledStamp = stamp
            lines.clear()
            offset = 0
            val tail = readNew(current)
            if (tail.size < max) lastLines(rolled.takeIf { it.exists() }?.readBytes() ?: ByteArray(0), max - tail.size).forEach(::add)
            tail.forEach(::add)
            return true
        }
        if (size == offset) return false
        return readNew(current).onEach(::add).isNotEmpty()
    }

    fun lines(): List<LogLine> = lines.toList()

    private fun add(line: LogLine) {
        lines.addLast(line)
        if (lines.size > max) lines.removeFirst()
    }

    /** Lines from [offset] to the last complete line; moves [offset] past them. */
    private fun readNew(file: File): List<LogLine> {
        if (!file.exists()) return emptyList()
        val bytes = RandomAccessFile(file, "r").use { f ->
            if (f.length() <= offset) return emptyList()
            f.seek(offset)
            ByteArray((f.length() - offset).toInt()).also(f::readFully)
        }
        val end = bytes.lastIndexOf('\n'.code.toByte()) + 1
        offset += end
        return lastLines(bytes.copyOf(end), max)
    }

    /** The last [n] parsed lines of [bytes] (split on '\n' bytes, so UTF-8 stays whole). */
    private fun lastLines(bytes: ByteArray, n: Int): List<LogLine> {
        val out = ArrayList<LogLine>()
        var to = bytes.size
        var i = bytes.size - 1
        while (i >= -1 && out.size < n) {
            if (i == -1 || bytes[i] == '\n'.code.toByte()) {
                if (to > i + 1) parseLogLine(String(bytes, i + 1, to - i - 1, Charsets.UTF_8))?.let(out::add)
                to = i
            }
            i--
        }
        return out.asReversed()
    }
}
