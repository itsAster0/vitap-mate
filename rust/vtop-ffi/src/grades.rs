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
use vtop_core::VtopError;
use vtop_core::inputs::{CourseId, SemesterId};
use vtop_core::types::{
    GradeDetailMark, GradeDetailsData, GradeHistoryData, GradeHistoryRecord, GradeViewData,
    SemesterData, TimetableData,
};

use crate::marks::{ScoreBand, is_reevaluation, normalized, strip_reevaluation};
use crate::{Core, CoreError, PAGES, SEMESTERS, TIMETABLE, traced, traced_sync};

const GRADES: &str = "grades.json";
/// One course's breakdown: `pages/<semester>/grade_details/<course id>.json`.
const DETAILS: &str = "grade_details";
/// Not per semester: `pages/grade_history.json`.
const HISTORY: &str = "grade_history.json";

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

#[uniffi::export(async_runtime = "tokio")]
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
            Ok(self.read::<GradeDetailsData>(&path).map(GradeDetails::from))
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
            let (semester, course) = (
                SemesterId::parse(&semester_id)?,
                CourseId::parse(&course_id)?,
            );
            let data = signed_in!(self, |client| client.grade_view_details(&semester, &course))?;
            if let Some(parent) = self.dir.join(&path).parent() {
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
                .read::<GradeHistoryData>(&format!("{PAGES}/{HISTORY}"))
                .map(|data| logged_history(GradeHistory::from(data))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the grade history and caches it.
    pub async fn fetch_grade_history(&self, flow: String) -> Result<GradeHistory, CoreError> {
        traced("fetch_grade_history", &flow, async {
            let data = signed_in!(self, |client| client.grade_history())?;
            let _ = fs::create_dir_all(self.dir.join(PAGES));
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
                .map(|data| plan_courses(&data))
                .unwrap_or_default();
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
        gpa: (credits > 0.0).then(|| points / credits),
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
            .read::<SemesterData>(SEMESTERS)
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
        self.read::<GradeViewData>(&format!("{PAGES}/{semester}/{GRADES}"))
            .map(|data| Grades::new(data, semester))
    }

    /// `semester` (one VTOP lists) fetched and cached.
    async fn fetch_grade_view(&self, semester: &str) -> Result<Grades, CoreError> {
        let id = SemesterId::parse(semester)?;
        let data = signed_in!(self, |client| client.grade_view(&id))?;
        let path = format!("{PAGES}/{semester}/{GRADES}");
        if let Some(parent) = self.dir.join(&path).parent() {
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
            let (section, title) = if parts.len() >= 2 {
                (
                    parts[0].trim().to_string(),
                    parts[1..].join("•").trim().to_string(),
                )
            } else {
                (data.class_course_type.clone(), mark.mark_title.clone())
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

/// S 10 ... E 5, F and N 0; none for P and anything else.
fn grade_points(grade: &str) -> Option<f64> {
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
fn session_gpa<'a>(records: impl Iterator<Item = &'a GradeHistoryRecord>) -> Option<f64> {
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

/// How many of each grade, S to F, N, P, then others in first-seen order;
/// blanks and "-" left out.
fn counts<'a>(grades: impl Iterator<Item = &'a str>) -> Vec<GradeCount> {
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

/// The first number in `raw` ("4", "1.5 credits", "2,5"), when above 0.
fn parse_credits(raw: &str) -> Option<f64> {
    let text = raw.trim().replace(',', ".");
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let rest = &text[start..];
    let mut end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if rest[end..].starts_with('.') {
        let fraction = rest[end + 1..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len() - end - 1);
        if fraction > 0 {
            end += 1 + fraction;
        }
    }
    rest[..end].parse().ok().filter(|value| *value > 0.0)
}

/// Every number in `raw`, as written.
fn numbers(raw: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let bytes = raw.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit() {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at + 1 < bytes.len() && bytes[at] == b'.' && bytes[at + 1].is_ascii_digit() {
            at += 1;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
        }
        found.push(&raw[start..at]);
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
        [name, year] if year.len() == 4 => format!("{name} '{}", &year[2..]),
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

/// The letter nearest a GPA.
fn letter_for(gpa: f64) -> &'static str {
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
fn plan_courses(data: &TimetableData) -> Vec<PlanCourse> {
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

#[cfg(test)]
mod tests {
    use vtop_core::types::{
        GradeCourseRecord, GradeHistoryAttempt, GradeHistoryCgpa, GradeHistoryStudentInfo,
        GradeRange, TimetableCourse,
    };

    use super::*;

    fn record(
        code: &str,
        kind: &str,
        credits: &str,
        grade: &str,
        month: &str,
    ) -> GradeHistoryRecord {
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
}
