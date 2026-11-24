//! The GPA arithmetic behind the planner: grade points, a session's GPA, credits, and the
//! semester's courses to plan.

use std::collections::{HashMap, HashSet};

use vtop_core::types::{GradeHistoryRecord, TimetableData};

use super::PlanCourse;

/// S 10 ... E 5, F and N 0; none for P and anything else.
pub(crate) fn grade_points(grade: &str) -> Option<f64> {
    Some(match grade.trim().to_uppercase().as_str() {
        "S" => 10.0,
        "A" => 9.0,
        "B" => 8.0,
        "C" => 7.0,
        "D" => 6.0,
        "E" => 5.0,
        "F" | "N" => 0.0,
        _ => return None,
    })
}

/// A session's GPA: N and grades without points left out, as are courses
/// without credits.
pub(super) fn session_gpa<'a>(
    records: impl Iterator<Item = &'a GradeHistoryRecord>,
) -> Option<f64> {
    let (mut credits, mut points) = (0.0, 0.0);
    for record in records {
        let grade = record.grade.trim().to_uppercase();
        let (Some(value), Some(credit)) = (grade_points(&grade), parse_credits(&record.credits))
        else {
            continue;
        };
        if grade == "N" {
            continue;
        }
        credits += credit;
        points += credit * value;
    }
    (credits > 0.0).then(|| points / credits)
}

/// The first number in `raw` ("4", "1.5 credits", "2,5"), when above 0.
pub(super) fn parse_credits(raw: &str) -> Option<f64> {
    let text = raw.trim().replace(',', ".");
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let rest = text.get(start..)?;
    let mut end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if let Some(after) = rest.get(end..).and_then(|after| after.strip_prefix('.')) {
        let fraction = after
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after.len());
        if fraction > 0 {
            end += 1 + fraction;
        }
    }
    rest.get(..end)?.parse().ok().filter(|value| *value > 0.0)
}

/// The letter nearest a GPA.
pub(super) fn letter_for(gpa: f64) -> &'static str {
    match gpa {
        g if g >= 9.5 => "S",
        g if g >= 8.5 => "A",
        g if g >= 7.5 => "B",
        g if g >= 6.5 => "C",
        g if g >= 5.5 => "D",
        g if g >= 4.5 => "E",
        _ => "F",
    }
}

/// The timetable's courses with their parts' credits added up, a part
/// counted once however many slots it has (Flutter's
/// `combineSemesterCourseCredits`).
pub(super) fn plan_courses(data: &TimetableData) -> Vec<PlanCourse> {
    let mut seen = HashSet::new();
    let mut courses: Vec<PlanCourse> = Vec::new();
    let names: HashMap<String, String> = data
        .courses
        .iter()
        .map(|course| {
            (
                course.course_code.trim().to_uppercase(),
                course.name.trim().to_string(),
            )
        })
        .collect();
    for course in &data.courses {
        let code = course.course_code.trim().to_uppercase();
        let kind = course.course_type.trim().to_uppercase();
        let Some(credits) = parse_credits(&course.credits) else {
            continue;
        };
        if code.is_empty() || !seen.insert((code.clone(), kind)) {
            continue;
        }
        match courses.iter_mut().find(|planned| planned.code == code) {
            Some(planned) => planned.credits += credits,
            None => courses.push(PlanCourse {
                name: names.get(&code).cloned().unwrap_or_default(),
                code,
                credits,
            }),
        }
    }
    courses
}
