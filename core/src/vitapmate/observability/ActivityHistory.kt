package vitapmate.observability

import java.io.File
import org.json.JSONArray
import org.json.JSONObject

/**
 * Finished [Run]s kept after the log files roll over, one JSON object per line in
 * `cache/activity.jsonl`, oldest first, so "Show older" in Activity reaches back further than
 * the logs. [save] adds the finished runs the logs hold that aren't saved yet (at start and after
 * a background sync); [StoreCap] trims the oldest lines.
 */
object ActivityHistory {
    private lateinit var file: File

    internal fun attach(cache: File) {
        file = File(cache, "activity.jsonl")
    }

    val bytes: Long get() = file.length()

    /** Saves the finished runs in the log files that are newer than the last saved one. Blocking. */
    @Synchronized
    fun save() {
        val last = load().maxOfOrNull { it.startT } ?: 0L
        val runs = runsOf(groupFlows(Inspector.read(Int.MAX_VALUE)), Inspector.startedAt)
            .filter { !it.running && !it.waiting && it.startT > last }
            .sortedBy { it.startT }
        if (runs.isEmpty()) return
        file.appendText(runs.joinToString("") { encode(it).toString() + "\n" })
        Log.info("activity", "history saved", fields = mapOf("runs" to "${runs.size}", "bytes" to "${file.length()}"))
    }

    /** Every saved run, newest first. Blocking. */
    fun load(): List<Run> {
        if (!file.exists()) return emptyList()
        return file.readLines().mapNotNull { line -> runCatching { decode(JSONObject(line)) }.getOrNull() }.asReversed()
    }

    /** Drops the oldest lines until the file is at most [max] bytes. Blocking. */
    @Synchronized
    fun trim(max: Long) {
        if (file.length() <= max) return
        val lines = file.readLines()
        var size = file.length()
        var from = 0
        while (from < lines.size && size > max) size -= lines[from++].toByteArray().size + 1
        val tmp = File(file.path + ".tmp")
        tmp.writeText(lines.drop(from).joinToString("") { "$it\n" })
        tmp.renameTo(file)
    }

    private fun encode(run: Run) = JSONObject()
        .put("id", run.id)
        .put("flows", JSONArray(run.flows))
        .put("title", run.title)
        .put("trigger", run.trigger.name)
        .put("t", run.startT)
        .put("route", run.route)
        .put("fallback", run.fallback)
        .put("steps", JSONArray(run.steps.map { s ->
            JSONObject()
                .put("label", s.label)
                .put("state", s.state.name)
                .put("t", s.startT)
                .put("end", s.endT)
                .put("reason", s.reason)
                .put("count", s.count)
                .put("pages", JSONArray(s.pages.map { p -> JSONObject().put("name", p.name).put("status", p.status).put("ms", p.ms) }))
        }))

    private fun decode(o: JSONObject): Run {
        val flows = o.getJSONArray("flows")
        val steps = o.getJSONArray("steps")
        return Run(
            o.getString("id"),
            List(flows.length()) { flows.getString(it) },
            o.getString("title"),
            RunTrigger.valueOf(o.getString("trigger")),
            o.getLong("t"),
            List(steps.length()) { i ->
                val s = steps.getJSONObject(i)
                val pages = s.getJSONArray("pages")
                RunStep(
                    s.getString("label"),
                    StepState.valueOf(s.getString("state")),
                    s.getLong("t"),
                    s.optLong("end").takeIf { s.has("end") },
                    s.optString("reason").takeIf { s.has("reason") },
                    List(pages.length()) { j ->
                        val p = pages.getJSONObject(j)
                        PageHit(p.getString("name"), p.getString("status"), p.optLong("ms").takeIf { p.has("ms") })
                    },
                    s.optInt("count", 1),
                )
            },
            o.optString("route").takeIf { o.has("route") },
            o.optString("fallback").takeIf { o.has("fallback") },
        )
    }
}
