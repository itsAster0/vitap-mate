use std::path::PathBuf;

use serde_json::{Value, json};

use super::*;
use crate::ATTENDANCE_DETAIL;

/// Monday 2025-07-07, the default calendar's first class day.
fn d0() -> i32 {
    records::epoch_day("2025-07-07").unwrap()
}

fn iso(day: i32) -> String {
    records::civil_date(i64::from(day))
}

/// "DD-MM-YYYY", as VTOP dates a history.
fn dmy(day: i32) -> String {
    let iso = iso(day);
    format!("{}-{}-{}", &iso[8..10], &iso[5..7], &iso[..4])
}

fn calendar(instructional: impl IntoIterator<Item = i32>, others: &[(i32, &str)]) -> Value {
    let mut entries: Vec<(i32, Value)> = instructional
        .into_iter()
        .map(|day| {
            (
                day,
                json!({"date": iso(day), "kind": "Instructional Day", "group": "General (Semester)", "note": ""}),
            )
        })
        .collect();
    entries.extend(others.iter().map(|&(day, kind)| {
        (
            day,
            json!({"date": iso(day), "kind": kind, "group": "General (Semester)", "note": ""}),
        )
    }));
    entries.sort_by_key(|(day, _)| *day);
    let entries: Vec<Value> = entries.into_iter().map(|(_, entry)| entry).collect();
    json!({"entries": entries, "semester_id": "AP2", "update_time": 1})
}

/// `kind` "ETH" (theory) or "ELA" (lab); no VTOP percentage, so all of `total` counts.
fn course(code: &str, kind: &str, attended: i32, total: i32) -> Value {
    json!({
        "serial": "1", "category": "", "course_name": format!("{code} - {code} name - Embedded"),
        "course_code": "", "course_type": kind, "faculty_detail": "",
        "classes_attended": attended.to_string(), "total_classes": total.to_string(),
        "attendance_percentage": "", "attendence_fat_cat": "-", "debar_status": "-",
        "course_id": format!("AM_{code}"),
    })
}

fn attendance(semester: &str, courses: Vec<Value>) -> Value {
    json!({"records": courses, "semester_id": semester, "update_time": 1})
}

/// Classes as (day, "08:00-08:50", "Present" / "Absent" / "On Duty" / "Not Posted", slot).
fn history(classes: &[(i32, &str, &str, &str)]) -> Value {
    let records: Vec<Value> = classes
        .iter()
        .map(|&(day, time, status, slot)| {
            json!({"serial": "1", "date": dmy(day), "slot": slot, "day_time": format!("MON / {time}"), "status": status, "remark": ""})
        })
        .collect();
    json!({"records": records, "semester_id": "AP2", "update_time": 1, "course_id": "", "course_type": ""})
}

fn general(day: i32, to: i32, status: &str, place: &str) -> Value {
    json!({"serial": "1", "place": place, "purpose": "home", "from_date": iso(day), "from_time": "09:00 AM",
        "to_date": iso(to), "to_time": "06:00 PM", "status": status, "pass_id": "", "cancel_id": ""})
}

fn weekend(day: i32, status: &str, place: &str) -> Value {
    json!({"serial": "1", "hostel_block": "", "room_number": "", "place": place, "purpose": "",
        "time_slot": "", "date": iso(day), "status": status, "pass_id": "", "cancel_id": ""})
}

/// Grades this semester as (code, grade).
fn grades(courses: &[(&str, &str)]) -> Value {
    let courses: Vec<Value> = courses
        .iter()
        .map(|&(code, grade)| {
            json!({"serial": "1", "course_code": code, "course_title": code, "course_type": "Theory Only",
                "grading_type": "RG", "grand_total": "90", "grade": grade, "course_id": format!("AP2_{code}")})
        })
        .collect();
    json!({"courses": courses, "semesters": [], "semester_id": "AP2", "update_time": 1})
}

/// The grade history as (code, credits, grade, exam month).
fn grade_history(courses: &[(&str, &str, &str, &str)], cgpa: &str, earned: &str) -> Value {
    let records: Vec<Value> = courses
        .iter()
        .map(|&(code, credits, grade, month)| {
            json!({"serial": "1", "course_code": code, "course_title": code, "course_type": "TH",
                "credits": credits, "grade": grade, "exam_month": month, "result_declared": "",
                "course_distribution": "PC", "attempts": []})
        })
        .collect();
    let student = json!({"reg_no": "", "name": "", "programme_branch": "", "programme_mode": "",
        "study_system": "", "gender": "", "year_joined": "", "edu_status": "", "school": "", "campus": ""});
    let cgpa = json!({"credits_registered": earned, "credits_earned": earned, "cgpa": cgpa,
        "s_grades": "0", "a_grades": "0", "b_grades": "0", "c_grades": "0", "d_grades": "0",
        "e_grades": "0", "f_grades": "0", "n_grades": "0"});
    json!({"student": student, "records": records, "cgpa": cgpa, "update_time": 1})
}

struct Fixture {
    core: Core,
    dir: PathBuf,
}

impl Fixture {
    /// AP2 (picked) after AP1, with the default calendar: classes on the 60 days from
    /// [`d0`], the FAT ten days after the last.
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-wrapped-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in [
            format!("{PAGES}/AP2/{ATTENDANCE_DETAIL}"),
            format!("{PAGES}/AP1"),
            format!("{PAGES}/biometric"),
        ] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let core = Core::for_test(&dir);
        core.write(
            SEMESTERS,
            &json!({"semesters": [{"id": "AP2", "name": "Fall Semester 2025-26"},
                {"id": "AP1", "name": "Winter Semester 2024-25"}], "update_time": 1}),
        );
        core.write(crate::SELECTED, &"AP2");
        let fixture = Self { core, dir };
        fixture.page(
            CALENDAR,
            &calendar(d0()..d0() + 60, &[(d0() + 70, "Final Assessment Test")]),
        );
        fixture
    }

    fn page(&self, file: &str, value: &Value) {
        self.core.write(&format!("{PAGES}/AP2/{file}"), value);
    }

    fn shared(&self, file: &str, value: &Value) {
        self.core.write(&format!("{PAGES}/{file}"), value);
    }

    fn history(&self, key: &str, classes: &[(i32, &str, &str, &str)]) {
        self.page(
            &format!("{ATTENDANCE_DETAIL}/{key}.json"),
            &history(classes),
        );
    }

    fn report(&self) -> WrappedReport {
        self.core.cached_wrapped("f".into()).expect("a report")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn none_before_classes_end() {
    let fixture = Fixture::new("before");
    let today = records::today_ist_day();
    fixture.page(CALENDAR, &calendar(today - 5..=today + 1, &[]));
    assert_eq!(fixture.core.cached_wrapped("f".into()), None);
    // The day after the last class is still too early.
    fixture.page(CALENDAR, &calendar(today - 5..today, &[]));
    assert_eq!(fixture.core.cached_wrapped("f".into()), None);
    fixture.page(
        CALENDAR,
        &calendar(today - 5..today - 1, &[(today + 3, "CAT - I")]),
    );
    let report = fixture.report();
    assert_eq!(
        (report.first_day, report.last_day, report.window_end),
        (today - 5, today - 2, today + 3)
    );
}

#[test]
fn none_without_instructional_days() {
    let fixture = Fixture::new("no-classes");
    fixture.page(CALENDAR, &calendar([], &[(d0(), "Holiday")]));
    assert_eq!(fixture.core.cached_wrapped("f".into()), None);
    assert!(fixture.core.wrapped_missing_days("f".into()).is_empty());
    let _ = fs::remove_file(fixture.dir.join(format!("{PAGES}/AP2/{CALENDAR}")));
    assert_eq!(fixture.core.cached_wrapped("f".into()), None);
}

#[test]
fn opening_facts_come_from_the_calendar_and_attendance() {
    let fixture = Fixture::new("opening");
    fixture.page(
        CALENDAR,
        &calendar(
            d0()..d0() + 5,
            &[(d0() + 6, "Holiday"), (d0() + 8, "Holiday")],
        ),
    );
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 1, 1),
                course("CSE1001", "ELA", 2, 2),
                course("MAT1001", "ETH", 1, 1),
            ],
        ),
    );
    let report = fixture.report();
    assert_eq!(
        (report.semester_id.as_str(), report.semester_name.as_str()),
        ("AP2", "Fall Semester 2025-26")
    );
    // d0 + 6 is a Sunday: only the weekday holiday counts.
    assert_eq!(
        (report.courses, report.class_day_count, report.holidays),
        (2, 5, 1)
    );
    assert_eq!(report.class_days, (d0()..d0() + 5).collect::<Vec<_>>());
}

#[test]
fn pooled_percent_rounds_up() {
    let fixture = Fixture::new("pooled");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 31, 34),
                course("MAT1001", "ETH", 10, 12),
            ],
        ),
    );
    let facts = fixture.report().attendance.unwrap();
    assert_eq!((facts.percent, facts.attended, facts.counted), (90, 41, 46));
}

#[test]
fn zero_counted_course_skipped_from_best_least() {
    let fixture = Fixture::new("zero");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 30, 40),
                course("PHY1001", "ETH", 0, 0),
                course("MAT1001", "ETH", 20, 20),
            ],
        ),
    );
    let facts = fixture.report().attendance.unwrap();
    assert_eq!((facts.attended, facts.counted), (50, 60));
    assert_eq!(
        (facts.best.as_str(), facts.best_percent),
        ("MAT1001 name", 100)
    );
    assert_eq!(
        (facts.least.as_str(), facts.least_percent),
        ("CSE1001 name", 75)
    );
    // No classes anywhere: no attendance fact, and nothing divides by zero.
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("PHY1001", "ETH", 0, 0)]),
    );
    assert_eq!(fixture.report().attendance, None);
}

#[test]
fn streak_runs_across_courses_in_time_order() {
    let fixture = Fixture::new("streak");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 2, 3),
                course("MAT1001", "ETH", 3, 3),
            ],
        ),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Present", "A1"),
            (d + 1, "08:00-08:50", "Present", "A1"),
            (d + 1, "09:00-09:50", "Present", "A1"),
            (d + 2, "10:00-10:50", "Absent", "A1"),
        ],
    );
    fixture.history(
        "MAT1001.theory",
        &[
            (d, "10:00-10:50", "Present", "B1"),
            (d + 1, "10:00-10:50", "Not Posted", "B1"),
            (d + 2, "08:00-08:50", "Present", "B1"),
            (d + 3, "08:00-08:50", "On Duty", "B1"),
        ],
    );
    let streak = fixture.report().streak.unwrap();
    assert_eq!(
        (streak.length, streak.first_day, streak.last_day),
        (5, d, d + 2)
    );
}

#[test]
fn perfect_weeks_need_a_posted_class_and_no_absence() {
    let fixture = Fixture::new("weeks");
    let d = d0();
    assert_eq!(records::weekday(d), 0);
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 3, 4)]),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            // Week 1: all present.
            (d, "08:00-08:50", "Present", "A1"),
            (d + 1, "08:00-08:50", "Present", "A1"),
            (d + 2, "08:00-08:50", "Present", "A1"),
            (d + 3, "08:00-08:50", "Present", "A1"),
            // Week 2: one absence.
            (d + 7, "08:00-08:50", "Present", "A1"),
            (d + 8, "08:00-08:50", "Absent", "A1"),
            // Week 3: nothing posted.
            (d + 14, "08:00-08:50", "Not Posted", "A1"),
            // Week 4: present on the Saturday.
            (d + 26, "08:00-08:50", "Present", "A1"),
        ],
    );
    assert_eq!(fixture.report().streak.unwrap().perfect_weeks, 2);
}

#[test]
fn never_missed_lists_only_full_courses() {
    let fixture = Fixture::new("never");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 20, 20),
                course("MAT1001", "ETH", 19, 20),
                course("PHY1001", "ETH", 0, 0),
            ],
        ),
    );
    assert_eq!(fixture.report().never_missed, ["CSE1001 name"]);
}

#[test]
fn comeback_needs_twenty_classes_and_five_points() {
    let fixture = Fixture::new("comeback");
    // 6 absences, then 14 present: 70% at the 20th class; then `more` present.
    let classes = |more: i32| -> Vec<(i32, &'static str, &'static str, &'static str)> {
        (0..20 + more)
            .map(|i| {
                let status = if i < 6 { "Absent" } else { "Present" };
                (d0() + i, "08:00-08:50", status, "A1")
            })
            .collect()
    };
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 17, 23)]),
    );
    // 17 of 23 is 74%.
    fixture.history("CSE1001.theory", &classes(3));
    assert_eq!(fixture.report().comeback, None);
    // 69 of 75 is 92%.
    fixture.history("CSE1001.theory", &classes(55));
    let comeback = fixture.report().comeback.unwrap();
    assert_eq!(
        (
            comeback.low_percent,
            comeback.low_day,
            comeback.final_percent
        ),
        (70, d0() + 19, 92)
    );
}

#[test]
fn worst_weekday_and_early_percent() {
    let fixture = Fixture::new("habits");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 2, 5)]),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Absent", "A1"),
            (d + 7, "08:00-08:50", "Absent", "A1"),
            (d + 1, "08:00-08:50", "Present", "A1"),
            (d + 2, "10:00-10:50", "Absent", "A1"),
            (d + 3, "08:30-09:20", "Present", "A1"),
        ],
    );
    let habits = fixture.report().habits.unwrap();
    assert_eq!(
        (
            habits.worst_weekday,
            habits.worst_weekday_misses,
            habits.misses
        ),
        (Some(0), 2, 3)
    );
    assert_eq!(
        (
            habits.early_percent,
            habits.early_classes,
            habits.early_attended
        ),
        (Some(50), 4, 2)
    );
}

#[test]
fn first_and_last_slot_days_follow_the_timetable() {
    let fixture = Fixture::new("slots");
    let d = d0();
    fixture.page(
        TIMETABLE,
        &timetable(&[
            (
                "MON", "A1", "CSE1001", "Theory", "08:00", "08:50", "101", "AB1", "X",
            ),
            (
                "MON", "B1", "CSE1001", "Theory", "18:00", "18:50", "101", "AB1", "X",
            ),
        ]),
    );
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 3, 4)]),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Present", "A1"),
            (d, "18:00-18:50", "Present", "B1"),
            (d + 7, "08:00-08:50", "Absent", "A1"),
            (d + 7, "18:00-18:50", "Present", "B1"),
        ],
    );
    let habits = fixture.report().habits.unwrap();
    assert_eq!((habits.first_slot_days, habits.last_slot_days), (1, 2));
}

/// Slots as (day, slot, code, kind, start, end, room, block, faculty).
#[expect(clippy::type_complexity, reason = "a table of test cases")]
fn timetable(slots: &[(&str, &str, &str, &str, &str, &str, &str, &str, &str)]) -> Value {
    let slots: Vec<Value> = slots
        .iter()
        .map(|&(day, slot, code, kind, start, end, room, block, faculty)| {
            json!({"serial": "1", "day": day, "slot": slot, "course_code": code, "course_type": "",
                "room_no": room, "block": block, "start_time": start, "end_time": end, "name": code,
                "kind": kind, "faculty": faculty, "credits": "3"})
        })
        .collect();
    json!({"slots": slots, "courses": [], "semester_id": "AP2", "update_time": 1})
}

#[test]
fn room_and_faculty_by_attended_classes() {
    let fixture = Fixture::new("places");
    let d = d0();
    fixture.page(
        TIMETABLE,
        &timetable(&[
            (
                "MON",
                "A1",
                "CSE1001",
                "Theory",
                "08:00",
                "08:50",
                "101",
                "AB1",
                "FAC ONE - SCOPE",
            ),
            (
                "TUE",
                "L1",
                "CSE1001",
                "Lab",
                "10:00",
                "10:50",
                "202",
                "AB2",
                "FAC TWO - SCOPE",
            ),
            (
                "TUE",
                "L2",
                "CSE1001",
                "Lab",
                "10:51",
                "11:40",
                "202",
                "AB2",
                "FAC TWO - SCOPE",
            ),
        ]),
    );
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 3, 4),
                course("CSE1001", "ELA", 4, 4),
            ],
        ),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Present", "A1"),
            (d + 7, "08:00-08:50", "Present", "A1"),
            (d + 14, "08:00-08:50", "On Duty", "A1"),
            (d + 21, "08:00-08:50", "Absent", "A1"),
        ],
    );
    fixture.history(
        "CSE1001.lab",
        &[
            (d + 1, "10:00-11:40", "Present", "L1+L2"),
            (d + 8, "10:00-11:40", "Present", "L1+L2"),
        ],
    );
    let places = fixture.report().places.unwrap();
    assert_eq!(
        (places.room.as_deref(), places.room_minutes),
        (Some("AB2-202"), 200)
    );
    // By time, not sessions: two 100-minute labs beat three 50-minute classes.
    assert_eq!(
        (places.faculty.as_deref(), places.faculty_minutes),
        (Some("Fac Two"), 200)
    );
    let time = fixture.report().time.unwrap();
    assert_eq!(
        (
            time.classes,
            time.minutes,
            time.lab_minutes,
            time.busiest_weekday
        ),
        (5, 350, 200, 1)
    );
}

#[test]
fn cat_change_averages_courses_with_both() {
    let fixture = Fixture::new("marks");
    let mark = |title: &str, scored: &str| {
        json!({"serial": "1", "markstitle": title, "maxmarks": "50", "weightage": "15",
            "status": "Present", "scoredmark": scored, "weightagemark": "", "remark": ""})
    };
    let course = |code: &str, marks: Vec<Value>| {
        json!({"serial": "1", "coursecode": code, "coursetitle": format!("{code} name"),
            "coursetype": "Embedded Theory", "faculity": "", "slot": "A1", "marks": marks})
    };
    fixture.page(
        "marks.json",
        &json!({"records": [
            course("CSE1001", vec![mark("Continuous Assessment Test - I", "30"), mark("Continuous Assessment Test - II", "40")]),
            course("MAT1001", vec![mark("CAT1", "25"), mark("CAT2", "30")]),
            course("PHY1001", vec![mark("CAT-1", "45")]),
        ], "semester_id": "AP2", "update_time": 1}),
    );
    let marks = fixture.report().marks.unwrap();
    assert_eq!(marks.cat_change, Some(15));
    assert_eq!(
        marks.most_improved,
        Some(WrappedRise {
            course: "CSE1001 name".into(),
            points: 20
        })
    );
    assert_eq!(
        marks.best_cat,
        Some(WrappedCat {
            course: "PHY1001 name".into(),
            exam: "CAT-I".into(),
            percent: 90
        })
    );
    // No course has half its weightage assessed.
    assert_eq!(marks.best_course, None);
}

#[test]
fn exams_count_papers_and_the_venue_used_most() {
    let fixture = Fixture::new("exams");
    let paper = |code: &str, venue: &str, time: &str| {
        json!({"serial": "1", "slot": "A1", "course_name": code, "course_code": code, "course_type": "ETH",
            "course_id": "", "exam_date": "01-Aug-2025", "exam_session": "FN", "reporting_time": "",
            "exam_time": time, "venue": venue, "seat_location": "", "seat_no": ""})
    };
    let timed = "09:30 AM - 11:00 AM";
    fixture.page(
        "exam_schedule.json",
        &json!({"exams": [
            {"exam_type": "CAT1", "records": [paper("A", "AB1-101", timed), paper("B", "AB1-102", "")]},
            {"exam_type": "CAT2", "records": [paper("A", "AB1-102", timed)]},
            {"exam_type": "FAT", "records": [paper("A", "-", ""), paper("B", "-", "02:00 PM - 05:30 PM")]},
        ], "semester_id": "AP2", "update_time": 1}),
    );
    let exams = fixture.report().exams.unwrap();
    assert_eq!(
        (exams.cats, exams.fats, exams.top_venue.as_deref()),
        (3, 2, Some("AB1-102"))
    );
    // Timed papers by their times; untimed ones 90 minutes for a CAT, 180 for a FAT.
    assert_eq!(exams.minutes, 90 + 90 + 90 + 180 + 210);
}

#[test]
fn grades_only_when_posted() {
    let fixture = Fixture::new("grades");
    // Not in the history yet either.
    fixture.shared(
        "grade_history.json",
        &grade_history(&[("OLD1001", "4", "A", "Apr-2025")], "9", "4"),
    );
    fixture.page("grades.json", &grades(&[("CSE1001", "-"), ("MAT1001", "")]));
    let report = fixture.report();
    assert_eq!((report.grades, report.cgpa), (None, None));
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("CSE1001", "4", "S", "Nov-2025"),
                ("MAT1001", "4", "A", "Nov-2025"),
            ],
            "9.5",
            "8",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("CSE1001", "S"), ("MAT1001", "A")]),
    );
    let grades = fixture.report().grades.unwrap();
    assert_eq!((grades.gpa, grades.s_count), (9.5, 1));
    assert_eq!(
        grades.mix,
        [
            GradeCount {
                grade: "S".into(),
                count: 1
            },
            GradeCount {
                grade: "A".into(),
                count: 1
            }
        ]
    );
}

#[test]
fn cgpa_before_leaves_this_semester_out() {
    let fixture = Fixture::new("cgpa");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("OLD1001", "4", "A", "Apr-2025"),
                ("OLD1002", "3", "B", "Apr-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
            ],
            "8.88",
            "11",
        ),
    );
    fixture.page("grades.json", &grades(&[("CSE1001", "S")]));
    let cgpa = fixture.report().cgpa.unwrap();
    let before = cgpa.before.unwrap();
    assert!((before - 60.0 / 7.0).abs() < 1e-9, "{before}");
    assert_eq!(
        (cgpa.after, cgpa.credits_semester, cgpa.credits_total),
        (8.88, 4.0, 11.0)
    );
}

#[test]
fn s_record_only_when_it_beats_every_earlier_semester() {
    let fixture = Fixture::new("s-record");
    let earlier = [
        ("OLD1001", "3", "S", "Apr-2025"),
        ("OLD1002", "3", "S", "Apr-2025"),
        ("OLD1003", "3", "A", "Apr-2025"),
    ];
    let now = [
        ("CSE1001", "3", "S", "Nov-2025"),
        ("MAT1001", "3", "S", "Nov-2025"),
        ("PHY1001", "3", "S", "Nov-2025"),
    ];
    let history = |count: usize| {
        let courses: Vec<_> = earlier.iter().chain(&now[..count]).copied().collect();
        grade_history(&courses, "9", "18")
    };
    let posted = |count: usize| {
        let courses: Vec<_> = now[..count].iter().map(|c| (c.0, "S")).collect();
        grades(&courses)
    };
    fixture.shared("grade_history.json", &history(2));
    fixture.page("grades.json", &posted(2));
    let grades = fixture.report().grades.unwrap();
    assert_eq!((grades.s_count, grades.s_record), (2, false));
    fixture.shared("grade_history.json", &history(3));
    fixture.page("grades.json", &posted(3));
    let grades = fixture.report().grades.unwrap();
    assert_eq!((grades.s_count, grades.s_record), (3, true));
}

#[test]
fn outings_window_and_status() {
    let fixture = Fixture::new("outings");
    let (d, end) = (d0(), d0() + 70);
    fixture.shared(
        "outing_general.json",
        &json!({"student": null, "notice": "", "records": [
            general(d - 1, d + 1, "Accepted", "Home"),
            general(d + 10, d + 11, "Waiting for approval", "Home"),
            general(d + 20, d + 22, "Accepted", "Vijayawada"),
        ], "out_hours": [], "in_hours": [], "place_max_length": 0, "purpose_max_length": 0,
            "max_days_ahead": 0, "max_days_away": 0, "update_time": 1}),
    );
    fixture.shared(
        "outing_weekend.json",
        &json!({"student": null, "notice": "", "records": [
            weekend(end, "Accepted", "Guntur"),
            weekend(end + 1, "Accepted", "Guntur"),
            weekend(d + 5, "Rejected", "Guntur"),
        ], "places": [], "time_slots": [], "purpose_max_length": 0, "max_days_ahead": 0,
            "weekdays": [], "update_time": 1}),
    );
    let outings = fixture.report().outings.unwrap();
    assert_eq!((outings.leaves, outings.weekends), (1, 1));
    assert_eq!(outings.longest_leave_days, Some(3));
    assert_eq!(outings.days_away, 3);
}

#[test]
fn days_away_clips_to_the_window() {
    let fixture = Fixture::new("away");
    let (d, end) = (d0(), d0() + 70);
    fixture.shared(
        "outing_general.json",
        &json!({"student": null, "notice": "", "records": [
            general(end - 1, end + 5, "Accepted", "Home"),
        ], "out_hours": [], "in_hours": [], "place_max_length": 0, "purpose_max_length": 0,
            "max_days_ahead": 0, "max_days_away": 0, "update_time": 1}),
    );
    let outings = fixture.report().outings.unwrap();
    assert_eq!(
        (outings.days_away, outings.longest_leave_days),
        (2, Some(7))
    );
    // On campus from d to end - 2.
    assert_eq!(outings.longest_stay_days, end - 1 - d);
    assert_eq!(outings.top_place.as_deref(), Some("Home"));
}

/// The title `report`'s traits give, with or without grades.
fn t(report: &WrappedReport, grades: bool) -> WrappedTitle {
    title(&traits(report), grades, true)
}

#[test]
fn title_follows_the_scores() {
    let both = WrappedReport {
        streak: Some(WrappedStreak {
            length: 45,
            total: 200,
            ..Default::default()
        }),
        attendance: Some(WrappedAttendance {
            percent: 95,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(t(&both, true), WrappedTitle::IronStreak);
    let topper = WrappedReport {
        grades: Some(WrappedGrades {
            gpa: 9.2,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(t(&topper, true), WrappedTitle::Topper);
    assert_eq!(t(&topper, false), WrappedTitle::SteadyCruiser);
    // A small comeback no longer beats a long streak; a big one ties it, and wins by order.
    let small = WrappedReport {
        comeback: Some(WrappedComeback {
            low_percent: 70,
            low_day: 0,
            final_percent: 80,
        }),
        ..both.clone()
    };
    assert_eq!(t(&small, true), WrappedTitle::IronStreak);
    let big = WrappedReport {
        comeback: Some(WrappedComeback {
            low_percent: 55,
            low_day: 0,
            final_percent: 90,
        }),
        ..both.clone()
    };
    assert_eq!(t(&big, true), WrappedTitle::Comeback);
    let lab = WrappedReport {
        time: Some(WrappedTime {
            classes: 10,
            minutes: 1000,
            lab_minutes: 400,
            busiest_weekday: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(t(&lab, true), WrappedTitle::LabRat);
}

#[test]
fn previous_semester_change() {
    let fixture = Fixture::new("previous");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 45, 50)]),
    );
    assert_eq!(fixture.report().previous, None);
    fixture.core.write(
        &format!("{PAGES}/AP1/{ATTENDANCE}"),
        &attendance("AP1", vec![course("MAT1001", "ETH", 40, 50)]),
    );
    assert_eq!(
        fixture.report().previous,
        Some(WrappedPrevious {
            semester_name: "Winter Semester 2024-25".into(),
            attendance_change: Some(10),
            gpa_change: None,
        })
    );
    // Lower than last semester: nothing to show.
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 35, 50)]),
    );
    assert_eq!(fixture.report().previous, None);
}

#[test]
fn missing_days_skip_complete_cache() {
    let fixture = Fixture::new("missing");
    let d = d0();
    fixture.page(CALENDAR, &calendar(d..d + 3, &[]));
    let biometric = |day: i32, update_time: i64, punches: &[(&str, &str)]| {
        let records: Vec<Value> = punches
            .iter()
            .map(|&(time, venue)| json!({"serial": "1", "punch_date": "", "punch_time": time, "venue": venue}))
            .collect();
        fixture.shared(
            &format!("biometric/{day}.json"),
            &json!({"records": records, "requested_date": "", "update_time": update_time}),
        );
    };
    // Fetched the next day: complete. Fetched at noon that day: not.
    biometric(
        d,
        i64::from(d + 1) * 86_400,
        &[
            ("07:12:00", "MH2-FACE-OUT-1"),
            ("21:48:00", "MH2-FACE-IN-1"),
            ("08:00:00", "AB1-FACE-OUT-1"),
        ],
    );
    biometric(
        d + 1,
        i64::from(d + 1) * 86_400 - 19_800 + 12 * 3600,
        &[("08:30:00", "MH2-FACE-IN-1"), ("16:00:00", "MH2-FACE-IN-1")],
    );
    assert_eq!(
        fixture.core.wrapped_missing_days("f".into()),
        [d + 1, d + 2]
    );
    // The day fetched at noon is still missing, so it isn't cached either.
    let campus = fixture.report().campus.unwrap();
    assert_eq!(
        (campus.days_cached, campus.days_total, campus.days_on_campus),
        (1, 3, 1)
    );
    assert_eq!(
        (
            campus.earliest_entry.as_deref(),
            campus.latest_return.as_deref()
        ),
        (Some("7:12 AM"), Some("9:48 PM"))
    );
}

/// A calendar whose classes ended two days ago and whose last entry (the FAT) is three weeks
/// ahead, so the outing and exam windows run past today.
fn calendar_ending_after_today(fixture: &Fixture) -> i32 {
    let today = records::today_ist_day();
    fixture.page(
        CALENDAR,
        &calendar(
            today - 40..today - 1,
            &[(today + 20, "Final Assessment Test")],
        ),
    );
    today
}

#[test]
fn exams_count_only_papers_already_sat() {
    let fixture = Fixture::new("exams-ahead");
    let today = calendar_ending_after_today(&fixture);
    let paper = |code: &str, day: i32, venue: &str| {
        json!({"serial": "1", "slot": "A1", "course_name": code, "course_code": code, "course_type": "ETH",
            "course_id": "", "exam_date": dmy(day), "exam_session": "FN", "reporting_time": "",
            "exam_time": "09:30 AM - 11:00 AM", "venue": venue, "seat_location": "", "seat_no": ""})
    };
    fixture.page(
        "exam_schedule.json",
        &json!({"exams": [
            {"exam_type": "CAT1", "records": [paper("A", today - 30, "AB1-101"), paper("B", today - 29, "AB1-101")]},
            {"exam_type": "FAT", "records": [paper("A", today, "AB2-201"), paper("B", today + 3, "AB2-201"),
                paper("C", today + 4, "AB2-201")]},
        ], "semester_id": "AP2", "update_time": 1}),
    );
    let exams = fixture.report().exams.unwrap();
    assert_eq!(
        (exams.cats, exams.fats, exams.top_venue.as_deref()),
        (2, 0, Some("AB1-101"))
    );
    // Nothing sat yet: no slide.
    fixture.page(
        "exam_schedule.json",
        &json!({"exams": [{"exam_type": "FAT", "records": [paper("A", today + 3, "AB2-201")]}],
            "semester_id": "AP2", "update_time": 1}),
    );
    assert_eq!(fixture.report().exams, None);
}

#[test]
fn outings_stop_at_yesterday() {
    let fixture = Fixture::new("outings-ahead");
    let today = calendar_ending_after_today(&fixture);
    fixture.shared(
        "outing_general.json",
        &json!({"student": null, "notice": "", "records": [
            general(today - 10, today - 8, "Accepted", "Home"),
            general(today - 1, today + 6, "Accepted", "Vijayawada"),
            general(today + 5, today + 7, "Accepted", "Guntur"),
        ], "out_hours": [], "in_hours": [], "place_max_length": 0, "purpose_max_length": 0,
            "max_days_ahead": 0, "max_days_away": 0, "update_time": 1}),
    );
    fixture.shared(
        "outing_weekend.json",
        &json!({"student": null, "notice": "", "records": [
            weekend(today - 3, "Accepted", "Guntur"),
            weekend(today, "Accepted", "Guntur"),
            weekend(today + 2, "Accepted", "Guntur"),
        ], "places": [], "time_slots": [], "purpose_max_length": 0, "max_days_ahead": 0,
            "weekdays": [], "update_time": 1}),
    );
    let outings = fixture.report().outings.unwrap();
    assert_eq!((outings.leaves, outings.weekends), (2, 1));
    // Three days home, then yesterday of the leave that runs on.
    assert_eq!(
        (outings.days_away, outings.longest_leave_days),
        (4, Some(3))
    );
    // From the first class day to the day before the first leave.
    assert_eq!(outings.longest_stay_days, 30);
    assert_eq!(outings.top_place.as_deref(), Some("Home"));
}

#[test]
fn outings_window_with_nothing_yet_is_none() {
    let fixture = Fixture::new("outings-none-yet");
    let today = calendar_ending_after_today(&fixture);
    fixture.shared(
        "outing_general.json",
        &json!({"student": null, "notice": "", "records": [
            general(today + 1, today + 3, "Accepted", "Home"),
        ], "out_hours": [], "in_hours": [], "place_max_length": 0, "purpose_max_length": 0,
            "max_days_ahead": 0, "max_days_away": 0, "update_time": 1}),
    );
    assert_eq!(fixture.report().outings, None);
}

#[test]
fn previous_gpa_from_the_grade_history() {
    let fixture = Fixture::new("previous-gpa");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 40, 50)]),
    );
    fixture.core.write(
        &format!("{PAGES}/AP1/{ATTENDANCE}"),
        &attendance("AP1", vec![course("MAT1001", "ETH", 45, 50)]),
    );
    let history = |old: &str| {
        grade_history(
            &[
                ("OLD0001", "4", "S", "Nov-2024"),
                ("OLD1001", "4", old, "Apr-2025"),
                ("OLD1002", "4", "A", "Apr-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
            ],
            "9",
            "16",
        )
    };
    fixture.shared("grade_history.json", &history("B"));
    fixture.page("grades.json", &grades(&[("CSE1001", "S")]));
    // Apr 2025 is 8.5; 10 now. Attendance fell, so only the GPA half.
    assert_eq!(
        fixture.report().previous,
        Some(WrappedPrevious {
            semester_name: "Winter Semester 2024-25".into(),
            attendance_change: None,
            gpa_change: Some(1.5),
        })
    );
    // An old F re-registered now still leaves its session as last semester.
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("CSE1001", "4", "F", "Apr-2025"),
                ("OLD1002", "4", "A", "Apr-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
            ],
            "7",
            "8",
        ),
    );
    assert_eq!(
        fixture.report().previous.and_then(|p| p.gpa_change),
        Some(5.5)
    );
    // Lower now: nothing.
    fixture.shared("grade_history.json", &history("S"));
    fixture.page("grades.json", &grades(&[("CSE1001", "B")]));
    assert_eq!(fixture.report().previous, None);
}

#[test]
fn cgpa_keeps_an_old_attempt_of_a_re_registered_course() {
    let fixture = Fixture::new("cgpa-reregistered");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("MAT1001", "4", "F", "Apr-2025"),
                ("OLD1001", "4", "A", "Apr-2025"),
                ("MAT1001", "4", "B", "Nov-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
            ],
            "7.5",
            "12",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("MAT1001", "B"), ("CSE1001", "S")]),
    );
    let cgpa = fixture.report().cgpa.unwrap();
    // Before: the F and the A, 36 points over 8 credits.
    assert_eq!(cgpa.before, Some(4.5));
    assert_eq!(cgpa.credits_semester, 8.0);
}

#[test]
fn cgpa_and_s_record_need_this_semester_in_the_history() {
    let fixture = Fixture::new("cgpa-not-yet");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("OLD1001", "4", "B", "Apr-2025"),
                // An earlier attempt of a course graded again now isn't this semester.
                ("CSE1001", "4", "F", "Apr-2025"),
            ],
            "4",
            "4",
        ),
    );
    fixture.page("grades.json", &grades(&[("CSE1001", "S")]));
    let report = fixture.report();
    assert_eq!(report.cgpa, None);
    let grades = report.grades.unwrap();
    assert_eq!((grades.s_count, grades.s_record), (1, false));
}

#[test]
fn grades_and_cgpa_from_the_history_without_the_grades_page() {
    let fixture = Fixture::new("grades-backfill");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 20, 20),
                course("MAT1001", "ETH", 20, 20),
            ],
        ),
    );
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("OLD1001", "4", "S", "Apr-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
                ("MAT1001", "4", "B", "Nov-2025"),
            ],
            "9.33",
            "12",
        ),
    );
    let report = fixture.report();
    let grades = report.grades.unwrap();
    assert_eq!((grades.gpa, grades.s_count), (9.0, 1));
    let cgpa = report.cgpa.unwrap();
    assert_eq!((cgpa.before, cgpa.after), (Some(10.0), 9.33));
    assert_eq!((cgpa.credits_semester, cgpa.credits_total), (8.0, 12.0));
}

#[test]
fn cgpa_before_also_when_it_fell() {
    let fixture = Fixture::new("cgpa-drop");
    let history = |cgpa: &str| {
        grade_history(
            &[
                ("OLD1001", "4", "A", "Apr-2025"),
                ("CSE1001", "4", "B", "Nov-2025"),
            ],
            cgpa,
            "8",
        )
    };
    fixture.page("grades.json", &grades(&[("CSE1001", "B")]));
    fixture.shared("grade_history.json", &history("8.5"));
    let cgpa = fixture.report().cgpa.unwrap();
    assert_eq!((cgpa.before, cgpa.after), (Some(9.0), 8.5));
    fixture.shared("grade_history.json", &history("9.2"));
    assert_eq!(fixture.report().cgpa.unwrap().before, Some(9.0));
}

#[test]
fn never_missed_needs_every_part() {
    let fixture = Fixture::new("never-parts");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 20, 20),
                course("CSE1001", "ELA", 18, 20),
                course("MAT1001", "ETH", 20, 20),
                course("MAT1001", "ELA", 20, 20),
            ],
        ),
    );
    assert_eq!(fixture.report().never_missed, ["MAT1001 name"]);
}

#[test]
fn weekday_bars_run_monday_to_sunday() {
    let fixture = Fixture::new("weekday-bars");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 3, 5),
                course("CSE1001", "ELA", 2, 2),
            ],
        ),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Present", "A1"),
            (d + 7, "08:00-08:50", "Absent", "A1"),
            (d + 2, "08:00-08:50", "Absent", "A1"),
            (d + 6, "08:00-08:50", "Present", "A1"),
            (d + 13, "08:00-08:50", "Present", "A1"),
        ],
    );
    fixture.history(
        "CSE1001.lab",
        &[(d + 13, "14:00-15:40", "Present", "L1+L2")],
    );
    let report = fixture.report();
    assert_eq!(
        report.habits.unwrap().misses_by_weekday,
        [1, 0, 1, 0, 0, 0, 0]
    );
    // In VTOP's units: the Sunday lab session is two.
    assert_eq!(
        report.time.unwrap().classes_by_weekday,
        [1, 0, 0, 0, 0, 0, 4]
    );
}

#[test]
fn campus_counts_only_complete_days() {
    let fixture = Fixture::new("campus-complete");
    let d = d0();
    fixture.page(CALENDAR, &calendar(d..d + 2, &[]));
    let biometric = |day: i32, update_time: i64| {
        fixture.shared(
            &format!("biometric/{day}.json"),
            &json!({"records": [{"serial": "1", "punch_date": "", "punch_time": "06:30:00",
                "venue": "MH2-FACE-IN-1"}], "requested_date": "", "update_time": update_time}),
        );
    };
    biometric(d, i64::from(d + 1) * 86_400);
    // Fetched at 06:40 that day: not complete, and still missing.
    biometric(
        d + 1,
        i64::from(d + 1) * 86_400 - 19_800 + 6 * 3600 + 40 * 60,
    );
    assert_eq!(fixture.core.wrapped_missing_days("f".into()), [d + 1]);
    let campus = fixture.report().campus.unwrap();
    assert_eq!(
        (campus.days_cached, campus.days_total, campus.days_on_campus),
        (1, 2, 1)
    );
}

/// A complete cached biometric copy of `day` (fetched the day after) with `punches` entries.
fn punched(fixture: &Fixture, day: i32, punches: usize) {
    let records: Vec<Value> = (0..punches)
        .map(|_| json!({"serial": "1", "punch_date": "", "punch_time": "21:00:00", "venue": "MH2-FACE-IN-1"}))
        .collect();
    fixture.shared(
        &format!("biometric/{day}.json"),
        &json!({"records": records, "requested_date": "", "update_time": i64::from(day + 1) * 86_400}),
    );
}

fn one_leave(fixture: &Fixture, from: i32, to: i32) {
    fixture.shared(
        "outing_general.json",
        &json!({"student": null, "notice": "", "records": [general(from, to, "Accepted", "Home")],
            "out_hours": [], "in_hours": [], "place_max_length": 0, "purpose_max_length": 0,
            "max_days_ahead": 0, "max_days_away": 0, "update_time": 1}),
    );
}

#[test]
fn leave_days_with_a_punch_are_on_campus() {
    let fixture = Fixture::new("leave-biometric");
    let d = d0();
    // A leave for the 10th to the 18th.
    one_leave(&fixture, d + 10, d + 18);
    // No biometric cached: every day of the leave is away, as before.
    let outings = fixture.report().outings.unwrap();
    assert_eq!(
        (outings.days_away, outings.longest_leave_days),
        (9, Some(9))
    );
    assert_eq!(outings.longest_stay_days, 70 - 18);
    // On campus the 11th, 12th, 16th and 17th: the trip home was the 13th to the 15th.
    for day in [11, 12, 16, 17] {
        punched(&fixture, d + day, 1);
    }
    let outings = fixture.report().outings.unwrap();
    assert_eq!(
        (
            outings.leaves,
            outings.days_away,
            outings.longest_leave_days
        ),
        (1, 3, Some(3))
    );
    // From the 16th to the window's end (d + 70).
    assert_eq!(outings.longest_stay_days, 55);
}

#[test]
fn a_cached_day_without_punches_stays_away() {
    let fixture = Fixture::new("leave-no-punch");
    let d = d0();
    one_leave(&fixture, d + 10, d + 14);
    punched(&fixture, d + 10, 1);
    punched(&fixture, d + 12, 0);
    let outings = fixture.report().outings.unwrap();
    assert_eq!(
        (outings.days_away, outings.longest_leave_days),
        (4, Some(4))
    );
}

#[test]
fn cgpa_counts_only_sessions_up_to_this_one() {
    let fixture = Fixture::new("cgpa-cut-off");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("MAT1001", "4", "F", "Apr-2025"),
                ("OLD1001", "4", "A", "Apr-2025"),
                ("MAT1001", "4", "B", "Nov-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
                // A later semester: left out of this one's story.
                ("NEW1001", "4", "E", "Apr-2026"),
            ],
            "8.2",
            "16",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("MAT1001", "B"), ("CSE1001", "S")]),
    );
    let cgpa = fixture.report().cgpa.unwrap();
    // Before: the F and the A. After: the re-registered course's latest grade only, B, with A
    // and S: 108 over 12.
    assert_eq!(cgpa.before, Some(4.5));
    assert_eq!(
        (cgpa.after, cgpa.credits_semester, cgpa.credits_total),
        (9.0, 8.0, 12.0)
    );
}

#[test]
fn first_semester_cgpa_has_no_before() {
    let fixture = Fixture::new("cgpa-first");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("CSE1001", "4", "S", "Nov-2025"),
                ("MAT1001", "4", "A", "Nov-2025"),
                ("NEW1001", "4", "B", "Apr-2026"),
            ],
            "9",
            "12",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("CSE1001", "S"), ("MAT1001", "A")]),
    );
    let cgpa = fixture.report().cgpa.unwrap();
    assert_eq!(
        (cgpa.before, cgpa.after, cgpa.credits_total),
        (None, 9.5, 8.0)
    );
}

#[test]
fn s_record_compares_only_earlier_semesters() {
    let fixture = Fixture::new("s-record-later");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("OLD1001", "3", "S", "Apr-2025"),
                ("CSE1001", "3", "S", "Nov-2025"),
                ("MAT1001", "3", "S", "Nov-2025"),
                ("NEW1001", "3", "S", "Apr-2026"),
                ("NEW1002", "3", "S", "Apr-2026"),
                ("NEW1003", "3", "S", "Apr-2026"),
            ],
            "10",
            "18",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("CSE1001", "S"), ("MAT1001", "S")]),
    );
    assert!(fixture.report().grades.unwrap().s_record);
}

/// Marks as (code, [(title, scored of 50)]).
fn marks_page(courses: &[(&str, &[(&str, &str)])]) -> Value {
    let records: Vec<Value> = courses
        .iter()
        .map(|&(code, marks)| {
            let marks: Vec<Value> = marks
                .iter()
                .map(|&(title, scored)| {
                    json!({"serial": "1", "markstitle": title, "maxmarks": "50", "weightage": "20",
                        "status": "Present", "scoredmark": scored, "weightagemark": "", "remark": ""})
                })
                .collect();
            json!({"serial": "1", "coursecode": code, "coursetitle": format!("{code} name"),
                "coursetype": "Embedded Theory", "faculity": "", "slot": "A1", "marks": marks})
        })
        .collect();
    json!({"records": records, "semester_id": "AP2", "update_time": 1})
}

#[test]
fn a_single_cat_is_named_cat_and_has_no_change() {
    let fixture = Fixture::new("single-cat");
    // A freshers' semester, as VTOP lists it: one mid-term, titled "CAT".
    fixture.page(
        "marks.json",
        &marks_page(&[
            (
                "SWE1004",
                &[("CAT", "33.0"), ("QUIZ1", "13.0"), ("FAT", "48.0")],
            ),
            ("ECE1005", &[("Assignment 1", "10.0"), ("CAT", "45.0")]),
        ]),
    );
    let marks = fixture.report().marks.unwrap();
    assert_eq!(
        marks.best_cat,
        Some(WrappedCat {
            course: "ECE1005 name".into(),
            exam: "CAT".into(),
            percent: 90
        })
    );
    assert_eq!((marks.cat_change, marks.most_improved), (None, None));
    let paper = |code: &str| {
        json!({"serial": "1", "slot": "A1", "course_name": code, "course_code": code, "course_type": "ETH",
            "course_id": "", "exam_date": "11-Nov-2024", "exam_session": "FN1", "reporting_time": "",
            "exam_time": "10:00 AM - 11:30 AM", "venue": "AB1-324", "seat_location": "", "seat_no": ""})
    };
    fixture.page(
        "exam_schedule.json",
        &json!({"exams": [
            {"exam_type": "FAT", "records": [paper("SWE1004")]},
            {"exam_type": "CAT", "records": [paper("SWE1004"), paper("ECE1005")]},
        ], "semester_id": "AP2", "update_time": 1}),
    );
    let exams = fixture.report().exams.unwrap();
    assert_eq!((exams.cats, exams.fats), (2, 1));
}

#[test]
fn cat_titles_in_their_variants() {
    for (title, exam) in [
        ("CAT", Some("CAT")),
        ("cat ", Some("CAT")),
        ("CAT1", Some("CAT-I")),
        ("CAT-1", Some("CAT-I")),
        ("CAT - 1", Some("CAT-I")),
        ("CAT I", Some("CAT-I")),
        ("CAT-II", Some("CAT-II")),
        ("CAT 2 (Theory)", Some("CAT-II")),
        ("Continuous Assessment Test - I", Some("CAT-I")),
        ("Continuous Assessment Test \u{2013} II", Some("CAT-II")),
        ("Continuous Assessment Test", Some("CAT")),
        ("Category test", None),
        ("QUIZ1", None),
        ("Digital Assessment-1", None),
    ] {
        assert_eq!(cat_of(title).as_deref(), exam, "{title}");
    }
}

#[test]
fn top_courses_by_percent_then_attended() {
    let fixture = Fixture::new("top-courses");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 18, 20),
                course("MAT1001", "ETH", 10, 10),
                course("PHY1001", "ETH", 0, 0),
                course("CHY1001", "ETH", 30, 30),
                course("CHY1001", "ELA", 20, 20),
                course("ENG1001", "ETH", 15, 20),
                course("ECE1001", "ETH", 27, 30),
            ],
        ),
    );
    let facts = fixture.report().attendance.unwrap();
    let top: Vec<(&str, i32)> = facts
        .top
        .iter()
        .map(|c| (c.name.as_str(), c.percent))
        .collect();
    // 100% twice, the one with more classes first, its lab not again; then 90% with more classes.
    assert_eq!(
        top,
        [
            ("CHY1001 name", 100),
            ("MAT1001 name", 100),
            ("ECE1001 name", 90)
        ]
    );
    assert_eq!(
        (facts.best.as_str(), facts.best_percent),
        ("CHY1001 name", 100)
    );
    assert_eq!(
        (facts.least.as_str(), facts.least_percent),
        ("ENG1001 name", 75)
    );
}

#[test]
fn top_courses_leave_out_the_least() {
    let fixture = Fixture::new("top-two");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 18, 20),
                course("MAT1001", "ETH", 10, 10),
            ],
        ),
    );
    let facts = fixture.report().attendance.unwrap();
    assert_eq!(facts.top.len(), 1);
    assert_eq!(facts.least, "CSE1001 name");
}

#[test]
fn streak_sits_among_all_posted_classes() {
    let fixture = Fixture::new("streak-total");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 4, 6)]),
    );
    fixture.history(
        "CSE1001.theory",
        &[
            (d, "08:00-08:50", "Absent", "A1"),
            (d + 1, "08:00-08:50", "Present", "A1"),
            (d + 2, "08:00-08:50", "Not Posted", "A1"),
            (d + 3, "08:00-08:50", "Present", "A1"),
            (d + 4, "08:00-08:50", "On Duty", "A1"),
            (d + 4, "10:00-10:50", "Present", "A1"),
            (d + 4, "11:00-11:50", "Present", "A1"),
            (d + 5, "08:00-08:50", "Absent", "A1"),
            (d + 6, "08:00-08:50", "Present", "A1"),
        ],
    );
    let streak = fixture.report().streak.unwrap();
    assert_eq!((streak.length, streak.offset, streak.total), (5, 1, 8));
    assert_eq!((streak.first_day, streak.last_day), (d + 1, d + 4));
}

#[test]
fn streak_needs_five_in_a_row() {
    let fixture = Fixture::new("streak-short");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 4, 5)]),
    );
    let mut classes: Vec<(i32, &str, &str, &str)> = (0..4)
        .map(|n| (d + n, "08:00-08:50", "Present", "A1"))
        .collect();
    classes.push((d + 4, "08:00-08:50", "Absent", "A1"));
    fixture.history("CSE1001.theory", &classes);
    assert_eq!(fixture.report().streak, None);
    classes.push((d + 5, "08:00-08:50", "Present", "A1"));
    classes.push((d + 6, "08:00-08:50", "Present", "A1"));
    classes.push((d + 7, "08:00-08:50", "Present", "A1"));
    classes.push((d + 8, "08:00-08:50", "Present", "A1"));
    classes.push((d + 9, "08:00-08:50", "Present", "A1"));
    fixture.history("CSE1001.theory", &classes);
    assert_eq!(fixture.report().streak.unwrap().length, 5);
}

#[test]
fn iron_streak_is_relative_to_all_classes() {
    let streak = |length, total| WrappedReport {
        streak: Some(WrappedStreak {
            length,
            total,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(t(&streak(59, 336), true), WrappedTitle::IronStreak);
    assert_eq!(t(&streak(45, 400), true), WrappedTitle::SteadyCruiser);
    // A large share of a short semester still needs 15 classes.
    assert_eq!(t(&streak(14, 50), true), WrappedTitle::SteadyCruiser);
    assert_eq!(t(&streak(15, 100), true), WrappedTitle::IronStreak);
}

#[test]
fn summer_semester_matches_its_same_month_session() {
    let fixture = Fixture::new("summer");
    // Classes from 2 to 26 June 2026, FATs the same month.
    let start = records::epoch_day("2026-06-02").unwrap();
    fixture.page(
        CALENDAR,
        &calendar(start..start + 25, &[(start + 27, "Final Assessment Test")]),
    );
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("OLD1001", "4", "B", "May-2026"),
                ("CSE1001", "4", "S", "Jun-2026"),
            ],
            "9",
            "8",
        ),
    );
    fixture.page("grades.json", &grades(&[("CSE1001", "S")]));
    let cgpa = fixture.report().cgpa.unwrap();
    assert_eq!((cgpa.before, cgpa.after), (Some(8.0), 9.0));
}

#[test]
fn previous_gpa_change_needs_a_hundredth() {
    assert_eq!(previous("W".into(), None, Some((9.5, 9.497))), None);
    let change = previous("W".into(), None, Some((9.5, 9.25)))
        .and_then(|p| p.gpa_change)
        .unwrap();
    assert!((change - 0.25).abs() < 1e-9, "{change}");
}

/// One class a day from [`d0`], as (count, status) runs in order.
fn runs(
    runs: &[(i32, &'static str)],
    slot: &'static str,
) -> Vec<(i32, &'static str, &'static str, &'static str)> {
    runs.iter()
        .flat_map(|&(n, status)| (0..n).map(move |_| status))
        .enumerate()
        .map(|(i, status)| (d0() + i as i32, "08:00-08:50", status, slot))
        .collect()
}

#[test]
fn close_call_finds_the_tightest_moment() {
    let fixture = Fixture::new("close-call");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 20, 24)]),
    );
    fixture.history(
        "CSE1001.theory",
        &runs(&[(12, "Present"), (4, "Absent"), (8, "Present")], "A1"),
    );
    let call = fixture.report().close_call.unwrap();
    // 12 of 16 is 75% with none to spare; 20 of 24 is 83.3%, 84 rounded up.
    assert_eq!(
        (
            call.course.as_str(),
            call.day,
            call.spare,
            call.percent,
            call.final_percent
        ),
        ("CSE1001 name", d0() + 15, Some(0), 75, 84)
    );
}

#[test]
fn close_call_skipped_with_room_to_spare() {
    let fixture = Fixture::new("close-call-room");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 30, 30)]),
    );
    fixture.history("CSE1001.theory", &runs(&[(30, "Present")], "A1"));
    assert_eq!(fixture.report().close_call, None);
}

#[test]
fn close_call_below_75_has_no_spare() {
    let fixture = Fixture::new("close-call-below");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 8, 14)]),
    );
    fixture.history(
        "CSE1001.theory",
        &runs(&[(6, "Present"), (6, "Absent"), (2, "Present")], "A1"),
    );
    let call = fixture.report().close_call.unwrap();
    // Under 75% from the 10th class on and never back: the lowest, 6 of 12.
    assert_eq!(
        (call.day, call.spare, call.percent, call.final_percent),
        (d0() + 11, None, 50, 58)
    );
}

#[test]
fn close_call_counts_a_lab_as_two() {
    let fixture = Fixture::new("close-call-lab");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ELA", 22, 24)]),
    );
    fixture.history(
        "CSE1001.lab",
        &runs(&[(5, "Present"), (1, "Absent"), (6, "Present")], "L1+L2"),
    );
    let call = fixture.report().close_call.unwrap();
    // 10 of 12 at the 6th session; one more missed session is 10 of 14, 72%.
    assert_eq!(
        (call.day, call.spare, call.percent, call.final_percent),
        (d0() + 5, Some(0), 84, 92)
    );
}

/// An exam schedule of one CAT with a paper on each of `days` (none: not dated yet).
fn exam_days(days: &[Option<i32>]) -> Value {
    let records: Vec<Value> = days
        .iter()
        .enumerate()
        .map(|(i, day)| {
            json!({"serial": "1", "slot": "A1", "course_name": format!("C{i}"), "course_code": format!("C{i}"),
                "course_type": "ETH", "course_id": "", "exam_date": day.map(dmy).unwrap_or_default(),
                "exam_session": "FN", "reporting_time": "", "exam_time": "09:30 AM - 11:00 AM",
                "venue": "AB1-101", "seat_location": "", "seat_no": ""})
        })
        .collect();
    json!({"exams": [{"exam_type": "CAT1", "records": records}], "semester_id": "AP2", "update_time": 1})
}

#[test]
fn crunch_finds_the_busiest_week() {
    let fixture = Fixture::new("crunch");
    let d = calendar_ending_after_today(&fixture) - 30;
    fixture.page(
        "exam_schedule.json",
        &exam_days(&[Some(d), Some(d + 2), Some(d + 2), Some(d + 5), Some(d + 20)]),
    );
    let c = fixture.report().crunch.unwrap();
    assert_eq!(
        (
            c.papers,
            c.first_day,
            c.last_day,
            c.most_in_a_day,
            c.most_day,
            c.exam_days
        ),
        (4, d, d + 6, 2, d + 2, 4)
    );
    assert_eq!(c.by_day, [1, 0, 2, 0, 0, 1, 0]);
}

#[test]
fn crunch_needs_three_papers() {
    let fixture = Fixture::new("crunch-few");
    let d = calendar_ending_after_today(&fixture) - 30;
    fixture.page("exam_schedule.json", &exam_days(&[Some(d), Some(d + 1)]));
    assert_eq!(fixture.report().crunch, None);
}

#[test]
fn crunch_skips_undated_and_future_papers() {
    let fixture = Fixture::new("crunch-ahead");
    let today = calendar_ending_after_today(&fixture);
    fixture.page(
        "exam_schedule.json",
        &exam_days(&[Some(today - 3), Some(today - 2), None, Some(today)]),
    );
    assert_eq!(fixture.report().crunch, None);
}

/// A complete cached biometric copy of `day` with `punches` as (time, venue), in any order.
fn punches(fixture: &Fixture, day: i32, punches: &[(&str, &str)]) {
    let records: Vec<Value> = punches
        .iter()
        .map(|&(time, venue)| json!({"serial": "1", "punch_date": "", "punch_time": time, "venue": venue}))
        .collect();
    fixture.shared(
        &format!("biometric/{day}.json"),
        &json!({"records": records, "requested_date": "", "update_time": i64::from(day + 1) * 86_400}),
    );
}

/// A hosteller's class day, as live VTOP sent it on 15 Oct 2026.
const HOSTEL_DAY: [(&str, &str); 9] = [
    ("06:27:00", "MH2-FACE-OUT-3"),
    ("07:27:00", "MH2-FACE-IN-4"),
    ("09:55:00", "MH2-FACE-OUT-3"),
    ("10:48:00", "CB-214-FACE"),
    ("10:58:00", "MH2-FACE-IN-4"),
    ("13:37:00", "MH2-FACE-OUT-3"),
    ("14:14:00", "MH2-FACE-IN-4"),
    ("17:37:00", "MH2-FACE-OUT-3"),
    ("19:1", "MH2-FACE-IN-4"),
];

#[test]
fn class_punch_reads_block_room_floor() {
    let read = |venue: &str| class_punch(venue).map(|p| (p.block, p.room, p.floor));
    assert_eq!(read("CB-214-FACE"), Some(("CB".into(), "214".into(), 2)));
    assert_eq!(read("AB1-G05-FACE"), Some(("AB1".into(), "G05".into(), 0)));
    assert_eq!(read("AB1-GF-(101)"), Some(("AB1".into(), "101".into(), 0)));
    assert_eq!(read("AB2-105-FACE").map(|p| p.2), Some(1));
    // Gates, also in July 2026's form with the gate's number in brackets.
    for venue in [
        "MH2-FACE-IN-7",
        "MH2-IN-3-(358)",
        "MH2-OUT-2-(342)",
        "AB1-FACE-OUT-1",
        "LIB-FACE",
        "Enrollment-Device-1",
        "",
    ] {
        assert_eq!(read(venue), None, "{venue}");
    }
}

#[test]
fn hostel_out_time_and_naps() {
    let fixture = Fixture::new("hostel");
    let d = d0();
    // Mon to Thu, and the next Tuesday.
    for day in [d, d + 1, d + 2, d + 3, d + 8] {
        let mut day_punches = HOSTEL_DAY.to_vec();
        // A class after the 13:37 OUT: the IN at 10:58 and that OUT were a nap.
        if day == d + 1 || day == d + 8 {
            day_punches.push(("14:30:00", "CB-214-FACE"));
        }
        punches(&fixture, day, &day_punches);
    }
    let h = fixture.report().hostel.unwrap();
    // 1:00 + 1:03 + 0:37 + 1:24 = 4:04 a day.
    assert_eq!(
        (h.days, h.out_tenths, h.naps, h.nap_weekday),
        (5, 41, 2, Some(1))
    );
    assert_eq!(h.out_by_half_hour.len(), 36);
    assert!(h.out_by_half_hour[8] && h.out_by_half_hour[1]);
    assert!(!h.out_by_half_hour[0] && !h.out_by_half_hour[30]);
}

#[test]
fn hostel_none_for_a_day_scholar() {
    let fixture = Fixture::new("hostel-day-scholar");
    for day in d0()..d0() + 5 {
        punches(
            &fixture,
            day,
            &[("09:00:00", "CB-214-FACE"), ("11:00:00", "AB1-G05-FACE")],
        );
    }
    let report = fixture.report();
    assert_eq!(report.hostel, None);
    assert!(report.moves.is_some());
}

#[test]
fn hostel_ignores_an_in_before_any_out() {
    let fixture = Fixture::new("hostel-in-first");
    for day in d0()..d0() + 5 {
        punches(
            &fixture,
            day,
            &[
                ("07:00:00", "MH2-FACE-IN-4"),
                ("08:00:00", "MH2-FACE-OUT-3"),
                ("09:00:00", "MH2-FACE-IN-4"),
                ("09:30:00", "LIB-FACE"),
            ],
        );
    }
    let h = fixture.report().hostel.unwrap();
    assert_eq!((h.days, h.out_tenths, h.naps), (5, 10, 0));
}

#[test]
fn moves_counts_floors_rooms_hops() {
    let fixture = Fixture::new("moves");
    let d = d0();
    // Out of time order, as VTOP may list them.
    punches(
        &fixture,
        d,
        &[
            ("12:00:00", "AB1-G05-FACE"),
            ("09:00:00", "CB-214-FACE"),
            ("08:00:00", "MH2-FACE-OUT-3"),
            ("14:00:00", "AB1-301-FACE"),
            ("10:00:00", "CB-105-FACE"),
        ],
    );
    for day in d + 1..d + 5 {
        punches(&fixture, day, &[("09:00:00", "CB-214-FACE")]);
    }
    let m = fixture.report().moves.unwrap();
    // 2 + 0 + 0 + 3 on the first day, 2 on each of the others; one hop, CB to AB1.
    assert_eq!(
        (m.floors, m.rooms, m.most_hops, m.most_hops_day),
        (13, 4, 1, d)
    );
    let blocks: Vec<(&str, i32)> = m
        .blocks
        .iter()
        .map(|b| (b.block.as_str(), b.rooms))
        .collect();
    assert_eq!(blocks, [("AB1", 2), ("CB", 2)]);
}

#[test]
fn moves_none_under_five_days() {
    let fixture = Fixture::new("moves-few");
    for day in d0()..d0() + 4 {
        punches(&fixture, day, &[("09:00:00", "CB-214-FACE")]);
    }
    punches(&fixture, d0() + 4, &[("09:00:00", "MH2-FACE-IN-4")]);
    assert_eq!(fixture.report().moves, None);
}

#[test]
fn on_duty_counts_classes_days_and_top_course() {
    let fixture = Fixture::new("on-duty");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 13, 15),
                course("MAT1001", "ETH", 7, 8),
            ],
        ),
    );
    let classes = |od: i32, from: i32, present: i32, absent: i32| {
        let mut classes = Vec::new();
        for (n, status) in [(od, "On Duty"), (present, "Present"), (absent, "Absent")] {
            for _ in 0..n {
                let day = d0() + from + classes.len() as i32;
                classes.push((day, "08:00-08:50", status, "A1"));
            }
        }
        classes
    };
    // CSE1001 on duty on days 0 to 7, MAT1001 on days 5 to 8: 9 days.
    fixture.history("CSE1001.theory", &classes(8, 0, 5, 2));
    fixture.history("MAT1001.theory", &classes(4, 5, 3, 1));
    let od = fixture.report().on_duty.unwrap();
    assert_eq!((od.classes, od.days, od.present, od.absent), (12, 9, 8, 3));
    assert_eq!(
        od.top,
        [WrappedCourse {
            name: "CSE1001 name".into(),
            percent: 8
        }]
    );
    // A tie names both.
    fixture.history("MAT1001.theory", &classes(8, 5, 0, 0));
    let names: Vec<(String, i32)> = fixture
        .report()
        .on_duty
        .unwrap()
        .top
        .into_iter()
        .map(|c| (c.name, c.percent))
        .collect();
    assert_eq!(
        names,
        [("CSE1001 name".into(), 8), ("MAT1001 name".into(), 8)]
    );
}

#[test]
fn on_duty_needs_ten() {
    let fixture = Fixture::new("on-duty-few");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 12, 12)]),
    );
    fixture.history(
        "CSE1001.theory",
        &runs(&[(9, "On Duty"), (3, "Present")], "A1"),
    );
    assert_eq!(fixture.report().on_duty, None);
}

/// Present classes of one course, as (count, "08:00-08:50") runs.
fn day_half_fixture(name: &str, runs: &[(i32, &'static str)]) -> Fixture {
    let fixture = Fixture::new(name);
    let total = runs.iter().map(|r| r.0).sum();
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", total, total)]),
    );
    let classes: Vec<_> = runs
        .iter()
        .flat_map(|&(n, time)| (0..n).map(move |_| time))
        .enumerate()
        .map(|(i, time)| (d0() + i as i32, time, "Present", "A1"))
        .collect();
    fixture.history("CSE1001.theory", &classes);
    fixture
}

#[test]
fn day_half_afternoon() {
    let fixture = day_half_fixture("day-half", &[(3, "08:00-08:50"), (7, "14:00-14:50")]);
    let half = fixture.report().day_half.unwrap();
    assert_eq!(
        (half.morning, half.percent, half.classes, half.total),
        (false, 70, 7, 10)
    );
    assert_eq!(
        (half.by_hour.len(), half.by_hour[0], half.by_hour[6]),
        (11, 3, 7)
    );
}

#[test]
fn day_half_skipped_when_mixed() {
    let fixture = day_half_fixture("day-half-mixed", &[(5, "08:00-08:50"), (5, "14:00-14:50")]);
    assert_eq!(fixture.report().day_half, None);
}

#[test]
fn assessment_kinds_read_vtop_titles() {
    for (title, kind) in [
        ("QUIZ1", "quizzes"),
        ("CAT1", "CATs"),
        ("CAT2", "CATs"),
        ("FAT", "FATs"),
        ("Experiment-3", "lab experiments"),
        ("Exp1+2", "lab experiments"),
        ("Exp1+10", "lab experiments"),
        ("Final LAB Report", "lab experiments"),
        ("Digital Assessment-2", "digital assessments"),
        ("DA-1", "digital assessments"),
        ("First Review", "reviews and reports"),
        ("Final Review", "reviews and reports"),
        ("Project Report", "reviews and reports"),
        ("ASSIGNMENT", "assignments"),
        ("Assignment", "assignments"),
        ("Viva", "other"),
    ] {
        assert_eq!(assessment_kind(title), kind, "{title}");
    }
}

/// Two courses: CSE1001 with 2 of 3 posted at full marks (and a re-evaluation, left out),
/// MAT1001 with 1 of 1 and one not posted.
fn full_marks_fixture(name: &str) -> Fixture {
    let fixture = Fixture::new(name);
    fixture.page(
        "marks.json",
        &marks_page(&[
            (
                "CSE1001",
                &[
                    ("QUIZ1", "50"),
                    ("CAT1", "50"),
                    ("CAT1 Re-evaluation", "50"),
                    ("Assignment", "30"),
                ],
            ),
            ("MAT1001", &[("QUIZ1", "50"), ("CAT2", "")]),
        ]),
    );
    fixture
}

fn kinds(by_kind: &[WrappedKindCount]) -> Vec<(&str, i32)> {
    by_kind.iter().map(|k| (k.kind.as_str(), k.count)).collect()
}

#[test]
fn full_marks_counts_max_scores() {
    let fixture = full_marks_fixture("full-marks");
    let full = fixture.report().full_marks.unwrap();
    assert_eq!((full.count, full.of), (3, 4));
    assert_eq!(kinds(&full.by_kind), [("quizzes", 2), ("CATs", 1)]);
    assert_eq!(
        (full.top_course.as_deref(), full.top_count),
        (Some("CSE1001 name"), 2)
    );
    // None at full marks.
    fixture.page(
        "marks.json",
        &marks_page(&[("CSE1001", &[("QUIZ1", "20")])]),
    );
    assert_eq!(fixture.report().full_marks, None);
}

#[test]
fn assessments_group_by_kind() {
    let fixture = full_marks_fixture("assessments");
    let a = fixture.report().assessments.unwrap();
    // Ties in the kinds' order.
    assert_eq!(
        (a.total, kinds(&a.by_kind)),
        (4, vec![("quizzes", 2), ("CATs", 1), ("assignments", 1)])
    );
    fixture.page("marks.json", &marks_page(&[("CSE1001", &[("QUIZ1", "")])]));
    assert_eq!(fixture.report().assessments, None);
}

#[test]
fn most_improved_shows_even_when_the_average_fell() {
    let fixture = Fixture::new("most-improved");
    fixture.page(
        "marks.json",
        &marks_page(&[
            ("CSE1001", &[("CAT1", "20"), ("CAT2", "35")]),
            ("MAT1001", &[("CAT1", "45"), ("CAT2", "20")]),
            ("PHY1001", &[("CAT1", "40"), ("CAT2", "30")]),
        ]),
    );
    let marks = fixture.report().marks.unwrap();
    // +30, -50 and -20 points: -13 on average.
    assert_eq!(marks.cat_change, Some(-13));
    assert_eq!(
        marks.most_improved,
        Some(WrappedRise {
            course: "CSE1001 name".into(),
            points: 30
        })
    );
    fixture.page(
        "marks.json",
        &marks_page(&[("CSE1001", &[("CAT1", "40"), ("CAT2", "30")])]),
    );
    assert_eq!(fixture.report().marks.unwrap().most_improved, None);
}

#[test]
fn hosteller_day_starts_at_the_hostel_out() {
    let fixture = Fixture::new("campus-hostel");
    let d = d0();
    fixture.page(CALENDAR, &calendar(d..d + 1, &[]));
    punches(&fixture, d, &HOSTEL_DAY);
    let campus = fixture.report().campus.unwrap();
    // Not the first IN back to the hostel (7:27 AM).
    assert_eq!(campus.earliest_entry.as_deref(), Some("6:27 AM"));
}

/// A calendar entry with a note, as VTOP names holidays.
fn named(day: i32, kind: &str, note: &str) -> Value {
    json!({"date": iso(day), "kind": kind, "group": "General (Semester)", "note": note})
}

#[test]
fn mosaic_marks_each_day_and_ranks_months() {
    let fixture = Fixture::new("mosaic");
    let d = d0();
    let mut cal = calendar(d..d + 60, &[(d + 70, "Final Assessment Test")]);
    cal["entries"]
        .as_array_mut()
        .unwrap()
        .push(named(d + 65, "Holiday", "Independence Day"));
    fixture.page(CALENDAR, &cal);
    let mut classes = vec![
        (d, "08:00-08:50", "Present", "A1"),
        (d + 1, "08:00-08:50", "Absent", "A1"),
        (d + 2, "08:00-08:50", "On Duty", "A1"),
        (d + 3, "08:00-08:50", "Present", "A1"),
        (d + 3, "09:00-09:50", "Absent", "A1"),
    ];
    // 14 days present in July (14 to 27 Jul), then August: 10 present, 3 absent.
    classes.extend((7..21).map(|i| (d + i, "08:00-08:50", "Present", "A1")));
    classes.extend((28..41).map(|i| {
        let status = if i < 31 { "Absent" } else { "Present" };
        (d + i, "08:00-08:50", status, "A1")
    }));
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 27, 32)]),
    );
    fixture.history("CSE1001.theory", &classes);
    let m = fixture.report().mosaic.unwrap();
    // 7 Jul is a Monday; the window ends Monday 15 Sep, so the grid runs to Sunday 21 Sep.
    assert_eq!((m.first_day, m.days.len(), m.class_days), (d, 77, 31));
    use WrappedDay::*;
    assert_eq!(
        (m.days[0], m.days[1], m.days[2], m.days[3], m.days[4]),
        (AllPresent, AllAbsent, OnDuty, Mixed, NoClass)
    );
    assert_eq!(
        (m.days[65], m.days[66], m.days[70], m.days[71]),
        (Holiday, NoClass, Exam, Outside)
    );
    let months: Vec<(i32, i32, Option<i32>)> = m
        .months
        .iter()
        .map(|m| (m.month, m.counted, m.percent))
        .collect();
    // July 17 of 19 (89.5, 90 rounded up), August 10 of 13 (77), September none.
    assert_eq!(months, [(7, 19, Some(90)), (8, 13, Some(77)), (9, 0, None)]);
    assert_eq!((m.months[0].first_day, m.months[2].last_day), (d, d + 70));
    assert_eq!((m.best_month, m.worst_month), (Some(0), Some(1)));
}

#[test]
fn mosaic_needs_fifteen_classes() {
    let fixture = Fixture::new("mosaic-few");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 14, 14)]),
    );
    fixture.history("CSE1001.theory", &runs(&[(14, "Present")], "A1"));
    let report = fixture.report();
    assert_eq!((report.mosaic, report.arc), (None, None));
}

#[test]
fn arc_follows_the_line_between_the_cats() {
    let fixture = Fixture::new("arc");
    let d = d0();
    fixture.page(
        CALENDAR,
        &calendar(
            (d..d + 60).filter(|&day| day != d + 15 && day != d + 30),
            &[
                (d + 15, "CAT - I"),
                (d + 30, "CAT - II"),
                (d + 70, "Final Assessment Test"),
            ],
        ),
    );
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 37, 45)]),
    );
    fixture.history(
        "CSE1001.theory",
        &runs(&[(15, "Present"), (8, "Absent"), (22, "Present")], "A1"),
    );
    let arc = fixture.report().arc.unwrap();
    let phases: Vec<(Option<&str>, Option<&str>, i32, bool)> = arc
        .phases
        .iter()
        .map(|p| (p.after.as_deref(), p.before.as_deref(), p.percent, p.lowest))
        .collect();
    // 15 of 15, then 7 of 15 (46.7, 47 rounded up), then 15 of 15.
    assert_eq!(
        phases,
        [
            (None, Some("CAT-I"), 100, false),
            (Some("CAT-I"), Some("CAT-II"), 47, true),
            (Some("CAT-II"), None, 100, false),
        ]
    );
    let exams: Vec<(&str, i32, i32)> = arc
        .exams
        .iter()
        .map(|e| (e.name.as_str(), e.first_day, e.last_day))
        .collect();
    assert_eq!(
        exams,
        [
            ("CAT-I", d + 15, d + 15),
            ("CAT-II", d + 30, d + 30),
            ("FAT", d + 70, d + 70)
        ]
    );
    assert_eq!(arc.shape, WrappedArcShape::Comeback);
    assert_eq!((arc.peak.day, arc.low.day), (d + 9, d + 22));
    // From the 10th class (100%) down to 15 of 23 (65.2%).
    let drop = arc.drop.unwrap();
    assert_eq!((drop.from.day, drop.to.day), (d + 9, d + 22));
    assert!((drop.to.percent - 1500.0 / 23.0).abs() < 1e-9);
    // 36 days from the 10th class, thinned to 20 with the peak and low kept.
    assert!(arc.points.len() <= ARC_POINTS + 2, "{}", arc.points.len());
    assert_eq!(arc.points.first().map(|p| p.day), Some(d + 9));
    assert_eq!(arc.points.last().map(|p| p.day), Some(d + 44));
    assert!(arc.points.iter().any(|p| p.day == d + 22));
    assert!(arc.points.windows(2).all(|w| w[0].day < w[1].day));
}

#[test]
fn arc_without_cats_has_no_phases() {
    let fixture = Fixture::new("arc-no-cats");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 25, 30)]),
    );
    fixture.history(
        "CSE1001.theory",
        &runs(&[(20, "Present"), (5, "Absent"), (5, "Present")], "A1"),
    );
    let arc = fixture.report().arc.unwrap();
    assert!(arc.phases.is_empty());
    // 100% down to 80% (20 of 25), ending at 83.3%: a slide, not a comeback.
    assert_eq!(arc.shape, WrappedArcShape::Slide);
}

#[test]
fn quietest_weekday_by_classes_a_day() {
    let fixture = Fixture::new("quietest");
    let d = d0();
    let mut classes = Vec::new();
    for week in 0..3 {
        let monday = d + 7 * week;
        for time in ["08:00-08:50", "09:00-09:50", "10:00-10:50"] {
            classes.push((monday, time, "Present", "A1"));
        }
        classes.push((monday + 1, "08:00-08:50", "Present", "A1"));
    }
    // Two Wednesdays only: left out.
    classes.push((d + 2, "08:00-08:50", "Present", "A1"));
    classes.push((d + 9, "08:00-08:50", "Absent", "A1"));
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 13, 14)]),
    );
    fixture.history("CSE1001.theory", &classes);
    let h = fixture.report().habits.unwrap();
    assert_eq!(h.classes_per_day_tenths, [30, 10, 0, 0, 0, 0, 0]);
    assert_eq!(h.quietest_weekday, Some(1));
}

/// Grades this semester as (code, grand total, grade).
fn totals(courses: &[(&str, &str, &str)]) -> Value {
    let courses: Vec<Value> = courses
        .iter()
        .map(|&(code, total, grade)| {
            json!({"serial": "1", "course_code": code, "course_title": code, "course_type": "Theory Only",
                "grading_type": "RG", "grand_total": total, "grade": grade, "course_id": format!("AP2_{code}")})
        })
        .collect();
    json!({"courses": courses, "semesters": [], "semester_id": "AP2", "update_time": 1})
}

#[test]
fn roi_pools_parts_and_picks_best_and_grind() {
    let fixture = Fixture::new("roi");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 20, 25),
                course("CSE1001", "ELA", 10, 15),
                course("MAT1001", "ETH", 38, 40),
                course("PHY1001", "ETH", 36, 40),
            ],
        ),
    );
    fixture.page(
        "grades.json",
        &totals(&[
            ("CSE1001", "88", "A"),
            ("MAT1001", "58", "C"),
            ("PHY1001", "80", "A"),
        ]),
    );
    let roi = fixture.report().roi.unwrap();
    let points: Vec<(&str, i32, i32)> = roi
        .points
        .iter()
        .map(|p| (p.course.as_str(), p.attendance, p.score))
        .collect();
    assert_eq!(
        points,
        [
            ("CSE1001 name", 75, 88),
            ("MAT1001 name", 95, 58),
            ("PHY1001 name", 90, 80)
        ]
    );
    assert_eq!((roi.best, roi.grind), (0, Some(1)));
    assert_eq!(
        (roi.source, roi.verdict),
        (WrappedScoreSource::Total, WrappedRoiVerdict::BunkedAndAced)
    );
}

#[test]
fn roi_falls_back_to_marks() {
    let fixture = Fixture::new("roi-marks");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 30, 40),
                course("MAT1001", "ETH", 36, 40),
                course("PHY1001", "ETH", 39, 40),
                course("ENG1001", "ETH", 39, 40),
            ],
        ),
    );
    // ENG1001 has only 20 of its 100 assessed: left out.
    fixture.page(
        "marks.json",
        &marks_page(&[
            ("CSE1001", &[("CAT1", "30"), ("CAT2", "30")]),
            ("MAT1001", &[("CAT1", "40"), ("CAT2", "40")]),
            ("PHY1001", &[("CAT1", "45"), ("CAT2", "45")]),
            ("ENG1001", &[("CAT1", "10")]),
        ]),
    );
    let roi = fixture.report().roi.unwrap();
    let scores: Vec<(i32, i32)> = roi.points.iter().map(|p| (p.attendance, p.score)).collect();
    assert_eq!(scores, [(75, 60), (90, 80), (98, 90)]);
    assert_eq!(
        (roi.source, roi.verdict, roi.best, roi.grind),
        (
            WrappedScoreSource::Marks,
            WrappedRoiVerdict::ShowedUpPaidOff,
            2,
            Some(0)
        )
    );
    // Two courses are not enough.
    fixture.page(
        "marks.json",
        &marks_page(&[
            ("CSE1001", &[("CAT1", "30"), ("CAT2", "30")]),
            ("MAT1001", &[("CAT1", "40"), ("CAT2", "40")]),
        ]),
    );
    assert_eq!(fixture.report().roi, None);
}

#[test]
fn degree_ranks_this_semester_among_earlier_ones() {
    let fixture = Fixture::new("degree");
    fixture.shared(
        "grade_history.json",
        &grade_history(
            &[
                ("X1001", "4", "S", "Nov-2024"),
                ("OLD1001", "4", "A", "Apr-2025"),
                ("OLD1002", "4", "B", "Apr-2025"),
                ("CSE1001", "4", "S", "Nov-2025"),
                ("MAT1001", "4", "A", "Nov-2025"),
                ("NEW1001", "4", "S", "Apr-2026"),
            ],
            "9.3",
            "24",
        ),
    );
    fixture.page(
        "grades.json",
        &grades(&[("CSE1001", "S"), ("MAT1001", "A")]),
    );
    let degree = fixture.report().degree.unwrap();
    let semesters: Vec<(&str, f64, bool)> = degree
        .semesters
        .iter()
        .map(|s| (s.short.as_str(), s.gpa, s.current))
        .collect();
    // A later semester is left out of this one's story.
    assert_eq!(
        semesters,
        [
            ("Nov '24", 10.0, false),
            ("Apr '25", 8.5, false),
            ("Nov '25", 9.5, true)
        ]
    );
    assert_eq!((degree.rank, degree.credits), (2, 20.0));
    assert_eq!(
        degree.by_distribution,
        [WrappedCredits {
            label: "Core".into(),
            credits: 20.0
        }]
    );
    assert!((degree.cgpa.unwrap() - 9.2).abs() < 1e-9);
}

#[test]
fn degree_needs_two_graded_semesters() {
    let fixture = Fixture::new("degree-first");
    fixture.shared(
        "grade_history.json",
        &grade_history(&[("CSE1001", "4", "S", "Nov-2025")], "10", "4"),
    );
    fixture.page("grades.json", &grades(&[("CSE1001", "S")]));
    let report = fixture.report();
    assert_eq!(report.degree, None);
    assert!(report.cgpa.is_some());
}

fn titles(tape: &WrappedMixtape) -> Vec<&str> {
    tape.tracks.iter().map(|t| t.title.as_str()).collect()
}

/// A hosteller's rich semester: snoozed 8 AMs, under 75% once, labs and naps, a crunch week
/// with two papers a day, and an S-grade record.
fn rich() -> WrappedReport {
    WrappedReport {
        semester_id: "AP2026272".into(),
        semester_name: "Fall Semester 2026-27".into(),
        first_day: records::epoch_day("2026-07-14").unwrap(),
        last_day: records::epoch_day("2026-11-14").unwrap(),
        courses: 7,
        class_day_count: 72,
        habits: Some(WrappedHabits {
            early_percent: Some(64),
            early_classes: 20,
            ..Default::default()
        }),
        close_call: Some(WrappedCloseCall {
            course: "WATER AND SOCIETY".into(),
            spare: None,
            percent: 71,
            final_percent: 78,
            ..Default::default()
        }),
        time: Some(WrappedTime {
            minutes: 6000,
            lab_minutes: 2460,
            classes: 100,
            ..Default::default()
        }),
        hostel: Some(WrappedHostel {
            naps: 6,
            ..Default::default()
        }),
        crunch: Some(WrappedCrunch {
            papers: 5,
            most_in_a_day: 2,
            most_day: records::epoch_day("2026-11-21").unwrap(),
            first_day: records::epoch_day("2026-11-18").unwrap(),
            last_day: records::epoch_day("2026-11-24").unwrap(),
            ..Default::default()
        }),
        grades: Some(WrappedGrades {
            gpa: 9.3,
            s_count: 4,
            s_record: true,
            ..Default::default()
        }),
        traits: vec![
            WrappedTrait {
                title: WrappedTitle::Topper,
                score: 69,
                grades: true,
                hostel: false,
            },
            WrappedTrait {
                title: WrappedTitle::Regular,
                score: 45,
                grades: false,
                hostel: false,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn mixtape_full_hosteller_five_unique_tracks() {
    let tape = mixtape(&rich(), "22BCE0001", true, true).unwrap();
    let numbers: Vec<i32> = tape.tracks.iter().map(|t| t.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
    let t = titles(&tape);
    assert_eq!(t[0], "8 AM Alarm (Snoozed Mix)");
    assert_eq!(t[1], "Negotiating With 75%");
    // Labs (92) and naps (91) are a near tie, broken by the hash.
    assert!(
        ["Lab Rat Anthem", "Hostel Nap Interlude"].contains(&t[2]),
        "{t:?}"
    );
    // One track per fact: the double header over the crunch week, the S record over 9-point.
    assert_eq!(
        (t[3], t[4]),
        ("Double-Header Panic", "All-Time S-Grade Run")
    );
    assert_eq!(
        tape.tracks.iter().map(|t| t.duration_secs).sum::<i32>(),
        tape.total_secs
    );
    assert_eq!(tape.tracks[0].duration, "6:04");
    assert_eq!(tape.tracks[1].stat_line, "Water And Society · 71% → 78%");
    assert_eq!(
        (tape.stamp.as_str(), tape.genre.as_str()),
        ("FALL '26", "Honor-Roll Synth")
    );
    assert!(["Nine-Pointer Tape", "S-Grade Sessions"].contains(&tape.album_title.as_str()));
    assert!(!tape.no_skips);
}

#[test]
fn mixtape_hide_grades_swaps_tracks_and_metadata() {
    let report = rich();
    let (with, without) = (
        mixtape(&report, "22BCE0001", true, true).unwrap(),
        mixtape(&report, "22BCE0001", false, true).unwrap(),
    );
    assert_ne!(with, without);
    let t = titles(&without);
    assert!(!t.contains(&"All-Time S-Grade Run"), "{t:?}");
    assert!(
        without
            .tracks
            .iter()
            .all(|track| track.slot != WrappedTrackSlot::Results)
    );
    // The closer borrows the other of labs and naps, a fact over 75.
    assert!(
        ["Lab Rat Anthem", "Hostel Nap Interlude"].contains(&t[4]),
        "{t:?}"
    );
    assert_eq!(without.genre, "75% Debar-Line Punk");
    assert!(["Clockwork Sem", "Daily Check-In EP"].contains(&without.album_title.as_str()));
}

#[test]
fn mixtape_sparse_day_scholar_no_biometric_no_grades() {
    let report = WrappedReport {
        semester_name: "Fall Semester 2026-27".into(),
        first_day: records::epoch_day("2026-07-14").unwrap(),
        last_day: records::epoch_day("2026-11-14").unwrap(),
        courses: 6,
        class_day_count: 72,
        attendance: Some(WrappedAttendance {
            percent: 85,
            attended: 340,
            counted: 400,
            ..Default::default()
        }),
        habits: Some(WrappedHabits {
            early_percent: Some(64),
            early_classes: 20,
            worst_weekday: Some(0),
            worst_weekday_misses: 6,
            misses: 20,
            ..Default::default()
        }),
        time: Some(WrappedTime {
            minutes: 6000,
            lab_minutes: 600,
            classes: 110,
            ..Default::default()
        }),
        exams: Some(WrappedExams {
            cats: 6,
            fats: 5,
            top_venue: Some("AB1-208".into()),
            minutes: 1440,
        }),
        ..Default::default()
    };
    let tape = mixtape(&report, "", false, true).unwrap();
    assert_eq!(
        titles(&tape),
        [
            "8 AM Alarm (Snoozed Mix)",
            "Monday Vanishing Act",
            // Nothing of its own for the grind: it borrows the next strong fact.
            "Assigned Seat: AB1-208",
            "Under the Hall Clock",
            "FALL '26 Credits Roll",
        ]
    );
    assert_eq!(tape.tracks[3].stat_line, "24 h in exam halls");
}

#[test]
fn mixtape_calendar_only_returns_none() {
    let report = WrappedReport {
        semester_name: "Fall Semester 2026-27".into(),
        first_day: 100,
        last_day: 200,
        courses: 6,
        class_day_count: 72,
        holidays: 20,
        ..Default::default()
    };
    assert_eq!(mixtape(&report, "", true, true), None);
}

#[test]
fn mixtape_deterministic_and_tie_break_scoped() {
    let mut report = rich();
    report.habits = Some(WrappedHabits {
        early_percent: Some(64),
        early_classes: 20,
        worst_weekday: Some(4),
        worst_weekday_misses: 9,
        misses: 20,
        ..Default::default()
    });
    assert_eq!(
        mixtape(&report, "22BCE0001", true, true),
        mixtape(&report, "22BCE0001", true, true)
    );
    for regno in ["", "A", "22BCE0001", "23BCE7777", "24MIS0420", "zz"] {
        let tape = mixtape(&report, regno, true, true).unwrap();
        // 96 never loses to 74, whatever the seed.
        assert_eq!(tape.tracks[1].title, "Negotiating With 75%", "{regno}");
        assert_eq!(
            tape.cover_seed,
            fnv1a(
                FNV_OFFSET_FOR_TESTS,
                format!("{regno}:AP2026272").as_bytes()
            )
        );
    }
}

const FNV_OFFSET_FOR_TESTS: u64 = 0xcbf2_9ce4_8422_2325;

#[test]
fn mixtape_low_attendance_and_cat_dip_stay_playful() {
    let report = WrappedReport {
        semester_name: "Winter Semester 2025-26".into(),
        first_day: records::epoch_day("2026-01-05").unwrap(),
        last_day: records::epoch_day("2026-04-30").unwrap(),
        courses: 6,
        class_day_count: 70,
        habits: Some(WrappedHabits {
            early_percent: Some(42),
            early_classes: 18,
            ..Default::default()
        }),
        close_call: Some(WrappedCloseCall {
            course: "Operating Systems".into(),
            spare: None,
            percent: 68,
            final_percent: 74,
            ..Default::default()
        }),
        marks: Some(WrappedMarks {
            cat_change: Some(-12),
            ..Default::default()
        }),
        ..Default::default()
    };
    let tape = mixtape(&report, "22BCE0001", true, true).unwrap();
    let t = titles(&tape);
    assert_eq!(t.len(), 5);
    for title in [
        "8 AM Alarm (Snoozed Mix)",
        "Negotiating With 75%",
        "Open-Book Plot Twist",
    ] {
        assert!(t.contains(&title), "{t:?}");
    }
    assert!(t.iter().any(|t| t.starts_with("WIN '26")) || t.contains(&"Day One on VTOP"));
    assert_eq!(tape.genre, "75% Debar-Line Punk");
}

#[test]
fn duration_encoders_always_valid_mss() {
    let valid = |secs: i32| {
        let text = clock_of(secs);
        let (m, s) = text.split_once(':').unwrap();
        assert!(s.len() == 2 && s.parse::<i32>().unwrap() < 60, "{text}");
        assert_eq!(
            m.parse::<i32>().unwrap() * 60 + s.parse::<i32>().unwrap(),
            secs
        );
    };
    for p in 0..=100 {
        valid(dur_pct(p));
    }
    for hundredths in 0..=1000 {
        valid(dur_gpa(f64::from(hundredths) / 100.0));
    }
    for n in [0, 1, 34, 59, 60, 420, 9999] {
        valid(dur_secs(n));
        valid(dur_pair(n, n));
    }
    for t in 0..=240 {
        valid(dur_pair(t / 10, (t % 10) * 6));
    }
    assert_eq!(
        (
            clock_of(dur_pct(64)),
            clock_of(dur_pct(100)),
            clock_of(dur_gpa(9.05))
        ),
        ("6:04".into(), "10:00".into(), "9:03".to_string())
    );
    assert_eq!(
        (clock_of(dur_pair(5, 7)), clock_of(dur_secs(137))),
        ("5:07".into(), "2:17".to_string())
    );
    assert_eq!(
        (
            dur_clock("7:05 AM"),
            dur_clock("12:00 PM"),
            dur_clock("11:59 PM"),
            dur_clock("")
        ),
        (Some(7 * 60 + 5), Some(12 * 60), Some(11 * 60 + 59), None)
    );
    assert_eq!(
        (minute_of("12:30 AM"), minute_of("9:48 PM")),
        (Some(30), Some(21 * 60 + 48))
    );
    assert_eq!(
        span(
            records::epoch_day("2026-11-28").unwrap(),
            records::epoch_day("2026-12-04").unwrap()
        ),
        "28 Nov to 4 Dec"
    );
}

#[test]
fn short_course_and_unicode_safe_truncation() {
    assert_eq!(
        short_course("Cloud Computing and its Applications"),
        "Cloud Computing"
    );
    assert_eq!(short_course("INTRODUCTION TO PROGRAMMING"), "Programming");
    let long = short_course("Systèmes Avancés de Réseaux Numériques");
    assert!(long.chars().count() <= 20, "{long}");
    assert_eq!(fit("Ünïcødéwordwithoutspaces", 5).chars().count(), 5);
    assert_eq!(short_faculty("KANKANALA SRINIVAS - SENSE"), "Srinivas");
    assert_eq!(short_faculty("Ravi Kumar"), "Ravi Kumar");
}

#[test]
fn close_call_skipped_with_two_to_spare() {
    let fixture = Fixture::new("close-call-two");
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 19, 20)]),
    );
    // 9 of 10 is 90%: two more misses keep it at 75%.
    fixture.history(
        "CSE1001.theory",
        &runs(&[(9, "Present"), (1, "Absent"), (10, "Present")], "A1"),
    );
    assert_eq!(fixture.report().close_call, None);
    // 9 of 11 is 82%: one to spare.
    fixture.history(
        "CSE1001.theory",
        &runs(&[(9, "Present"), (2, "Absent"), (10, "Present")], "A1"),
    );
    assert_eq!(fixture.report().close_call.unwrap().spare, Some(1));
}

#[test]
fn mosaic_slump_only_under_85() {
    let fixture = Fixture::new("mosaic-slump");
    let d = d0();
    // July: 11 present. August (from d + 25): 12 of 13, 93% rounded up.
    let mut classes: Vec<(i32, &str, &str, &str)> = (0..11)
        .map(|i| (d + i, "08:00-08:50", "Present", "A1"))
        .collect();
    classes.extend((25..38).map(|i| {
        let status = if i == 30 { "Absent" } else { "Present" };
        (d + i, "08:00-08:50", status, "A1")
    }));
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 23, 24)]),
    );
    fixture.history("CSE1001.theory", &classes);
    let m = fixture.report().mosaic.unwrap();
    assert_eq!((m.best_month, m.worst_month), (Some(0), None));
}

#[test]
fn guesses_hold_the_answer_in_the_second_or_third() {
    let ranges = |p| {
        guesses(p)
            .into_iter()
            .map(|g| (g.from, g.to, g.off))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ranges(86),
        [
            (80, Some(84), 2),
            (85, Some(89), 0),
            (90, Some(94), -4),
            (95, None, -9)
        ]
    );
    assert_eq!(
        ranges(87),
        [
            (75, Some(79), 8),
            (80, Some(84), 3),
            (85, Some(89), 0),
            (90, None, -3)
        ]
    );
    // Moved down to end at 100.
    assert_eq!(ranges(100)[3], (95, None, 0));
    assert_eq!(ranges(100)[0], (80, Some(84), 16));
    assert_eq!(ranges(3)[0], (0, Some(4), 0));
    assert_eq!(ranges(61)[1], (55, Some(59), 2));
    for p in 0..=100 {
        let g = guesses(p);
        assert_eq!(g.len(), 4);
        assert_eq!(g.iter().filter(|g| g.off == 0).count(), 1, "{p}");
    }
}

#[test]
fn receipt_lines_keep_each_part() {
    let fixture = Fixture::new("lines");
    fixture.page(
        ATTENDANCE,
        &attendance(
            "AP2",
            vec![
                course("CSE1001", "ETH", 30, 40),
                course("CSE1001", "ELA", 20, 20),
                course("PHY1001", "ETH", 0, 0),
            ],
        ),
    );
    let lines: Vec<(bool, i32, i32)> = fixture
        .report()
        .attendance
        .unwrap()
        .lines
        .iter()
        .map(|l| (l.lab, l.attended, l.percent))
        .collect();
    assert_eq!(lines, [(false, 30, 75), (true, 20, 100)]);
}

/// `classes` as [`history`], each absence with VTOP's missing-punch remark unless `plain`.
fn trapped_history(classes: &[(i32, &str, &str, &str)], plain: &[i32]) -> Value {
    let mut value = history(classes);
    for record in value["records"].as_array_mut().unwrap() {
        let day = records::history_day(record["date"].as_str().unwrap()).unwrap();
        if record["status"] == "Absent" && !plain.contains(&day) {
            record["remark"] = json!("No punch found during class hours between faculty punch.");
        }
    }
    value
}

#[test]
fn traps_count_missing_punch_absences() {
    let fixture = Fixture::new("traps");
    let d = d0();
    fixture.page(
        ATTENDANCE,
        &attendance("AP2", vec![course("CSE1001", "ETH", 4, 9)]),
    );
    let classes = [
        (d, "08:00-08:50", "Absent", "A1"),
        (d + 1, "08:00-08:50", "Absent", "A1"),
        (d + 2, "08:00-08:50", "Absent", "A1"),
        (d + 3, "08:00-08:50", "Absent", "A1"),
        (d + 4, "08:00-08:50", "Absent", "A1"),
        (d + 7, "08:00-08:50", "Present", "A1"),
    ];
    fixture.page(
        &format!("{ATTENDANCE_DETAIL}/CSE1001.theory.json"),
        &trapped_history(&classes, &[d + 4]),
    );
    // Punched in on time on d; on d + 1 only long after the class.
    punches(&fixture, d, &[("07:50:00", "CB-214-FACE")]);
    punches(&fixture, d + 1, &[("10:00:00", "CB-214-FACE")]);
    let traps = fixture.report().traps.unwrap();
    assert_eq!(
        (traps.trapped, traps.absences, traps.checked, traps.punched),
        (4, 5, 2, 1)
    );
    assert_eq!(
        traps.by_course,
        [WrappedCourse {
            name: "CSE1001 name".into(),
            percent: 4
        }]
    );
    // Under three, no slide.
    fixture.page(
        &format!("{ATTENDANCE_DETAIL}/CSE1001.theory.json"),
        &trapped_history(&classes, &[d + 2, d + 3, d + 4]),
    );
    assert_eq!(fixture.report().traps, None);
}

#[test]
fn moves_climb_from_the_ground_in_another_block() {
    let fixture = Fixture::new("moves-blocks");
    let d = d0();
    punches(
        &fixture,
        d,
        &[("09:00:00", "CB-105-FACE"), ("10:00:00", "AB1-301-FACE")],
    );
    for day in d + 1..d + 5 {
        punches(&fixture, day, &[("09:00:00", "CB-G05-FACE")]);
    }
    // 1, then 3 up from AB1's ground floor; the same block would have been 2.
    assert_eq!(fixture.report().moves.unwrap().floors, 4);
}

/// A timetable with an earlier semester graded, so FFCS counts.
fn ffcs_fixture(name: &str, earlier: bool) -> Fixture {
    let fixture = Fixture::new(name);
    fixture.page(
        TIMETABLE,
        &timetable(&[
            (
                "MON", "A1", "CSE1001", "Theory", "08:00", "08:50", "101", "CB ALL", "",
            ),
            (
                "MON", "B1", "CSE1002", "Theory", "09:00", "09:50", "201", "AB1", "",
            ),
            (
                "MON", "C1", "CSE1003", "Theory", "11:00", "11:50", "201", "AB1", "",
            ),
            (
                "MON", "D2", "CSE1004", "Theory", "14:00", "14:50", "201", "AB1", "",
            ),
            (
                "TUE", "L1+L2", "CSE1001", "Lab", "10:00", "11:40", "301", "CB", "",
            ),
            (
                "TUE", "E1", "CSE1002", "Theory", "12:00", "12:50", "101", "CB", "",
            ),
        ]),
    );
    let month = if earlier { "Nov-2024" } else { "Nov-2025" };
    fixture.shared(
        "grade_history.json",
        &grade_history(&[("MAT1001", "4", "A", month)], "9", "4"),
    );
    fixture
}

#[test]
fn ffcs_grades_the_week() {
    let f = ffcs_fixture("ffcs", true).report().ffcs.unwrap();
    // 25 - 6 for one 8 AM, 25 - 5 for 130 minutes waiting (70 + 40 outside lunch + 20), 12
    // for lunch kept free on one day of two, 25 for four free days.
    assert_eq!(
        (f.early, f.gap_minutes, f.lunch_free_days, f.class_days),
        (1, 130, 1, 2)
    );
    assert_eq!(f.free_days, [2, 3, 4, 5]);
    assert_eq!((f.score, f.grade), (76, WrappedGrade::A));
    assert_eq!(
        (f.best, f.worst),
        (WrappedFfcsPart::FreeDays, WrappedFfcsPart::Lunch)
    );
    assert_eq!(
        (f.sprints, f.sprint_from.as_deref(), f.sprint_to.as_deref()),
        (1, Some("CB"), Some("AB1"))
    );
    assert_eq!(f.week.len(), 6);
    assert!(f.week[4].lab && f.week[4].day == 1);
}

#[test]
fn ffcs_hidden_in_a_first_semester() {
    assert_eq!(ffcs_fixture("ffcs-first", false).report().ffcs, None);
    let fixture = ffcs_fixture("ffcs-no-history", true);
    fs::remove_file(fixture.dir.join(format!("{PAGES}/grade_history.json"))).unwrap();
    assert_eq!(fixture.report().ffcs, None);
}

#[test]
fn hide_hostel_skips_hostel_titles_and_tracks() {
    let report = WrappedReport {
        outings: Some(WrappedOutings {
            leaves: 6,
            weekends: 4,
            ..Default::default()
        }),
        attendance: Some(WrappedAttendance {
            percent: 91,
            ..Default::default()
        }),
        ..Default::default()
    };
    let earned = traits(&report);
    assert_eq!(
        earned.iter().map(|t| t.title).collect::<Vec<_>>(),
        [
            WrappedTitle::Explorer,
            WrappedTitle::Regular,
            WrappedTitle::SteadyCruiser
        ]
    );
    assert_eq!(title(&earned, true, true), WrappedTitle::Explorer);
    assert_eq!(title(&earned, true, false), WrappedTitle::Regular);
    let rich = rich();
    let tape = mixtape(&rich, "22BCE0001", true, false).unwrap();
    assert!(!titles(&tape).contains(&"Hostel Nap Interlude"), "{tape:?}");
    assert_ne!(tape.genre, "Hostel-Nap Dream Pop");
}
