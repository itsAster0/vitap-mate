//! Grades: a semester's grades with each course's mark breakdown, the grade
//! history across semesters with CGPA, and the GPA planner's arithmetic.
//! Port of the Flutter app's `lib/features/grades/`, `grade_history/` and
//! `gpa/` (`gpa_calculator.dart`, `semester_summary.dart`,
//! `grade_history_page.dart`, `grade_details.dart`).
//!
//! Grade points: S 10, A 9, B 8, C 7, D 6, E 5, F 0, N 0, credit-weighted.
//! P (pass/fail) and other grades carry none and are left out. As in
//! Flutter, a semester's GPA on the grades page counts N as 0 while the
//! history's session GPAs leave N out.

use std::collections::{HashMap, HashSet};
use std::fs;

use tracing::{info, warn};
use vtop_core::spec;
use vtop_core::types::{
    GradeDetailMark, GradeDetailsData, GradeHistoryData, GradeHistoryRecord, GradeViewData,
    SemesterData, TimetableData,
};

mod plan;

pub(crate) use plan::grade_points;
use plan::{letter_for, parse_credits, plan_courses, session_gpa};

use crate::pages::marks::{ScoreBand, is_reevaluation, normalized, strip_reevaluation};
use crate::{Core, CoreError, PAGES, SEMESTERS, TIMETABLE, traced, traced_sync};

pub(crate) const GRADES: &str = spec::GRADES.file;
/// One course's breakdown: `pages/<semester>/grade_details/<course id>.json`.
const DETAILS: &str = spec::GRADE_DETAILS.file;
/// Not per semester: `pages/grade_history.json`.
const HISTORY: &str = spec::GRADE_HISTORY.file;

/// Grades in the order the app lists them; others follow.
const ORDER: [&str; 9] = ["S", "A", "B", "C", "D", "E", "F", "N", "P"];

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Grades {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// By VTOP's serial.
    pub courses: Vec<GradeCourse>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GradeCourse {
    /// VTOP's class id, for [`Core::fetch_grade_details`].
    pub course_id: String,
    pub code: String,
    pub title: String,
    /// "Embedded Theory and Lab", "Theory Only", ...
    pub course_type: String,
    /// "RG" (relative) or "AG" (absolute); may be empty.
    pub grading_type: String,
    /// VTOP's grand total, as given.
    pub total: String,
    /// Uppercase; empty or "-" when none.
    pub grade: String,
}

/// A grade and how many courses got it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GradeCount {
    pub grade: String,
    pub count: i32,
}

/// A semester's GPA from its grades, with credits from the grade history.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SemesterSummary {
    /// None without credits for any graded course.
    pub gpa: Option<f64>,
    /// The credits the GPA counts.
    pub credits: f64,
    /// Each course's credits from the history, in [`Grades::courses`] order.
    pub course_credits: Vec<Option<f64>>,
    /// S, A, ... F, N, P, then any other grade.
    pub counts: Vec<GradeCount>,
}

/// A course's grade cut-offs and marks by section.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GradeDetails {
    pub semester_id: String,
    pub course_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// VTOP's grand total, as given.
    pub grand_total: String,
    pub cutoffs: Vec<GradeCutoff>,
    pub sections: Vec<MarkSection>,
    /// A cut-off came as VTOP's "#" placeholder: fetch again when opened.
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GradeCutoff {
    /// Uppercase.
    pub grade: String,
    /// ">=83 and <88" → "83–88", ">=88" → "≥88", "<50" → "<50".
    pub range: String,
}

/// The marks before "•" in their title ("ETH • CAT-1"), in first-seen order.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MarkSection {
    pub title: String,
    /// Weighted scored / weightage ("44.3 / 55"); empty before any mark.
    pub total: String,
    pub marks: Vec<GradeMark>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GradeMark {
    pub title: String,
    pub scored: String,
    pub max: String,
    pub weighted: String,
    pub weightage: String,
    /// `scored / max`, 0–100.
    pub percent: Option<f64>,
    pub band: ScoreBand,
}

/// Every graded course across semesters, with VTOP's CGPA.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GradeHistory {
    /// Unix seconds.
    pub updated_at: i64,
    pub name: String,
    pub reg_no: String,
    /// VTOP's, as given ("9.12").
    pub cgpa: String,
    /// "10", "10.5".
    pub credits_earned: String,
    pub credits_registered: String,
    /// VTOP's grade counts, S to N, without zeros.
    pub mix: Vec<GradeCount>,
    /// The grades the courses have, with how many: S to N, then others.
    pub filters: Vec<GradeCount>,
    pub course_count: i32,
    /// By exam session, in VTOP's order.
    pub sessions: Vec<GradeSession>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GradeSession {
    /// VTOP's "Jan-2025"; empty for courses without one.
    pub month: String,
    /// "Jan 2025", "Other".
    pub label: String,
    /// "Jan '25", for the GPA trend.
    pub short: String,
    /// The session's credit-weighted GPA; P and N left out.
    pub gpa: Option<f64>,
    pub courses: Vec<HistoryCourse>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct HistoryCourse {
    /// Unique in the history.
    pub key: String,
    pub code: String,
    pub title: String,
    /// "PC", "UE", ...
    pub distribution: String,
    pub credits: Option<f64>,
    /// Uppercase.
    pub grade: String,
    /// The theory, lab and project parts of an embedded course.
    pub components: Vec<GradeComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GradeComponent {
    /// "Theory component", "Lab component", "Project component".
    pub label: String,
    /// When its result was declared, as VTOP gives it.
    pub declared: String,
}

/// A course of this semester for the planner: its parts' credits combined.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PlanCourse {
    pub code: String,
    pub name: String,
    pub credits: f64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PlannedCourse {
    pub credits: f64,
    /// S to F.
    pub grade: String,
}

/// What the planned semester needs for the target CGPA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TargetOutlook {
    /// Reached whatever the semester's grades.
    Reached,
    /// The planned grades get there.
    OnTrack,
    /// Needs [`GpaPlan::needed`].
    Needs,
    /// Needs over 10.
    OutOfReach,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GpaPlan {
    /// 0 without credits.
    pub semester_gpa: f64,
    pub credits: f64,
    /// The CGPA typed in, when it reads as 0–10.
    pub current: Option<f64>,
    /// The CGPA after the semester; none without a current one.
    pub projected: Option<f64>,
    /// `projected - current`.
    pub delta: f64,
    /// The target given, else the next quarter point above the current CGPA
    /// (9 without one).
    pub target: f64,
    /// The semester GPA the target needs; none without a current CGPA or
    /// credits.
    pub needed: Option<f64>,
    pub outlook: Option<TargetOutlook>,
    /// The letter nearest the semester GPA (for its colour).
    pub letter: String,
}

#[uniffi::export]
impl Core {
    /// `semester_id`'s cached grades. With an empty id the picked semester's,
    /// or, while it has none (grades come once a semester ends), the one
    /// before it, as the Flutter app opens the page.
    pub fn cached_grades(&self, flow: String, semester_id: String) -> Option<Grades> {
        traced_sync("cached_grades", &flow, || {
            let (semester, previous) = self.grade_semesters(&semester_id)?;
            let own = self.cached_grade_view(&semester);
            let fallback = || {
                previous
                    .as_deref()
                    .and_then(|previous| self.cached_grade_view(previous))
                    .filter(|grades| !grades.courses.is_empty())
            };
            Ok(match own {
                Some(grades) if !grades.courses.is_empty() => Some(grades),
                own => fallback().or(own),
            }
            .map(logged))
        })
        .ok()
        .flatten()
    }

    /// Fetches `semester_id`'s grades and caches them. With an empty id the
    /// picked semester's, and the one before it while the picked one has
    /// none (that one's cached copy when its fetch fails).
    pub async fn fetch_grades(
        &self,
        flow: String,
        semester_id: String,
    ) -> Result<Grades, CoreError> {
        traced("fetch_grades", &flow, async {
            let (semester, previous) = self.grade_semesters(&semester_id)?;
            let own = self.fetch_grade_view(&semester).await?;
            let Some(previous) = previous.filter(|_| own.courses.is_empty()) else {
                return Ok(logged(own));
            };
            info!(semester = %semester, previous = %previous, "no grades yet, the semester before");
            match self.fetch_grade_view(&previous).await {
                Ok(grades) if !grades.courses.is_empty() => Ok(logged(grades)),
                Ok(_) => Ok(logged(own)),
                Err(error) => {
                    warn!(error = error.kind(), "semester before not fetched");
                    Ok(logged(
                        self.cached_grade_view(&previous)
                            .filter(|grades| !grades.courses.is_empty())
                            .unwrap_or(own),
                    ))
                }
            }
        })
        .await
    }

    /// The cached breakdown of `course_id` in `semester_id`'s grades.
    pub fn cached_grade_details(
        &self,
        flow: String,
        semester_id: String,
        course_id: String,
    ) -> Option<GradeDetails> {
        traced_sync("cached_grade_details", &flow, || {
            let path = self.details_path(&semester_id, &course_id)?;
            Ok(self
                .read_kept::<GradeDetailsData>(&path)
                .map(GradeDetails::from))
        })
        .ok()
        .flatten()
    }

    /// Fetches the breakdown of `course_id` (in `semester_id`'s cached
    /// grades) and caches it.
    pub async fn fetch_grade_details(
        &self,
        flow: String,
        semester_id: String,
        course_id: String,
    ) -> Result<GradeDetails, CoreError> {
        traced("fetch_grade_details", &flow, async {
            let path = self.details_path(&semester_id, &course_id)?;
            let data = self
                .page(
                    &spec::GRADE_DETAILS,
                    Some(&semester_id),
                    &[("course_id", &course_id)],
                )
                .await?;
            if let Some(parent) = self.at(&path).parent() {
                #[expect(
                    clippy::let_underscore_must_use,
                    reason = "a failed mkdir shows up in the write that follows"
                )]
                let _ = fs::create_dir_all(parent);
            }
            self.write(&path, &data);
            let details = GradeDetails::from(data);
            info!(
                cutoffs = details.cutoffs.len(),
                sections = details.sections.len(),
                placeholder = details.placeholder,
                "grade details"
            );
            Ok(details)
        })
        .await
    }

    /// The cached grade history (not per semester).
    pub fn cached_grade_history(&self, flow: String) -> Option<GradeHistory> {
        traced_sync("cached_grade_history", &flow, || {
            Ok(self
                .read_kept::<GradeHistoryData>(&format!("{PAGES}/{HISTORY}"))
                .map(|data| logged_history(GradeHistory::from(data))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the grade history and caches it.
    pub async fn fetch_grade_history(&self, flow: String) -> Result<GradeHistory, CoreError> {
        traced("fetch_grade_history", &flow, async {
            let data = self.page(&spec::GRADE_HISTORY, None, &[]).await?;
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a failed mkdir shows up in the write that follows"
            )]
            let _ = fs::create_dir_all(self.at(PAGES));
            self.write(&format!("{PAGES}/{HISTORY}"), &data);
            Ok(logged_history(GradeHistory::from(data)))
        })
        .await
    }

    /// The picked semester's courses from the cached timetable, for the
    /// planner; empty without one. No network.
    pub fn planner_courses(&self, flow: String) -> Vec<PlanCourse> {
        traced_sync("planner_courses", &flow, || {
            let courses = self
                .read_page::<TimetableData>(TIMETABLE)
                .map_or_default(|data| plan_courses(&data));
            info!(courses = courses.len(), "planner courses");
            Ok(courses)
        })
        .unwrap_or_default()
    }
}

/// `courses`' GPA, with credits looked up by code in `history` (P and other
/// grades without points, and courses without credits, left out).
#[uniffi::export]
pub fn semester_summary(
    courses: Vec<GradeCourse>,
    history: Option<GradeHistory>,
) -> SemesterSummary {
    // The last course of a code wins, as in Flutter's map.
    let by_code: HashMap<String, f64> = history
        .iter()
        .flat_map(|history| &history.sessions)
        .flat_map(|session| &session.courses)
        .filter_map(|course| Some((course.code.trim().to_uppercase(), course.credits?)))
        .collect();
    let course_credits: Vec<Option<f64>> = courses
        .iter()
        .map(|course| by_code.get(&course.code.trim().to_uppercase()).copied())
        .collect();
    let (mut credits, mut points) = (0.0, 0.0);
    for (course, credit) in courses.iter().zip(&course_credits) {
        if let (Some(value), Some(credit)) = (grade_points(&course.grade), *credit)
            && credit > 0.0
        {
            credits += credit;
            points += credit * value;
        }
    }
    SemesterSummary {
        gpa: (credits > 0.0).then_some(points / credits),
        credits,
        course_credits,
        counts: counts(courses.iter().map(|course| course.grade.as_str())),
    }
}

/// The planner: the semester GPA of `courses`, the CGPA after it from
/// `cgpa` over `completed_credits` (as typed), and what `target` (else
/// the next quarter point) needs.
#[uniffi::export]
pub fn plan_gpa(
    courses: Vec<PlannedCourse>,
    cgpa: String,
    completed_credits: String,
    target: Option<f64>,
) -> GpaPlan {
    let (mut credits, mut points) = (0.0, 0.0);
    for course in &courses {
        let value = grade_points(&course.grade).filter(|_| course.grade.trim() != "N");
        if let Some(value) = value.filter(|_| course.credits.is_finite() && course.credits > 0.0) {
            credits += course.credits;
            points += course.credits * value;
        }
    }
    let semester_gpa = if credits == 0.0 {
        0.0
    } else {
        points / credits
    };
    let current = cgpa
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|value| (0.0..=10.0).contains(value));
    let completed = parse_credits(&completed_credits).unwrap_or(0.0);
    let projected = current.map(|current| {
        let total = completed + credits;
        if total == 0.0 {
            0.0
        } else {
            (current * completed + points) / total
        }
    });
    let target = target.unwrap_or_else(|| {
        current.map_or(9.0, |current| {
            (((current + 0.01) * 4.0).ceil() / 4.0).min(10.0)
        })
    });
    let needed = current
        .filter(|_| credits != 0.0)
        .map(|current| (target * (completed + credits) - current * completed) / credits);
    let outlook = needed.map(|needed| {
        if needed <= 0.0 {
            TargetOutlook::Reached
        } else if needed <= semester_gpa + 0.0001 {
            TargetOutlook::OnTrack
        } else if needed > 10.0 {
            TargetOutlook::OutOfReach
        } else {
            TargetOutlook::Needs
        }
    });
    GpaPlan {
        semester_gpa,
        credits,
        current,
        projected,
        delta: match (projected, current) {
            (Some(projected), Some(current)) => projected - current,
            _ => 0.0,
        },
        target,
        needed,
        outlook,
        letter: letter_for(semester_gpa).to_string(),
    }
}

impl Core {
    /// The semester to show and, for an empty `requested`, the one before
    /// the pick to fall back to.
    fn grade_semesters(&self, requested: &str) -> Result<(String, Option<String>), CoreError> {
        let list = self
            .read_kept::<SemesterData>(SEMESTERS)
            .ok_or(CoreError::SemesterNeeded)?;
        if !requested.is_empty() {
            return if list
                .semesters
                .iter()
                .any(|semester| semester.id == requested)
            {
                Ok((requested.to_string(), None))
            } else {
                Err(CoreError::Other {
                    reason: format!("unknown semester {requested}"),
                })
            };
        }
        let picked = self.pick(&list).ok_or(CoreError::SemesterNeeded)?;
        let previous = list
            .semesters
            .iter()
            .position(|semester| semester.id == picked)
            .and_then(|at| list.semesters.get(at + 1))
            .map(|semester| semester.id.clone());
        Ok((picked, previous))
    }

    fn cached_grade_view(&self, semester: &str) -> Option<Grades> {
        self.read_kept::<GradeViewData>(&format!("{PAGES}/{semester}/{GRADES}"))
            .map(|data| Grades::new(data, semester))
    }

    /// `semester` (one VTOP lists) fetched and cached.
    async fn fetch_grade_view(&self, semester: &str) -> Result<Grades, CoreError> {
        let data = self.page(&spec::GRADES, Some(semester), &[]).await?;
        let path = format!("{PAGES}/{semester}/{GRADES}");
        if let Some(parent) = self.at(&path).parent() {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a failed mkdir shows up in the write that follows"
            )]
            let _ = fs::create_dir_all(parent);
        }
        self.write(&path, &data);
        Ok(Grades::new(data, semester))
    }

    /// The breakdown's cache file, named from the course found in the
    /// semester's cached grades, never from the caller's text alone.
    fn details_path(&self, semester: &str, course_id: &str) -> Result<String, CoreError> {
        self.grade_semesters(semester)?;
        let known = self
            .cached_grade_view(semester)
            .is_some_and(|grades| grades.courses.iter().any(|c| c.course_id == course_id));
        if semester.is_empty() || !known {
            return Err(CoreError::Other {
                reason: format!("no course {course_id} in {semester}'s grades"),
            });
        }
        let file = course_id.replace(['/', '\\', '.'], "_");
        Ok(format!("{PAGES}/{semester}/{DETAILS}/{file}.json"))
    }
}

fn logged(grades: Grades) -> Grades {
    info!(semester = %grades.semester_id, courses = grades.courses.len(), "grades");
    grades
}

/// Counts only: never the name or number.
fn logged_history(history: GradeHistory) -> GradeHistory {
    info!(
        courses = history.course_count,
        sessions = history.sessions.len(),
        "grade history"
    );
    history
}

impl Grades {
    fn new(data: GradeViewData, semester: &str) -> Self {
        let serial = |course: &vtop_core::types::GradeCourseRecord| {
            course.serial.trim().parse::<i64>().unwrap_or(0)
        };
        let mut records = data.courses;
        records.sort_by_key(serial);
        Self {
            semester_id: if data.semester_id.is_empty() {
                semester.to_string()
            } else {
                data.semester_id
            },
            updated_at: data.update_time as i64,
            courses: records
                .into_iter()
                .map(|course| GradeCourse {
                    course_id: course.course_id.trim().to_string(),
                    code: course.course_code.trim().to_string(),
                    title: course.course_title.trim().to_string(),
                    course_type: course.course_type.trim().to_string(),
                    grading_type: course.grading_type.trim().to_string(),
                    total: course.grand_total.trim().to_string(),
                    grade: course.grade.trim().to_uppercase(),
                })
                .collect(),
        }
    }
}

impl From<GradeDetailsData> for GradeDetails {
    fn from(data: GradeDetailsData) -> Self {
        let cutoffs = data
            .grade_ranges
            .iter()
            .filter(|range| !range.range.replace('#', "").trim().is_empty())
            .map(|range| GradeCutoff {
                grade: range.grade.trim().to_uppercase(),
                range: short_range(&range.range),
            })
            .collect();
        let mut sections: Vec<(String, Vec<GradeDetailMark>)> = Vec::new();
        for mark in &data.marks {
            let parts: Vec<&str> = mark.mark_title.split('•').collect();
            let (section, title) = match parts.split_first() {
                Some((section, rest)) if !rest.is_empty() => (
                    section.trim().to_string(),
                    rest.join("•").trim().to_string(),
                ),
                _ => (data.class_course_type.clone(), mark.mark_title.clone()),
            };
            let mark = GradeDetailMark {
                mark_title: title,
                ..mark.clone()
            };
            match sections.iter_mut().find(|(name, _)| *name == section) {
                Some((_, marks)) => marks.push(mark),
                None => sections.push((section, vec![mark])),
            }
        }
        Self {
            placeholder: data
                .grade_ranges
                .iter()
                .any(|range| range.range.contains('#')),
            semester_id: data.semester_id,
            course_id: data.course_id,
            updated_at: data.update_time as i64,
            grand_total: data.grand_total.trim().to_string(),
            cutoffs,
            sections: sections
                .into_iter()
                .map(|(title, marks)| MarkSection {
                    title,
                    total: weighted_total(&marks),
                    marks: marks.iter().map(GradeMark::from).collect(),
                })
                .collect(),
        }
    }
}

impl From<&GradeDetailMark> for GradeMark {
    fn from(mark: &GradeDetailMark) -> Self {
        let scored = number(&mark.scored_mark);
        let max = number(&mark.max_mark).unwrap_or(0.0);
        let percent = scored
            .filter(|_| max > 0.0)
            .map(|scored| scored / max * 100.0);
        Self {
            title: mark.mark_title.trim().to_string(),
            scored: mark.scored_mark.trim().to_string(),
            max: mark.max_mark.trim().to_string(),
            weighted: mark.weightage_mark.trim().to_string(),
            weightage: mark.weightage.trim().to_string(),
            percent,
            band: ScoreBand::of(percent),
        }
    }
}

/// The weighted marks over their weightage, a re-evaluation counted once
/// as the better (Flutter's `calculateWeightageTotals`); empty before any.
fn weighted_total(marks: &[GradeDetailMark]) -> String {
    let mut best: HashMap<String, (f64, f64)> = HashMap::new();
    for mark in marks {
        let title = normalized(&mark.mark_title);
        if title.is_empty() {
            continue;
        }
        let base = normalized(&strip_reevaluation(&mark.mark_title));
        let key = if is_reevaluation(&mark.mark_title) && !base.is_empty() {
            base
        } else {
            title
        };
        let entry = best.entry(key).or_insert((0.0, 0.0));
        entry.0 = entry.0.max(number(&mark.weightage_mark).unwrap_or(0.0));
        entry.1 = entry.1.max(number(&mark.weightage).unwrap_or(0.0));
    }
    let (gained, possible) = best
        .values()
        .fold((0.0, 0.0), |(g, p), (gained, possible)| {
            (g + gained, p + possible)
        });
    if gained == 0.0 {
        String::new()
    } else if possible > 0.0 {
        format!("{} / {}", one_decimal(gained), one_decimal(possible))
    } else {
        one_decimal(gained)
    }
}

impl From<GradeHistoryData> for GradeHistory {
    fn from(data: GradeHistoryData) -> Self {
        let cgpa = &data.cgpa;
        let mix = [
            ("S", &cgpa.s_grades),
            ("A", &cgpa.a_grades),
            ("B", &cgpa.b_grades),
            ("C", &cgpa.c_grades),
            ("D", &cgpa.d_grades),
            ("E", &cgpa.e_grades),
            ("F", &cgpa.f_grades),
            ("N", &cgpa.n_grades),
        ]
        .into_iter()
        .map(|(grade, count)| GradeCount {
            grade: grade.to_string(),
            count: count.trim().parse().unwrap_or(0),
        })
        .filter(|count| count.count != 0)
        .collect();
        let mut filters = counts(data.records.iter().map(|record| record.grade.as_str()));
        // P comes after the others here, as Flutter's filter order has no P.
        if let Some(at) = filters.iter().position(|count| count.grade == "P") {
            let pass = filters.remove(at);
            filters.push(pass);
        }
        let mut sessions: Vec<GradeSession> = Vec::new();
        let mut keys = HashSet::new();
        for record in &data.records {
            let month = record.exam_month.trim().to_string();
            let course = HistoryCourse::new(record, &mut keys);
            match sessions.iter_mut().find(|session| session.month == month) {
                Some(session) => session.courses.push(course),
                None => sessions.push(GradeSession {
                    label: if month.is_empty() {
                        "Other".to_string()
                    } else {
                        month.replace('-', " ")
                    },
                    short: short_session(&month),
                    gpa: None,
                    courses: vec![course],
                    month,
                }),
            }
        }
        for session in &mut sessions {
            session.gpa = session_gpa(
                data.records
                    .iter()
                    .filter(|record| record.exam_month.trim() == session.month),
            );
        }
        Self {
            updated_at: data.update_time as i64,
            name: data.student.name.trim().to_string(),
            reg_no: data.student.reg_no.trim().to_string(),
            cgpa: cgpa.cgpa.trim().to_string(),
            credits_earned: plain_number(&cgpa.credits_earned),
            credits_registered: plain_number(&cgpa.credits_registered),
            mix,
            filters,
            course_count: data.records.len() as i32,
            sessions,
        }
    }
}

impl HistoryCourse {
    fn new(record: &GradeHistoryRecord, keys: &mut HashSet<String>) -> Self {
        let code = record.course_code.trim().to_string();
        let mut key = code.clone();
        let mut n = 1;
        while !keys.insert(key.clone()) {
            n += 1;
            key = format!("{code}.{n}");
        }
        Self {
            key,
            title: record.course_title.trim().to_string(),
            distribution: record.course_distribution.trim().to_string(),
            credits: record
                .credits
                .trim()
                .parse()
                .ok()
                .filter(|c: &f64| c.is_finite()),
            grade: record.grade.trim().to_uppercase(),
            // Attempts of the course's own type are re-registrations, not parts.
            components: record
                .attempts
                .iter()
                .filter(|attempt| attempt.course_type.trim() != record.course_type.trim())
                .map(|attempt| GradeComponent {
                    label: component_label(&attempt.course_type),
                    declared: attempt.result_declared.trim().to_string(),
                })
                .collect(),
            code,
        }
    }
}

fn component_label(kind: &str) -> String {
    match kind.trim().to_uppercase().as_str() {
        "ETH" | "TH" => "Theory component".to_string(),
        "ELA" | "LO" => "Lab component".to_string(),
        "EPJ" | "PJT" => "Project component".to_string(),
        _ => kind.trim().to_string(),
    }
}

/// How many of each grade, S to F, N, P, then others in first-seen order;
/// blanks and "-" left out.
pub(crate) fn counts<'a>(grades: impl Iterator<Item = &'a str>) -> Vec<GradeCount> {
    let mut found: Vec<GradeCount> = Vec::new();
    for grade in grades {
        let grade = grade.trim().to_uppercase();
        if grade.is_empty() || grade == "-" {
            continue;
        }
        match found.iter_mut().find(|count| count.grade == grade) {
            Some(count) => count.count += 1,
            None => found.push(GradeCount { grade, count: 1 }),
        }
    }
    let rank = |grade: &str| {
        ORDER
            .iter()
            .position(|g| *g == grade)
            .unwrap_or(ORDER.len())
    };
    // Stable: others keep their order.
    found.sort_by_key(|count| rank(&count.grade));
    found
}

/// Every number in `raw`, as written.
fn numbers(raw: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let bytes = raw.as_bytes();
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    let mut at = 0;
    while at < bytes.len() {
        if !digit(at) {
            at += 1;
            continue;
        }
        let start = at;
        while digit(at) {
            at += 1;
        }
        if bytes.get(at) == Some(&b'.') && digit(at + 1) {
            at += 1;
            while digit(at) {
                at += 1;
            }
        }
        found.extend(raw.get(start..at));
    }
    found
}

/// ">=83 and <88" → "83–88", ">=88" → "≥88", "<50" → "<50".
fn short_range(raw: &str) -> String {
    let range = raw.replace('#', "");
    let range = range.trim();
    match numbers(range)[..] {
        [low, high, ..] => format!("{low}–{high}"),
        [only] if range.starts_with('<') => format!("<{only}"),
        [only] => format!("≥{only}"),
        _ => range.to_string(),
    }
}

/// "Jan-2025" → "Jan '25".
fn short_session(month: &str) -> String {
    match month.trim().split('-').collect::<Vec<_>>()[..] {
        [name, year] if year.len() == 4 => format!("{name} '{}", year.get(2..).unwrap_or_default()),
        _ => month.to_string(),
    }
}

/// "4" for whole numbers, else one decimal: "2.5". An exact half rounds
/// up, as Dart's `toStringAsFixed`.
fn one_decimal(value: f64) -> String {
    if value == value.round() {
        return format!("{value:.0}");
    }
    let tenths = value * 10.0;
    if tenths.fract().abs() == 0.5 && tenths / 10.0 == value {
        return format!("{:.1}", tenths.round() / 10.0);
    }
    format!("{value:.1}")
}

/// "10" for 10.0, "10.5", or the text when it is no number.
fn plain_number(raw: &str) -> String {
    match raw.trim().parse::<f64>() {
        Ok(value) if value == value.round() => format!("{value:.0}"),
        Ok(value) => value.to_string(),
        Err(_) => raw.trim().to_string(),
    }
}

/// VTOP's cell as a number; none when empty or not one.
fn number(raw: &str) -> Option<f64> {
    raw.replace('%', "")
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}

#[cfg(test)]
mod tests;
