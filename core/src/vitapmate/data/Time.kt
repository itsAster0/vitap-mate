package vitapmate.data

import java.util.TimeZone

/** One day in milliseconds; local epoch days are wall-clock ms over this (no java.time: minSdk 24). */
const val DAY_MS = 86_400_000L

/** Today's local epoch day for the wall clock [ms]. */
fun epochDayOf(ms: Long): Int = Math.floorDiv(ms + TimeZone.getDefault().getOffset(ms), DAY_MS).toInt()

/** Minutes since local midnight for the wall clock [ms]. */
fun minuteOf(ms: Long): Int = (Math.floorMod(ms + TimeZone.getDefault().getOffset(ms), DAY_MS) / 60_000).toInt()
