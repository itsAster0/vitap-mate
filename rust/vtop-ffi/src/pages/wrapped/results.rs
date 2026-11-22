//! Wrapped's results slides: marks, assessments, exams, grades and CGPA.

use super::*;

/// Assessment kinds, in the order [`assessment_kind`] tries them.
pub(super) const KINDS: [&str; 8] = [
    "quizzes",
    "CATs",
    "FATs",
    "lab experiments",
    "digital assessments",
    "reviews and reports",
    "assignments",
    "other",
];
/// Papers the exam crunch's week needs.
pub(super) const CRUNCH_MIN: i32 = 3;

/// The CAT a marks title names, as it is named: "CAT" for a semester's only one (freshers'
/// first semester), else "CAT-I", "CAT-II", ... ("Continuous Assessment Test - II", "CAT1",
/// "CAT 2 (Theory)").
pub(super) fn cat_of(title: &str) -> Option<String> {
    let lower = title
        .trim()
        .to_lowercase()
        .replace(['\u{2013}', '\u{2014}'], "-");
    let rest = lower
        .strip_prefix("continuous assessment test")
        .or_else(|| lower.strip_prefix("cat"))?;
    // The number, before any note in brackets.
    let number = rest
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(['-', '_', ' '])
        .trim();
    if number.is_empty() {
        return Some("CAT".to_string());
    }
    Some(exam_name(&format!("cat {number}"))).filter(|name| name.starts_with("CAT-"))
}

pub(super) fn marks(marks: &Marks) -> Option<WrappedMarks> {
    let mut best_cat: Option<(f64, WrappedCat)> = None;
    let mut rises: Vec<(&str, f64)> = Vec::new();
    for course in &marks.courses {
        // The course's CATs as named, each its first scored attempt.
        let cats: Vec<(String, f64)> = course
            .assessments
            .iter()
            .filter(|a| !a.reevaluation)
            .filter_map(|a| Some((cat_of(&a.title)?, a.percent?)))
            .fold(Vec::new(), |mut cats, (exam, percent)| {
                if !cats.iter().any(|(seen, _)| *seen == exam) {
                    cats.push((exam, percent));
                }
                cats
            });
        let cat = |exam: &str| cats.iter().find(|(name, _)| name == exam).map(|c| c.1);
        for (exam, percent) in &cats {
            if best_cat.as_ref().is_none_or(|(best, _)| percent > best) {
                best_cat = Some((
                    *percent,
                    WrappedCat {
                        course: course.name.clone(),
                        exam: exam.clone(),
                        percent: percent.round() as i32,
                    },
                ));
            }
        }
        if let (Some(one), Some(two)) = (cat("CAT-I"), cat("CAT-II")) {
            rises.push((&course.name, two - one));
        }
    }
    let best_course = marks
        .courses
        .iter()
        .filter(|course| course.max_weighted >= 50.0)
        .filter_map(|course| Some((course, course.percent?)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let report = WrappedMarks {
        best_cat: best_cat.map(|(_, cat)| cat),
        best_course: best_course.map(|(course, _)| course.name.clone()),
        best_course_percent: best_course.map_or(0, |(_, percent)| percent.round() as i32),
        cat_change: (!rises.is_empty())
            .then(|| (rises.iter().map(|r| r.1).sum::<f64>() / rises.len() as f64).round() as i32),
        most_improved: rises
            .iter()
            .filter(|r| r.1 > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|r| r.0.to_string()),
    };
    (report.best_cat.is_some() || report.best_course.is_some() || report.cat_change.is_some())
        .then_some(report)
}

/// A marks title's kind of [`KINDS`], the first that matches ignoring case: "QUIZ1" is
/// quizzes, "Exp1+2" and "Final LAB Report" lab experiments, "DA-1" a digital assessment.
pub(super) fn assessment_kind(title: &str) -> &'static str {
    let lower = title.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| lower.contains(word));
    let da = lower
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word == "da");
    let rules = [
        has(&["quiz"]),
        cat_of(title).is_some() || title.trim().starts_with("CAT"),
        has(&["fat"]),
        has(&["exp", "lab"]),
        da || has(&["digital assessment"]),
        has(&["review", "project", "report"]),
        has(&["assignment"]),
    ];
    rules
        .iter()
        .position(|&hit| hit)
        .and_then(|at| KINDS.get(at))
        .or(KINDS.last())
        .copied()
        .unwrap_or_default()
}

/// Posted assessments, re-evaluations left out, with their course's name.
pub(super) fn posted(marks: &Marks) -> impl Iterator<Item = (&str, &Assessment)> {
    marks.courses.iter().flat_map(|course| {
        course
            .assessments
            .iter()
            .filter(|a| a.scored.is_some() && !a.reevaluation)
            .map(move |a| (course.name.as_str(), a))
    })
}

/// The kinds of `titles` with one or more, most first, ties in [`KINDS`] order.
pub(super) fn by_kind<'a>(titles: impl Iterator<Item = &'a str>) -> Vec<WrappedKindCount> {
    let kinds: Vec<&str> = titles.map(assessment_kind).collect();
    let mut counts: Vec<WrappedKindCount> = KINDS
        .iter()
        .map(|&kind| WrappedKindCount {
            kind: kind.to_string(),
            count: kinds.iter().filter(|&&k| k == kind).count() as i32,
        })
        .filter(|kind| kind.count > 0)
        .collect();
    counts.sort_by_key(|kind| std::cmp::Reverse(kind.count));
    counts
}

/// None without a full mark.
pub(super) fn full_marks(marks: &Marks) -> Option<WrappedFullMarks> {
    let full: Vec<(&str, &Assessment)> = posted(marks)
        .filter(|(_, a)| a.max > 0.0 && a.scored.is_some_and(|scored| scored >= a.max))
        .collect();
    let (course, top_count) = most(full.iter().map(|(name, _)| (name.to_string(), 1)))?;
    Some(WrappedFullMarks {
        count: full.len() as i32,
        of: posted(marks).count() as i32,
        by_kind: by_kind(full.iter().map(|(_, a)| a.title.as_str())),
        top_course: (top_count > 1).then_some(course),
        top_count,
    })
}

/// None without a posted assessment.
pub(super) fn assessments(marks: &Marks) -> Option<WrappedAssessments> {
    let total = posted(marks).count() as i32;
    (total > 0).then(|| WrappedAssessments {
        total,
        by_kind: by_kind(posted(marks).map(|(_, a)| a.title.as_str())),
    })
}

/// Papers dated before `today`: the ones already sat.
pub(super) fn exams(exams: &Exams, today: i32) -> Option<WrappedExams> {
    let sat = |group: &'_ crate::pages::exams::ExamGroup| {
        group
            .papers
            .iter()
            .filter(|paper| paper.day.is_some_and(|day| day < today))
            .count() as i32
    };
    let papers = |matches: fn(&str) -> bool| -> i32 {
        exams
            .groups
            .iter()
            .filter(|group| matches(&group.name))
            .map(sat)
            .sum()
    };
    let (cats, fats) = (
        papers(|n| n == "CAT" || n.starts_with("CAT-")),
        papers(|n| n == "FAT"),
    );
    let venues = exams
        .groups
        .iter()
        .flat_map(|group| &group.papers)
        .filter(|paper| paper.day.is_some_and(|day| day < today) && !paper.venue.is_empty())
        .map(|paper| (paper.venue.clone(), 1));
    (cats + fats > 0).then(|| WrappedExams {
        cats,
        fats,
        top_venue: most(venues).map(|(venue, _)| venue),
    })
}

/// Papers dated before `today`, across every exam: the busiest 7 days from a paper's day, the
/// earliest of equals; none under [`CRUNCH_MIN`] papers in it.
pub(super) fn crunch(exams: &Exams, today: i32) -> Option<WrappedCrunch> {
    let mut days: Vec<i32> = exams
        .groups
        .iter()
        .flat_map(|group| &group.papers)
        .filter_map(|paper| paper.day.filter(|&day| day < today))
        .collect();
    days.sort_unstable();
    let on = |day: i32| days.iter().filter(|&&d| d == day).count() as i32;
    let (papers, first_day) = days
        .iter()
        .map(|&from| ((from..from + 7).map(on).sum::<i32>(), from))
        .max_by_key(|&(n, from)| (n, std::cmp::Reverse(from)))
        .filter(|&(n, _)| n >= CRUNCH_MIN)?;
    let (most_in_a_day, most_day) = days
        .iter()
        .map(|&day| (on(day), day))
        .max_by_key(|&(n, day)| (n, std::cmp::Reverse(day)))?;
    let by_day = (first_day..first_day + 7).map(on).collect();
    days.dedup();
    Some(WrappedCrunch {
        papers,
        first_day,
        last_day: first_day + 6,
        by_day,
        most_in_a_day,
        most_day,
        exam_days: days.len() as i32,
    })
}

/// The semester's course codes, uppercase.
pub(super) fn grade_codes(grades: &Grades) -> HashSet<String> {
    grades
        .courses
        .iter()
        .map(|course| course.code.trim().to_uppercase())
        .collect()
}

/// A session's exam month ("Nov-2025") as months since year 0; none for "Other".
pub(super) fn session_month(session: &GradeSession) -> Option<i32> {
    let month = session.month.trim();
    if month.is_empty() {
        return None;
    }
    day_month(records::history_day(&format!("01-{month}"))?)
}

/// `day`'s month, as [`session_month`].
pub(super) fn day_month(day: i32) -> Option<i32> {
    let date = records::civil_date(i64::from(day));
    let mut parts = date.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: i32 = parts.next()?.parse().ok()?;
    Some(year * 12 + month - 1)
}

/// The indices in `history.sessions` of this semester's exam session and of the one before it.
/// This semester's is the newest dated session after the month classes started (`first_day`),
/// or in it when classes also ended that month (`last_day`, a short summer semester), that
/// holds one of its course `codes`, so an earlier attempt of a re-registered course stays in its
/// own session. The one before is the newest dated session older than that (or than every
/// session, while this semester isn't in the history yet).
pub(super) fn sessions(
    history: &GradeHistory,
    codes: &HashSet<String>,
    first_day: i32,
    last_day: i32,
) -> (Option<usize>, Option<usize>) {
    let dated = || {
        history
            .sessions
            .iter()
            .enumerate()
            .filter_map(|(at, session)| Some((at, session_month(session)?, session)))
    };
    let (started, ended) = (day_month(first_day), day_month(last_day));
    let this = dated()
        .filter(|&(_, month, session)| {
            started.is_some_and(|started| {
                month > started || (month == started && ended == Some(started))
            }) && session
                .courses
                .iter()
                .any(|c| codes.contains(&c.code.trim().to_uppercase()))
        })
        .max_by_key(|&(at, month, _)| (month, at))
        .map(|(at, month, _)| (at, month));
    let before = dated()
        .filter(|&(at, month, _)| this.is_none_or(|(this, newest)| at != this && month < newest))
        .max_by_key(|&(at, month, _)| (month, at))
        .map(|(at, _, _)| at);
    (this.map(|(at, _)| at), before)
}

/// None until VTOP posts a grade with points. From the semester's own grades page, else from
/// `this`, its session in the grade history (an old semester's page isn't cached); the S record
/// needs `this` too.
pub(super) fn grades(
    own: Option<&Grades>,
    history: Option<&GradeHistory>,
    this: Option<usize>,
) -> Option<WrappedGrades> {
    let session = history
        .zip(this)
        .and_then(|(history, this)| history.sessions.get(this));
    let graded = own.filter(|own| own.courses.iter().any(|c| grade_points(&c.grade).is_some()));
    let (gpa, mix) = match graded {
        Some(own) => {
            let summary = semester_summary(own.courses.clone(), history.cloned());
            (summary.gpa?, summary.counts)
        }
        None => {
            let session = session?;
            let mix = counts(session.courses.iter().map(|c| c.grade.as_str()));
            (session.gpa?, mix)
        }
    };
    let s_count = mix
        .iter()
        .find(|count| count.grade == "S")
        .map_or(0, |count| count.count);
    // The S grades of each earlier semester (dated exam session) in the history.
    let this_month = session.and_then(session_month);
    let earlier: Vec<i32> = history
        .iter()
        .flat_map(|history| &history.sessions)
        .filter(|session| {
            !session.courses.is_empty()
                && this_month
                    .zip(session_month(session))
                    .is_some_and(|(this, m)| m < this)
        })
        .map(|session| session.courses.iter().filter(|c| c.grade == "S").count() as i32)
        .collect();
    Some(WrappedGrades {
        gpa,
        mix,
        s_count,
        s_record: s_count > 0 && !earlier.is_empty() && earlier.iter().all(|&n| s_count > n),
    })
}

/// `this` is this semester's session in the grade history. The CGPA as it was then: before over
/// the dated sessions older than this one (none for a first semester), after over those and
/// this one, so later semesters are left out of an old semester's story. After is VTOP's own
/// while no later session exists.
pub(super) fn cgpa(history: &GradeHistory, this: usize) -> Option<WrappedCgpa> {
    let session = history.sessions.get(this)?;
    let month = session_month(session)?;
    let newest = !history
        .sessions
        .iter()
        .any(|s| session_month(s).is_some_and(|m| m > month));
    let earned = session
        .courses
        .iter()
        .filter(|course| passed(&course.grade))
        .filter_map(|course| course.credits.filter(|credit| *credit > 0.0))
        .sum();
    let vtop = |raw: &str| raw.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    let (after, total) = if newest {
        (
            vtop(&history.cgpa)?,
            vtop(&history.credits_earned).unwrap_or(0.0),
        )
    } else {
        let (cgpa, total) = cumulative(history, |m| m <= month);
        (cgpa?, total)
    };
    Some(WrappedCgpa {
        before: cumulative(history, |m| m < month).0,
        after,
        credits_semester: earned,
        credits_total: total,
    })
}

/// A grade that earns the course's credits.
pub(super) fn passed(grade: &str) -> bool {
    !matches!(grade, "F" | "N" | "" | "-")
}

/// The CGPA (none without a graded credit) and credits earned over the dated sessions whose
/// month `counts`. A course taken more than once counts its latest attempt only, as VTOP's CGPA
/// does; N and grades without points are left out of the CGPA, as in a session's GPA.
pub(super) fn cumulative(
    history: &GradeHistory,
    counts: impl Fn(i32) -> bool,
) -> (Option<f64>, f64) {
    let mut latest: HashMap<String, (i32, &crate::pages::grades::HistoryCourse)> = HashMap::new();
    for session in &history.sessions {
        let Some(month) = session_month(session).filter(|&m| counts(m)) else {
            continue;
        };
        for course in &session.courses {
            let code = course.code.trim().to_uppercase();
            if latest.get(&code).is_none_or(|&(seen, _)| month >= seen) {
                latest.insert(code, (month, course));
            }
        }
    }
    let (mut credits, mut points, mut earned) = (0.0, 0.0, 0.0);
    for (_, course) in latest.values() {
        let Some(credit) = course.credits.filter(|credit| *credit > 0.0) else {
            continue;
        };
        if passed(&course.grade) {
            earned += credit;
        }
        if let Some(value) = grade_points(&course.grade).filter(|_| course.grade != "N") {
            credits += credit;
            points += credit * value;
        }
    }
    ((credits > 0.0).then(|| points / credits), earned)
}

/// `value` in hundredths, as the story draws a GPA.
pub(super) fn hundredths(value: f64) -> f64 {
    (value * 100.0).round()
}
