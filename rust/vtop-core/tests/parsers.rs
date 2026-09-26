//! Parser tests against saved HTML fixtures.
//!
//! Each `fixtures/<name>.html` has a `fixtures/<name>.expected.json` with the
//! parsed result (`update_time` zeroed). Regenerate the snapshots after an
//! intended parser change with:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p vtop-core --test parsers
//! ```
//!
//! Fixtures are synthetic or redacted. Never commit a real student's page.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;
use vtop_core::parser::{
    attendance, biometric, calendar, exam_schedule, grade_history, grades, marks, timetable,
};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(format!("{name}.html")))
        .unwrap_or_else(|error| panic!("missing fixture {name}.html: {error}"))
}

fn zero_update_time(value: &mut Value) {
    if let Value::Object(map) = value {
        if map.contains_key("update_time") {
            map.insert("update_time".into(), Value::from(0));
        }
    }
}

fn assert_snapshot(name: &str, parsed: &impl Serialize) {
    let mut actual = serde_json::to_value(parsed).expect("parsed data serialises");
    zero_update_time(&mut actual);
    let path = fixture_dir().join(format!("{name}.expected.json"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let pretty = serde_json::to_string_pretty(&actual).expect("pretty json");
        std::fs::write(&path, pretty + "\n").expect("write snapshot");
        return;
    }
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("missing snapshot {}: {error}", path.display())),
    )
    .expect("snapshot is valid JSON");
    assert_eq!(
        actual, expected,
        "{name} no longer matches its snapshot; rerun with UPDATE_SNAPSHOTS=1 if intended"
    );
}

#[test]
fn attendance_fixture() {
    let data = attendance::parse_attendance(&fixture("attendance"), "AP2026271");
    assert_eq!(data.records.len(), 3);
    assert_eq!(data.records[0].course_name, "Data Structures");
    assert_eq!(data.records[0].course_id, "AP2026271000123");
    assert_eq!(data.records[0].course_type, "ETH");
    // A 10-column row has no FAT/CAT column.
    assert_eq!(data.records[2].attendence_fat_cat, "-");
    assert_snapshot("attendance", &data);
}

#[test]
fn full_attendance_fixture() {
    let data = attendance::parse_full_attendance(
        &fixture("full_attendance"),
        "AP2026271",
        "AP2026271000123",
        "ETH",
    );
    assert_eq!(data.records.len(), 3);
    assert_eq!(data.records[1].status, "Absent");
    assert_snapshot("full_attendance", &data);
}

#[test]
fn semesters_fixture() {
    let data = timetable::parse_semid_timetable(&fixture("semesters"));
    assert_eq!(data.semesters.len(), 3);
    assert_eq!(data.semesters[0].id, "AP2026271");
    // Existing behaviour: " - AMR" is stripped after trimming, leaving a
    // trailing space. Kept because the name is shown and stored as-is.
    assert_eq!(data.semesters[0].name, "Fall Semester 2026-27 ");
    assert_snapshot("semesters", &data);
}

#[test]
fn timetable_fixture() {
    let data = timetable::parse_timetable(&fixture("timetable"), "AP2026271");
    assert_eq!(data.courses.len(), 3);
    assert!(data.slots.len() >= 5, "got {} slots", data.slots.len());
    let monday_a1 = data
        .slots
        .iter()
        .find(|slot| slot.day == "MON" && slot.slot == "A1")
        .expect("MON A1 slot");
    assert_eq!(monday_a1.course_code, "CSE2001");
    assert_eq!(monday_a1.start_time, "08:00");
    assert_eq!(monday_a1.end_time, "08:50");
    assert_eq!(monday_a1.name, "Data Structures");
    assert_eq!(monday_a1.faculty, "Dr. Test Faculty - SCOPE");
    assert_eq!(monday_a1.credits, "3.0");
    assert_snapshot("timetable", &data);
}

#[test]
fn marks_fixture() {
    let data = marks::parse_marks(&fixture("marks"), "AP2026271");
    assert_eq!(data.records.len(), 2);
    assert_eq!(data.records[0].marks.len(), 2);
    assert_eq!(data.records[0].marks[1].markstitle, "Quiz 1");
    assert!(data.records[1].marks.is_empty());
    assert_snapshot("marks", &data);
}

#[test]
fn exam_schedule_fixture() {
    let data = exam_schedule::parse_schedule(&fixture("exam_schedule"), "AP2026271");
    assert_eq!(data.exams.len(), 2);
    assert_eq!(data.exams[0].exam_type, "CAT1");
    assert_eq!(data.exams[0].records.len(), 2);
    assert_eq!(data.exams[1].records[0].venue, "-");
    assert_snapshot("exam_schedule", &data);
}

#[test]
fn grade_view_fixture() {
    let data = grades::parse_grade_view(&fixture("grade_view"), "AP2025261");
    assert_eq!(data.semesters.len(), 2);
    assert_eq!(data.semesters[1].name, "Fall Semester 2025-26");
    assert_eq!(data.courses.len(), 2);
    assert_eq!(data.courses[0].course_id, "AP2025261000123");
    assert_eq!(data.courses[1].course_id, "AP2025261000200");
    assert_snapshot("grade_view", &data);
}

#[test]
fn grade_details_fixture() {
    let data =
        grades::parse_grade_view_details(&fixture("grade_details"), "AP2025261", "AP2025261000123");
    assert_eq!(data.class_number, "AP2025261000123 | AP2025261000124");
    assert_eq!(data.class_course_type, "ETH | ELA");
    assert_eq!(data.grand_total, "44.30 | 45.00");
    assert_eq!(data.marks.len(), 3);
    assert_eq!(data.marks[0].mark_title, "ETH • CAT-1");
    assert_eq!(data.grade_ranges.len(), 7);
    assert_eq!(data.grade_ranges[0].range, ">= 90");
    assert_snapshot("grade_details", &data);
}

#[test]
fn grade_history_fixture() {
    let data = grade_history::parse_grade_history(&fixture("grade_history"));
    assert_eq!(data.student.reg_no, "22BCE0000");
    assert_eq!(data.records.len(), 3);
    assert_eq!(data.records[1].attempts.len(), 2);
    assert!(data.records[2].attempts.is_empty());
    assert_eq!(data.cgpa.cgpa, "9.12");
    assert_snapshot("grade_history", &data);
}

#[test]
fn biometric_fixture() {
    let data = biometric::parse_biometric(&fixture("biometric"), "18/07/2026");
    assert_eq!(data.records.len(), 2);
    assert_eq!(data.records[1].punch_time, "19:38");
    assert_snapshot("biometric", &data);
}

#[test]
fn calendar_months_fixture() {
    let months = calendar::parse_calendar_months(&fixture("calendar_months"));
    assert_eq!(
        months,
        [
            "01-JUL-2026",
            "01-AUG-2026",
            "01-SEP-2026",
            "01-OCT-2026",
            "01-NOV-2026",
            "01-DEC-2026"
        ]
    );
}

#[test]
fn calendar_month_fixture() {
    let entries = calendar::parse_calendar_month(&fixture("calendar_month"), "01-OCT-2026");
    // October 2026 has 31 days, each with one event.
    assert_eq!(entries.len(), 31);
    assert_eq!(entries[0].date, "2026-10-01");
    assert_eq!(entries[0].kind, "CAT - II");
    assert_eq!(entries[1].kind, "Holiday");
    assert_eq!(entries[1].note, "Mahatma Gandhi Jayanti");
    assert!(entries
        .iter()
        .any(|entry| entry.kind == "Instructional Day" && entry.note == "LAB FAT"));
    assert!(entries
        .iter()
        .all(|entry| entry.group == "General (Semester)"));
    assert_snapshot("calendar_month", &entries);
}

/// Every parser survives truncated and mangled pages (VTOP sometimes
/// returns partial HTML) without panicking.
#[test]
fn parsers_never_panic_on_truncated_pages() {
    let names = [
        "attendance",
        "full_attendance",
        "semesters",
        "timetable",
        "marks",
        "exam_schedule",
        "grade_view",
        "grade_details",
        "grade_history",
        "biometric",
        "calendar_months",
        "calendar_month",
    ];
    for name in names {
        let html = fixture(name);
        let cut_points = (0..html.len())
            .step_by(37)
            .filter(|&i| html.is_char_boundary(i));
        for cut in cut_points {
            let page = &html[..cut];
            attendance::parse_attendance(page, "S");
            attendance::parse_full_attendance(page, "S", "C", "T");
            timetable::parse_semid_timetable(page);
            timetable::parse_timetable(page, "S");
            marks::parse_marks(page, "S");
            exam_schedule::parse_schedule(page, "S");
            grades::parse_grade_view(page, "S");
            grades::parse_grade_view_details(page, "S", "C");
            grade_history::parse_grade_history(page);
            biometric::parse_biometric(page, "01/01/2026");
            calendar::parse_calendar_months(page);
            calendar::parse_calendar_month(page, "01-OCT-2026");
        }
    }
}

#[test]
fn exam_rows_before_any_group_are_kept() {
    // Two header rows, then a record with no group title before it. The old
    // parser indexed exams[-1] here and panicked.
    let html = "<table><tr><td>h</td></tr><tr><td>h</td></tr><tr>".to_string()
        + &"<td>x</td>".repeat(13)
        + "</tr></table>";
    let data = exam_schedule::parse_schedule(&html, "S");
    assert_eq!(data.exams.len(), 1);
    assert_eq!(data.exams[0].exam_type, "");
    assert_eq!(data.exams[0].records.len(), 1);
}
