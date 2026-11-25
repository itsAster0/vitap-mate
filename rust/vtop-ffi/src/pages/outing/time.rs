//! Days and times in India as the outing screens show them and VTOP's forms post them:
//! epoch days and minutes, VTOP's date and slot strings, and their labels.

use vtop_core::dates;
use vtop_core::inputs::{OutingDate, OutingTime};
use vtop_core::{VtopError, VtopResult};

use super::OutingMoment;
use crate::pages::records;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAY_NAMES: [&str; 7] = [
    "Sundays",
    "Mondays",
    "Tuesdays",
    "Wednesdays",
    "Thursdays",
    "Fridays",
    "Saturdays",
];

/// VTOP's `YYYY-MM-DD` with an optional `hh:mm AM`, as minutes since 1970.
pub(super) fn moment(date: &str, time: &str) -> Option<i64> {
    let day = records::epoch_day(date)?;
    Some(i64::from(day) * 1440 + i64::from(records::minutes(time).unwrap_or(0)))
}

pub(super) fn at(moment: OutingMoment) -> i64 {
    i64::from(moment.day) * 1440 + i64::from(moment.minute)
}

pub(super) fn day_of(at: i64) -> i32 {
    at.div_euclid(1440) as i32
}

pub(super) fn minute_of(at: i64) -> i32 {
    at.rem_euclid(1440) as i32
}

/// The `h:mm AM` times in `text`, as minutes since midnight.
fn clock_times(text: &str) -> Vec<i32> {
    let bytes = text.as_bytes();
    let mut times = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
            continue;
        }
        let mut j = i;
        while bytes.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        let colon = j - i <= 2
            && bytes.get(j) == Some(&b':')
            && bytes.get(j + 1).is_some_and(u8::is_ascii_digit)
            && bytes.get(j + 2).is_some_and(u8::is_ascii_digit);
        if colon {
            let mut k = j + 3;
            while bytes.get(k) == Some(&b' ') {
                k += 1;
            }
            let suffix = text.get(k..k + 2).map(str::to_ascii_uppercase);
            if matches!(suffix.as_deref(), Some("AM" | "PM"))
                && let Some(minutes) = text.get(i..k + 2).and_then(records::minutes)
            {
                times.push(minutes);
                i = k + 2;
                continue;
            }
        }
        i = j;
    }
    times
}

/// The leaving and return of a weekend slot like `9:30 AM- 3:30PM`.
pub(super) fn time_slot(slot: &str) -> Option<(i32, i32)> {
    match clock_times(slot).as_slice() {
        [start, end] => Some((*start, *end)),
        _ => None,
    }
}

/// `9:30 AM – 3:30 PM` for VTOP's `9:30 AM- 3:30PM`.
pub(super) fn slot_label(slot: &str) -> String {
    time_slot(slot).map_or_else(
        || slot.trim().to_string(),
        |(start, end)| format!("{} – {}", clock(start), clock(end)),
    )
}

/// "Sundays and Mondays", or "Sun & Mon" when `short`.
pub(super) fn list_days(days: &[u8], and: &str, short: bool) -> String {
    let names: Vec<&str> = days
        .iter()
        .filter_map(|d| DAY_NAMES.get(usize::from(*d)))
        .map(|name| {
            if short {
                name.get(..3).unwrap_or(name)
            } else {
                name
            }
        })
        .collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} {and} {last}", rest.join(", ")),
    }
}

/// (year, month 1–12, day) of an epoch day.
fn ymd(day: i32) -> (i32, usize, i32) {
    let iso = records::civil_date(i64::from(day));
    let mut parts = iso.split('-').map(|p| p.parse::<i32>().unwrap_or(1));
    let (year, month, day) = (
        parts.next().unwrap_or(1970),
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(1),
    );
    (year, month.clamp(1, 12) as usize, day)
}

/// `day` as "EEE d MMM" ("Sat 10 Oct"), "d MMM" or "EEE d MMM yyyy".
pub(crate) fn format_day(day: i32, pattern: &str) -> String {
    let (year, month, date) = ymd(day);
    let weekday = DAY_NAMES
        .get(usize::from(dates::weekday(i64::from(day))))
        .map_or("", |name| name.get(..3).unwrap_or(name));
    let month = MONTHS
        .get(month.saturating_sub(1))
        .copied()
        .unwrap_or_default();
    match pattern {
        "d MMM" => format!("{date} {month}"),
        "EEE d MMM yyyy" => format!("{weekday} {date} {month} {year}"),
        _ => format!("{weekday} {date} {month}"),
    }
}

/// `Sat 10 Oct`, with the year when it is not this year.
pub(super) fn short_date(day: i32, today: i32) -> String {
    if ymd(day).0 == ymd(today).0 {
        format_day(day, "EEE d MMM")
    } else {
        format_day(day, "EEE d MMM yyyy")
    }
}

/// 360 → "6:00 AM".
pub(super) fn clock(minutes: i32) -> String {
    let (hour, minute) = (minutes / 60, minutes % 60);
    let half = if hour >= 12 { "PM" } else { "AM" };
    let hour = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{hour}:{minute:02} {half}")
}

/// 360 → "6 AM", 390 → "6:30 AM".
pub(super) fn short_clock(minutes: i32) -> String {
    let full = clock(minutes);
    if minutes % 60 == 0 {
        full.replace(":00", "")
    } else {
        full
    }
}

/// `11-Oct-2026`, what the outing date fields post.
pub(super) fn outing_date(day: i32) -> VtopResult<OutingDate> {
    let (year, month, date) = ymd(day);
    let month = MONTHS
        .get(month.saturating_sub(1))
        .copied()
        .unwrap_or_default();
    OutingDate::parse(&format!("{date:02}-{month}-{year:04}"))
}

pub(super) fn outing_time(minute: i32) -> VtopResult<OutingTime> {
    let invalid = || VtopError::ConfigurationError("outing time must be a real time of day".into());
    let hour = u8::try_from(minute / 60).map_err(|_| invalid())?;
    let minute = u8::try_from(minute % 60).map_err(|_| invalid())?;
    OutingTime::new(hour, minute)
}
