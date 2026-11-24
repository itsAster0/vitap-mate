use vtop_core::types::{
    GradeCourseRecord, GradeHistoryAttempt, GradeHistoryCgpa, GradeHistoryStudentInfo, GradeRange,
    TimetableCourse,
};

use super::plan::{parse_credits, plan_courses};
use super::*;

fn record(code: &str, kind: &str, credits: &str, grade: &str, month: &str) -> GradeHistoryRecord {
    GradeHistoryRecord {
        serial: String::new(),
        course_code: code.into(),
        course_title: format!("{code} title"),
        course_type: kind.into(),
        credits: credits.into(),
        grade: grade.into(),
        exam_month: month.into(),
        result_declared: String::new(),
        course_distribution: "PC".into(),
        attempts: Vec::new(),
    }
}

fn attempt(kind: &str, grade: &str, declared: &str) -> GradeHistoryAttempt {
    GradeHistoryAttempt {
        course_code: String::new(),
        course_title: String::new(),
        course_type: kind.into(),
        credits: "4".into(),
        grade: grade.into(),
        exam_month: String::new(),
        result_declared: declared.into(),
    }
}

fn history(records: Vec<GradeHistoryRecord>) -> GradeHistory {
    GradeHistory::from(GradeHistoryData {
        student: GradeHistoryStudentInfo {
            reg_no: " 22BCE7000 ".into(),
            name: "Student".into(),
            programme_branch: String::new(),
            programme_mode: String::new(),
            study_system: String::new(),
            gender: String::new(),
            year_joined: String::new(),
            edu_status: String::new(),
            school: String::new(),
            campus: String::new(),
        },
        records,
        cgpa: GradeHistoryCgpa {
            credits_registered: "10.5".into(),
            credits_earned: "10.0".into(),
            cgpa: "9.12".into(),
            s_grades: "1".into(),
            a_grades: "2".into(),
            b_grades: "0".into(),
            c_grades: "0".into(),
            d_grades: "0".into(),
            e_grades: "0".into(),
            f_grades: "1".into(),
            n_grades: "".into(),
        },
        update_time: 5,
    })
}

fn course(code: &str, grade: &str) -> GradeCourse {
    GradeCourse {
        course_id: format!("AP{code}"),
        code: code.into(),
        title: String::new(),
        course_type: String::new(),
        grading_type: String::new(),
        total: String::new(),
        grade: grade.into(),
    }
}

#[test]
fn sessions_group_by_exam_month_with_their_gpa() {
    let mut embedded = record("CSE1001", "ETL", "4", "S", "Jan-2023");
    embedded.attempts = vec![
        attempt("ETL", "S", ""),
        attempt("ETH", "S", "10-Jan-2023"),
        attempt("ELA", "A", "12-Jan-2023"),
    ];
    let history = history(vec![
        embedded,
        record("MAT1001", "TH", "3", "A", "May-2023"),
        record("HUM1001", "TH", "2", "P", "May-2023"),
        record("PHY1001", "TH", "4", "N", "May-2023"),
        record("ENG1001", "TH", "1.5", "B", "Jan-2023"),
    ]);
    let [jan, may] = &history.sessions[..] else {
        panic!("two sessions");
    };
    assert_eq!(
        (jan.label.as_str(), jan.short.as_str()),
        ("Jan 2023", "Jan '23")
    );
    // (4·10 + 1.5·8) / 5.5
    assert_eq!(jan.gpa.map(|g| (g * 1e4).round() / 1e4), Some(9.4545));
    // P and N left out: only MAT1001.
    assert_eq!(may.gpa, Some(9.0));
    let components: Vec<_> = jan.courses[0]
        .components
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(components, ["Theory component", "Lab component"]);
    assert_eq!(history.reg_no, "22BCE7000");
    assert_eq!(
        (
            history.credits_earned.as_str(),
            history.credits_registered.as_str()
        ),
        ("10", "10.5")
    );
    let mix: Vec<_> = history
        .mix
        .iter()
        .map(|c| (c.grade.as_str(), c.count))
        .collect();
    assert_eq!(mix, [("S", 1), ("A", 2), ("F", 1)]);
    let filters: Vec<_> = history
        .filters
        .iter()
        .map(|c| (c.grade.as_str(), c.count))
        .collect();
    assert_eq!(filters, [("S", 1), ("A", 1), ("B", 1), ("N", 1), ("P", 1)]);
}

#[test]
fn a_reregistered_course_keeps_one_row_and_unique_keys() {
    let mut failed_then_passed = record("MAT1001", "TH", "4", "A", "May-2023");
    failed_then_passed.attempts = vec![attempt("TH", "F", ""), attempt("TH", "A", "")];
    let history = history(vec![
        failed_then_passed,
        record("MAT1001", "TH", "4", "A", "Jan-2024"),
    ]);
    let courses: Vec<_> = history.sessions.iter().flat_map(|s| &s.courses).collect();
    assert!(courses[0].components.is_empty());
    assert_eq!(
        (courses[0].key.as_str(), courses[1].key.as_str()),
        ("MAT1001", "MAT1001.2")
    );
}

#[test]
fn semester_gpa_weights_by_history_credits() {
    let history = history(vec![
        record("CSE2001", "ETL", "4", "A", "Jan-2026"),
        record("MAT1001", "TH", "3", "B", "Jan-2026"),
        record("HUM1001", "TH", "2", "P", "Jan-2026"),
        record("SOC1001", "TH", "0", "N", "Jan-2026"),
    ]);
    let summary = semester_summary(
        vec![
            course("CSE2001", "A"),
            course("mat1001 ", "B"),
            course("HUM1001", "P"),
            course("SOC1001", "N"),
            course("NEW1001", "S"),
        ],
        Some(history),
    );
    // (4·9 + 3·8) / 7; P has no points, SOC has no credits, NEW no history.
    assert_eq!(summary.gpa.map(|g| (g * 1e4).round() / 1e4), Some(8.5714));
    assert_eq!(summary.credits, 7.0);
    assert_eq!(
        summary.course_credits,
        [Some(4.0), Some(3.0), Some(2.0), Some(0.0), None]
    );
    let counts: Vec<_> = summary
        .counts
        .iter()
        .map(|c| (c.grade.as_str(), c.count))
        .collect();
    assert_eq!(counts, [("S", 1), ("A", 1), ("B", 1), ("N", 1), ("P", 1)]);
    // N counts as 0 on this page, as in Flutter.
    let n = semester_summary(
        vec![course("A1", "S"), course("N1", "N")],
        Some(GradeHistory::from(history_data(vec![
            record("A1", "TH", "3", "S", ""),
            record("N1", "TH", "3", "N", ""),
        ]))),
    );
    assert_eq!(n.gpa, Some(5.0));
    assert_eq!(semester_summary(vec![course("A1", "S")], None).gpa, None);
}

fn history_data(records: Vec<GradeHistoryRecord>) -> GradeHistoryData {
    GradeHistoryData {
        student: GradeHistoryStudentInfo {
            reg_no: String::new(),
            name: String::new(),
            programme_branch: String::new(),
            programme_mode: String::new(),
            study_system: String::new(),
            gender: String::new(),
            year_joined: String::new(),
            edu_status: String::new(),
            school: String::new(),
            campus: String::new(),
        },
        records,
        cgpa: GradeHistoryCgpa {
            credits_registered: String::new(),
            credits_earned: String::new(),
            cgpa: String::new(),
            s_grades: String::new(),
            a_grades: String::new(),
            b_grades: String::new(),
            c_grades: String::new(),
            d_grades: String::new(),
            e_grades: String::new(),
            f_grades: String::new(),
            n_grades: String::new(),
        },
        update_time: 0,
    }
}

#[test]
fn planner_projects_the_cgpa_and_what_a_target_needs() {
    let planned = |grades: &[(f64, &str)]| {
        grades
            .iter()
            .map(|(credits, grade)| PlannedCourse {
                credits: *credits,
                grade: grade.to_string(),
            })
            .collect::<Vec<_>>()
    };
    let plan = plan_gpa(
        planned(&[(4.0, "S"), (3.0, "B"), (1.0, "F")]),
        "8.0".into(),
        "40".into(),
        None,
    );
    // (40 + 24 + 0) / 8
    assert_eq!(plan.semester_gpa, 8.0);
    assert_eq!(plan.projected, Some(8.0));
    assert_eq!(plan.target, 8.25);
    // (8.25·48 − 8·40) / 8
    assert_eq!(plan.needed, Some(9.5));
    assert_eq!(plan.outlook, Some(TargetOutlook::Needs));
    assert_eq!(plan.letter, "B");

    let plan = plan_gpa(
        planned(&[(4.0, "S")]),
        "9,1".into(),
        "100 credits".into(),
        Some(9.1),
    );
    assert_eq!(plan.current, Some(9.1));
    assert!(plan.delta > 0.0);
    assert_eq!(plan.outlook, Some(TargetOutlook::OnTrack));
    let plan = plan_gpa(
        planned(&[(4.0, "S")]),
        "9.0".into(),
        "100".into(),
        Some(8.0),
    );
    assert_eq!(plan.outlook, Some(TargetOutlook::Reached));
    let plan = plan_gpa(
        planned(&[(1.0, "S")]),
        "6.0".into(),
        "100".into(),
        Some(9.0),
    );
    assert_eq!(plan.outlook, Some(TargetOutlook::OutOfReach));
    // A 9.95 CGPA tops out at 10; no CGPA, no projection.
    assert_eq!(
        plan_gpa(Vec::new(), "9.95".into(), "10".into(), None).target,
        10.0
    );
    let none = plan_gpa(planned(&[(4.0, "A")]), "11".into(), "".into(), None);
    assert_eq!(
        (none.current, none.projected, none.needed, none.target),
        (None, None, None, 9.0)
    );
    assert_eq!(
        plan_gpa(Vec::new(), String::new(), String::new(), None).semester_gpa,
        0.0
    );
}

#[test]
fn details_split_sections_and_shorten_cutoffs() {
    let mark = |title: &str, weightage: &str, weighted: &str| GradeDetailMark {
        serial: String::new(),
        mark_title: title.into(),
        max_mark: "50".into(),
        weightage: weightage.into(),
        status: String::new(),
        scored_mark: "40".into(),
        weightage_mark: weighted.into(),
    };
    let range = |grade: &str, range: &str| GradeRange {
        grade: grade.into(),
        range: range.into(),
    };
    let details = GradeDetails::from(GradeDetailsData {
        semester_id: "AP1".into(),
        course_id: "C1".into(),
        class_number: String::new(),
        class_course_type: "TH".into(),
        grand_total: String::new(),
        marks: vec![
            mark("ETH • CAT-1", "15", "12.30"),
            mark("ETH • CAT-1 Re-Evaluation", "15", "13.5"),
            mark("ELA • Lab FAT", "50", "45.00"),
            mark("Quiz", "10", ""),
        ],
        grade_ranges: vec![
            range("s", ">= 90"),
            range("A", ">=83 and <88"),
            range("F", "< 50"),
            range("E", "#"),
        ],
        update_time: 0,
    });
    let sections: Vec<_> = details
        .sections
        .iter()
        .map(|s| (s.title.as_str(), s.total.as_str(), s.marks.len()))
        .collect();
    assert_eq!(
        sections,
        [
            ("ETH", "13.5 / 15", 2),
            ("ELA", "45 / 50", 1),
            ("TH", "", 1)
        ]
    );
    assert_eq!(details.sections[0].marks[1].title, "CAT-1 Re-Evaluation");
    assert_eq!(details.sections[0].marks[0].band, ScoreBand::Strong);
    let cutoffs: Vec<_> = details
        .cutoffs
        .iter()
        .map(|c| (c.grade.as_str(), c.range.as_str()))
        .collect();
    assert_eq!(cutoffs, [("S", "≥90"), ("A", "83–88"), ("F", "<50")]);
    assert!(details.placeholder);
}

#[test]
fn grades_sort_by_serial() {
    let row = |serial: &str, code: &str| GradeCourseRecord {
        serial: serial.into(),
        course_code: code.into(),
        course_title: String::new(),
        course_type: String::new(),
        grading_type: String::new(),
        grand_total: String::new(),
        grade: " a ".into(),
        course_id: String::new(),
    };
    let grades = Grades::new(
        GradeViewData {
            courses: vec![row("10", "B"), row("2", "A")],
            semesters: Vec::new(),
            semester_id: String::new(),
            update_time: 3,
        },
        "AP1",
    );
    let codes: Vec<_> = grades.courses.iter().map(|c| c.code.as_str()).collect();
    assert_eq!(codes, ["A", "B"]);
    assert_eq!(
        (
            grades.semester_id.as_str(),
            grades.courses[0].grade.as_str()
        ),
        ("AP1", "A")
    );
}

#[test]
fn planner_courses_add_up_parts_once() {
    let part = |code: &str, kind: &str, credits: &str| TimetableCourse {
        course_code: code.into(),
        name: "Data Structures".into(),
        course_type: kind.into(),
        credits: credits.into(),
    };
    let courses = plan_courses(&TimetableData {
        slots: Vec::new(),
        courses: vec![
            part("CSE2001", "ETH", "3"),
            part("CSE2001", "ETH", "3"),
            part("cse2001", "ELA", "1"),
            part("X", "TH", "0"),
        ],
        semester_id: String::new(),
        update_time: 0,
    });
    assert_eq!(
        courses,
        [PlanCourse {
            code: "CSE2001".into(),
            name: "Data Structures".into(),
            credits: 4.0
        }]
    );
}

#[test]
fn numbers_read_like_flutter() {
    assert_eq!(parse_credits("1.5 credits"), Some(1.5));
    assert_eq!(parse_credits("2,5"), Some(2.5));
    assert_eq!(parse_credits("0"), None);
    assert_eq!(parse_credits("4."), Some(4.0));
    assert_eq!(short_range("#>=88#"), "≥88");
    assert_eq!(short_range("fail"), "fail");
    assert_eq!(
        (one_decimal(12.25), one_decimal(2.0)),
        ("12.3".to_string(), "2".to_string())
    );
    assert_eq!(plain_number(" x "), "x");
    assert_eq!(short_session("Other"), "Other");
}
