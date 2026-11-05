package vitapmate.ui

import android.text.format.DateFormat
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.produceState
import androidx.compose.ui.platform.LocalContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import kotlinx.coroutines.delay

// Port of logic/time_format.dart plus the agenda's date helpers. Dates are local epoch days (no
// java.time: minSdk 24), times are minutes since midnight, as the Rust records give them.

private const val DAY_MS = 86_400_000L

/** Today's local epoch day for the wall clock [ms]. */
fun epochDayOf(ms: Long): Int = Math.floorDiv(ms + TimeZone.getDefault().getOffset(ms), DAY_MS).toInt()

/** Minutes since local midnight for the wall clock [ms]. */
fun minuteOf(ms: Long): Int = (Math.floorMod(ms + TimeZone.getDefault().getOffset(ms), DAY_MS) / 60_000).toInt()

/** ISO weekday (1 = Monday) of an epoch day; 1970-01-01 was a Thursday. */
fun isoWeekday(day: Int): Int = Math.floorMod(day + 3, 7) + 1

/** The Monday of [day]'s week. */
fun mondayOf(day: Int): Int = day - isoWeekday(day) + 1

/** [day] in a `SimpleDateFormat` [pattern] ("EEEE", "d MMMM", "EEE d MMM"), US English like intl. */
fun formatDay(day: Int, pattern: String): String =
    SimpleDateFormat(pattern, Locale.US).apply { timeZone = TimeZone.getTimeZone("UTC") }.format(Date(day * DAY_MS))

/** Whether the phone shows 24-hour time (Flutter's `MediaQuery.alwaysUse24HourFormat`). */
@Composable
fun is24Hour(): Boolean = DateFormat.is24HourFormat(LocalContext.current)

/** 845 → "2:05 PM", or "14:05" on a 24-hour phone (`to12H`). */
fun clock(minutes: Int, h24: Boolean): String {
    val h = minutes / 60
    val m = minutes % 60
    if (h24) return "%02d:%02d".format(h, m)
    return "${if (h % 12 == 0) 12 else h % 12}:%02d ${if (h >= 12) "PM" else "AM"}".format(m)
}

/** "2 hr 30 min", "45 min", "1 hr". */
fun formatMinutes(minutes: Int): String {
    val h = minutes / 60
    val m = minutes % 60
    return listOfNotNull(if (h > 0) "$h hr" else null, if (m > 0 || h == 0) "$m min" else null).joinToString(" ")
}

/** 129 → "2h 9m", 45 → "45m"; short enough to share a row with a tag. */
fun compactMinutes(minutes: Int): String {
    val h = minutes / 60
    val m = minutes % 60
    return if (h == 0) "${m}m" else if (m == 0) "${h}h" else "${h}h ${m}m"
}

/** "Tomorrow", a weekday name within the coming week, else "Tue 6 Oct". */
fun dayLabel(day: Int, today: Int): String = when (val days = day - today) {
    1 -> "Tomorrow"
    in 2..6 -> formatDay(day, "EEEE")
    else -> formatDay(day, "EEE d MMM")
}

/** Wall clock in ms, ticking on each [periodMs] boundary (minute: status; second: countdowns). */
@Composable
fun rememberNow(periodMs: Long = 60_000): State<Long> = produceState(System.currentTimeMillis()) {
    while (true) {
        delay(periodMs - System.currentTimeMillis() % periodMs)
        value = System.currentTimeMillis()
    }
}
