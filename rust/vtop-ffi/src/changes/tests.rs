use super::*;
use vtop_core::types::{
    AttendanceRecord, ExamScheduleRecord, FullAttendanceRecord, GradeCourseRecord, MarksRecord,
    MarksRecordEach, PerExamScheduleRecord, TimetableSlot,
};

fn json(value: &impl Serialize) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn dt(page: &str, old: &[u8], new: &[u8]) -> Vec<Draft> {
    detect(page, old, new, &Context::default())
}

fn att(attended: &str, total: &str, percent: &str) -> AttendanceData {
    AttendanceData {
        records: vec![AttendanceRecord {
            serial: "1".into(),
            category: String::new(),
            course_name: "CSE4007 - Digital Image Processing".into(),
            course_code: "CSE4007".into(),
            course_type: "ETH".into(),
            faculty_detail: String::new(),
            classes_attended: attended.into(),
            total_classes: total.into(),
            attendance_percentage: percent.into(),
            attendence_fat_cat: String::new(),
            debar_status: String::new(),
            course_id: "AM_CSE4007".into(),
        }],
        semester_id: "S".into(),
        update_time: 0,
    }
}

#[test]
fn attendance_posted_and_crossings() {
    let posted = dt(
        ATTENDANCE,
        &json(&att("30", "36", "84%")),
        &json(&att("31", "38", "82%")),
    );
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].title, "Digital Image Processing (Theory)");
    assert!(
        posted[0].body.starts_with("Attendance posted · "),
        "{}",
        posted[0].body
    );
    assert_eq!(posted[0].course_key.as_deref(), Some("CSE4007.theory"));
    assert!(
        posted[0].body.contains("1 class present, 1 class absent"),
        "{}",
        posted[0].body
    );
    let down = dt(
        ATTENDANCE,
        &json(&att("27", "36", "75%")),
        &json(&att("27", "37", "73%")),
    );
    assert!(down[0].body.starts_with("Attendance below 75%"));
    assert!(down[0].urgent);
    let up = dt(
        ATTENDANCE,
        &json(&att("27", "37", "73%")),
        &json(&att("28", "37", "76%")),
    );
    assert!(up[0].body.starts_with("Attendance back at 75%"));
    assert!(!up[0].urgent);
}

#[test]
fn debarred_is_its_own_change() {
    let mut debarred = att("20", "36", "56%");
    debarred.records[0].debar_status = "FAT : Debarred".into();
    let out = dt(ATTENDANCE, &json(&att("20", "36", "56%")), &json(&debarred));
    assert_eq!(out.len(), 1);
    assert!(out[0].body.starts_with("Debarred"), "{}", out[0].body);
    assert!(out[0].urgent);
    let back = dt(ATTENDANCE, &json(&debarred), &json(&att("20", "36", "56%")));
    assert!(back[0].body.starts_with("No longer debarred"));
}

#[test]
fn unchanged_data_records_nothing() {
    let a = json(&att("30", "36", "84%"));
    assert!(dt(ATTENDANCE, &a, &a).is_empty());
    let t = json(&tt(vec![slot("MON", "08:00", "321")]));
    assert!(dt(TIMETABLE, &t, &t).is_empty());
    assert!(dt("unknown.json", &a, &a).is_empty());
}

#[test]
fn history_corrections() {
    let rec = |status: &str| FullAttendanceRecord {
        serial: "1".into(),
        date: "03-Nov-2026".into(),
        slot: "B2".into(),
        day_time: String::new(),
        status: status.into(),
        remark: String::new(),
    };
    let data = |status: &str| FullAttendanceData {
        records: vec![rec(status)],
        semester_id: "S".into(),
        update_time: 0,
        course_id: "AM_CSE4007".into(),
        course_type: "ETH".into(),
    };
    let page = format!("{ATTENDANCE_DETAIL}/CSE4007.theory.json");
    let fixed = dt(&page, &json(&data("Absent")), &json(&data("On Duty")));
    assert_eq!(fixed.len(), 1);
    assert_eq!(
        fixed[0].title, "CSE4007 (Theory)",
        "no name known: the code"
    );
    assert_eq!(
        fixed[0].body,
        "Attendance corrected · 03-Nov-2026: absent → on duty"
    );
    let named = Context {
        names: [(
            "CSE4007".to_string(),
            "Digital Image Processing".to_string(),
        )]
        .into(),
        today: 0,
    };
    let fixed = detect(
        &page,
        &json(&data("Absent")),
        &json(&data("On Duty")),
        &named,
    );
    assert_eq!(fixed[0].title, "Digital Image Processing (Theory)");
    // Posted for the first time: the summary counts it, not the history.
    assert!(dt(&page, &json(&data("")), &json(&data("Present"))).is_empty());
}

fn marks_of(score: &str, titles: &[&str]) -> MarksData {
    MarksData {
        records: vec![MarksRecord {
            serial: "1".into(),
            coursecode: "CSE4007".into(),
            coursetitle: "Digital Image Processing".into(),
            coursetype: "ETH".into(),
            faculity: String::new(),
            slot: String::new(),
            marks: titles
                .iter()
                .enumerate()
                .map(|(i, t)| MarksRecordEach {
                    serial: format!("{}", i + 1),
                    markstitle: (*t).into(),
                    maxmarks: "50".into(),
                    weightage: String::new(),
                    status: String::new(),
                    scoredmark: score.into(),
                    weightagemark: String::new(),
                    remark: String::new(),
                })
                .collect(),
        }],
        semester_id: "S".into(),
        update_time: 0,
    }
}

#[test]
fn marks_published_and_updated() {
    let new = dt(
        MARKS,
        &json(&marks_of("40", &["CAT-1"])),
        &json(&marks_of("40", &["CAT-1", "CAT-2"])),
    );
    assert_eq!(new.len(), 1);
    assert_eq!(new[0].title, "Digital Image Processing");
    assert_eq!(new[0].body, "New marks · CAT-2 40/50");
    let filled = dt(
        MARKS,
        &json(&marks_of("", &["CAT-1"])),
        &json(&marks_of("41", &["CAT-1"])),
    );
    assert_eq!(filled[0].body, "New marks · CAT-1 41/50");
    let changed = dt(
        MARKS,
        &json(&marks_of("40", &["CAT-1"])),
        &json(&marks_of("42", &["CAT-1"])),
    );
    assert_eq!(changed[0].body, "Marks updated · CAT-1 40 → 42/50");
    let mut reweighed = marks_of("40", &["CAT-1"]);
    reweighed.records[0].marks[0].weightage = "15".into();
    let mut was = marks_of("40", &["CAT-1"]);
    was.records[0].marks[0].weightage = "10".into();
    let out = dt(MARKS, &json(&was), &json(&reweighed));
    assert_eq!(out[0].body, "Weightage changed · CAT-1 10 → 15");
    // The same score written another way is no change.
    assert!(
        dt(
            MARKS,
            &json(&marks_of("40", &["CAT-1"])),
            &json(&marks_of("40.0", &["CAT-1"]))
        )
        .is_empty()
    );
    // A row without a score yet is not news.
    assert!(
        dt(
            MARKS,
            &json(&marks_of("", &["CAT-1"])),
            &json(&marks_of("", &["CAT-1", "CAT-2"]))
        )
        .is_empty()
    );
}

fn slot(day: &str, start: &str, room: &str) -> TimetableSlot {
    TimetableSlot {
        serial: String::new(),
        day: day.into(),
        slot: "A1".into(),
        course_code: "CSE4007".into(),
        course_type: "ETH".into(),
        room_no: room.into(),
        block: "CB".into(),
        start_time: start.into(),
        end_time: String::new(),
        name: "Digital Image Processing".into(),
        kind: vtop_core::types::ClassKind::Theory,
        faculty: String::new(),
        credits: String::new(),
    }
}

fn tt(slots: Vec<TimetableSlot>) -> TimetableData {
    TimetableData {
        slots,
        courses: vec![],
        semester_id: "S".into(),
        update_time: 0,
    }
}

#[test]
fn timetable_moves_are_one_change_per_course() {
    let moved = dt(
        TIMETABLE,
        &json(&tt(vec![slot("MON", "08:00", "321")])),
        &json(&tt(vec![slot("MON", "08:00", "512")])),
    );
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].title, "Digital Image Processing");
    assert_eq!(
        moved[0].body,
        "Timetable changed · MON 08:00 room CB 321 → CB 512"
    );
    // Spacing and case alone are no change.
    let mut spaced = slot("MON", "08:00", " 321 ");
    spaced.block = "cb".into();
    assert!(
        dt(
            TIMETABLE,
            &json(&tt(vec![slot("MON", "08:00", "321")])),
            &json(&tt(vec![spaced]))
        )
        .is_empty()
    );
    let added = dt(
        TIMETABLE,
        &json(&tt(vec![slot("MON", "08:00", "321")])),
        &json(&tt(vec![
            slot("MON", "08:00", "321"),
            slot("TUE", "09:00", "321"),
        ])),
    );
    assert_eq!(added[0].body, "Timetable changed · + TUE 09:00 · CB 321");
}

fn exam(date: &str, venue: &str) -> ExamScheduleData {
    ExamScheduleData {
        exams: vec![PerExamScheduleRecord {
            records: vec![ExamScheduleRecord {
                serial: String::new(),
                slot: String::new(),
                course_name: "Digital Image Processing".into(),
                course_code: "CSE4007".into(),
                course_type: "ETH".into(),
                course_id: "AM_CSE4007".into(),
                exam_date: date.into(),
                exam_session: String::new(),
                reporting_time: String::new(),
                exam_time: "10:00".into(),
                venue: venue.into(),
                seat_location: String::new(),
                seat_no: String::new(),
            }],
            exam_type: "FAT".into(),
        }],
        semester_id: "S".into(),
        update_time: 0,
    }
}

#[test]
fn exams_added_changed_removed() {
    let empty = ExamScheduleData {
        exams: vec![],
        semester_id: "S".into(),
        update_time: 0,
    };
    let added = dt(EXAMS, &json(&empty), &json(&exam("16-Nov-2026", "")));
    assert_eq!(added[0].title, "Digital Image Processing");
    assert_eq!(added[0].body, "FAT scheduled · 16-Nov-2026 · 10:00");
    let unposted = dt(EXAMS, &json(&empty), &json(&exam("-", "-")));
    assert_eq!(unposted[0].body, "FAT scheduled · 10:00");
    let moved = dt(
        EXAMS,
        &json(&exam("16-Nov-2026", "")),
        &json(&exam("18-Nov-2026", "CB 101")),
    );
    assert_eq!(
        moved[0].body,
        "FAT updated · date 16-Nov-2026 → 18-Nov-2026, venue CB 101"
    );
    let posted = dt(
        EXAMS,
        &json(&exam("-", "")),
        &json(&exam("16-Nov-2026", "")),
    );
    assert_eq!(posted[0].body, "FAT updated · date 16-Nov-2026");
    let removed = dt(EXAMS, &json(&exam("16-Nov-2026", "")), &json(&empty));
    assert_eq!(removed[0].body, "FAT removed");
}

fn grade(g: &str) -> GradeViewData {
    GradeViewData {
        courses: vec![GradeCourseRecord {
            serial: String::new(),
            course_code: "CSE4007".into(),
            course_title: "Digital Image Processing".into(),
            course_type: "ETH".into(),
            grading_type: String::new(),
            grand_total: String::new(),
            grade: g.into(),
            course_id: "AM_CSE4007".into(),
        }],
        semesters: vec![],
        semester_id: "S".into(),
        update_time: 0,
    }
}

#[test]
fn grades_published_and_changed() {
    let out = dt(GRADES, &json(&grade("")), &json(&grade("A")));
    assert_eq!(
        (out[0].title.as_str(), out[0].body.as_str()),
        ("Digital Image Processing", "Grade published · A")
    );
    let out = dt(GRADES, &json(&grade("B")), &json(&grade("A")));
    assert_eq!(out[0].body, "Grade changed · B → A");
    assert!(dt(GRADES, &json(&grade("")), &json(&grade("F")))[0].urgent);
    assert!(dt(GRADES, &json(&grade("A")), &json(&grade("A"))).is_empty());
}

fn general(status: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "student": null, "notice": "", "out_hours": [], "in_hours": [],
        "place_max_length": 0, "purpose_max_length": 0, "max_days_ahead": 0,
        "max_days_away": 0, "update_time": 0,
        "records": [{
            "serial": "7", "place": "Vijayawada", "purpose": "", "from_date": "2026-11-22",
            "from_time": "", "to_date": "", "to_time": "", "status": status, "pass_id": "",
        }],
    }))
    .unwrap()
}

#[test]
fn outings_decided() {
    let out = dt(GENERAL, &general("Pending"), &general("Accepted"));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, ChangeKind::Outing);
    assert_eq!(out[0].title, "General outing");
    assert_eq!(out[0].body, "Accepted · Vijayawada, Sun 22 Nov");
    assert!(!out[0].urgent);
    assert!(dt(GENERAL, &general("Pending"), &general("Rejected"))[0].urgent);
    assert!(dt(GENERAL, &general("Pending"), &general("pending")).is_empty());
}

fn plan(paths: &[&str]) -> Vec<u8> {
    let lectures: Vec<_> = paths
        .iter()
        .map(|p| {
            serde_json::json!({
                "serial": "1", "date": "", "day": "", "topic": format!("Topic {p}"),
                "materials": [{ "label": "Download", "path": p }],
            })
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "semester_id": "S", "course_id": "AM_CSE4007", "all_materials_path": "",
        "general_materials_path": "", "syllabus_path": "", "has_course_plan": true,
        "update_time": 0, "lectures": lectures,
        "class": {
            "class_id": "C1", "erp_id": "", "class_group": "", "course_code": "CSE4007",
            "course_title": "Digital Image Processing", "course_type": "Embedded Theory",
            "slot": "", "faculty": "", "faculty_school": "",
        },
    }))
    .unwrap()
}

#[test]
fn new_course_files() {
    let page = format!("{PLANS}/C1.json");
    let out = dt(&page, &plan(&["a"]), &plan(&["a", "b", "c"]));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "Digital Image Processing");
    assert_eq!(out[0].body, "2 new files · Topic b, Topic c");
    assert!(dt(&page, &plan(&["a", "b"]), &plan(&["a"])).is_empty());
}

fn cal(entries: &[(&str, &str, &str)]) -> Vec<u8> {
    let entries: Vec<_> = entries
        .iter()
        .map(|(date, kind, note)| {
            serde_json::json!({
                "date": date, "kind": kind, "group": "General (Semester)", "note": note,
            })
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "entries": entries, "semester_id": "S", "update_time": 0,
    }))
    .unwrap()
}

#[test]
fn calendar_days_from_today_on() {
    let context = Context {
        names: BTreeMap::new(),
        today: epoch_day("2026-11-21").unwrap(),
    };
    let old = cal(&[
        ("2026-11-10", "Instructional Day", ""),
        ("2026-11-22", "Instructional Day", ""),
    ]);
    let new = cal(&[
        ("2026-11-10", "Holiday", ""),
        ("2026-11-22", "Holiday", "Deepavali"),
    ]);
    let out = detect(CALENDAR, &old, &new, &context);
    assert_eq!(out.len(), 1, "a past day is no news");
    assert_eq!(out[0].title, "Academic calendar");
    assert_eq!(
        out[0].body,
        "Day changed · Sun 22 Nov: Instructional Day → Holiday (Deepavali)"
    );
    assert!(detect(CALENDAR, &new, &new, &context).is_empty());
}

#[test]
fn old_changes_read_subject_first() {
    let old = |title: &str, body: &str, key: Option<&str>| Change {
        id: "1".into(),
        kind: ChangeKind::Attendance,
        at_ms: 1,
        title: title.into(),
        body: body.into(),
        course_key: key.map(str::to_string),
        semester: "S".into(),
        urgent: false,
    };
    let c = migrate(old(
        "Attendance below 75%",
        "Digital Image Processing (Theory): 1 class absent, now 74%",
        None,
    ));
    assert_eq!(c.title, "Digital Image Processing (Theory)");
    assert_eq!(c.body, "Attendance below 75% · 1 class absent, now 74%");
    assert!(c.urgent);
    let c = migrate(old(
        "Attendance corrected",
        "CSE4007 on 03-Nov-2026: absent → on duty",
        Some("CSE4007.lab"),
    ));
    assert_eq!(c.title, "CSE4007 (Lab)");
    assert_eq!(
        c.body,
        "Attendance corrected · 03-Nov-2026: absent → on duty"
    );
    let c = migrate(old(
        "Exam scheduled",
        "Digital Image Processing FAT: 16-Nov-2026 · 10:00",
        None,
    ));
    assert_eq!(c.title, "Digital Image Processing");
    assert_eq!(c.body, "FAT scheduled · 16-Nov-2026 · 10:00");
    let c = migrate(old("Exam removed", "Digital Image Processing FAT", None));
    assert_eq!(c.body, "FAT removed");
    let c = migrate(old(
        "New marks published",
        "Digital Image Processing: CAT-2 40/50",
        None,
    ));
    assert_eq!(c.body, "New marks · CAT-2 40/50");
    // Already new, or unknown: left alone.
    let new = old("General outing", "Accepted · Vijayawada, Sun 22 Nov", None);
    assert_eq!(migrate(new.clone()), new);
    let store = upgraded(Store {
        changes: vec![old(
            "Grade changed",
            "Digital Image Processing: B → A",
            None,
        )],
        ..Store::default()
    });
    assert_eq!(store.format, FORMAT);
    assert_eq!(store.changes[0].body, "Grade changed · B → A");
    // Converted once: a later title that looks old is not touched again.
    assert_eq!(upgraded(store).changes[0].body, "Grade changed · B → A");
}

#[test]
fn store_primes_dedupes_and_trims() {
    let mut store = Store::default();
    // First write of a page (or first after a new pick) is not compared.
    assert!(!prime(&mut store, Some("S"), "S", ATTENDANCE));
    assert!(prime(&mut store, Some("S"), "S", ATTENDANCE));
    assert!(!prime(&mut store, Some("T"), "T", ATTENDANCE));
    let d = |body: &str| draft(ChangeKind::Marks, "Course", body, "", None);
    assert_eq!(add(&mut store, vec![d("a")], "S", 1_000), 1);
    assert_eq!(add(&mut store, vec![d("a"), d("b")], "S", 2_000), 1);
    assert_eq!(store.changes[0].body, "b");
    for n in 0..200 {
        add(
            &mut store,
            vec![d(&format!("change {n}"))],
            "S",
            10_000_000 + n,
        );
    }
    trim(&mut store, 4_000);
    assert!(serde_json::to_vec(&store).unwrap().len() <= 4_000);
    assert!(!store.changes.is_empty());
    assert_eq!(store.changes[0].body, "change 199", "the newest stays");
}

#[test]
fn core_records_only_after_a_primed_copy() {
    let dir = std::env::temp_dir().join(format!("changes-{}", now_ms()));
    fs::create_dir_all(dir.join(format!("{PAGES}/S"))).unwrap();
    let core = Core {
        dir: dir.clone(),
        cache: dir.clone(),
        slot: Mutex::new(None),
    };
    core.write_plain(SELECTED, &"S".to_string());
    let file = format!("{PAGES}/S/{ATTENDANCE}");
    // First copy, then the same again: nothing.
    core.write(&file, &att("30", "36", "84%"));
    core.write(&file, &att("30", "36", "84%"));
    assert_eq!(core.unseen_changes("t".into()), 0);
    core.write(&file, &att("31", "37", "84%"));
    assert_eq!(core.unseen_changes("t".into()), 1);
    assert_eq!(core.unseen_change_ids("t".into()).len(), 1);
    // Another page leaves it new; its own page sees it.
    core.mark_page_seen("t".into(), "marks".into());
    assert_eq!(core.unseen_changes("t".into()), 1);
    core.mark_page_seen("t".into(), "attendance".into());
    assert_eq!(core.unseen_changes("t".into()), 0);
    std::thread::sleep(std::time::Duration::from_millis(2));
    core.write(&file, &att("32", "39", "82%"));
    assert_eq!(core.unseen_changes("t".into()), 1);
    core.mark_changes_seen("t".into());
    assert_eq!(core.unseen_changes("t".into()), 0);
    assert_eq!(core.changes("t".into()).len(), 2);
    core.clear_changes("t".into());
    assert!(core.changes("t".into()).is_empty());
    let _ = fs::remove_dir_all(dir);
}
