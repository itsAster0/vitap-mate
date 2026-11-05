package vitapmate.data

import android.content.Context
import android.content.SharedPreferences
import java.util.Calendar
import java.util.TimeZone
import androidx.compose.runtime.mutableStateOf

// Port of the Flutter app's features/docs/logic/mess_menu.dart: the VIT-AP monthly mess menu
// spreadsheet, one sheet per menu ("Veg & Non-Veg", "Special"), a header row of meals, and day
// blocks whose first cell names the weekday and the dates it covers ("Thu\n1, 15, 29"). The file
// itself is never changed.

enum class Meal(val label: String) { Breakfast("Breakfast"), Lunch("Lunch"), Snacks("Snacks"), Dinner("Dinner") }

data class MessItem(val name: String, val nonVeg: Boolean = false)

class MessSheet(
    val name: String,
    /** 1–12 when the title names a month ("... FOR THE MONTH OF OCTOBER"). */
    val month: Int?,
    /** Raw cell text per day of month, per meal; date-specific variants are resolved by [itemsFor]. */
    val days: Map<Int, Map<Meal, List<String>>>,
    /** Every date sharing a day's block ("Thu 1, 15, 29"). */
    val blocks: Map<Int, List<Int>>,
    val instructions: List<String>,
) {
    fun itemsFor(day: Int, meal: Meal): List<MessItem> =
        days[day]?.get(meal).orEmpty().mapNotNull { resolveMessItem(it, day, blocks[day].orEmpty()) }
}

class MessMenu(val sheets: List<MessSheet>) {
    val month: Int? get() = sheets.firstNotNullOfOrNull { it.month }
}

class MessMenuFormatException(message: String) : Exception(message)

private val months = listOf("january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december")
private val dayCell = Regex("""^\s*(mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?[\s,:/-]*((?:\d{1,2}[\s,&and]*)+)$""", RegexOption.IGNORE_CASE)
private val number = Regex("""\d{1,2}""")

/**
 * Parses decoded sheets (name → rows of cell text); sheets that don't look like a mess menu are
 * skipped. With the menu's year from [yearOf] (given the month the title names), a listed date that
 * falls on another weekday than its block's day is dropped (the sheet says "Tue 10, 25" where 25 is
 * a Wednesday), and a date no block lists takes the block two weeks before or after it on the same
 * weekday (the menu repeats fortnightly), so each date has exactly one block.
 */
fun parseMessMenu(tables: Map<String, List<List<String>>>, yearOf: (month: Int?) -> Int? = { null }): MessMenu {
    val sheets = tables.mapNotNull { (name, rows) -> parseSheet(name, rows, yearOf) }
    if (sheets.isEmpty()) throw MessMenuFormatException("This spreadsheet doesn't look like a mess menu.")
    return MessMenu(sheets)
}

private val weekdays = listOf("mon", "tue", "wed", "thu", "fri", "sat", "sun")

/** One day block of the sheet: its ISO weekday, the dates it lists and each meal's cells. */
private class Block(val weekday: Int, val dates: MutableList<Int>, val meals: MutableMap<Meal, MutableList<String>> = mutableMapOf())

private fun parseSheet(name: String, rows: List<List<String>>, yearOf: (Int?) -> Int?): MessSheet? {
    var month: Int? = null
    var headerRow: Int? = null
    val mealColumns = mutableMapOf<Int, Meal>()
    var r = 0
    while (r < rows.size && headerRow == null) {
        for (cell in rows[r]) {
            val lower = cell.lowercase()
            if (month == null && "menu" in lower) months.forEachIndexed { m, label -> if (label in lower) month = m + 1 }
        }
        val found = mutableMapOf<Int, Meal>()
        rows[r].forEachIndexed { c, cell ->
            val lower = cell.trim().lowercase()
            Meal.entries.forEach { if (lower == it.label.lowercase()) found[c] = it }
        }
        if (found.size >= 3) {
            headerRow = r
            mealColumns += found
        }
        r++
    }
    val header = headerRow ?: return null
    val blocks = mutableListOf<Block>()
    val instructions = mutableListOf<String>()
    var inInstructions = false
    for (row in rows.drop(header + 1)) {
        if ("instruction" in row.joinToString(" ").lowercase()) {
            inInstructions = true
            continue
        }
        if (inInstructions) {
            row.flatMap { it.split('\n') }.map { it.trim().replaceFirst(Regex("""^\d+\.\s*"""), "") }.filter { it.isNotEmpty() }.forEach(instructions::add)
            continue
        }
        for (cell in row) {
            val match = dayCell.find(cell.trim()) ?: continue
            val dates = number.findAll(match.groupValues[2]).map { it.value.toInt() }.filter { it in 1..31 }.distinct().toMutableList()
            blocks += Block(weekdays.indexOf(match.groupValues[1].lowercase()) + 1, dates)
            break
        }
        val block = blocks.lastOrNull() ?: continue
        mealColumns.forEach { (column, meal) ->
            val text = row.getOrNull(column)?.trim().orEmpty()
            if (text.isNotEmpty()) block.meals.getOrPut(meal) { mutableListOf() } += text
        }
    }
    val year = month?.let { yearOf(it) }
    if (year != null) assignDates(blocks, year, month)
    val days = mutableMapOf<Int, Map<Meal, List<String>>>()
    val byDay = mutableMapOf<Int, List<Int>>()
    for (b in blocks) for (d in b.dates) if (d !in days) {
        days[d] = b.meals
        byDay[d] = b.dates
    }
    if (days.isEmpty()) return null
    return MessSheet(name.trim(), month, days, byDay, instructions)
}

/** Keeps each block's dates on its weekday, then gives unlisted dates the block two weeks away. */
private fun assignDates(blocks: List<Block>, year: Int, month: Int) {
    val cal = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { clear(); set(year, month - 1, 1) }
    val length = cal.getActualMaximum(Calendar.DAY_OF_MONTH)
    // ISO weekday (1 = Monday) of each date.
    val firstIso = (cal.get(Calendar.DAY_OF_WEEK) + 5) % 7 + 1
    fun weekdayOf(day: Int) = (firstIso - 1 + day - 1) % 7 + 1
    blocks.forEach { b -> b.dates.retainAll { it <= length && weekdayOf(it) == b.weekday } }
    for (day in 1..length) {
        if (blocks.any { day in it.dates }) continue
        blocks.firstOrNull { b -> b.weekday == weekdayOf(day) && (day - 14 in b.dates || day + 14 in b.dates) }?.let { b ->
            b.dates += day
            b.dates.sort()
        }
    }
}

private val datesInParens = Regex("""\((\d{1,2}(?:\s*,\s*\d{1,2})*)\)""")
private val nonVegTag = Regex("""\(\s*(?:non[\s-]*veg|nv)\s*\)""", RegexOption.IGNORE_CASE)
// The "(Veg)" twin of a non-veg dish; the pill's colour already says which is which.
private val vegTag = Regex("""\(\s*veg\s*\)""", RegexOption.IGNORE_CASE)
private val spaces = Regex("""\s+""")

/**
 * "Aloo Mutter Curry (1), Chickpea Masala (15, 29)" on the 15th → "Chickpea Masala"; null when
 * no variant falls on [day]. Numbers in brackets only count as dates when they all belong to the
 * day's [block], so "(2)" as a quantity is left alone.
 */
internal fun resolveMessItem(raw: String, day: Int, block: List<Int>): MessItem? {
    var text = raw.replace(spaces, " ").trim()
    val nonVeg = nonVegTag.containsMatchIn(text)
    text = text.replace(nonVegTag, "").replace(vegTag, "").trim()
    val matches = datesInParens.findAll(text).toList()
    val variants = matches.isNotEmpty() && matches.all { m -> Regex("""\d+""").findAll(m.groupValues[1]).all { it.value.toInt() in block } }
    if (variants) {
        val picked = mutableListOf<String>()
        var start = 0
        for (m in matches) {
            val dates = Regex("""\d+""").findAll(m.groupValues[1]).map { it.value.toInt() }
            val label = text.substring(start, m.range.first).replaceFirst(Regex("""^[\s,&/]*(and\s+)?""", RegexOption.IGNORE_CASE), "").trim()
            if (day in dates && label.isNotEmpty()) picked += label
            start = m.range.last + 1
        }
        if (picked.isEmpty()) return null
        val tail = text.substring(start).trim()
        text = listOfNotNull(picked.joinToString(", "), tail.ifEmpty { null }).joinToString(" ")
    }
    text = text.replace(spaces, " ").trim()
    return if (text.isEmpty()) null else MessItem(text, nonVeg)
}

/** A meal's serving hours, minutes since midnight. */
data class MealWindow(val start: Int, val end: Int)

/** VIT-AP mess serving hours; editable in Settings → Mess Timings. */
val defaultMealWindows: Map<Meal, MealWindow> = mapOf(
    Meal.Breakfast to MealWindow(7 * 60 + 15, 9 * 60),
    Meal.Lunch to MealWindow(12 * 60 + 30, 14 * 60),
    Meal.Snacks to MealWindow(16 * 60 + 30, 18 * 60 + 15),
    Meal.Dinner to MealWindow(19 * 60 + 15, 21 * 60),
)

/** "435-540,750-840,990-1095,1155-1260", breakfast to dinner (the Flutter setting's format). */
fun encodeMealWindows(windows: Map<Meal, MealWindow>): String = Meal.entries.joinToString(",") { "${windows.getValue(it).start}-${windows.getValue(it).end}" }

/** Falls back to [defaultMealWindows] for anything missing or malformed. */
fun decodeMealWindows(raw: String?): Map<Meal, MealWindow> {
    val parts = raw?.split(',').orEmpty()
    return Meal.entries.associateWith { meal ->
        val p = parts.getOrNull(meal.ordinal)?.split('-')?.map { it.toIntOrNull() }
        if (p != null && p.size == 2 && p[0] != null && p[1] != null) MealWindow(p[0]!!, p[1]!!) else defaultMealWindows.getValue(meal)
    }
}

/** The meal being served at [minute] of the day, else the next one; null after dinner. */
fun currentOrNextMeal(minute: Int, windows: Map<Meal, MealWindow> = defaultMealWindows): Meal? =
    Meal.entries.firstOrNull { minute < windows.getValue(it).end }

/**
 * The menu only names the month; the year comes from the document name ("… October 2026") or is
 * the one nearest to today ([nowYear], [nowMonth] 1–12).
 */
fun messMenuYear(month: Int?, name: String, nowYear: Int, nowMonth: Int): Int {
    Regex("""\b(20\d\d)\b""").find(name)?.let { return it.groupValues[1].toInt() }
    if (month == null) return nowYear
    if (month - nowMonth > 6) return nowYear - 1
    if (nowMonth - month > 6) return nowYear + 1
    return nowYear
}

/** Settings → Mess Timings (the Flutter key `settings_mess_timings`), as Compose state. */
object MessTimings {
    private const val KEY = "settings_mess_timings"
    private var prefs: SharedPreferences? = null
    private val state = mutableStateOf(defaultMealWindows)

    var windows: Map<Meal, MealWindow>
        get() = state.value
        set(value) {
            state.value = value
            prefs?.edit()?.putString(KEY, encodeMealWindows(value))?.apply()
        }

    fun reset() {
        state.value = defaultMealWindows
        prefs?.edit()?.remove(KEY)?.apply()
    }

    fun load(context: Context) {
        val p = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
        prefs = p
        state.value = decodeMealWindows(p.getString(KEY, null))
    }
}
