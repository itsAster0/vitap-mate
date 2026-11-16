use std::path::PathBuf;
use std::sync::Mutex;

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
        let core = Core {
            dir: dir.clone(),
            cache: dir.clone(),
            slot: Mutex::new(None),
        };
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
        (4, d, d + 2)
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
            (d + 2, "08:00-08:50", "Present", "A1"),
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
    assert_eq!((habits.early_percent, habits.early_classes), (Some(50), 4));
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
#[allow(clippy::type_complexity)]
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
    assert_eq!(
        (places.faculty.as_deref(), places.faculty_classes),
        (Some("Fac One"), 3)
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
    assert_eq!(marks.most_improved.as_deref(), Some("CSE1001 name"));
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
    let paper = |code: &str, venue: &str| {
        json!({"serial": "1", "slot": "A1", "course_name": code, "course_code": code, "course_type": "ETH",
            "course_id": "", "exam_date": "01-Aug-2025", "exam_session": "FN", "reporting_time": "",
            "exam_time": "09:30 AM - 11:00 AM", "venue": venue, "seat_location": "", "seat_no": ""})
    };
    fixture.page(
        "exam_schedule.json",
        &json!({"exams": [
            {"exam_type": "CAT1", "records": [paper("A", "AB1-101"), paper("B", "AB1-102")]},
            {"exam_type": "CAT2", "records": [paper("A", "AB1-102")]},
            {"exam_type": "FAT", "records": [paper("A", "-")]},
        ], "semester_id": "AP2", "update_time": 1}),
    );
    let exams = fixture.report().exams.unwrap();
    assert_eq!(
        (exams.cats, exams.fats, exams.top_venue.as_deref()),
        (3, 1, Some("AB1-102"))
    );
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

#[test]
fn title_follows_the_order() {
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
    assert_eq!(title(&both, true), WrappedTitle::IronStreak);
    let topper = WrappedReport {
        grades: Some(WrappedGrades {
            gpa: 9.2,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(title(&topper, true), WrappedTitle::Topper);
    assert_eq!(title(&topper, false), WrappedTitle::FreeSpirit);
    let comeback = WrappedReport {
        comeback: Some(WrappedComeback {
            low_percent: 70,
            low_day: 0,
            final_percent: 80,
        }),
        ..both.clone()
    };
    assert_eq!(title(&comeback, true), WrappedTitle::Comeback);
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
    assert_eq!(title(&lab, true), WrappedTitle::LabRat);
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
            ("07:12:00", "MH2-FACE-IN-1"),
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
            (d + 5, "08:00-08:50", "Absent", "A1"),
            (d + 6, "08:00-08:50", "Present", "A1"),
        ],
    );
    let streak = fixture.report().streak.unwrap();
    assert_eq!((streak.length, streak.offset, streak.total), (3, 1, 6));
    assert_eq!((streak.first_day, streak.last_day), (d + 1, d + 4));
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
    assert_eq!(title(&streak(59, 336), true), WrappedTitle::IronStreak);
    assert_eq!(title(&streak(45, 400), true), WrappedTitle::FreeSpirit);
    // A large share of a short semester still needs 15 classes.
    assert_eq!(title(&streak(14, 50), true), WrappedTitle::FreeSpirit);
    assert_eq!(title(&streak(15, 100), true), WrappedTitle::IronStreak);
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
