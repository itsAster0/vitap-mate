package vitapmate.observability

/**
 * One line of `cache/logs/app.jsonl` (written by Rust's `obs.rs`, for Kotlin and Rust alike):
 * `t` epoch ms, `l` level, `src`, `msg`, optional `flow`, `regno`, `ms` (span close / flow end),
 * `outcome` (`ok` or an error kind); every other field is in [fields].
 */
data class LogLine(
    val t: Long,
    val level: String,
    val src: String,
    val msg: String,
    val flow: String?,
    val regno: String?,
    val ms: Long?,
    val outcome: String?,
    val fields: Map<String, String>,
)

/** One flow: its [name], first line's time, duration and outcome (null while it runs), and its lines in order. */
data class FlowSummary(val id: String, val name: String, val startT: Long, val ms: Long?, val outcome: String?, val lines: List<LogLine>)

/** Inspector filters: lines at [minLevel] or above, whose src starts with [src], containing [text] (any field). */
data class LogFilter(val minLevel: String = "debug", val src: String = "", val text: String = "") {
    fun matches(line: LogLine): Boolean =
        levelRank(line.level) >= levelRank(minLevel) &&
            line.src.startsWith(src) &&
            (text.isEmpty() || line.msg.contains(text, true) || line.src.contains(text, true) ||
                line.flow?.contains(text, true) == true || line.fields.values.any { it.contains(text, true) })

    /** Flows with a matching line, each narrowed to its matching lines. */
    fun apply(flows: List<FlowSummary>): List<FlowSummary> =
        if (this == LogFilter()) flows
        else flows.mapNotNull { f -> f.lines.filter(::matches).takeIf { it.isNotEmpty() }?.let { f.copy(lines = it) } }
}

private fun levelRank(level: String) = when (level.lowercase()) {
    "trace" -> 0
    "debug" -> 1
    "info" -> 2
    "warn" -> 3
    "error" -> 4
    else -> 2
}

/** Parses one JSON line; null when it is not a JSON object (a torn or corrupt line). */
fun parseLogLine(text: String): LogLine? {
    val obj = JsonLine(text).parse() ?: return null
    return LogLine(
        t = obj.remove("t")?.toLongOrNull() ?: 0,
        level = obj.remove("l") ?: "info",
        src = obj.remove("src").orEmpty(),
        msg = obj.remove("msg").orEmpty(),
        flow = obj.remove("flow"),
        regno = obj.remove("regno"),
        // Rust writes ms as a number, Kotlin's flow() as a string.
        ms = obj.remove("ms")?.toLongOrNull(),
        outcome = obj.remove("outcome"),
        fields = obj,
    )
}

/**
 * Groups lines by flow, newest flow first (by where each starts in the log, not its clock time,
 * which jumps when the phone's clock is changed). A flow's name comes from Kotlin's `flow()` start
 * line (`step`), else its first line's src; its ms and outcome from the `flow()` end line (both
 * null while it runs, or when the end line is missing).
 */
fun groupFlows(lines: List<LogLine>): List<FlowSummary> {
    val byFlow = LinkedHashMap<String, MutableList<LogLine>>()
    for (line in lines) line.flow?.let { byFlow.getOrPut(it, ::mutableListOf).add(line) }
    return byFlow.map { (id, ls) ->
        val start = ls.firstOrNull { it.src == "flow" && it.fields["ev"] == "start" }
        val end = ls.lastOrNull { it.src == "flow" && it.fields["ev"] in END_EVENTS }
        FlowSummary(
            id = id,
            name = start?.fields?.get("step") ?: end?.fields?.get("step") ?: ls.first().src,
            startT = ls.first().t,
            ms = end?.let { it.ms ?: ls.lastOrNull { l -> l.ms != null }?.ms },
            outcome = end?.let { it.outcome ?: if (it.fields["ev"] == "done") "ok" else it.fields["ev"] },
            lines = ls,
        )
    }.asReversed()
}

private val END_EVENTS = setOf("done", "failed", "cancelled")

/**
 * A small parser for one flat JSON object (what `obs.rs` writes): values become strings;
 * nested objects or arrays are kept as their raw JSON text.
 */
private class JsonLine(private val s: String) {
    private var i = 0

    fun parse(): MutableMap<String, String>? = runCatching {
        val out = HashMap<String, String>()
        ws()
        expect('{')
        ws()
        if (peek() == '}') return@runCatching out
        while (true) {
            ws()
            val key = string()
            ws()
            expect(':')
            ws()
            value()?.let { out[key] = it }
            ws()
            when (s[i++]) {
                ',' -> continue
                '}' -> break
                else -> error("expected , or }")
            }
        }
        out
    }.getOrNull()

    private fun peek() = s[i]

    private fun ws() {
        while (i < s.length && s[i].isWhitespace()) i++
    }

    private fun expect(c: Char) {
        check(s[i++] == c) { "expected $c" }
    }

    private fun value(): String? = when (peek()) {
        '"' -> string()
        '{', '[' -> raw()
        'n' -> null.also { i += 4 }
        't' -> "true".also { i += 4 }
        'f' -> "false".also { i += 5 }
        else -> {
            val from = i
            while (i < s.length && s[i] !in ",}] \t") i++
            s.substring(from, i).also { check(it.isNotEmpty()) }
        }
    }

    private fun string(): String {
        expect('"')
        val from = i
        while (s[i] != '"' && s[i] != '\\') i++
        if (s[i] == '"') return s.substring(from, i++)
        val sb = StringBuilder().append(s, from, i)
        while (true) {
            when (val c = s[i++]) {
                '"' -> return sb.toString()
                '\\' -> when (val e = s[i++]) {
                    'n' -> sb.append('\n')
                    't' -> sb.append('\t')
                    'r' -> sb.append('\r')
                    'b' -> sb.append('\b')
                    'f' -> sb.append('\u000c')
                    'u' -> sb.append(s.substring(i, i + 4).toInt(16).toChar()).also { i += 4 }
                    else -> sb.append(e)
                }
                else -> sb.append(c)
            }
        }
    }

    /** A nested object or array, as text (strings inside may hold brackets). */
    private fun raw(): String {
        val from = i
        var depth = 0
        do {
            when (s[i]) {
                '"' -> string().also { i-- }
                '{', '[' -> depth++
                '}', ']' -> depth--
            }
            i++
        } while (depth > 0)
        return s.substring(from, i)
    }
}
