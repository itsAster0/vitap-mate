//! VIT-AP monthly mess menu spreadsheet parser, dish variant resolution, and meal serving windows.
//!
//! Port of the Flutter app's `mess_menu.dart`: one sheet per menu ("Veg & Non-Veg", "Special"),
//! a header row of meals, and day blocks whose first cell names the weekday and dates it covers
//! ("Thu\n1, 15, 29"). Spreadsheet zip/XML decoding stays in Kotlin (`Spreadsheet.kt`).

use std::collections::BTreeMap;
use std::fmt;

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, uniffi::Enum)]
pub enum Meal {
    Breakfast,
    Lunch,
    Snacks,
    Dinner,
}

impl Meal {
    const ALL: [Self; 4] = [Self::Breakfast, Self::Lunch, Self::Snacks, Self::Dinner];

    fn from_header(lower: &str) -> Option<Self> {
        match lower {
            "breakfast" => Some(Self::Breakfast),
            "lunch" => Some(Self::Lunch),
            "snacks" => Some(Self::Snacks),
            "dinner" => Some(Self::Dinner),
            _ => None,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Breakfast => 0,
            Self::Lunch => 1,
            Self::Snacks => 2,
            Self::Dinner => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessItem {
    pub name: String,
    pub non_veg: bool,
}

/// A meal's serving hours, minutes since midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct MealWindow {
    pub start: i32,
    pub end: i32,
}

/// One decoded spreadsheet sheet, in workbook order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessTable {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

/// One day of a parsed mess menu sheet, with its raw meal cells and shared block dates.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessDay {
    pub day: i32,
    pub block: Vec<i32>,
    pub breakfast: Vec<String>,
    pub lunch: Vec<String>,
    pub snacks: Vec<String>,
    pub dinner: Vec<String>,
}

impl MessDay {
    fn raw_meal(&self, meal: Meal) -> &[String] {
        match meal {
            Meal::Breakfast => &self.breakfast,
            Meal::Lunch => &self.lunch,
            Meal::Snacks => &self.snacks,
            Meal::Dinner => &self.dinner,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessSheet {
    pub name: String,
    /// 1–12 when the title names a month ("... FOR THE MONTH OF OCTOBER").
    pub month: Option<i32>,
    /// Days present in this sheet, sorted by day of month.
    pub days: Vec<MessDay>,
    pub instructions: Vec<String>,
}

impl MessSheet {
    pub fn items_for(&self, day: i32, meal: Meal) -> Vec<MessItem> {
        let Some(entry) = self.days.iter().find(|d| d.day == day) else {
            return Vec::new();
        };
        entry
            .raw_meal(meal)
            .iter()
            .filter_map(|raw| resolve_item(raw, day, &entry.block))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MessMenu {
    pub sheets: Vec<MessSheet>,
    pub month: Option<i32>,
}

/// Why a spreadsheet failed to parse as a mess menu. Field is `reason` (not `message`) because
/// Kotlin `Exception` already declares `message`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum MessMenuError {
    Format { reason: String },
}

impl fmt::Display for MessMenuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format { reason } => f.write_str(reason),
        }
    }
}

impl std::error::Error for MessMenuError {}

/// Returns the first month (1–12) named by any mess-menu sheet in `tables`, so callers can work
/// out the menu's year before calling [`parse_mess_menu`].
#[uniffi::export]
pub fn detect_mess_menu_month(tables: Vec<MessTable>) -> Option<i32> {
    tables
        .iter()
        .find_map(|table| detect_sheet_month(&table.rows))
}

/// Parses decoded sheets (in workbook order); sheets that don't look like a mess menu are skipped.
/// With `year`, dates that fall on another weekday than their block's day are dropped, and unlisted
/// dates take the block two weeks before or after on the same weekday.
#[uniffi::export]
pub fn parse_mess_menu(
    tables: Vec<MessTable>,
    year: Option<i32>,
) -> Result<MessMenu, MessMenuError> {
    let sheets: Vec<MessSheet> = tables
        .into_iter()
        .filter_map(|table| parse_sheet(&table.name, &table.rows, year))
        .collect();
    if sheets.is_empty() {
        return Err(MessMenuError::Format {
            reason: "This spreadsheet doesn't look like a mess menu.".into(),
        });
    }
    let month = sheets.iter().find_map(|sheet| sheet.month);
    Ok(MessMenu { sheets, month })
}

/// "Aloo Mutter Curry (1), Chickpea Masala (15, 29)" on the 15th → "Chickpea Masala"; None when
/// no variant falls on `day`. Numbers in brackets only count as dates when all belong to `block`.
#[uniffi::export]
pub fn resolve_mess_item(raw: String, day: i32, block: Vec<i32>) -> Option<MessItem> {
    resolve_item(&raw, day, &block)
}

/// VIT-AP mess serving hours, Breakfast to Dinner; editable in Settings → Mess Timings.
#[uniffi::export]
pub fn default_meal_windows() -> Vec<MealWindow> {
    DEFAULT_WINDOWS.to_vec()
}

const DEFAULT_WINDOWS: [MealWindow; 4] = [
    MealWindow {
        start: 7 * 60 + 15,
        end: 9 * 60,
    },
    MealWindow {
        start: 12 * 60 + 30,
        end: 14 * 60,
    },
    MealWindow {
        start: 16 * 60 + 30,
        end: 18 * 60 + 15,
    },
    MealWindow {
        start: 19 * 60 + 15,
        end: 21 * 60,
    },
];

/// `"435-540,750-840,990-1095,1155-1260"`, Breakfast to Dinner (the Flutter setting's format).
#[uniffi::export]
pub fn encode_meal_windows(windows: Vec<MealWindow>) -> String {
    Meal::ALL
        .iter()
        .map(|meal| {
            let w = window_at(&windows, *meal);
            format!("{}-{}", w.start, w.end)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Falls back to [`default_meal_windows`] for anything missing or malformed.
#[uniffi::export]
pub fn decode_meal_windows(raw: Option<String>) -> Vec<MealWindow> {
    let parts: Vec<&str> = raw
        .as_deref()
        .map_or_else(Vec::new, |s| s.split(',').collect());
    Meal::ALL
        .iter()
        .map(|meal| {
            let default = DEFAULT_WINDOWS
                .get(meal.index())
                .copied()
                .unwrap_or(MealWindow { start: 0, end: 0 });
            let Some(part) = parts.get(meal.index()) else {
                return default;
            };
            let mut pieces = part.split('-');
            let (Some(from_s), Some(to_s), None) = (pieces.next(), pieces.next(), pieces.next())
            else {
                return default;
            };
            match (from_s.parse::<i32>(), to_s.parse::<i32>()) {
                (Ok(start), Ok(end)) => MealWindow { start, end },
                _ => default,
            }
        })
        .collect()
}

/// The meal being served at `minute` of the day, else the next one; None after dinner.
#[uniffi::export]
pub fn current_or_next_meal(minute: i32, windows: Vec<MealWindow>) -> Option<Meal> {
    Meal::ALL
        .into_iter()
        .find(|meal| minute < window_at(&windows, *meal).end)
}

/// The menu only names the month; the year comes from the document name ("… October 2026") or is
/// the one nearest to today (`now_year`, `now_month` 1–12).
#[uniffi::export]
pub fn mess_menu_year(month: Option<i32>, name: String, now_year: i32, now_month: i32) -> i32 {
    if let Some(year) = find_four_digit_20xx(&name) {
        return year;
    }
    let Some(month) = month else {
        return now_year;
    };
    if month - now_month > 6 {
        return now_year - 1;
    }
    if now_month - month > 6 {
        return now_year + 1;
    }
    now_year
}

fn window_at(windows: &[MealWindow], meal: Meal) -> MealWindow {
    windows
        .get(meal.index())
        .or_else(|| DEFAULT_WINDOWS.get(meal.index()))
        .copied()
        .unwrap_or(MealWindow { start: 0, end: 0 })
}

struct Block {
    weekday: i32,
    dates: Vec<i32>,
    meals: [Vec<String>; 4],
}

fn detect_sheet_month(rows: &[Vec<String>]) -> Option<i32> {
    let mut month = None;
    for row in rows {
        for cell in row {
            let lower = cell.to_lowercase();
            if month.is_none() && lower.contains("menu") {
                for (m, label) in MONTHS.iter().enumerate() {
                    if lower.contains(label) {
                        month = Some((m + 1) as i32);
                    }
                }
            }
        }
        let found = find_meal_columns(row);
        if found.len() >= 3 {
            return month;
        }
    }
    None
}

fn find_meal_columns(row: &[String]) -> Vec<(usize, Meal)> {
    let mut found = Vec::new();
    for (col, cell) in row.iter().enumerate() {
        let lower = cell.trim().to_lowercase();
        if let Some(meal) = Meal::from_header(&lower) {
            found.push((col, meal));
        }
    }
    found
}

fn parse_sheet(name: &str, rows: &[Vec<String>], year: Option<i32>) -> Option<MessSheet> {
    let mut month: Option<i32> = None;
    let mut header_row: Option<usize> = None;
    let mut meal_columns: Vec<(usize, Meal)> = Vec::new();

    for (r, row) in rows.iter().enumerate() {
        for cell in row {
            let lower = cell.to_lowercase();
            if month.is_none() && lower.contains("menu") {
                for (m, label) in MONTHS.iter().enumerate() {
                    if lower.contains(label) {
                        month = Some((m + 1) as i32);
                    }
                }
            }
        }
        let found = find_meal_columns(row);
        if found.len() >= 3 {
            header_row = Some(r);
            meal_columns = found;
            break;
        }
    }

    let header = header_row?;
    let mut blocks: Vec<Block> = Vec::new();
    let mut instructions: Vec<String> = Vec::new();
    let mut in_instructions = false;

    for row in rows.iter().skip(header + 1) {
        if row.join(" ").to_lowercase().contains("instruction") {
            in_instructions = true;
            continue;
        }
        if in_instructions {
            for cell in row {
                for line in cell.split('\n') {
                    let cleaned = strip_instruction_number(line.trim());
                    if !cleaned.is_empty() {
                        instructions.push(cleaned.to_owned());
                    }
                }
            }
            continue;
        }
        for cell in row {
            if let Some((weekday, dates)) = parse_day_cell(cell.trim()) {
                blocks.push(Block {
                    weekday,
                    dates,
                    meals: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                });
                break;
            }
        }
        let Some(block) = blocks.last_mut() else {
            continue;
        };
        for &(column, meal) in &meal_columns {
            let text = row.get(column).map_or("", |s| s.trim());
            if !text.is_empty()
                && let Some(slot) = block.meals.get_mut(meal.index())
            {
                slot.push(text.to_owned());
            }
        }
    }

    if let (Some(year), Some(m)) = (year, month) {
        assign_dates(&mut blocks, year, m);
    }

    let mut days_map: BTreeMap<i32, MessDay> = BTreeMap::new();
    for block in &blocks {
        for &d in &block.dates {
            days_map.entry(d).or_insert_with(|| MessDay {
                day: d,
                block: block.dates.clone(),
                breakfast: block.meals.first().cloned().unwrap_or_default(),
                lunch: block.meals.get(1).cloned().unwrap_or_default(),
                snacks: block.meals.get(2).cloned().unwrap_or_default(),
                dinner: block.meals.get(3).cloned().unwrap_or_default(),
            });
        }
    }

    if days_map.is_empty() {
        return None;
    }

    Some(MessSheet {
        name: name.trim().to_owned(),
        month,
        days: days_map.into_values().collect(),
        instructions,
    })
}

/// Keeps each block's dates on its weekday, then gives unlisted dates the block two weeks away.
fn assign_dates(blocks: &mut [Block], year: i32, month: i32) {
    let Some(length) = days_in_month(year, month) else {
        return;
    };
    let first_iso = first_iso_weekday(year, month);
    let weekday_of = |day: i32| (first_iso - 1 + day - 1).rem_euclid(7) + 1;

    for block in blocks.iter_mut() {
        block
            .dates
            .retain(|&d| d <= length && weekday_of(d) == block.weekday);
    }
    for day in 1..=length {
        if blocks.iter().any(|b| b.dates.contains(&day)) {
            continue;
        }
        let target_weekday = weekday_of(day);
        if let Some(block) = blocks.iter_mut().find(|b| {
            b.weekday == target_weekday
                && (b.dates.contains(&(day - 14)) || b.dates.contains(&(day + 14)))
        }) {
            block.dates.push(day);
            block.dates.sort_unstable();
        }
    }
}

fn days_in_month(year: i32, month: i32) -> Option<i32> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if leap { 29 } else { 28 }),
        _ => None,
    }
}

/// ISO weekday (1 = Monday .. 7 = Sunday) of `year`-`month`-01 in the proleptic Gregorian calendar.
fn first_iso_weekday(year: i32, month: i32) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_epoch = era * 146_097 + doe - 719_468;
    // 1970-01-01 was a Thursday (ISO weekday 4).
    (days_since_epoch + 3).rem_euclid(7) + 1
}

/// Matches `^\s*(mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?[\s,:/-]*((?:\d{1,2}[\s,&and]*)+)$` (ci).
fn parse_day_cell(trimmed: &str) -> Option<(i32, Vec<i32>)> {
    let lower = trimmed.to_lowercase();
    let (weekday, mut rest) = WEEKDAYS.iter().enumerate().find_map(|(i, prefix)| {
        lower
            .strip_prefix(prefix)
            .map(|after| ((i + 1) as i32, after))
    })?;
    rest = rest.trim_start_matches(|c: char| c.is_ascii_lowercase());
    if let Some(after_dot) = rest.strip_prefix('.') {
        rest = after_dot;
    }
    rest =
        rest.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ':' | '/' | '-'));
    if rest.is_empty() {
        return None;
    }

    let mut dates = Vec::new();
    let mut matched_any = false;
    while !rest.is_empty() {
        let digit_len = rest.bytes().take(2).take_while(u8::is_ascii_digit).count();
        if digit_len == 0 {
            return None;
        }
        matched_any = true;
        let num_str = rest.get(..digit_len)?;
        let n = num_str.parse::<i32>().ok()?;
        if (1..=31).contains(&n) && !dates.contains(&n) {
            dates.push(n);
        }
        rest = rest.get(digit_len..)?;
        rest = rest.trim_start_matches(|c: char| {
            c.is_whitespace() || matches!(c, ',' | '&' | 'a' | 'n' | 'd')
        });
    }
    matched_any.then_some((weekday, dates))
}

fn strip_instruction_number(s: &str) -> &str {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return s;
    }
    let Some(after_digits) = s.get(digits..) else {
        return s;
    };
    let Some(after_dot) = after_digits.strip_prefix('.') else {
        return s;
    };
    after_dot.trim_start()
}

fn resolve_item(raw: &str, day: i32, block: &[i32]) -> Option<MessItem> {
    let collapsed = collapse_spaces(raw);
    let (mut text, non_veg) = strip_diet_tags(&collapsed);
    text = text.trim().to_owned();

    let matches = find_dates_in_parens(&text);
    let variants = !matches.is_empty()
        && matches
            .iter()
            .all(|m| m.dates.iter().all(|d| block.contains(d)));

    if variants {
        let mut picked = Vec::new();
        let mut start = 0usize;
        for m in &matches {
            let segment = text.get(start..m.start).unwrap_or("");
            let label = strip_variant_prefix(segment);
            if m.dates.contains(&day) && !label.is_empty() {
                picked.push(label.to_owned());
            }
            start = m.end;
        }
        if picked.is_empty() {
            return None;
        }
        let tail = text.get(start..).unwrap_or("").trim();
        let joined = picked.join(", ");
        text = if tail.is_empty() {
            joined
        } else {
            format!("{joined} {tail}")
        };
    }

    let final_text = collapse_spaces(&text);
    if final_text.is_empty() {
        None
    } else {
        Some(MessItem {
            name: final_text,
            non_veg,
        })
    }
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// Strips `\(\s*(?:non[\s-]*veg|nv)\s*\)` and `\(\s*veg\s*\)` (case-insensitive) and reports
/// whether any non-veg tag was present.
fn strip_diet_tags(s: &str) -> (String, bool) {
    let mut out = String::with_capacity(s.len());
    let mut non_veg = false;
    let mut cursor = 0usize;

    while let Some(rest) = s.get(cursor..) {
        let Some(rel_open) = rest.find('(') else {
            out.push_str(rest);
            break;
        };
        let open = cursor + rel_open;
        let Some(after_open) = s.get(open + 1..) else {
            out.push_str(rest);
            break;
        };
        let Some(rel_close) = after_open.find(')') else {
            out.push_str(rest);
            break;
        };
        let inner = after_open.get(..rel_close).unwrap_or("");
        if inner.contains('(') {
            // Only the innermost `(...)` can be a tag; advance past this `(`.
            if let Some(prefix) = s.get(cursor..=open) {
                out.push_str(prefix);
            }
            cursor = open + 1;
            continue;
        }
        let is_nv = is_non_veg_tag(inner);
        let is_v = !is_nv && inner.trim().eq_ignore_ascii_case("veg");
        if is_nv || is_v {
            if is_nv {
                non_veg = true;
            }
            if let Some(before) = s.get(cursor..open) {
                out.push_str(before);
            }
            cursor = open + 1 + rel_close + 1;
        } else {
            let end = open + 1 + rel_close + 1;
            if let Some(span) = s.get(cursor..end) {
                out.push_str(span);
            }
            cursor = end;
        }
    }

    (out, non_veg)
}

fn is_non_veg_tag(inner: &str) -> bool {
    let trimmed = inner.trim().to_ascii_lowercase();
    if trimmed == "nv" {
        return true;
    }
    let Some(mid) = trimmed
        .strip_prefix("non")
        .and_then(|s| s.strip_suffix("veg"))
    else {
        return false;
    };
    mid.chars().all(|c| c.is_whitespace() || c == '-')
}

struct ParenDates {
    start: usize,
    end: usize,
    dates: Vec<i32>,
}

/// Finds non-overlapping matches of `\((\d{1,2}(?:\s*,\s*\d{1,2})*)\)`.
fn find_dates_in_parens(s: &str) -> Vec<ParenDates> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    while let Some(rest) = s.get(cursor..) {
        let Some(rel_open) = rest.find('(') else {
            break;
        };
        let open = cursor + rel_open;
        let Some(after_open) = s.get(open + 1..) else {
            break;
        };
        let Some(rel_close) = after_open.find(')') else {
            break;
        };
        let inner = after_open.get(..rel_close).unwrap_or("");
        if inner.contains('(') {
            cursor = open + 1;
            continue;
        }
        if let Some(dates) = parse_paren_date_list(inner) {
            let end = open + 1 + rel_close + 1;
            out.push(ParenDates {
                start: open,
                end,
                dates,
            });
            cursor = end;
        } else {
            cursor = open + 1;
        }
    }
    out
}

fn parse_paren_date_list(inner: &str) -> Option<Vec<i32>> {
    let mut rest = inner;
    let mut dates = Vec::new();
    loop {
        let digits = rest.bytes().take(2).take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let n = rest.get(..digits)?.parse::<i32>().ok()?;
        dates.push(n);
        rest = rest.get(digits..)?;
        if rest.is_empty() {
            return Some(dates);
        }
        rest = rest.trim_start();
        rest = rest.strip_prefix(',')?;
        rest = rest.trim_start();
    }
}

/// Strips `^[\s,&/]*(and\s+)?` (case-insensitive) and trims.
fn strip_variant_prefix(s: &str) -> &str {
    let rest = s.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ',' | '&' | '/'));
    let rest = if let (Some(prefix), Some(after)) = (rest.get(..3), rest.get(3..))
        && prefix.eq_ignore_ascii_case("and")
        && after.chars().next().is_some_and(char::is_whitespace)
    {
        after.trim_start()
    } else {
        rest
    };
    rest.trim()
}

/// Finds the first `\b(20\d\d)\b` in `s`.
fn find_four_digit_20xx(s: &str) -> Option<i32> {
    let mut start: Option<usize> = None;
    for (idx, ch) in s.char_indices() {
        let is_word = ch.is_alphanumeric() || ch == '_';
        match (start, is_word) {
            (None, true) => start = Some(idx),
            (Some(b), false) => {
                if let Some(year) = parse_20xx(s.get(b..idx).unwrap_or("")) {
                    return Some(year);
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(b) = start
        && let Some(year) = parse_20xx(s.get(b..).unwrap_or(""))
    {
        return Some(year);
    }
    None
}

fn parse_20xx(word: &str) -> Option<i32> {
    if word.len() == 4 && word.starts_with("20") && word.bytes().all(|b| b.is_ascii_digit()) {
        word.parse::<i32>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
