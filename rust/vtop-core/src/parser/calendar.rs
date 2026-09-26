//! Academic calendar: the semester's month list and each month's grid.

use std::sync::LazyLock;

use scraper::{Html, Selector};

use super::{selector, word_text, TD};
use crate::types::CalendarEntry;

static MONTH_BUTTON: LazyLock<Selector> = LazyLock::new(|| selector("a[onclick]"));
static SPAN: LazyLock<Selector> = LazyLock::new(|| selector("span"));

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// The `calDate` of every month button on the semester's calendar page
/// (`getDateForSemesterPreview`), e.g. `01-JUL-2026`, in page order.
pub fn parse_calendar_months(html: &str) -> Vec<String> {
    let document = Html::parse_fragment(html);
    document
        .select(&MONTH_BUTTON)
        .filter_map(|link| {
            let onclick = link.value().attr("onclick")?;
            let (_, rest) = onclick.split_once("processViewCalendar(")?;
            let date = rest.split('\'').nth(1)?;
            month_of(date).map(|_| date.to_string())
        })
        .collect()
}

/// The entries of one month page (`processViewCalendar`). [`cal_date`] is
/// the `01-OCT-2026` the page was requested with; the cells only hold the
/// day of the month.
pub fn parse_calendar_month(html: &str, cal_date: &str) -> Vec<CalendarEntry> {
    let Some((year, month)) = month_of(cal_date) else {
        return Vec::new();
    };
    let document = Html::parse_fragment(html);
    let mut entries = Vec::new();

    for cell in document.select(&TD) {
        let spans: Vec<String> = cell.select(&SPAN).map(|span| word_text(&span)).collect();
        let Some(day) = spans.first().and_then(|day| day.parse::<u32>().ok()) else {
            continue;
        };
        if !(1..=31).contains(&day) {
            continue;
        }
        let date = format!("{year:04}-{month:02}-{day:02}");
        // After the day: "Kind - Group" then "(Note)", once per event.
        for pair in spans[1..].chunks(2) {
            let event = pair[0].as_str();
            if event.is_empty() {
                continue;
            }
            let (kind, group) = event.rsplit_once(" - ").unwrap_or((event, ""));
            let note = pair.get(1).map(String::as_str).unwrap_or_default();
            let note = note
                .strip_prefix('(')
                .and_then(|note| note.strip_suffix(')'))
                .unwrap_or(note);
            entries.push(CalendarEntry {
                date: date.clone(),
                kind: kind.trim().to_string(),
                group: group.trim().to_string(),
                note: note.trim().to_string(),
            });
        }
    }
    entries
}

/// (year, month 1-12) of a `01-OCT-2026` calendar date.
fn month_of(cal_date: &str) -> Option<(u32, u32)> {
    let mut parts = cal_date.split('-');
    let _day = parts.next()?;
    let month = parts.next()?.to_ascii_uppercase();
    let year = parts.next()?.parse().ok()?;
    let index = MONTHS.iter().position(|name| *name == month)?;
    Some((year, index as u32 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_of_reads_vtop_dates() {
        assert_eq!(month_of("01-OCT-2026"), Some((2026, 10)));
        assert_eq!(month_of("01-oct-2026"), Some((2026, 10)));
        assert_eq!(month_of("01-XYZ-2026"), None);
        assert_eq!(month_of(""), None);
    }

    #[test]
    fn a_cell_with_two_events_gives_two_entries() {
        let html = "<table><tr><td><span>5</span>\
            <span>Holiday - General (Semester)</span><span>(Holiday)</span>\
            <span>CAT - I - Freshers</span><span>(Exam Days)</span></td></tr></table>";
        let entries = parse_calendar_month(html, "01-AUG-2026");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-08-05");
        assert_eq!(entries[1].kind, "CAT - I");
        assert_eq!(entries[1].group, "Freshers");
        assert_eq!(entries[1].note, "Exam Days");
    }
}
