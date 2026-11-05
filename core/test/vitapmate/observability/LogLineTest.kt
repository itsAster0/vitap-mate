package vitapmate.observability

import java.io.File
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class LogLineTest {
    // Real lines from the emulator's files/logs/app.jsonl.
    private val sample = """
        {"ev":"start","flow":"e6995947","l":"info","msg":"core_init start","src":"flow","step":"core_init","t":1793868775720}
        {"flow":"e6995947","l":"info","ms":0,"msg":"done","outcome":"ok","src":"core.new","t":1793868775727}
        {"ev":"done","flow":"e6995947","l":"info","ms":"10","msg":"core_init done","src":"flow","step":"core_init","t":1793868775728,"outcome":"ok"}
        {"l":"info","msg":"tab timetable","src":"ui.shell","t":1793868776070}
        {"ev":"start","flow":"aa","l":"info","msg":"attendance.refresh start","src":"flow","step":"attendance.refresh","t":1793868777000}
        {"flow":"aa","l":"warn","ms":812,"msg":"done","outcome":"Offline","error":"no route to host","src":"core.fetch_attendance","t":1793868777812,"regno":"23BCE1234"}
        {"ev":"failed","flow":"aa","l":"warn","ms":"815","msg":"attendance.refresh failed: Offline","src":"flow","step":"attendance.refresh","t":1793868777815,"outcome":"Offline"}
        {"ev":"start","flow":"bb","l":"info","msg":"timetable.refresh start","src":"flow","step":"timetable.refresh","t":1793868779000}
    """.trimIndent()

    @Test
    fun parsesFieldsAndKeepsTheRest() {
        val line = parseLogLine("""{"flow":"aa","l":"warn","ms":812,"msg":"done \"q\" é\n","outcome":"Offline","src":"core.x","t":5,"regno":"23BCE1234","ok":true,"n":null,"nested":{"a":[1,"}"]}}""")!!
        assertEquals(5, line.t)
        assertEquals("warn", line.level)
        assertEquals("core.x", line.src)
        assertEquals("done \"q\" é\n", line.msg)
        assertEquals("aa", line.flow)
        assertEquals("23BCE1234", line.regno)
        assertEquals(812, line.ms)
        assertEquals("Offline", line.outcome)
        assertEquals(mapOf("ok" to "true", "nested" to """{"a":[1,"}"]}"""), line.fields)
    }

    @Test
    fun corruptLinesAreNull() {
        assertNull(parseLogLine("""{"t":1,"msg":"torn"""))
        assertNull(parseLogLine("not json"))
        assertNull(parseLogLine(""))
    }

    @Test
    fun groupsFlowsNewestFirst() {
        val flows = groupFlows(sample.lines().mapNotNull(::parseLogLine))
        assertEquals(listOf("bb", "aa", "e6995947"), flows.map { it.id })
        val (running, failed, init) = flows
        assertEquals("timetable.refresh", running.name)
        assertNull(running.ms)
        assertNull(running.outcome)
        assertEquals("attendance.refresh", failed.name)
        assertEquals(815, failed.ms)
        assertEquals("Offline", failed.outcome)
        assertEquals(3, failed.lines.size)
        assertEquals("core_init", init.name)
        assertEquals(10, init.ms)
        assertEquals("ok", init.outcome)
        assertEquals(1793868775720, init.startT)
    }

    @Test
    fun filters() {
        val flows = groupFlows(sample.lines().mapNotNull(::parseLogLine))
        assertEquals(listOf("aa"), LogFilter(minLevel = "warn").apply(flows).map { it.id })
        assertEquals(2, LogFilter(minLevel = "warn").apply(flows).single().lines.size)
        assertEquals(listOf("e6995947"), LogFilter(src = "core.new").apply(flows).map { it.id })
        assertEquals(listOf("aa"), LogFilter(text = "no route").apply(flows).map { it.id })
        assertEquals(flows, LogFilter().apply(flows))
    }

    @Test
    fun tailReadsAppendsRollsAndKeepsTheLastN() {
        val dir = Files.createTempDirectory("logs").toFile()
        val log = File(dir, "app.jsonl")
        fun line(n: Int) = """{"t":$n,"l":"info","src":"s","msg":"m$n","flow":"f${n % 3}"}""" + "\n"
        File(dir, "app.jsonl.1").writeText((1..5).joinToString("") { line(it) })
        log.writeText((6..8).joinToString("") { line(it) } + """{"t":9,"l":"in""")
        val tail = LogTail(dir, max = 6)
        assertTrue(tail.poll())
        assertEquals((3..8).toList(), tail.lines().map { it.t.toInt() })
        assertFalse(tail.poll())
        // The torn line completes, one more arrives.
        log.appendText("""fo","src":"s","msg":"m9"}""" + "\n" + line(10))
        assertTrue(tail.poll())
        assertEquals((5..10).toList(), tail.lines().map { it.t.toInt() })
        // Roll-over: app.jsonl -> .1, a new app.jsonl.
        log.renameTo(File(dir, "app.jsonl.1"))
        File(dir, "app.jsonl.1").setLastModified(System.currentTimeMillis() + 5_000)
        log.writeText(line(11))
        assertTrue(tail.poll())
        assertEquals((6..11).toList(), tail.lines().map { it.t.toInt() })
        dir.deleteRecursively()
    }

    @Test
    fun aFullMegabyteLoadsFast() {
        val dir = Files.createTempDirectory("logs").toFile()
        val one = """{"flow":"aa","l":"warn","ms":812,"msg":"done","outcome":"Offline","error":"no route to host","src":"core.fetch_attendance","t":1793868777812,"regno":"23BCE1234"}""" + "\n"
        val text = one.repeat(1024 * 1024 / one.length)
        File(dir, "app.jsonl.1").writeText(text)
        File(dir, "app.jsonl").writeText(text)
        val start = System.nanoTime()
        val tail = LogTail(dir)
        tail.poll()
        val flows = groupFlows(tail.lines())
        val ms = (System.nanoTime() - start) / 1_000_000
        println("2 MB of logs: last ${tail.lines().size} lines, ${flows.size} flows in $ms ms")
        assertEquals(2000, tail.lines().size)
        assertTrue(ms < 1000, "took $ms ms")
        dir.deleteRecursively()
    }
}
