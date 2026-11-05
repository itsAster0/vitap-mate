//! The academic calendar: VTOP's entries classified per day, the month view,
//! and what the timetable needs (class days, Lab FATs, exams, days left).
//! Port of the Flutter app's `lib/features/calendar/logic/`
//! (`semester_calendar.dart`, `calendar_events.dart`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use vtop_core::types::{AcademicCalendarData, CalendarEntry};

use crate::records::{CalendarMark, ClassDay, civil_date, epoch_day, weekday};

const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const SUNDAY: i32 = 6;

/// What a calendar day is.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CalendarKind {
    /// Classes as usual; `title` names any event on the day.
    Instructional,
    /// Classes on a day that is usually off, or on another weekday's
    /// timetable: `follows` is that weekday (0 = Monday) when VTOP names it.
    WorkingDay {
        follows: Option<i32>,
    },
    /// A class day whose labs are the lab exam; theory classes meet.
    LabFat,
    Holiday,
    NoInstructional,
    /// "CAT-I", "CAT-II", "FAT".
    Exam {
        name: String,
    },
    /// Something the app does not know; no classes.
    Other,
}

/// The semester calendar, ready to draw.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct AcademicCalendar {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// Today in India, days since 1970-01-01; `days_left` counts from it.
    pub today: i32,
    /// Every month from the first listed day's to the last's.
    pub months: Vec<CalendarMonth>,
    /// Instructional days after today.
    pub days_left: i32,
    /// The next weekday holiday from today on (Sundays are always off).
    pub next_holiday: Option<CalendarDay>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CalendarMonth {
    /// `YYYY-MM`.
    pub month: String,
    /// Every day of the month, listed or not, from the 1st.
    pub days: Vec<CalendarDay>,
    /// The rows touching this month, in date order; a run from a
    /// neighbouring month is included whole.
    pub events: Vec<CalendarEvent>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CalendarDay {
    /// Days since 1970-01-01.
    pub day: i32,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The most notable entry's kind (exam over Lab FAT over holiday over
    /// class day); none when the calendar does not list the day.
    pub kind: Option<CalendarKind>,
    pub mark: Option<CalendarMark>,
    /// "Deepavali", "CAT-II", "Lab FAT", "No instructional day"; empty when
    /// not listed.
    pub title: String,
    /// VTOP's text for every entry of the day: "Holiday (Deepavali)".
    pub label: String,
}

/// One row of a month's list: a day, or a run of days with the same title
/// (an exam, the Lab FAT, back-to-back days off).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CalendarEvent {
    /// Days since 1970-01-01, ends included.
    pub first: i32,
    pub last: i32,
    /// `YYYY-MM-DD`.
    pub first_date: String,
    pub last_date: String,
    pub kind: CalendarKind,
    pub mark: CalendarMark,
    /// The row's name; "3 days off" for days off with different reasons.
    pub title: String,
    /// The reasons inside a run of days off, each over its own days (one
    /// part when they all have the same reason).
    pub parts: Vec<CalendarEventPart>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CalendarEventPart {
    pub first: i32,
    pub last: i32,
    pub mark: CalendarMark,
    pub title: String,
}

/// One classified entry.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    kind: CalendarKind,
    mark: CalendarMark,
    title: String,
    label: String,
}

impl Entry {
    fn of(entry: &CalendarEntry, day: i32) -> Self {
        let raw = entry.kind.trim();
        let kind = raw.to_lowercase();
        let note = entry.note.trim();
        let note_lower = note.to_lowercase();
        let plain_note =
            note.is_empty() || note_lower == "workingday" || note_lower == "working day";
        let (kind, mark, title) = if kind == "instructional day" {
            let follows = day_order(&note_lower).filter(|&order| order != weekday(day));
            if note_lower.contains("lab fat") {
                (
                    CalendarKind::LabFat,
                    CalendarMark::LabFat,
                    "Lab FAT".to_string(),
                )
            } else if follows.is_some() || weekday(day) == SUNDAY {
                let title = if plain_note { "Working day" } else { note };
                (
                    CalendarKind::WorkingDay { follows },
                    CalendarMark::Special,
                    title.to_string(),
                )
            } else if plain_note {
                (
                    CalendarKind::Instructional,
                    CalendarMark::Classes,
                    raw.to_string(),
                )
            } else {
                (
                    CalendarKind::Instructional,
                    CalendarMark::Special,
                    note.to_string(),
                )
            }
        } else if kind == "holiday" {
            let title = if note.is_empty() { "Holiday" } else { note };
            (
                CalendarKind::Holiday,
                CalendarMark::Holiday,
                title.to_string(),
            )
        } else if kind.starts_with("no instructional") {
            let title = if note.is_empty() || note_lower == "no instructional day" {
                "No instructional day"
            } else {
                note
            };
            (
                CalendarKind::NoInstructional,
                CalendarMark::NoClasses,
                title.to_string(),
            )
        } else if is_exam(&kind) || note_lower.contains("exam") {
            let name = exam_name(raw);
            (
                CalendarKind::Exam { name: name.clone() },
                CalendarMark::Exam,
                name,
            )
        } else {
            let title = if note.is_empty() { raw } else { note };
            (
                CalendarKind::Other,
                CalendarMark::Special,
                title.to_string(),
            )
        };
        let label = if note.is_empty() || note.eq_ignore_ascii_case(raw) {
            raw.to_string()
        } else {
            format!("{raw} ({note})")
        };
        Self {
            kind,
            mark,
            title,
            label,
        }
    }

    fn day_off(&self) -> bool {
        matches!(self.mark, CalendarMark::Holiday | CalendarMark::NoClasses)
    }

    /// A day off with its own name, not the weekly "Holiday" Sunday or a
    /// plain "No instructional day".
    fn named(&self) -> bool {
        !matches!(self.title.as_str(), "Holiday" | "No instructional day")
    }

    /// Which day of a run of days off names it: a named holiday, then a
    /// named no-class day, then a plain holiday.
    fn day_off_rank(&self) -> u8 {
        match (self.named(), self.mark == CalendarMark::Holiday) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        }
    }
}

/// The weekday (0 = Monday) a note names, e.g. "Monday Day Order",
/// "Working Day - TUE timetable".
fn day_order(note: &str) -> Option<i32> {
    note.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() >= 3)
        .find_map(|word| {
            WEEKDAYS
                .iter()
                .position(|day| *day == word || (word.len() == 3 && day.starts_with(word)))
        })
        .map(|day| day as i32)
}

fn is_exam(kind: &str) -> bool {
    let words: Vec<&str> = kind
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().any(|word| {
        matches!(
            *word,
            "cat" | "fat" | "exam" | "exams" | "assessment" | "test"
        )
    }) || kind.starts_with("cat")
}

/// One name for an exam: the calendar's "CAT - II" or "Final Assessment
/// Test", the exam schedule's "CAT2" or "FAT", all read "CAT-II" / "FAT".
pub(crate) fn exam_name(raw: &str) -> String {
    let kind = raw.trim();
    let lower = kind.to_lowercase();
    if lower == "final assessment test" || lower == "fat" {
        return "FAT".to_string();
    }
    if let Some(rest) = lower.strip_prefix("cat") {
        let number = rest.trim_start().trim_start_matches('-').trim();
        let roman = match number {
            "1" => Some("I"),
            "2" => Some("II"),
            "3" => Some("III"),
            "4" => Some("IV"),
            _ => None,
        };
        if let Some(roman) = roman {
            return format!("CAT-{roman}");
        }
        if !number.is_empty() && number.chars().all(|c| matches!(c, 'i' | 'v' | 'x')) {
            return format!("CAT-{}", number.to_uppercase());
        }
    }
    kind.replace(" - ", "-")
}

/// The calendar as the timetable and the month view use it.
#[derive(Debug, Default)]
pub(crate) struct SemesterCalendar {
    /// Every entry of each listed day, in VTOP's order.
    entries: BTreeMap<i32, Vec<Entry>>,
    instructional: BTreeSet<i32>,
    /// Class days marked Lab FAT: labs on them are the lab exam.
    lab_fat_days: BTreeSet<i32>,
    /// The weekday each class day follows when it is not its own.
    follows: HashMap<i32, i32>,
    /// Each exam by its first day, in date order.
    exams: Vec<(i32, String)>,
    /// Weekday holidays (Sundays are always off), in date order.
    holidays: Vec<i32>,
}

impl SemesterCalendar {
    /// Uses the "General (Semester)" entries when there are any: the
    /// combined view also lists other class groups' days.
    pub(crate) fn of(data: &AcademicCalendarData) -> Self {
        let general: Vec<&CalendarEntry> = data
            .entries
            .iter()
            .filter(|entry| entry.group.to_lowercase().starts_with("general"))
            .collect();
        let entries = if general.is_empty() {
            data.entries.iter().collect()
        } else {
            general
        };
        let mut calendar = Self::default();
        let mut exams: HashMap<String, i32> = HashMap::new();
        for raw in entries {
            let Some(day) = epoch_day(&raw.date) else {
                continue;
            };
            let entry = Entry::of(raw, day);
            if raw.kind.trim().eq_ignore_ascii_case("instructional day") {
                calendar.instructional.insert(day);
                if entry.kind == CalendarKind::LabFat {
                    calendar.lab_fat_days.insert(day);
                }
                if let CalendarKind::WorkingDay {
                    follows: Some(order),
                } = entry.kind
                {
                    calendar.follows.insert(day, order);
                }
            }
            if let CalendarKind::Exam { name } = &entry.kind {
                let start = exams.entry(name.clone()).or_insert(day);
                *start = (*start).min(day);
            }
            if entry.kind == CalendarKind::Holiday && weekday(day) != SUNDAY {
                calendar.holidays.push(day);
            }
            calendar.entries.entry(day).or_default().push(entry);
        }
        calendar.holidays.sort_unstable();
        calendar.holidays.dedup();
        calendar.exams = exams.into_iter().map(|(name, day)| (day, name)).collect();
        calendar.exams.sort();
        calendar
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.instructional.is_empty()
    }

    /// The day's most notable entry (exam over Lab FAT over holiday over
    /// class day).
    fn entry_on(&self, day: i32) -> Option<&Entry> {
        self.entries
            .get(&day)?
            .iter()
            .min_by_key(|entry| entry.mark as u8)
    }

    /// The day's mark and title, for the agenda.
    pub(crate) fn mark_on(&self, day: i32) -> Option<(CalendarMark, String)> {
        self.entry_on(day)
            .map(|entry| (entry.mark, entry.title.clone()))
    }

    /// Whether `day` falls between the first and last listed day; outside
    /// that the calendar says nothing and the plain timetable applies.
    pub(crate) fn covers(&self, day: i32) -> bool {
        match (self.entries.keys().next(), self.entries.keys().next_back()) {
            (Some(&first), Some(&last)) => (first..=last).contains(&day),
            _ => false,
        }
    }

    /// The timetable weekday (0 = Monday) `day` follows: its own, or the
    /// one a working day names.
    pub(crate) fn weekday_of(&self, day: i32) -> i32 {
        self.follows
            .get(&day)
            .copied()
            .unwrap_or_else(|| weekday(day))
    }

    /// Whether classes of this kind are held on `day`: on class days, and
    /// for labs only before the first Lab FAT day.
    pub(crate) fn holds_classes(&self, day: i32, lab: bool) -> bool {
        self.instructional.contains(&day)
            && (!lab || self.lab_fat_days.first().is_none_or(|&start| day < start))
    }

    /// How a class of this kind goes on `day`. On Lab FAT days a course's
    /// lab is its Lab FAT on the first such day it has a lab
    /// (`lab_weekdays`, 0 = Monday); its other labs those days don't meet.
    pub(crate) fn class_day(&self, day: i32, lab: bool, lab_weekdays: &[i32]) -> ClassDay {
        if !self.covers(day) {
            return ClassDay::Held;
        }
        if lab && self.lab_fat_days.contains(&day) {
            let first = self
                .lab_fat_days
                .iter()
                .find(|&&fat| lab_weekdays.contains(&self.weekday_of(fat)));
            return if first == Some(&day) {
                ClassDay::LabFat
            } else {
                ClassDay::Off
            };
        }
        if self.holds_classes(day, lab) {
            ClassDay::Held
        } else {
            ClassDay::Off
        }
    }

    pub(crate) fn first_class_day(&self) -> Option<i32> {
        self.instructional.first().copied()
    }

    /// The last day theory classes meet.
    pub(crate) fn last_class_day(&self) -> Option<i32> {
        self.instructional.last().copied()
    }

    /// The first exam starting after `day`, whether or not classes come
    /// before it.
    pub(crate) fn upcoming_exam(&self, day: i32) -> Option<(i32, String)> {
        self.exams.iter().find(|(start, _)| *start > day).cloned()
    }

    /// The month view as of `today`.
    pub(crate) fn view(&self, data: &AcademicCalendarData, today: i32) -> AcademicCalendar {
        let events = self.events();
        let mut months = Vec::new();
        if let (Some(&first), Some(&last)) =
            (self.entries.keys().next(), self.entries.keys().next_back())
        {
            let mut start = month_start(first);
            while start <= last {
                let next = month_start(start + 31);
                months.push(CalendarMonth {
                    month: civil_date(i64::from(start))[..7].to_string(),
                    days: (start..next).map(|day| self.day(day)).collect(),
                    events: events
                        .iter()
                        .filter(|event| event.last >= start && event.first < next)
                        .cloned()
                        .collect(),
                });
                start = next;
            }
        }
        AcademicCalendar {
            semester_id: data.semester_id.clone(),
            updated_at: data.update_time as i64,
            today,
            months,
            days_left: self.instructional.range(today + 1..).count() as i32,
            next_holiday: self
                .holidays
                .iter()
                .find(|&&day| day >= today)
                .map(|&day| self.day(day)),
        }
    }

    fn day(&self, day: i32) -> CalendarDay {
        let entries = self.entries.get(&day);
        let top = self.entry_on(day);
        CalendarDay {
            day,
            date: civil_date(i64::from(day)),
            kind: top.map(|entry| entry.kind.clone()),
            mark: top.map(|entry| entry.mark),
            title: top.map(|entry| entry.title.clone()).unwrap_or_default(),
            label: entries.map_or_else(String::new, |entries| {
                entries
                    .iter()
                    .map(|entry| entry.label.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ")
            }),
        }
    }

    /// Whether a class day lies strictly between `a` and `b`, so two exam
    /// blocks around a class week stay separate rows.
    fn class_day_between(&self, a: i32, b: i32) -> bool {
        (a + 1..b).any(|day| {
            self.entry_on(day).is_some_and(|entry| {
                matches!(
                    entry.mark,
                    CalendarMark::Classes | CalendarMark::Special | CalendarMark::LabFat
                )
            })
        })
    }

    /// The semester's notable days as rows, in date order: holidays, exams,
    /// no-class days and named events. Plain class days are left out, and
    /// so are runs of plain days off (the weekly Sunday and Monday).
    fn events(&self) -> Vec<CalendarEvent> {
        struct Row {
            first: i32,
            last: i32,
            entry: Entry,
            parts: Vec<(i32, i32, Entry)>,
        }
        let mut rows: Vec<Row> = Vec::new();
        for (&day, entries) in &self.entries {
            for entry in entries {
                if entry.mark == CalendarMark::Classes {
                    continue;
                }
                if entry.day_off() {
                    // Back-to-back days off are one row, named by its most
                    // telling day.
                    match rows.iter_mut().rev().find(|row| row.entry.day_off()) {
                        Some(row) if day - row.last <= 1 => {
                            row.last = day;
                            match row.parts.last_mut() {
                                Some(part)
                                    if part.2.title == entry.title && part.2.mark == entry.mark =>
                                {
                                    part.1 = day;
                                }
                                _ => row.parts.push((day, day, entry.clone())),
                            }
                            if entry.day_off_rank() < row.entry.day_off_rank() {
                                row.entry = entry.clone();
                            }
                        }
                        _ => rows.push(Row {
                            first: day,
                            last: day,
                            entry: entry.clone(),
                            parts: vec![(day, day, entry.clone())],
                        }),
                    }
                    continue;
                }
                // The latest row of the same kind, even with a holiday row after it.
                let previous = rows.iter().rposition(|row| {
                    row.entry.mark == entry.mark && row.entry.title == entry.title
                });
                match previous {
                    Some(index) if !self.class_day_between(rows[index].last, day) => {
                        rows[index].last = day;
                        rows[index].parts[0].1 = day;
                    }
                    _ => rows.push(Row {
                        first: day,
                        last: day,
                        entry: entry.clone(),
                        parts: vec![(day, day, entry.clone())],
                    }),
                }
            }
        }
        rows.into_iter()
            .filter(|row| !row.entry.day_off() || row.parts.iter().any(|part| part.2.named()))
            .map(|row| {
                let days = row.last - row.first + 1;
                // A run of days off with any holiday in it reads as a holiday.
                let holiday = row.entry.day_off()
                    && row
                        .parts
                        .iter()
                        .any(|part| part.2.mark == CalendarMark::Holiday);
                let (kind, mark) = if holiday {
                    (CalendarKind::Holiday, CalendarMark::Holiday)
                } else {
                    (row.entry.kind.clone(), row.entry.mark)
                };
                let title = if row.parts.len() > 1 {
                    format!("{days} days off")
                } else {
                    row.entry.title.clone()
                };
                CalendarEvent {
                    first: row.first,
                    last: row.last,
                    first_date: civil_date(i64::from(row.first)),
                    last_date: civil_date(i64::from(row.last)),
                    kind,
                    mark,
                    title,
                    parts: row
                        .parts
                        .into_iter()
                        .map(|(first, last, entry)| CalendarEventPart {
                            first,
                            last,
                            mark: entry.mark,
                            title: entry.title,
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

/// The 1st of `day`'s month.
fn month_start(day: i32) -> i32 {
    let date = civil_date(i64::from(day));
    epoch_day(&format!("{}-01", &date[..7])).unwrap_or(day)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn entry(date: &str, kind: &str, note: &str) -> CalendarEntry {
        CalendarEntry {
            date: date.into(),
            kind: kind.into(),
            group: "General (Semester)".into(),
            note: note.into(),
        }
    }

    fn day(date: &str) -> i32 {
        epoch_day(date).unwrap()
    }

    #[test]
    fn entries_classify_like_the_flutter_app() {
        let of = |date, kind, note| {
            let raw = entry(date, kind, note);
            let entry = Entry::of(&raw, day(date));
            (entry.kind, entry.mark, entry.title)
        };
        // 2026-11-05 is a Thursday.
        assert_eq!(
            of("2026-11-05", "Instructional Day", "WorkingDay"),
            (
                CalendarKind::Instructional,
                CalendarMark::Classes,
                "Instructional Day".into()
            )
        );
        assert_eq!(
            of("2026-11-05", "Instructional Day", "LAB FAT"),
            (CalendarKind::LabFat, CalendarMark::LabFat, "Lab FAT".into())
        );
        assert_eq!(
            of("2026-07-14", "Instructional Day", "First Instructional Day").1,
            CalendarMark::Special
        );
        assert_eq!(
            of("2026-11-07", "Holiday", "Deepavali"),
            (
                CalendarKind::Holiday,
                CalendarMark::Holiday,
                "Deepavali".into()
            )
        );
        assert_eq!(
            of("2026-11-02", "No Instructional Day", "No Instructional Day").2,
            "No instructional day"
        );
        assert_eq!(
            of("2026-09-26", "No Instructional Day", "Convocation Day").2,
            "Convocation Day"
        );
        assert_eq!(
            of("2026-09-28", "CAT - II", "Exam Days").0,
            CalendarKind::Exam {
                name: "CAT-II".into()
            }
        );
        assert_eq!(
            of("2026-11-16", "Final Assessment Test", "Exam Days").2,
            "FAT"
        );
        assert_eq!(of("2026-11-14", "Sports Meet", "").0, CalendarKind::Other);
        // A Saturday on Monday's timetable; a Sunday with classes.
        assert_eq!(
            of("2026-11-14", "Instructional Day", "Monday Day Order").0,
            CalendarKind::WorkingDay { follows: Some(0) }
        );
        assert_eq!(
            of("2026-11-15", "Instructional Day", "WorkingDay"),
            (
                CalendarKind::WorkingDay { follows: None },
                CalendarMark::Special,
                "Working day".into()
            )
        );
        // Naming the day's own weekday changes nothing.
        assert_eq!(
            of("2026-11-05", "Instructional Day", "Thursday"),
            (
                CalendarKind::Instructional,
                CalendarMark::Special,
                "Thursday".into()
            )
        );
    }

    #[test]
    fn exam_names() {
        assert_eq!(exam_name("CAT - I"), "CAT-I");
        assert_eq!(exam_name("CAT2"), "CAT-II");
        assert_eq!(exam_name("cat-iii"), "CAT-III");
        assert_eq!(exam_name("Final Assessment Test"), "FAT");
        assert_eq!(exam_name("Mid - Term"), "Mid-Term");
        assert_eq!(day_order("working day - tue timetable"), Some(1));
        assert_eq!(day_order("saturday"), Some(5));
        assert_eq!(day_order("workingday"), None);
    }

    /// A week of the real Fall 2026-27 calendar around the Lab FAT and
    /// Deepavali, then the FAT.
    pub(crate) fn fall_2026() -> AcademicCalendarData {
        let entries = [
            ("2026-10-30", "Instructional Day", "WorkingDay"),
            ("2026-10-31", "Instructional Day", "LAB FAT"),
            ("2026-11-01", "Holiday", "Holiday"),
            ("2026-11-02", "No Instructional Day", "No Instructional Day"),
            ("2026-11-03", "Instructional Day", "LAB FAT"),
            ("2026-11-04", "Instructional Day", "LAB FAT"),
            ("2026-11-05", "Instructional Day", "LAB FAT"),
            ("2026-11-06", "Instructional Day", "LAB FAT"),
            ("2026-11-07", "Holiday", "Deepavali"),
            ("2026-11-08", "Holiday", "Deepavali"),
            ("2026-11-09", "Holiday", "Deepavali"),
            ("2026-11-10", "Holiday", "Deepavali"),
            ("2026-11-11", "Instructional Day", "WorkingDay"),
            ("2026-11-12", "Instructional Day", "WorkingDay"),
            ("2026-11-13", "Instructional Day", "WorkingDay"),
            (
                "2026-11-14",
                "Instructional Day",
                "Engineering Clinics Expo*",
            ),
            ("2026-11-15", "Holiday", "Holiday"),
            ("2026-11-16", "Final Assessment Test", "Exam Days"),
            ("2026-11-17", "Final Assessment Test", "Exam Days"),
            ("2026-11-21", "Final Assessment Test", "Exam Days"),
            ("2026-11-22", "Holiday", "Holiday"),
            ("2026-11-23", "Final Assessment Test", "Exam Days"),
            ("2026-12-01", "Final Assessment Test", "Exam Days"),
        ];
        let mut entries: Vec<CalendarEntry> = entries
            .iter()
            .map(|(date, kind, note)| entry(date, kind, note))
            .collect();
        // Another class group's day is ignored when General ones exist.
        entries.push(CalendarEntry {
            group: "Freshers".into(),
            ..entry("2026-11-12", "Holiday", "Freshers Day")
        });
        AcademicCalendarData {
            entries,
            semester_id: "AP2026272".into(),
            update_time: 9,
        }
    }

    #[test]
    fn months_days_and_events() {
        let data = fall_2026();
        let calendar = SemesterCalendar::of(&data);
        let today = day("2026-11-05");
        let view = calendar.view(&data, today);
        assert_eq!(
            view.months
                .iter()
                .map(|m| m.month.as_str())
                .collect::<Vec<_>>(),
            ["2026-10", "2026-11", "2026-12"]
        );
        let november = &view.months[1];
        assert_eq!(november.days.len(), 30);
        assert_eq!(november.days[0].date, "2026-11-01");
        assert_eq!(november.days[17].day, day("2026-11-18"));
        assert_eq!(november.days[17].kind, None);
        assert_eq!(november.days[11].mark, Some(CalendarMark::Classes));
        assert_eq!(november.days[6].label, "Holiday (Deepavali)");
        // Fri 6 (Lab FAT), Wed 11, Thu 12, Fri 13 and Sat 14.
        assert_eq!(view.days_left, 5);
        assert_eq!(
            view.next_holiday.map(|day| day.title),
            Some("Deepavali".to_string())
        );
        let rows: Vec<(String, String, String)> = november
            .events
            .iter()
            .map(|e| (e.first_date.clone(), e.last_date.clone(), e.title.clone()))
            .collect();
        let row = |a: &str, b: &str, t: &str| (a.to_string(), b.to_string(), t.to_string());
        assert_eq!(
            rows,
            [
                // From October, whole; the weekly Sunday and Monday off are
                // not rows of their own.
                row("2026-10-31", "2026-11-06", "Lab FAT"),
                row("2026-11-07", "2026-11-10", "Deepavali"),
                row("2026-11-14", "2026-11-14", "Engineering Clinics Expo*"),
                // Holidays inside the exam weeks don't split it.
                row("2026-11-16", "2026-12-01", "FAT"),
            ]
        );
        assert_eq!(november.events[1].mark, CalendarMark::Holiday);
        assert_eq!(view.months[2].events.len(), 1);
    }

    #[test]
    fn mixed_days_off_are_one_row_with_parts() {
        let data = AcademicCalendarData {
            entries: vec![
                entry("2026-10-17", "Instructional Day", "WorkingDay"),
                entry("2026-10-18", "Holiday", "Holiday"),
                entry("2026-10-19", "No Instructional Day", "No Instructional Day"),
                entry("2026-10-20", "Holiday", "Vijaya Dashami / Dussehra"),
                entry("2026-10-21", "Instructional Day", "WorkingDay"),
            ],
            semester_id: "S".into(),
            update_time: 1,
        };
        let calendar = SemesterCalendar::of(&data);
        let events = calendar.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "3 days off");
        assert_eq!(events[0].mark, CalendarMark::Holiday);
        assert_eq!(
            events[0]
                .parts
                .iter()
                .map(|p| p.title.as_str())
                .collect::<Vec<_>>(),
            [
                "Holiday",
                "No instructional day",
                "Vijaya Dashami / Dussehra"
            ]
        );
    }
}
