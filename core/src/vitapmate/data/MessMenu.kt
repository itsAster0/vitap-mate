package vitapmate.data

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.mutableStateOf
import vitapmate.core.MessMenuException
import vitapmate.core.MessTable
import vitapmate.core.detectMessMenuMonth
import vitapmate.core.resolveMessItem
import vitapmate.core.Meal as RustMeal
import vitapmate.core.MealWindow as RustMealWindow
import vitapmate.core.MessItem as RustMessItem
import vitapmate.core.MessSheet as RustMessSheet
import vitapmate.core.currentOrNextMeal as rustCurrentOrNextMeal
import vitapmate.core.decodeMealWindows as rustDecodeMealWindows
import vitapmate.core.defaultMealWindows as rustDefaultMealWindows
import vitapmate.core.encodeMealWindows as rustEncodeMealWindows
import vitapmate.core.messMenuYear as rustMessMenuYear
import vitapmate.core.parseMessMenu as rustParseMessMenu

// Thin Kotlin bridge over Rust's mess menu parser (`rust/vtop-ffi/src/mess_menu.rs`).
// Spreadsheet zip/XML decoding stays in Kotlin (`Spreadsheet.kt`); all parsing, date assignment,
// dish resolution and meal-window helpers run in Rust.

enum class Meal(val label: String) {
    Breakfast("Breakfast"),
    Lunch("Lunch"),
    Snacks("Snacks"),
    Dinner("Dinner"),
}

typealias MessItem = RustMessItem

typealias MealWindow = RustMealWindow

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

class MessMenuFormatException(message: String, cause: Throwable? = null) : Exception(message, cause)

/**
 * Parses decoded sheets (name → rows of cell text, in workbook order) via Rust; sheets that don't
 * look like a mess menu are skipped.
 */
fun parseMessMenu(tables: Map<String, List<List<String>>>, yearOf: (month: Int?) -> Int? = { null }): MessMenu {
    val list = tables.entries.map { (name, rows) -> MessTable(name, rows) }
    val year = yearOf(detectMessMenuMonth(list))
    val parsed = try {
        rustParseMessMenu(list, year)
    } catch (e: MessMenuException.Format) {
        throw MessMenuFormatException(e.reason, e)
    }
    return MessMenu(parsed.sheets.map(RustMessSheet::toKotlin))
}

/** VIT-AP mess serving hours; editable in Settings → Mess Timings. */
val defaultMealWindows: Map<Meal, MealWindow> by lazy { rustDefaultMealWindows().toWindowMap() }

/** "435-540,750-840,990-1095,1155-1260", breakfast to dinner (the Flutter setting's format). */
fun encodeMealWindows(windows: Map<Meal, MealWindow>): String = rustEncodeMealWindows(windows.toWindowList())

/** Falls back to [defaultMealWindows] for anything missing or malformed. */
fun decodeMealWindows(raw: String?): Map<Meal, MealWindow> = rustDecodeMealWindows(raw).toWindowMap()

/** The meal being served at [minute] of the day, else the next one; null after dinner. */
fun currentOrNextMeal(minute: Int, windows: Map<Meal, MealWindow> = defaultMealWindows): Meal? =
    rustCurrentOrNextMeal(minute, windows.toWindowList())?.toKotlin()

/**
 * The menu only names the month; the year comes from the document name ("… October 2026") or is
 * the one nearest to today ([nowYear], [nowMonth] 1–12).
 */
fun messMenuYear(month: Int?, name: String, nowYear: Int, nowMonth: Int): Int =
    rustMessMenuYear(month, name, nowYear, nowMonth)

/** Settings → Mess Timings (the Flutter key `settings_mess_timings`), as Compose state. */
object MessTimings {
    private const val KEY = "settings_mess_timings"
    private var prefs: SharedPreferences? = null
    private val state = mutableStateOf<Map<Meal, MealWindow>?>(null)

    var windows: Map<Meal, MealWindow>
        get() = state.value ?: defaultMealWindows
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

private fun RustMeal.toKotlin(): Meal = when (this) {
    RustMeal.BREAKFAST -> Meal.Breakfast
    RustMeal.LUNCH -> Meal.Lunch
    RustMeal.SNACKS -> Meal.Snacks
    RustMeal.DINNER -> Meal.Dinner
}

private fun List<MealWindow>.toWindowMap(): Map<Meal, MealWindow> =
    Meal.entries.associateWith { meal -> this[meal.ordinal] }

private fun Map<Meal, MealWindow>.toWindowList(): List<MealWindow> =
    Meal.entries.map { getValue(it) }

private fun RustMessSheet.toKotlin(): MessSheet = MessSheet(
    name = name,
    month = month,
    days = days.associate { entry ->
        entry.day to buildMap {
            if (entry.breakfast.isNotEmpty()) put(Meal.Breakfast, entry.breakfast)
            if (entry.lunch.isNotEmpty()) put(Meal.Lunch, entry.lunch)
            if (entry.snacks.isNotEmpty()) put(Meal.Snacks, entry.snacks)
            if (entry.dinner.isNotEmpty()) put(Meal.Dinner, entry.dinner)
        }
    },
    blocks = days.associate { it.day to it.block },
    instructions = instructions,
)
