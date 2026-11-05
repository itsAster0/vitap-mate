package vitapmate.features.timetable

import java.util.TimeZone

// VTOP's times are India time. "Now" is read in IST too, so the live class, countdowns and the
// now line stay right on a phone set to another zone; the agenda then says so (zoneNote).

private val IST: TimeZone = TimeZone.getTimeZone("Asia/Kolkata")
private const val DAY_MS = 86_400_000L

/**
 * Test hook for agents and humans: `adb shell setprop debug.vitapmate.clock_shift_min -540`, then
 * restart the app, moves the timetable's clock 9 h back (live class, countdowns, now line). Read
 * once; 0 when unset. Logged with the agenda days (`clock_shift_min`).
 */
val clockShiftMin: Long = runCatching {
    Class.forName("android.os.SystemProperties").getMethod("get", String::class.java, String::class.java)
        .invoke(null, "debug.vitapmate.clock_shift_min", "0") as String
}.getOrNull()?.trim()?.toLongOrNull() ?: 0L

private fun shifted(ms: Long) = ms + clockShiftMin * 60_000

/** The epoch day in India at wall clock [ms]. */
fun istDay(ms: Long): Int = shifted(ms).let { Math.floorDiv(it + IST.getOffset(it), DAY_MS).toInt() }

/** Minutes since midnight in India at [ms]. */
fun istMinute(ms: Long): Int = shifted(ms).let { (Math.floorMod(it + IST.getOffset(it), DAY_MS) / 60_000).toInt() }

/** Seconds since midnight in India at [ms]. */
fun istSecond(ms: Long): Int = shifted(ms).let { (Math.floorMod(it + IST.getOffset(it), DAY_MS) / 1000).toInt() }

/** Null on a phone on India time, else a note that times are VTOP's (IST), naming the phone's zone. */
fun zoneNote(ms: Long): String? {
    val local = TimeZone.getDefault()
    if (local.getOffset(ms) == IST.getOffset(ms)) return null
    return "Times are India time (IST), as VTOP gives them. Your phone is on ${local.getDisplayName(local.inDaylightTime(java.util.Date(ms)), TimeZone.SHORT)}."
}
