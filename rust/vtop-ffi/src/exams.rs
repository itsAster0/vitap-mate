//! Exam schedule: each exam (CAT-I, CAT-II, FAT) with its papers dated,
//! timed and sorted, and the papers the timetable shows on their days. Port
//! of the Flutter app's `lib/features/exams/` (`exam_time.dart`,
//! `exam_schedule_page.dart`).

use std::collections::HashSet;

use tracing::info;
use vtop_core::types::{ExamScheduleData, ExamScheduleRecord};

use crate::calendar::exam_name;
use crate::records::{self, ExamPaper, TimetableDay};
use crate::{ClassKind, Core, CoreError, traced, traced_sync};

pub(crate) const EXAMS: &str = "exam_schedule.json";

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Exams {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// CAT-I, CAT-II, FAT, then any other, whatever VTOP's order; only exams
    /// with papers.
    pub groups: Vec<ExamGroup>,
    /// The group to open on: the first with a paper to come or not dated
    /// yet, else the last with dates (as of now in India).
    pub opens: i32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ExamGroup {
    /// "CAT-I", "FAT".
    pub name: String,
    /// By day and start; papers without a date last.
    pub papers: Vec<ScheduledPaper>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ScheduledPaper {
    /// `<code>.theory` or `<code>.lab` (as `AttendanceCourse::key`), unique
    /// in its group.
    pub key: String,
    pub code: String,
    pub name: String,
    pub kind: ClassKind,
    /// VTOP's "ETH", "TH", "ELA", ...
    pub course_type: String,
    pub slot: String,
    /// Days since 1970-01-01; none until VTOP dates it.
    pub day: Option<i32>,
    /// Minutes since midnight.
    pub start: Option<i32>,
    pub end: Option<i32>,
    pub reporting: Option<i32>,
    /// "FN" / "AN"; empty when VTOP gives none.
    pub session: String,
    pub venue: String,
    /// "R6 · C6" for VTOP's "R6C6", else as given.
    pub seat: String,
    pub seat_no: String,
}

#[uniffi::export(async_runtime = "tokio")]
impl Core {
    /// The selected semester's cached exam schedule.
    pub fn cached_exams(&self, flow: String) -> Option<Exams> {
        traced_sync("cached_exams", &flow, || {
            Ok(self
                .read_page::<ExamScheduleData>(EXAMS)
                .map(|data| logged(Exams::new(data, records::ist_now()))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the selected semester's exam schedule and caches it; the
    /// timetable's days read their papers from that copy.
    pub async fn fetch_exams(&self, flow: String) -> Result<Exams, CoreError> {
        traced("fetch_exams", &flow, async {
            let data = self
                .fetch(EXAMS, |client, semester| {
                    Box::pin(client.exam_schedule(semester))
                })
                .await?;
            Ok(logged(Exams::new(data, records::ist_now())))
        })
        .await
    }
}

fn logged(exams: Exams) -> Exams {
    info!(
        groups = exams.groups.len(),
        papers = exams
            .groups
            .iter()
            .map(|group| group.papers.len())
            .sum::<usize>(),
        opens = exams.opens,
        "exams"
    );
    exams
}

/// Puts each dated paper of `data` on its day in `days`, by start.
pub(crate) fn fill_papers(days: &mut [TimetableDay], data: Option<ExamScheduleData>) {
    let Some(data) = data else {
        return;
    };
    for group in Exams::new(data, (0, 0)).groups {
        for paper in group.papers {
            let Some(day) = paper.day else {
                continue;
            };
            if let Some(target) = days.iter_mut().find(|d| d.day == day) {
                target.papers.push(ExamPaper {
                    name: group.name.clone(),
                    course: paper.name,
                    day,
                    start: paper.start,
                });
            }
        }
    }
    for day in days {
        day.papers
            .sort_by_key(|paper| paper.start.unwrap_or(i32::MAX));
    }
}

impl Exams {
    /// `now`: days since 1970-01-01 and minutes since midnight, in India.
    fn new(data: ExamScheduleData, now: (i32, i32)) -> Self {
        let mut groups: Vec<ExamGroup> = data
            .exams
            .into_iter()
            .filter(|group| !group.records.is_empty())
            .map(|group| {
                let mut keys = HashSet::new();
                let mut papers: Vec<ScheduledPaper> = group
                    .records
                    .iter()
                    .map(|record| {
                        let mut paper = ScheduledPaper::from(record);
                        // Two rows of one code and kind would share a key.
                        if !keys.insert(paper.key.clone()) {
                            paper.key = format!("{}.{}", paper.key, paper.slot);
                        }
                        paper
                    })
                    .collect();
                papers.sort_by_key(|paper| {
                    (
                        paper.day.unwrap_or(i32::MAX),
                        paper.start.unwrap_or(i32::MAX),
                    )
                });
                ExamGroup {
                    name: exam_name(&group.exam_type),
                    papers,
                }
            })
            .collect();
        groups.sort_by_key(|group| rank(&group.name));
        let opens = opens(&groups, now);
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            groups,
            opens,
        }
    }
}

/// CAT-I, CAT-II, ... first, then FAT, then anything else.
fn rank(name: &str) -> i32 {
    match name.strip_prefix("CAT-") {
        Some(roman) => ["I", "II", "III", "IV"]
            .iter()
            .position(|r| *r == roman)
            .map_or(5, |i| i as i32 + 1),
        None if name.contains("FAT") => 10,
        None => 20,
    }
}

fn opens(groups: &[ExamGroup], (today, minute): (i32, i32)) -> i32 {
    let ahead = |paper: &ScheduledPaper| match paper.day {
        None => true,
        // Without a time a paper lasts the day, as in the Flutter app.
        Some(day) => day > today || (day == today && paper.start.unwrap_or(23 * 60 + 59) > minute),
    };
    groups
        .iter()
        .position(|group| group.papers.iter().any(ahead))
        .or_else(|| {
            groups
                .iter()
                .rposition(|group| group.papers.iter().any(|paper| paper.day.is_some()))
        })
        .unwrap_or(0) as i32
}

impl From<&ExamScheduleRecord> for ScheduledPaper {
    fn from(record: &ExamScheduleRecord) -> Self {
        let course_type = record.course_type.trim().to_string();
        let lower = course_type.to_lowercase();
        let lab = matches!(lower.as_str(), "ela" | "lo") || lower.contains("lab");
        let code = record.course_code.trim().to_string();
        let (start, end) = record
            .exam_time
            .split_once('-')
            .map_or((None, None), |(start, end)| {
                (records::minutes(start), records::minutes(end))
            });
        Self {
            key: format!("{code}.{}", if lab { "lab" } else { "theory" }),
            name: record.course_name.trim().to_string(),
            kind: if lab {
                ClassKind::Lab
            } else {
                ClassKind::Theory
            },
            slot: value(&record.slot),
            day: records::history_day(&record.exam_date),
            start: start.or_else(|| records::minutes(&record.exam_time)),
            end,
            reporting: records::minutes(&record.reporting_time),
            session: value(&record.exam_session),
            venue: value(&record.venue),
            seat: seat(&value(&record.seat_location)),
            seat_no: value(&record.seat_no),
            course_type,
            code,
        }
    }
}

/// VTOP's cell, empty for "-".
fn value(raw: &str) -> String {
    let raw = raw.trim();
    if raw == "-" { "" } else { raw }.to_string()
}

/// "R6C6" → "R6 · C6"; anything else as given.
fn seat(raw: &str) -> String {
    let upper = raw.to_ascii_uppercase();
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    match upper
        .strip_prefix('R')
        .and_then(|rest| rest.split_once('C'))
    {
        Some((row, column)) if digits(row) && digits(column) => format!("R{row} · C{column}"),
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use vtop_core::types::PerExamScheduleRecord;

    use super::*;

    fn record(code: &str, kind: &str, date: &str, time: &str) -> ExamScheduleRecord {
        ExamScheduleRecord {
            serial: "1".into(),
            slot: "A1+TA1".into(),
            course_name: format!("{code} name"),
            course_code: code.into(),
            course_type: kind.into(),
            course_id: "AP1".into(),
            exam_date: date.into(),
            exam_session: "FN".into(),
            reporting_time: "09:15 AM".into(),
            exam_time: time.into(),
            venue: "AB1-101".into(),
            seat_location: "R6C6".into(),
            seat_no: "14".into(),
        }
    }

    fn data(groups: Vec<(&str, Vec<ExamScheduleRecord>)>) -> ExamScheduleData {
        ExamScheduleData {
            exams: groups
                .into_iter()
                .map(|(exam_type, records)| PerExamScheduleRecord {
                    exam_type: exam_type.into(),
                    records,
                })
                .collect(),
            semester_id: "AP1".into(),
            update_time: 1_000,
        }
    }

    fn day(date: &str) -> i32 {
        records::epoch_day(date).unwrap()
    }

    #[test]
    fn papers_are_dated_timed_and_sorted() {
        let exams = Exams::new(
            data(vec![
                ("FAT", vec![record("CSE2001", "ETH", "-", "-")]),
                (
                    "CAT2",
                    vec![
                        record("MAT1001", "TH", "11-Aug-2026", "02:00 PM - 03:30 PM"),
                        record("CSE2001", "ETH", "10-Aug-2026", "09:30 AM - 11:00 AM"),
                        record("CSE2001", "ELA", "-", "-"),
                    ],
                ),
                ("CAT1", vec![record("X", "TH", "01-Jul-2026", "")]),
                ("Mid", vec![]),
            ]),
            (day("2026-08-01"), 0),
        );
        let names: Vec<_> = exams.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["CAT-I", "CAT-II", "FAT"]);
        let cat2 = &exams.groups[1].papers;
        let keys: Vec<_> = cat2.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["CSE2001.theory", "MAT1001.theory", "CSE2001.lab"]);
        let first = &cat2[0];
        assert_eq!(first.day, Some(day("2026-08-10")));
        assert_eq!(
            (first.start, first.end, first.reporting),
            (Some(570), Some(660), Some(555))
        );
        assert_eq!(
            (first.seat.as_str(), first.session.as_str()),
            ("R6 · C6", "FN")
        );
        assert_eq!(cat2[2].kind, ClassKind::Lab);
        let undated = &exams.groups[2].papers[0];
        assert_eq!(
            (undated.day, undated.start, undated.venue.as_str()),
            (None, None, "AB1-101")
        );
        // CAT-I is over; CAT-II is still to come.
        assert_eq!(exams.opens, 1);
    }

    #[test]
    fn opens_on_the_next_exam_else_the_last_dated() {
        let schedule = || {
            data(vec![
                (
                    "CAT1",
                    vec![record("A", "TH", "10-Aug-2026", "09:30 AM - 11:00 AM")],
                ),
                ("CAT2", vec![record("A", "TH", "10-Sep-2026", "")]),
            ])
        };
        // Today, before the paper starts.
        assert_eq!(Exams::new(schedule(), (day("2026-08-10"), 9 * 60)).opens, 0);
        assert_eq!(
            Exams::new(schedule(), (day("2026-08-10"), 12 * 60)).opens,
            1
        );
        // Untimed papers last the day.
        assert_eq!(
            Exams::new(schedule(), (day("2026-09-10"), 20 * 60)).opens,
            1
        );
        assert_eq!(Exams::new(schedule(), (day("2026-09-11"), 0)).opens, 1);
        assert_eq!(Exams::new(data(vec![]), (0, 0)).opens, 0);
    }

    #[test]
    fn seats_and_dashes() {
        assert_eq!(seat("r12c3"), "R12 · C3");
        assert_eq!(seat("Row 2"), "Row 2");
        assert_eq!(seat("RC"), "RC");
        assert_eq!(value(" - "), "");
        assert_eq!(rank("CAT-III"), 3);
        assert!(rank("FAT") < rank("Lab"));
    }

    #[test]
    fn dated_papers_land_on_their_days() {
        let mut days: Vec<TimetableDay> = (day("2026-08-10")..day("2026-08-12"))
            .map(|d| TimetableDay {
                day: d,
                classes: Vec::new(),
                mark: None,
                title: None,
                holds_theory: true,
                last_class_day: false,
                papers: Vec::new(),
                class_days_left: 0,
                upcoming_exam: None,
            })
            .collect();
        fill_papers(
            &mut days,
            Some(data(vec![(
                "CAT1",
                vec![
                    record("B", "TH", "10-Aug-2026", "02:00 PM - 03:30 PM"),
                    record("A", "TH", "10-Aug-2026", "09:30 AM - 11:00 AM"),
                    record("C", "TH", "-", "-"),
                    record("D", "TH", "20-Aug-2026", "09:30 AM - 11:00 AM"),
                ],
            )])),
        );
        let first: Vec<_> = days[0]
            .papers
            .iter()
            .map(|p| (p.name.as_str(), p.course.as_str(), p.start))
            .collect();
        assert_eq!(
            first,
            [
                ("CAT-I", "A name", Some(570)),
                ("CAT-I", "B name", Some(840))
            ]
        );
        assert!(days[1].papers.is_empty());
    }
}
