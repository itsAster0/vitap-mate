#[test]
fn semester_names_split() {
    let s = super::Semester::new("AP1".into(), "Winter Semester 2024-25 Freshers".into());
    assert_eq!(
        (s.season.as_str(), s.year.as_str(), s.note.as_str()),
        ("Winter", "2024-25", "Freshers")
    );
    let s = super::Semester::new("AP2".into(), "FALL SEMESTER 2026-27".into());
    assert_eq!(
        (s.season.as_str(), s.year.as_str(), s.note.as_str()),
        ("Fall", "2026-27", "")
    );
    let s = super::Semester::new("AP3".into(), "Special Course Weekend".into());
    assert_eq!(
        (s.season.as_str(), s.year.as_str()),
        ("Special Course Weekend", "")
    );
}

use super::*;

// Cases from test/attendance_standing_test.dart.
#[test]
fn safe_course_reports_skips() {
    let s = standing(22, 24, None, false);
    assert!(s.safe);
    assert_eq!(s.can_skip, 5);
    assert_eq!(s.advice, "Can skip 5");
}

#[test]
fn short_course_reports_classes_needed() {
    let s = standing(6, 10, None, false);
    assert!(!s.safe);
    assert_eq!(s.must_attend, 6);
    assert_eq!(s.advice, "Attend 6");
}

#[test]
fn exactly_on_the_line_cannot_skip() {
    let s = standing(3, 4, None, false);
    assert!(s.safe);
    assert_eq!(s.can_skip, 0);
    assert_eq!(s.advice, "Don't skip");
}

#[test]
fn no_classes_yet() {
    let s = standing(0, 0, None, false);
    assert!(s.safe);
    assert_eq!(s.advice, "No classes yet");
}

#[test]
fn course_names_split() {
    let split = |full| split_course_name(full, "X");
    assert_eq!(
        split("CSE2001 - Data Structures - Embedded Theory"),
        ("CSE2001".into(), "Data Structures".into())
    );
    assert_eq!(
        split("MAT1001 - Calculus-I - Theory Only"),
        ("MAT1001".into(), "Calculus-I".into())
    );
    assert_eq!(
        split("ENG1001 - English"),
        ("ENG1001".into(), "English".into())
    );
    assert_eq!(
        split("Data Structures"),
        ("X".into(), "Data Structures".into())
    );
}

#[test]
fn names_and_times() {
    assert_eq!(person_name("PRASHANTH KUMAR - SCOPE"), "Prashanth Kumar");
    assert_eq!(person_name("Dr. Md Rafi - SCOPE"), "Dr. Md Rafi");
    assert_eq!(minutes("08:00"), Some(480));
    assert_eq!(minutes("2:05 PM"), Some(845));
    assert_eq!(minutes("12:10 am"), Some(10));
    assert_eq!(minutes("-"), None);
}

fn record(course_type: &str, attended: &str, total: &str, percent: &str) -> raw::AttendanceRecord {
    raw::AttendanceRecord {
        serial: "1".into(),
        category: "Program Core".into(),
        course_name: "SWE1002 - Principles of Software Engineering - Embedded Lab".into(),
        course_code: "AP2025262000671 - L10+L11 - 409".into(),
        course_type: course_type.into(),
        faculty_detail: "SHARMILA SHAIK - SCOPE".into(),
        classes_attended: attended.into(),
        total_classes: total.into(),
        attendance_percentage: percent.into(),
        attendence_fat_cat: "-".into(),
        debar_status: "-".into(),
        course_id: "AM_SWE1002_00200".into(),
    }
}

fn courses(records: Vec<raw::AttendanceRecord>) -> Vec<AttendanceCourse> {
    Attendance::from(AttendanceData {
        records,
        semester_id: "AP2025262".into(),
        update_time: 1,
    })
    .courses
}

#[test]
fn lab_standing_counts_sessions() {
    let courses = courses(vec![
        record("ELA", "11", "14", "79%"),
        record("ETH", "11", "14", "79%"),
    ]);
    assert_eq!(
        (courses[0].kind, courses[0].percent, courses[0].key.as_str()),
        (ClassKind::Lab, 79.0, "SWE1002.lab")
    );
    // Lab: 5 of 7 sessions by the counts, but VTOP says 79%: safe.
    assert_eq!((courses[0].safe, courses[0].must_attend), (true, 0));
    assert_eq!(courses[0].advice, "Don't skip");
    assert_eq!(courses[1].key, "SWE1002.theory");
    assert_eq!(courses[1].advice, "Don't skip");
}

// Fall 2025-26, SWE1002 lab: 11 sessions attended, 16 held, 2 not posted;
// VTOP counts 22 of 32 periods but reports 11 of 14 = 79%.
#[test]
fn standing_follows_vtops_percentage() {
    let lab = &courses(vec![record("ELA", "22", "32", "79%")])[0];
    assert_eq!((lab.percent, lab.counted), (79.0, 28));
    assert!(lab.safe);
    assert_eq!((lab.must_attend, lab.advice.as_str()), (0, "Don't skip"));
    // Below VTOP's line: always something to attend, even when the
    // counts alone would say safe.
    let short = &courses(vec![record("ETH", "30", "40", "74%")])[0];
    assert!(!short.safe);
    assert!(short.must_attend >= 1);
    assert!(short.advice.starts_with("Attend"));
    // VTOP's 74.5 rounds to 75: safe.
    let s = standing(149, 200, Some(75.0), false);
    assert!(s.safe && s.must_attend == 0);
    // No percentage from VTOP: the counts decide.
    let s = standing(6, 10, None, false);
    assert_eq!((s.safe, s.must_attend), (false, 6));
}

#[test]
fn counted_total_drops_unposted_classes() {
    assert_eq!(counted_total(44, 57, Some(79.0)), 56);
    assert_eq!(counted_total(33, 44, Some(77.0)), 43);
    assert_eq!(counted_total(16, 26, Some(67.0)), 24);
    assert_eq!(counted_total(0, 10, Some(0.0)), 10);
    assert_eq!(counted_total(5, 10, Some(99.0)), 10);
    assert_eq!(counted_total(5, 10, None), 10);
    // VTOP rounds up: 31 of 34 (91.2%) shows as 92%, 35 of 41 as 86%.
    assert_eq!(counted_total(31, 34, Some(92.0)), 34);
    assert_eq!(counted_total(35, 41, Some(86.0)), 41);
}

#[test]
fn debar_status_names_the_exam() {
    let debar = Debar::parse("CAT - II : Debarred & Permitted").unwrap();
    assert_eq!(
        (
            debar.exam.as_str(),
            debar.permitted,
            debar.next_exam.as_deref()
        ),
        ("CAT-II", true, Some("FAT"))
    );
    let debar = Debar::parse("FAT : Debarred").unwrap();
    assert_eq!(
        (debar.exam.as_str(), debar.permitted, debar.next_exam),
        ("FAT", false, None)
    );
    assert_eq!(Debar::parse("Debarred").unwrap().exam, "");
    assert_eq!(Debar::parse("-"), None);
}

#[test]
fn plans_count_classes_left_by_the_calendar() {
    use crate::calendar::{SemesterCalendar, tests::fall_2026};
    let calendar = SemesterCalendar::of(&fall_2026());
    let slot = |day, start, kind| ClassSlot {
        day,
        start,
        end: start + 50,
        code: "SWE1002".into(),
        name: String::new(),
        kind,
        room: String::new(),
        block: String::new(),
        slot: String::new(),
        faculty: String::new(),
        credits: None,
    };
    let slots = [
        slot(0, 480, ClassKind::Theory),
        slot(3, 600, ClassKind::Theory),
        slot(1, 660, ClassKind::Lab),
    ];
    // Thursday 2026-11-05, 09:00: Thursday's 10:00 today and next
    // Thursday's, until the last class day (14 Nov); Monday 9 Nov is a
    // holiday. The FAT starts after the last class day: one plan.
    let theory = &courses(vec![record("ETH", "30", "40", "75%")])[0];
    let plans = attendance_plans(theory, &slots, &calendar, 20_762, 540, None);
    assert_eq!(plans.len(), 1);
    let plan = &plans[0];
    assert_eq!((plan.until.as_str(), plan.until_day), ("", 20_772));
    assert_eq!((plan.upcoming, plan.unposted), (2, 0));
    // Today (Thursday 5 Nov) and next Thursday (12 Nov), one class each.
    assert_eq!(
        plan.days,
        vec![
            PlanDay {
                day: 20_762,
                classes: 1
            },
            PlanDay {
                day: 20_769,
                classes: 1
            },
        ]
    );
    assert_eq!((plan.can_miss, plan.must_attend), (0, Some(2)));
    assert!((plan.best_percent - 32.0 / 42.0 * 100.0).abs() < 1e-9);
    // Labs stop at the Lab FAT; the 2 sessions not posted yet still count.
    let lab = &courses(vec![record("ELA", "22", "32", "79%")])[0];
    let plan = &attendance_plans(lab, &slots, &calendar, 20_762, 540, None)[0];
    assert_eq!(
        (plan.attended, plan.held, plan.upcoming, plan.unposted),
        (11, 14, 0, 2)
    );
    assert_eq!((plan.can_miss, plan.must_attend), (1, Some(1)));
    // Too far below: even attending every class left is not enough.
    let short = &courses(vec![record("ETH", "10", "40", "25%")])[0];
    let plan = &attendance_plans(short, &slots, &calendar, 20_762, 540, None)[0];
    assert_eq!(plan.must_attend, None);
}

fn slot_of(code: &str, day: i32, start: i32, kind: ClassKind, name: &str) -> ClassSlot {
    ClassSlot {
        day,
        start,
        end: start + 50,
        code: code.into(),
        name: String::new(),
        kind,
        room: String::new(),
        block: String::new(),
        slot: name.into(),
        faculty: String::new(),
        credits: None,
    }
}

#[test]
fn next_classes_follow_the_calendar() {
    use crate::calendar::{SemesterCalendar, tests::fall_2026};
    let calendar = SemesterCalendar::of(&fall_2026());
    let slots = [
        slot_of("LAB1", 1, 660, ClassKind::Lab, "L1+L2"),
        slot_of("THU1", 3, 600, ClassKind::Theory, "A1"),
        slot_of("MON1", 0, 480, ClassKind::Theory, "B1"),
    ];
    // Thursday 2026-11-05, 09:00 IST.
    let now = 20_762 * 86_400 + 9 * 3_600 - 330 * 60;
    let next = next_classes(&slots, now, 330, Some(&calendar));
    let found = |code: &str| {
        next.iter()
            .find(|class| class.code == code)
            .map(|class| (class.days_ahead, class.start))
    };
    assert_eq!(found("THU1"), Some((0, 600)));
    // Labs stopped at the Lab FAT; Monday 9 Nov is Deepavali and 16 Nov
    // the FAT: no Monday class before the semester ends.
    assert_eq!((found("LAB1"), found("MON1")), (None, None));
    // Without the calendar, the plain week.
    let plain = next_classes(&slots, now, 330, None);
    assert_eq!(
        plain
            .iter()
            .find(|class| class.code == "LAB1")
            .map(|class| class.days_ahead),
        Some(5)
    );
}

#[test]
fn held_classes_take_their_posted_status() {
    let row = |day, slot: &str, start, status| AttendedClass {
        day,
        weekday: 0,
        slot: slot.into(),
        start: Some(start),
        end: None,
        status,
        remark: String::new(),
    };
    let mut days = timetable_days(
        &[
            slot_of("CSE1", 0, 480, ClassKind::Theory, "A1"),
            slot_of("CSE1", 0, 600, ClassKind::Theory, "TA1"),
        ],
        20_759,
        2,
        None,
    );
    let histories = HashMap::from([(
        ("CSE1".to_string(), ClassKind::Theory),
        vec![
            row(20_759, "TA1", 600, Some(ClassStatus::Absent)),
            row(20_759, "A1", 480, Some(ClassStatus::Present)),
        ],
    )]);
    fill_posted(&mut days, &histories);
    let posted: Vec<_> = days[0].classes.iter().map(|class| class.posted).collect();
    assert_eq!(
        posted,
        [Some(ClassStatus::Present), Some(ClassStatus::Absent)]
    );
    assert!(days[1].classes.is_empty());
}

// VTOP's summary runs to Tuesday 3 Nov; Wednesday's class was held but is
// not on VTOP yet: "1+1 left" on Thursday morning.
#[test]
fn classes_held_after_the_summary_count_as_not_posted() {
    use crate::calendar::{SemesterCalendar, tests::fall_2026};
    let calendar = SemesterCalendar::of(&fall_2026());
    let slots = [
        slot_of("SWE1002", 2, 480, ClassKind::Theory, "A1"),
        slot_of("SWE1002", 3, 600, ClassKind::Theory, "B1"),
    ];
    let theory = &courses(vec![record("ETH", "30", "40", "75%")])[0];
    let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 540, Some(20_760))[0];
    // Thu 5 Nov 10:00 and Wed 11 / Thu 12 Nov to come.
    assert_eq!((plan.unposted, plan.upcoming), (1, 3));
    // Up to date: nothing held after it.
    let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 540, Some(20_762))[0];
    assert_eq!(plan.unposted, 0);
    // After 10:00 today's class is held too.
    let plan = &attendance_plans(theory, &slots, &calendar, 20_762, 660, Some(20_760))[0];
    assert_eq!((plan.unposted, plan.upcoming), (2, 2));
}

#[test]
fn histories_match_their_summary() {
    let lab = courses(vec![record("ELA", "22", "32", "79%")]).remove(0);
    let class = |status| AttendedClass {
        day: 0,
        weekday: 0,
        slot: String::new(),
        start: None,
        end: None,
        status,
        remark: String::new(),
    };
    let mut classes: Vec<_> = (0..11).map(|_| class(Some(ClassStatus::Present))).collect();
    classes.extend((0..3).map(|_| class(Some(ClassStatus::Absent))));
    classes.extend((0..2).map(|_| class(None)));
    assert!(history_in_sync(&lab, &classes));
    classes.pop();
    assert!(!history_in_sync(&lab, &classes));
    assert_eq!(counted_through(&lab, &classes), None);
}

#[test]
fn history_is_newest_first_with_times() {
    let course = courses(vec![record("ELA", "22", "32", "79%")]).remove(0);
    let row = |date: &str, status: &str| raw::FullAttendanceRecord {
        serial: "1".into(),
        date: date.into(),
        slot: "L10+L11".into(),
        day_time: "WED / 11:00-12:40".into(),
        status: status.into(),
        remark: String::new(),
    };
    let detail = AttendanceDetail::new(
        course,
        FullAttendanceData {
            records: vec![
                row("12-11-2025", "Present"),
                row("19-11-2025", "Not Posted"),
                row("01-Jul-2026", "On Duty"),
                row("bad", "Absent"),
            ],
            semester_id: "AP2025262".into(),
            update_time: 9,
            course_id: "AM_SWE1002_00200".into(),
            course_type: "ELA".into(),
        },
        Vec::new(),
    );
    let days: Vec<_> = detail.classes.iter().map(|class| class.day).collect();
    // 2026-07-01, 2025-11-19, 2025-11-12.
    assert_eq!(days, [20_635, 20_411, 20_404]);
    assert_eq!(civil_date(20_411.into()), "2025-11-19");
    let last = &detail.classes[2];
    assert_eq!(
        (last.weekday, last.start, last.end),
        (2, Some(660), Some(760))
    );
    assert_eq!(
        detail
            .classes
            .iter()
            .map(|class| class.status)
            .collect::<Vec<_>>(),
        [Some(ClassStatus::OnDuty), None, Some(ClassStatus::Present)]
    );
}

use ClassStatus::{Absent as A, OnDuty as OD, Present as P};

#[test]
fn advice_names_labs() {
    // Labs count in sessions: 9 of 10, then 2 of 3.
    let c = courses(vec![
        record("ELA", "18", "20", "90%"),
        record("ELA", "4", "6", "67%"),
        record("ETH", "18", "20", "90%"),
    ]);
    let advice: Vec<_> = c.iter().map(|course| course.advice.as_str()).collect();
    assert_eq!(advice, ["Can skip 2 labs", "Attend 1 lab", "Can skip 4"]);
}

#[test]
fn debar_line_matches_kotlin() {
    let debarred = |status: &str, percent: &str| {
        let mut r = record("ETH", "6", "9", percent);
        r.debar_status = status.into();
        courses(vec![r]).remove(0)
    };
    let c = debarred("CAT - II : Debarred & Permitted", "67%");
    assert_eq!(
        c.debar_line.as_deref(),
        Some("Debarred for CAT-II · Permitted · Clear for FAT at 3 more")
    );
    let c = debarred("CAT - I : Debarred", "80%");
    assert_eq!(
        c.debar_line.as_deref(),
        Some("Debarred for CAT-I · Clear for CAT-II")
    );
    assert_eq!(debarred("-", "67%").debar_line, None);
}

/// A history of `statuses`, newest first, one class a day.
fn detail(statuses: &[Option<ClassStatus>]) -> AttendanceDetail {
    let records = statuses
        .iter()
        .enumerate()
        .map(|(i, status)| raw::FullAttendanceRecord {
            serial: "1".into(),
            date: format!("{:02}-11-2025", 20 - i),
            slot: "A1".into(),
            day_time: "WED / 08:00-08:50".into(),
            status: match status {
                Some(P) => "Present",
                Some(OD) => "On Duty",
                Some(A) => "Absent",
                Some(ClassStatus::Other) => "Cancelled",
                None => "Not Posted",
            }
            .into(),
            remark: String::new(),
        })
        .collect();
    AttendanceDetail::new(
        courses(vec![record("ETH", "4", "5", "80%")]).remove(0),
        FullAttendanceData {
            records,
            semester_id: "AP2025262".into(),
            update_time: 9,
            course_id: "AM_SWE1002_00200".into(),
            course_type: "ETH".into(),
        },
        Vec::new(),
    )
}

/// The day of the `i`th class (newest first) in [`detail`].
fn day_of(i: usize) -> i32 {
    history_day(&format!("{:02}-11-2025", 20 - i)).unwrap()
}

#[test]
fn detail_counts_and_streak() {
    let d = detail(&[Some(P), Some(OD), Some(P), Some(A), Some(P), None]);
    assert_eq!((d.present, d.on_duty, d.absent, d.streak), (3, 1, 1, 3));
    assert_eq!(d.last_missed, Some(day_of(3)));
    let d = detail(&[None, Some(A), Some(P)]);
    assert_eq!((d.streak, d.last_missed), (0, Some(day_of(1))));
    assert_eq!(detail(&[Some(P)]).last_missed, None);
}

#[test]
fn longest_run_skips_unposted_and_orders_by_time() {
    let run = longest_run(&[
        (4, Some(480), Some(P)),
        (1, Some(540), None),
        (3, Some(480), Some(A)),
        (2, Some(480), Some(OD)),
        (1, Some(480), Some(P)),
    ]);
    assert_eq!(run, Some((2, 1, 2)));
    assert_eq!(longest_run(&[(1, None, Some(A)), (2, None, None)]), None);
}

#[test]
fn cancelled_classes_neither_extend_nor_break_a_run() {
    let run = longest_run(&[
        (1, Some(480), Some(P)),
        (2, Some(480), Some(ClassStatus::Other)),
        (3, Some(480), Some(OD)),
        (4, Some(480), Some(A)),
    ]);
    assert_eq!(run, Some((2, 1, 3)));
    let d = detail(&[Some(P), Some(ClassStatus::Other), Some(P), Some(A)]);
    assert_eq!(d.streak, 2);
}

#[test]
fn held_classes_never_read_no_classes_yet() {
    // VTOP's total counts classes held but not posted; nothing posted is still "Attend".
    for (kind, total) in [("ETH", "2"), ("ETH", "1"), ("ELA", "2"), ("ELA", "1")] {
        for percent in ["", "0", "0%"] {
            let course = courses(vec![record(kind, "0", total, percent)]).remove(0);
            assert!(course.counted > 0, "{kind} {total} {percent}");
            assert!(
                course.advice.starts_with("Attend"),
                "{kind} {total} {percent}: {}",
                course.advice
            );
        }
    }
    let none = courses(vec![record("ETH", "0", "0", "")]).remove(0);
    assert_eq!(none.advice, "No classes yet");
}

#[test]
fn next_class_per_course() {
    let slot = |day, start, code: &str, kind| ClassSlot {
        day,
        start,
        end: start + 50,
        code: code.into(),
        name: String::new(),
        kind,
        room: String::new(),
        block: String::new(),
        slot: String::new(),
        faculty: String::new(),
        credits: None,
    };
    let slots = [
        slot(0, 480, "CSE2001", ClassKind::Theory),
        slot(2, 600, "CSE2001", ClassKind::Theory),
        slot(0, 840, "CSE2001", ClassKind::Lab),
        slot(1, 540, "MAT1001", ClassKind::Theory),
    ];
    // Monday 2026-11-02, 10:00 IST (04:30 UTC).
    let monday_10am = 1_793_593_800;
    let next = next_classes(&slots, monday_10am, 330, None);
    let found = |code: &str, kind| {
        let class = next
            .iter()
            .find(|c| c.code == code && c.kind == kind)
            .unwrap();
        (class.days_ahead, class.start)
    };
    assert_eq!(found("CSE2001", ClassKind::Lab), (0, 840));
    assert_eq!(found("MAT1001", ClassKind::Theory), (1, 540));
    // Monday's 08:00 has passed, so Wednesday's comes next.
    assert_eq!(found("CSE2001", ClassKind::Theory), (2, 600));
    assert_eq!(next[0].kind, ClassKind::Lab);
    // Only class already over today: same weekday next week.
    let late = next_classes(&slots[..1], monday_10am, 330, None);
    assert_eq!((late[0].days_ahead, late[0].start), (7, 480));
}

#[test]
fn timetable_days_follow_weekdays() {
    let slot = |day, start| ClassSlot {
        day,
        start,
        end: start + 50,
        code: "CSE2001".into(),
        name: String::new(),
        kind: ClassKind::Theory,
        room: String::new(),
        block: String::new(),
        slot: String::new(),
        faculty: String::new(),
        credits: None,
    };
    let slots = [slot(0, 480), slot(0, 600), slot(2, 540)];
    // Epoch day 20759 is Monday 2026-11-02.
    let days = timetable_days(&slots, 20_759, 3, None);
    assert_eq!(
        days.iter().map(|day| day.day).collect::<Vec<_>>(),
        [20_759, 20_760, 20_761]
    );
    assert_eq!(days[0].classes.len(), 2);
    assert!(days[0].holds_theory && !days[1].holds_theory);
    assert_eq!(days[2].classes[0].slot.start, 540);
    assert_eq!(days[2].classes[0].state, ClassDay::Held);
}

fn class(day: i32, start: i32, code: &str, kind: ClassKind) -> ClassSlot {
    ClassSlot {
        day,
        start,
        end: start + 50,
        code: code.into(),
        name: String::new(),
        kind,
        room: String::new(),
        block: String::new(),
        slot: String::new(),
        faculty: String::new(),
        credits: None,
    }
}

#[test]
fn the_calendar_shapes_timetable_days() {
    use crate::calendar::{SemesterCalendar, tests::fall_2026};
    let slots = [
        class(1, 600, "CSE1001", ClassKind::Lab),
        class(3, 480, "CSE1001", ClassKind::Theory),
        class(3, 840, "CSE1001", ClassKind::Lab),
        class(4, 480, "MAT1001", ClassKind::Theory),
        class(5, 600, "MAT1001", ClassKind::Theory),
    ];
    let calendar = SemesterCalendar::of(&fall_2026());
    let monday = epoch_day("2026-11-02").unwrap();
    let days = timetable_days(&slots, monday, 15, Some(&calendar));
    let on = |date: &str| &days[(epoch_day(date).unwrap() - monday) as usize];
    let states = |date: &str| on(date).classes.iter().map(|c| c.state).collect::<Vec<_>>();

    let mon = on("2026-11-02");
    assert_eq!(
        (mon.mark, mon.title.as_deref(), mon.holds_theory),
        (
            Some(CalendarMark::NoClasses),
            Some("No instructional day"),
            false
        )
    );
    // Lab FAT week: the course's first lab of it is the exam, its later
    // one that week does not meet; theory carries on.
    assert_eq!(states("2026-11-03"), [ClassDay::LabFat]);
    assert_eq!(states("2026-11-05"), [ClassDay::Held, ClassDay::Off]);
    assert_eq!(on("2026-11-05").mark, Some(CalendarMark::LabFat));
    assert!(on("2026-11-05").holds_theory);
    // Deepavali: no classes.
    let sat = on("2026-11-07");
    assert_eq!(states("2026-11-07"), [ClassDay::Off]);
    assert_eq!(
        (sat.mark, sat.title.as_deref(), sat.holds_theory),
        (Some(CalendarMark::Holiday), Some("Deepavali"), false)
    );
    // After the Lab FAT, labs stop.
    assert_eq!(states("2026-11-12"), [ClassDay::Held, ClassDay::Off]);
    // Fri 6, Thu 12, Fri 13 and Sat 14 still have classes.
    assert_eq!(on("2026-11-05").class_days_left, 4);
    assert_eq!(on("2026-11-14").class_days_left, 0);
    assert!(on("2026-11-14").last_class_day && !on("2026-11-13").last_class_day);
    assert!(on("2026-11-12").classes[0].last_class);
    assert!(!on("2026-11-06").classes[0].last_class && on("2026-11-14").classes[0].last_class);
    assert_eq!(
        on("2026-11-05").upcoming_exam,
        Some(ExamStart {
            name: "FAT".into(),
            day: epoch_day("2026-11-16").unwrap()
        })
    );
    let exam = on("2026-11-16");
    assert_eq!(
        (exam.mark, exam.title.as_deref()),
        (Some(CalendarMark::Exam), Some("FAT"))
    );
    assert!(exam.classes.is_empty() && !exam.holds_theory);
}

#[test]
fn a_working_day_follows_the_weekday_it_names() {
    use crate::calendar::{SemesterCalendar, tests::entry};
    let data = raw::AcademicCalendarData {
        entries: vec![
            entry("2026-11-13", "Instructional Day", "WorkingDay"),
            entry("2026-11-14", "Instructional Day", "Tuesday Day Order"),
            entry("2026-11-15", "Holiday", "Holiday"),
        ],
        semester_id: "S".into(),
        update_time: 1,
    };
    let calendar = SemesterCalendar::of(&data);
    let slots = [
        class(1, 540, "PHY1001", ClassKind::Theory),
        class(5, 600, "MAT1001", ClassKind::Theory),
    ];
    let saturday = epoch_day("2026-11-14").unwrap();
    let days = timetable_days(&slots, saturday, 1, Some(&calendar));
    assert_eq!(days[0].classes.len(), 1);
    assert_eq!(days[0].classes[0].slot.code, "PHY1001");
    assert_eq!(days[0].classes[0].state, ClassDay::Held);
    assert_eq!(days[0].title.as_deref(), Some("Tuesday Day Order"));
    // Outside the listed days the plain timetable applies.
    let later = timetable_days(&slots, saturday + 7, 1, Some(&calendar));
    assert_eq!(later[0].classes[0].slot.code, "MAT1001");
    assert_eq!(later[0].mark, None);
}

#[test]
fn epoch_days_round_trip() {
    for day in [-1, 0, 20_759, 20_762, 11_016] {
        assert_eq!(epoch_day(&civil_date(i64::from(day))), Some(day));
    }
    assert_eq!(epoch_day("2026-11-02"), Some(20_759));
    assert_eq!(epoch_day("2026-13-02"), None);
    assert_eq!(weekday(20_759), 0);
}

#[test]
fn labs_merge_and_slots_sort() {
    let slot = |day: &str, slot: &str, start: &str, end: &str, kind| raw::TimetableSlot {
        serial: "1".into(),
        day: day.into(),
        slot: slot.into(),
        course_code: "CSE2001".into(),
        course_type: "ELA".into(),
        room_no: "107".into(),
        block: "AB1".into(),
        start_time: start.into(),
        end_time: end.into(),
        name: "Data Structures".into(),
        kind,
        faculty: "Dr. Test - SCOPE".into(),
        credits: "1.0".into(),
    };
    let data = TimetableData {
        slots: vec![
            slot("TUE", "A1", "08:00", "08:50", raw::ClassKind::Theory),
            slot("MON", "L2", "14:50", "15:40", raw::ClassKind::Lab),
            slot("MON", "L1", "14:00", "14:50", raw::ClassKind::Lab),
        ],
        courses: vec![],
        semester_id: "AP2026".into(),
        update_time: 1,
    };
    let slots = Timetable::from(data).slots;
    assert_eq!(slots.len(), 2);
    assert_eq!((slots[0].day, slots[0].start, slots[0].end), (0, 840, 940));
    assert_eq!(slots[0].slot, "L1+L2");
    assert_eq!((slots[1].day, slots[1].credits), (1, Some(1.0)));
}
