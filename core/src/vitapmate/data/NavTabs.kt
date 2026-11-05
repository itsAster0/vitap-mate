package vitapmate.data

/**
 * The floating nav bar's tabs (feature ids), as pure rules: at most [MAX], no duplicates or
 * blanks, and always [MORE] (it lists every feature, Settings and the Inspector, so nothing
 * becomes unreachable); [MORE] stays last unless moved.
 */
object NavTabs {
    const val MAX = 5
    const val MORE = "more"
    val DEFAULT = listOf("timetable", "attendance", MORE)

    /** [ids] cleaned: trimmed, deduped, blanks dropped, [MORE] kept (added last if missing), at most [MAX]. */
    fun normalize(ids: List<String>): List<String> {
        val clean = ids.map(String::trim).filter(String::isNotEmpty).distinct()
        if (MORE in clean.take(MAX)) return clean.take(MAX)
        return clean.filter { it != MORE }.take(MAX - 1) + MORE
    }

    /** Pins [id] before [MORE] if it ends last; unchanged when already pinned or full. */
    fun add(tabs: List<String>, id: String): List<String> {
        if (id in tabs || tabs.size >= MAX) return tabs
        val at = tabs.indexOf(MORE).takeIf { it == tabs.lastIndex } ?: tabs.size
        return normalize(tabs.toMutableList().apply { add(at, id) })
    }

    /** Unpins [id]; [MORE] can't be removed. */
    fun remove(tabs: List<String>, id: String): List<String> = if (id == MORE) tabs else tabs.filter { it != id }

    /** Moves [id] to index [to] (clamped). */
    fun move(tabs: List<String>, id: String, to: Int): List<String> {
        if (id !in tabs) return tabs
        val rest = tabs.filter { it != id }
        return rest.toMutableList().apply { add(to.coerceIn(0, rest.size), id) }
    }

    fun encode(tabs: List<String>) = tabs.joinToString(",")

    fun decode(saved: String?): List<String> = saved?.let { normalize(it.split(',')) } ?: DEFAULT
}
