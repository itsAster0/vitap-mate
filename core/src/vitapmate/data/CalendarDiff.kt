package vitapmate.data

/** An event the calendar sync writes (app's CalendarSync.kt); [key] stays the same across syncs. Times are epoch ms. */
data class SyncEvent(
    val key: String,
    val title: String,
    val start: Long,
    val end: Long,
    val allDay: Boolean = false,
    val location: String = "",
    val description: String = "",
)

/** What a sync writes: [insert] new events, [update] changed rows (row id to event), [delete] row ids. */
data class CalendarDiff(val insert: List<SyncEvent>, val update: Map<Long, SyncEvent>, val delete: List<Long>) {
    companion object {
        /**
         * From the calendar's [rows] (row id to what it holds, null without a key) to [wanted]. What
         * ends before [from] is left alone: a past row stays unless [wanted] moves it later, and a
         * past event isn't written. A row without a key, or a second row with the same key, goes.
         */
        fun of(rows: Map<Long, SyncEvent?>, wanted: List<SyncEvent>, from: Long): CalendarDiff {
            val want = wanted.filter { it.end >= from }.associateBy { it.key }
            val seen = HashSet<String>()
            val update = HashMap<Long, SyncEvent>()
            val delete = ArrayList<Long>()
            for ((id, row) in rows) {
                val w = row?.let { want[it.key] }
                if (w == null && row != null && row.end < from) seen += row.key
                else if (w == null || !seen.add(w.key)) delete += id
                else if (w != row) update[id] = w
            }
            return CalendarDiff(want.values.filter { it.key !in seen }, update, delete)
        }
    }
}
