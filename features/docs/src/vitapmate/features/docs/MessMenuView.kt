package vitapmate.features.docs

import android.content.Context
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.CornerSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.ChevronLeft
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.ChevronUp
import com.composables.icons.lucide.ClipboardList
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.UtensilsCrossed
import java.util.Calendar
import java.util.TimeZone
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.Doc
import vitapmate.data.Docs
import vitapmate.data.Meal
import vitapmate.data.MealWindow
import vitapmate.data.MessItem
import vitapmate.data.MessMenu
import vitapmate.data.MessSheet
import vitapmate.data.MessTimings
import vitapmate.data.currentOrNextMeal
import vitapmate.data.epochDayOf
import vitapmate.data.messMenuYear
import vitapmate.data.minuteOf
import vitapmate.data.parseMessMenu
import vitapmate.data.readSpreadsheet
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.PressScale
import vitapmate.ui.Space
import vitapmate.ui.Surface
import vitapmate.ui.Tone
import vitapmate.ui.ToneBadge
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.isoWeekday
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/** Reads [doc] as the mess menu; throws [vitapmate.data.MessMenuFormatException] when it isn't one. */
internal suspend fun loadMessMenu(doc: Doc): MessMenu = withContext(Dispatchers.Default) {
    val now = Calendar.getInstance()
    parseMessMenu(withContext(Dispatchers.IO) { Docs.withFile(doc, ::readSpreadsheet) }) { month -> messMenuYear(month, doc.name, now.get(Calendar.YEAR), now.get(Calendar.MONTH) + 1) }
}

private const val SHEET_KEY = "mess_menu_sheet"

/**
 * A spreadsheet read as the monthly mess menu: opens on today, with the meal on now (or up next)
 * marked by Settings → Mess Timings. [compact] is the "Continue viewing" card: that meal only.
 */
@Composable
internal fun MessMenuView(doc: Doc, compact: Boolean, onFailed: (Throwable) -> Unit) {
    val menu by produceState<MessMenu?>(null, doc.uri) { value = runCatching { loadMessMenu(doc) }.onFailure(onFailed).getOrNull() }
    val data = menu ?: return Loading()
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("settings", Context.MODE_PRIVATE) }
    // The Regular/Special choice, remembered across opens (the Flutter key).
    var sheetIndex by rememberSaveable { mutableIntStateOf(data.sheets.indexOfFirst { it.name == prefs.getString(SHEET_KEY, null) }.coerceAtLeast(0)) }
    val now by rememberNow()
    val today = epochDayOf(now)
    val cal = remember(today) { calendarOf(today) }
    val year = messMenuYear(data.month, doc.name, cal.get(Calendar.YEAR), cal.get(Calendar.MONTH) + 1)
    val month = data.month ?: (cal.get(Calendar.MONTH) + 1)
    val sheet = data.sheets[sheetIndex.coerceIn(0, data.sheets.lastIndex)]
    val windows = MessTimings.windows
    val minute = minuteOf(now)
    if (compact) return UpNext(sheet, year, month, today, minute, windows)

    val first = epochDay(year, month, 1)
    val last = epochDay(year, month + 1, 1) - 1
    var selected by rememberSaveable { mutableIntStateOf(if (today in first..last) today else first) }
    var showRules by rememberSaveable { mutableStateOf(false) }
    LazyColumn(Modifier.testTag("docs.mess"), contentPadding = PaddingValues(Space.md, Space.sm, Space.md, 96.dp)) {
        if (data.sheets.size > 1) item {
            ConnectedChoice(
                data.sheets.map { it.name },
                sheet.name,
                onSelect = { name ->
                    sheetIndex = data.sheets.indexOfFirst { it.name == name }
                    prefs.edit().putString(SHEET_KEY, name).apply()
                },
                tag = "docs.mess.sheet",
                key = { it.lowercase().replace(Regex("[^a-z0-9]+"), "-") },
                label = { it },
                modifier = Modifier.padding(bottom = Space.md),
            )
        }
        item { WeekStrip(first, last, today, selected) { selected = it } }
        // The day's meals slide in from the side the day lies on, with a spring.
        item("day") {
            AnimatedContent(
                selected,
                transitionSpec = {
                    val dir = if (targetState > initialState) 1 else -1
                    (slideInHorizontally(spring(Spring.DampingRatioLowBouncy, Spring.StiffnessMediumLow)) { it / 3 * dir } + fadeIn()) togetherWith
                        (slideOutHorizontally(spring(stiffness = Spring.StiffnessMedium)) { -it / 3 * dir } + fadeOut())
                },
                label = "day",
            ) { day ->
                val dom = day - first + 1
                val onToday = day == today
                val focusMeal = if (onToday) currentOrNextMeal(minute, windows) else null
                Column {
                    Text(
                        if (onToday) "Today" else formatDay(day, "EEEE"),
                        Modifier.padding(start = 2.dp, top = Space.lg),
                        style = MaterialTheme.typography.headlineMediumEmphasized,
                    )
                    Text(
                        formatDay(day, if (onToday) "EEEE, d MMMM" else "d MMMM"),
                        Modifier.padding(start = 2.dp, bottom = Space.md),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (!sheet.days.containsKey(dom)) {
                        EmptyView(Lucide.UtensilsCrossed, "No menu for this day")
                    } else {
                        // The day's meals as one segmented stack: small inner corners, the ends rounded.
                        val end = MaterialTheme.shapes.extraLarge.topStart
                        val inner = CornerSize(8.dp)
                        Meal.entries.forEach { meal ->
                            val status = when {
                                focusMeal == null -> if (onToday) MealStatus.Past else MealStatus.Normal
                                meal == focusMeal -> if (minute >= windows.getValue(meal).start) MealStatus.Now else MealStatus.Next
                                meal.ordinal < focusMeal.ordinal -> MealStatus.Past
                                else -> MealStatus.Normal
                            }
                            val shape = RoundedCornerShape(
                                topStart = if (meal.ordinal == 0) end else inner,
                                topEnd = if (meal.ordinal == 0) end else inner,
                                bottomEnd = if (meal.ordinal == Meal.entries.lastIndex) end else inner,
                                bottomStart = if (meal.ordinal == Meal.entries.lastIndex) end else inner,
                            )
                            MealCard(meal, sheet.itemsFor(dom, meal), windows.getValue(meal), status, Modifier.padding(bottom = 3.dp), shape = shape)
                        }
                    }
                }
            }
        }
        if (sheet.instructions.isNotEmpty()) {
            item {
                Surface(
                    Modifier.fillMaxWidth().padding(top = Space.sm).clip(MaterialTheme.shapes.extraLarge).testTag("docs.mess.rules"),
                    onPress = { showRules = !showRules },
                    padding = PaddingValues(Space.lg),
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Lucide.ClipboardList, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(Space.md))
                        Text("Mess instructions (${sheet.instructions.size})", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                        Icon(if (showRules) Lucide.ChevronUp else Lucide.ChevronDown, null, Modifier.size(18.dp))
                    }
                }
            }
            item {
                AnimatedVisibility(showRules) {
                    Column(Modifier.padding(Space.sm)) {
                        sheet.instructions.forEachIndexed { i, rule ->
                            Text("${i + 1}. $rule", Modifier.padding(bottom = Space.sm), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
        }
    }
}

/** The meal on now or next (tomorrow's breakfast after dinner). */
@Composable
private fun UpNext(sheet: MessSheet, year: Int, month: Int, today: Int, minute: Int, windows: Map<Meal, MealWindow>) {
    val meal = currentOrNextMeal(minute, windows) ?: Meal.Breakfast
    val day = if (currentOrNextMeal(minute, windows) == null) today + 1 else today
    val cal = calendarOf(day)
    val inMenu = cal.get(Calendar.YEAR) == year && cal.get(Calendar.MONTH) + 1 == month && sheet.days.containsKey(cal.get(Calendar.DAY_OF_MONTH))
    if (!inMenu) {
        return Box(Modifier.fillMaxWidth().padding(Space.lg), Alignment.Center) {
            Text("No menu for ${formatDay(day, "d MMMM")} in this file", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
    val window = windows.getValue(meal)
    Column(Modifier.verticalScroll(rememberScrollState()).padding(Space.md)) {
        MealCard(
            meal,
            sheet.itemsFor(cal.get(Calendar.DAY_OF_MONTH), meal),
            window,
            if (day == today && minute >= window.start) MealStatus.Now else MealStatus.Next,
            framed = false,
            whenLabel = "Tomorrow".takeIf { day != today },
        )
    }
}

/** Mon–Sun of the selected week, with arrows to step through the month. */
@Composable
private fun WeekStrip(first: Int, last: Int, today: Int, selected: Int, onSelect: (Int) -> Unit) {
    val monday = selected - isoWeekday(selected) + 1
    val week = (0 until 7).map { monday + it }
    val scheme = MaterialTheme.colorScheme
    fun step(weeks: Int) = onSelect((selected + 7 * weeks).coerceIn(first, last))
    Row(Modifier.fillMaxWidth().testTag("docs.mess.week"), verticalAlignment = Alignment.CenterVertically) {
        IconButton({ step(-1) }, Modifier.width(32.dp), enabled = monday > first) { Icon(Lucide.ChevronLeft, "Previous week", Modifier.size(18.dp)) }
        week.forEach { date ->
            val outside = date < first || date > last
            val isSelected = date == selected
            val fill by animateColorAsState(if (isSelected) scheme.primary else Color.Transparent, spring(stiffness = Spring.StiffnessMediumLow))
            // The picked day rounds into a pill with a little bounce.
            val corner by animateDpAsState(if (isSelected) 27.dp else 12.dp, spring(Spring.DampingRatioMediumBouncy, Spring.StiffnessMedium))
            PressScale({ if (!outside) onSelect(date) }, Modifier.weight(1f).padding(horizontal = 2.dp), formatDay(date, "EEEE, d MMMM"), scale = 0.94f) {
                Column(
                    Modifier.fillMaxWidth().height(54.dp).background(fill, RoundedCornerShape(corner))
                        .then(if (date == today && !isSelected) Modifier.border(1.5.dp, scheme.primary.copy(alpha = 0.5f), RoundedCornerShape(corner)) else Modifier)
                        .alpha(if (outside) 0.25f else 1f)
                        .testTag("docs.mess.day.${date - first + 1}"),
                    verticalArrangement = Arrangement.Center,
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(formatDay(date, "EEE").uppercase(), style = MaterialTheme.typography.labelSmall, color = if (isSelected) scheme.onPrimary.copy(alpha = 0.7f) else scheme.onSurfaceVariant)
                    Text(
                        "${calendarOf(date).get(Calendar.DAY_OF_MONTH)}",
                        style = MaterialTheme.typography.titleMediumEmphasized.nums(),
                        color = when {
                            isSelected -> scheme.onPrimary
                            date == today -> scheme.primary
                            else -> scheme.onSurface
                        },
                    )
                }
            }
        }
        IconButton({ step(1) }, Modifier.width(32.dp), enabled = week.last() < last) { Icon(Lucide.ChevronRight, "Next week", Modifier.size(18.dp)) }
    }
}

private enum class MealStatus { Normal, Past, Now, Next }

/**
 * A meal: name, serving hours and its dishes; the one on now (or next) is the hero, on the accent
 * tone with a larger title. [shape] is its place in the day's stack.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun MealCard(
    meal: Meal,
    items: List<MessItem>,
    window: MealWindow,
    status: MealStatus,
    modifier: Modifier = Modifier,
    framed: Boolean = true,
    whenLabel: String? = null,
    shape: Shape = RoundedCornerShape(8.dp),
) {
    val scheme = MaterialTheme.colorScheme
    val accent = AppTheme.tones.accentTone
    val live = status == MealStatus.Now || status == MealStatus.Next
    val hero = live && framed
    val h24 = is24Hour()
    val container by animateColorAsState(if (hero) accent.subtle else AppTheme.colors.card, spring(stiffness = Spring.StiffnessMediumLow))
    val content = scheme.onSurface
    val secondary = if (hero) accent.onSubtle else content.copy(alpha = 0.72f)
    val body = @Composable {
        Column {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(
                        listOfNotNull(whenLabel, meal.label).joinToString(" · "),
                        style = if (hero) MaterialTheme.typography.headlineSmallEmphasized else MaterialTheme.typography.titleMediumEmphasized,
                        color = content,
                    )
                    Text(
                        "${clock(window.start, h24)} – ${clock(window.end, h24)}",
                        style = MaterialTheme.typography.labelMedium.nums(),
                        color = secondary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                if (status == MealStatus.Now) ToneBadge("Now", tone = accent, solid = true)
                // On the hero the subtle fill would vanish, so Next sits on the inset instead.
                if (status == MealStatus.Next) ToneBadge("Next", tone = if (hero) Tone(accent.base, scheme.surfaceContainerLowest.copy(alpha = 0.7f), accent.onSubtle) else accent)
            }
            Spacer(Modifier.height(Space.md))
            if (items.isEmpty()) {
                Text("Nothing listed", style = MaterialTheme.typography.bodyMedium, color = secondary)
            } else {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) { items.forEach { ItemPill(it, hero) } }
            }
        }
    }
    if (!framed) return body()
    val fade by animateFloatAsState(if (status == MealStatus.Past) 0.55f else 1f, spring(stiffness = Spring.StiffnessMediumLow))
    Box(
        modifier.fillMaxWidth().alpha(fade).background(container, shape).padding(horizontal = 18.dp, vertical = Space.lg)
            .testTag("docs.mess.meal.${meal.name.lowercase()}"),
    ) { body() }
}

/** A dish as a chip; non-veg ones carry a small danger-tone dot. */
@Composable
private fun ItemPill(item: MessItem, onLive: Boolean) {
    val scheme = MaterialTheme.colorScheme
    Row(
        Modifier.background(if (onLive) scheme.surfaceContainerLowest.copy(alpha = 0.7f) else scheme.surfaceContainerHigh, RoundedCornerShape(10.dp))
            .padding(horizontal = 10.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (item.nonVeg) {
            Box(Modifier.size(7.dp).background(AppTheme.tones.danger.base, CircleShape))
            Spacer(Modifier.width(6.dp))
        }
        Text(item.name, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurface)
    }
}

private fun calendarOf(day: Int): Calendar = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { timeInMillis = day * 86_400_000L }

/** The epoch day of [year]-[month]-[day]; month 13 is next January. */
private fun epochDay(year: Int, month: Int, day: Int): Int =
    (Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { clear(); set(year, month - 1, day) }.timeInMillis / 86_400_000L).toInt()
