//! Marks: each course's assessments with numbers parsed, its weighted total
//! so far, and when the app first saw each mark posted (VTOP dates none).
//! Port of the Flutter app's `lib/features/marks/` (`weightage_totals.dart`,
//! `marks_card.dart`).

use std::collections::{HashMap, HashSet};

use tracing::info;
use vtop_core::types::{MarksData, MarksRecord, MarksRecordEach};

use crate::{ClassKind, Core, CoreError, PAGES, traced, traced_sync};

const MARKS: &str = "marks.json";
/// When the app first saw each assessment posted (`<course key>/<title>` →
/// Unix seconds), next to the marks: `pages/<semester>/marks_seen.json`.
const SEEN: &str = "marks_seen.json";

/// How an assessment, or a course so far, went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ScoreBand {
    /// 75% or more.
    Strong,
    /// 60% to 75%.
    Fair,
    /// Under 60%.
    Low,
    Absent,
    NotPosted,
}

impl ScoreBand {
    pub(crate) fn of(percent: Option<f64>) -> Self {
        match percent {
            None => Self::NotPosted,
            Some(p) if p >= 75.0 => Self::Strong,
            Some(p) if p >= 60.0 => Self::Fair,
            Some(_) => Self::Low,
        }
    }
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Marks {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    pub courses: Vec<MarksCourse>,
}

/// One course's marks: theory and lab of an embedded course are two.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MarksCourse {
    /// `<code>.theory` or `<code>.lab`, as `AttendanceCourse::key`.
    pub key: String,
    pub code: String,
    pub name: String,
    pub kind: ClassKind,
    /// VTOP's "Embedded Theory", "Lab Only", ...
    pub course_type: String,
    pub faculty: String,
    pub slot: String,
    /// In VTOP's order, numbers first ("Experiment-2" before "Experiment-10").
    pub assessments: Vec<Assessment>,
    /// Weighted marks scored so far, out of [`Self::max_weighted`]; a
    /// re-evaluation counts once, the better of the two.
    pub weighted: f64,
    /// The weightage of what was assessed so far (out of 100).
    pub max_weighted: f64,
    /// `weighted / max_weighted`, 0–100; none before anything is assessed.
    pub percent: Option<f64>,
    pub band: ScoreBand,
    /// The assessment posted last, as far as the app saw: an index into
    /// [`Self::assessments`].
    pub latest: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Assessment {
    pub title: String,
    /// None until posted.
    pub scored: Option<f64>,
    pub max: f64,
    /// The share of the course's 100 it counts for; none when VTOP gives 0.
    pub weightage: Option<f64>,
    /// What it adds to the course's total: VTOP's, else worked out.
    pub weighted: Option<f64>,
    /// `scored / max`, 0–100.
    pub percent: Option<f64>,
    pub band: ScoreBand,
    /// VTOP's status ("Present", "Absent", ...).
    pub status: String,
    /// VTOP's remark; empty when it only repeats the score.
    pub remark: String,
    /// Re-marks an earlier assessment of the same title.
    pub reevaluation: bool,
    /// When the app first saw it posted, Unix seconds.
    pub posted_at: Option<i64>,
}

#[uniffi::export(async_runtime = "tokio")]
impl Core {
    /// The selected semester's cached marks.
    pub fn cached_marks(&self, flow: String) -> Option<Marks> {
        traced_sync("cached_marks", &flow, || {
            Ok(self.read_page::<MarksData>(MARKS).map(|data| {
                let seen = self.read_page(SEEN).unwrap_or_default();
                logged(Marks::new(data, &seen))
            }))
        })
        .ok()
        .flatten()
    }

    /// Fetches the selected semester's marks and caches them, noting when
    /// each assessment first showed up.
    pub async fn fetch_marks(&self, flow: String) -> Result<Marks, CoreError> {
        traced("fetch_marks", &flow, async {
            let data = self
                .fetch(MARKS, |client, semester| Box::pin(client.marks(semester)))
                .await?;
            let path = format!("{PAGES}/{}/{SEEN}", data.semester_id);
            let mut seen = self.read(&path).unwrap_or_default();
            if note_posted(&data, &mut seen) {
                self.write(&path, &seen);
            }
            Ok(logged(Marks::new(data, &seen)))
        })
        .await
    }
}

fn logged(marks: Marks) -> Marks {
    info!(
        courses = marks.courses.len(),
        assessments = marks
            .courses
            .iter()
            .map(|course| course.assessments.len())
            .sum::<usize>(),
        "marks"
    );
    marks
}

/// Adds the fetch time for each posted assessment not seen before; whether
/// any was new.
fn note_posted(data: &MarksData, seen: &mut HashMap<String, i64>) -> bool {
    let mut added = false;
    for course in Marks::new(data.clone(), seen).courses {
        for assessment in course.assessments {
            if assessment.scored.is_some() || assessment.band == ScoreBand::Absent {
                let id = seen_id(&course.key, &assessment.title);
                seen.entry(id).or_insert_with(|| {
                    added = true;
                    data.update_time as i64
                });
            }
        }
    }
    added
}

fn seen_id(key: &str, title: &str) -> String {
    format!("{key}/{title}")
}

impl Marks {
    fn new(data: MarksData, seen: &HashMap<String, i64>) -> Self {
        let mut keys = HashSet::new();
        let courses = data
            .records
            .into_iter()
            .map(|record| {
                let mut course = MarksCourse::new(record, seen);
                // Two rows of one code and kind would share a key.
                if !keys.insert(course.key.clone()) {
                    course.key = format!("{}.{}", course.key, course.slot);
                }
                course
            })
            .collect();
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            courses,
        }
    }
}

impl MarksCourse {
    fn new(record: MarksRecord, seen: &HashMap<String, i64>) -> Self {
        let kind_text = record.coursetype.trim().to_lowercase();
        let lab = kind_text.ends_with("lab") || kind_text.ends_with("lab only");
        let code = record.coursecode.trim().to_string();
        let key = format!("{code}.{}", if lab { "lab" } else { "theory" });
        let assessments: Vec<Assessment> = record
            .marks
            .iter()
            .map(|mark| {
                let mut assessment = Assessment::from(mark);
                assessment.posted_at = seen.get(&seen_id(&key, &assessment.title)).copied();
                assessment
            })
            .collect();
        let (weighted, max_weighted) = totals(&assessments);
        let percent = (max_weighted > 0.0).then(|| weighted / max_weighted * 100.0);
        // Ties (one fetch saw them all) go to the last in order.
        let latest = assessments
            .iter()
            .enumerate()
            .filter(|(_, a)| a.scored.is_some() || a.band == ScoreBand::Absent)
            .max_by_key(|(_, a)| a.posted_at.unwrap_or(0))
            .map(|(i, _)| i as i32);
        Self {
            key,
            name: record.coursetitle.trim().to_string(),
            code,
            kind: if lab {
                ClassKind::Lab
            } else {
                ClassKind::Theory
            },
            course_type: record.coursetype.trim().to_string(),
            faculty: title_case(&record.faculity),
            slot: record.slot.trim().to_string(),
            weighted: round2(weighted),
            max_weighted: round2(max_weighted),
            percent,
            band: ScoreBand::of(percent),
            latest,
            assessments,
        }
    }
}

impl From<&MarksRecordEach> for Assessment {
    fn from(mark: &MarksRecordEach) -> Self {
        let scored = number(&mark.scoredmark);
        let max = number(&mark.maxmarks).unwrap_or(0.0);
        let weightage = number(&mark.weightage).filter(|w| *w > 0.0);
        let percent = scored.filter(|_| max > 0.0).map(|s| s / max * 100.0);
        let weighted = number(&mark.weightagemark)
            .filter(|_| scored.is_some())
            .or_else(|| Some(percent? / 100.0 * weightage?));
        let absent = mark.status.trim().eq_ignore_ascii_case("absent");
        let remark = mark.remark.trim();
        Self {
            title: mark.markstitle.trim().to_string(),
            weighted: weighted.map(round2),
            percent,
            band: if absent {
                ScoreBand::Absent
            } else {
                ScoreBand::of(percent)
            },
            status: mark.status.trim().to_string(),
            remark: if number(remark).is_some_and(|r| Some(r) == scored) {
                String::new()
            } else {
                remark.to_string()
            },
            reevaluation: is_reevaluation(&mark.markstitle),
            posted_at: None,
            scored,
            max,
            weightage,
        }
    }
}

/// Weighted scored and assessed so far. A re-evaluation counts as the
/// assessment it re-marks, and only the better of the two is kept.
fn totals(assessments: &[Assessment]) -> (f64, f64) {
    let mut best: HashMap<String, (f64, f64)> = HashMap::new();
    for assessment in assessments {
        let assessed = assessment.scored.is_some() || assessment.band == ScoreBand::Absent;
        let (Some(weightage), true) = (assessment.weightage, assessed) else {
            continue;
        };
        let title = normalized(&assessment.title);
        let base = normalized(&strip_reevaluation(&assessment.title));
        let key = if assessment.reevaluation && !base.is_empty() {
            base
        } else {
            title
        };
        let entry = best.entry(key).or_insert((0.0, 0.0));
        entry.0 = entry.0.max(assessment.weighted.unwrap_or(0.0));
        entry.1 = entry.1.max(weightage);
    }
    best.values()
        .fold((0.0, 0.0), |(gained, possible), (g, p)| {
            (gained + g, possible + p)
        })
}

/// Where "re-evaluation", "re evaluation" or "reevaluation" stands as a
/// word in `lower` (lowercase).
fn reevaluation_at(lower: &str) -> Option<(usize, usize)> {
    let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    lower.match_indices("re").find_map(|(start, _)| {
        let rest = lower[start + 2..].trim_start_matches([' ', '-']);
        let after = rest.strip_prefix("evaluation")?;
        let end = lower.len() - after.len();
        (boundary(lower[..start].chars().next_back()) && boundary(after.chars().next()))
            .then_some((start, end))
    })
}

pub(crate) fn is_reevaluation(title: &str) -> bool {
    reevaluation_at(&title.to_lowercase()).is_some()
}

/// `title` lowercase, without its "re-evaluation".
pub(crate) fn strip_reevaluation(title: &str) -> String {
    let lower = title.to_lowercase();
    reevaluation_at(&lower).map_or_else(
        || lower.clone(),
        |(start, end)| format!("{}{}", &lower[..start], &lower[end..]),
    )
}

pub(crate) fn normalized(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// VTOP's cell as a number: "10.65", "15 %"; none when empty or not one.
fn number(raw: &str) -> Option<f64> {
    raw.replace('%', "")
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// "SHAIK WASIM AKRAM" → "Shaik Wasim Akram"; mixed case is kept (as
/// `records::person_name`, whose names carry a school suffix).
fn title_case(raw: &str) -> String {
    let name = raw.trim();
    if name != name.to_uppercase() {
        return name.to_string();
    }
    name.split_whitespace()
        .map(|word| match word.chars().next() {
            Some(first) if word.chars().count() > 1 && !word.contains('.') => {
                format!("{first}{}", word[first.len_utf8()..].to_lowercase())
            }
            _ => word.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(
        title: &str,
        max: &str,
        weightage: &str,
        status: &str,
        scored: &str,
        weighted: &str,
    ) -> MarksRecordEach {
        MarksRecordEach {
            serial: String::new(),
            markstitle: title.into(),
            maxmarks: max.into(),
            weightage: weightage.into(),
            status: status.into(),
            scoredmark: scored.into(),
            weightagemark: weighted.into(),
            remark: String::new(),
        }
    }

    fn record(code: &str, kind: &str, marks: Vec<MarksRecordEach>) -> MarksRecord {
        MarksRecord {
            serial: "1".into(),
            coursecode: code.into(),
            coursetitle: "Analysis of Algorithms".into(),
            coursetype: kind.into(),
            faculity: "BILEESH P BABU".into(),
            slot: "A2+TA2".into(),
            marks,
        }
    }

    fn data(records: Vec<MarksRecord>) -> MarksData {
        MarksData {
            records,
            semester_id: "AP1".into(),
            update_time: 1_000,
        }
    }

    #[test]
    fn courses_are_keyed_like_attendance_with_totals_so_far() {
        let marks = Marks::new(
            data(vec![
                record(
                    "SWE2009",
                    "Embedded Theory",
                    vec![
                        mark("CAT1", "50", "15", "Present", "43.0", "12.9"),
                        mark("CAT2", "50", "15", "Present", "48.0", "14.4"),
                        mark("QUIZ1", "10", "10", "Present", "10.0", "10"),
                    ],
                ),
                record("SWE2009", "Embedded Lab", vec![]),
                record("CSE4007", "Lab Only", vec![]),
            ]),
            &HashMap::new(),
        );
        let keys: Vec<_> = marks.courses.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["SWE2009.theory", "SWE2009.lab", "CSE4007.lab"]);
        let theory = &marks.courses[0];
        assert_eq!((theory.weighted, theory.max_weighted), (37.3, 40.0));
        assert_eq!(theory.band, ScoreBand::Strong);
        assert_eq!(theory.faculty, "Bileesh P Babu");
        assert_eq!(theory.assessments[0].percent, Some(86.0));
        // Nothing assessed yet.
        let lab = &marks.courses[1];
        assert_eq!(
            (lab.percent, lab.band, lab.latest),
            (None, ScoreBand::NotPosted, None)
        );
    }

    #[test]
    fn unposted_and_absent_marks() {
        let course = MarksCourse::new(
            record(
                "MAT1001",
                "Theory Only",
                vec![
                    mark("CAT1", "50", "15", "Absent", "", ""),
                    mark("CAT2", "50", "15", "", "", ""),
                    mark("Quiz", "10", "0", "Present", "8", ""),
                ],
            ),
            &HashMap::new(),
        );
        let [absent, unposted, unweighted] = &course.assessments[..] else {
            panic!("three assessments");
        };
        assert_eq!(
            (absent.band, absent.scored, absent.weighted),
            (ScoreBand::Absent, None, None)
        );
        assert_eq!(
            (unposted.band, unposted.scored),
            (ScoreBand::NotPosted, None)
        );
        assert_eq!(
            (
                unweighted.weightage,
                unweighted.weighted,
                unweighted.percent
            ),
            (None, None, Some(80.0))
        );
        // The absence counts against the total; the unposted CAT does not.
        assert_eq!(
            (course.weighted, course.max_weighted, course.band),
            (0.0, 15.0, ScoreBand::Low)
        );
        assert_eq!(course.latest, Some(2));
    }

    #[test]
    fn a_reevaluation_counts_once_the_better_mark() {
        let course = MarksCourse::new(
            record(
                "X",
                "Theory Only",
                vec![
                    mark("CAT1", "50", "15", "Present", "20", "6"),
                    mark("CAT1 Re-Evaluation", "50", "15", "Present", "30", "9"),
                    mark("Quiz", "10", "10", "Present", "5", ""),
                ],
            ),
            &HashMap::new(),
        );
        assert!(course.assessments[1].reevaluation && !course.assessments[0].reevaluation);
        // VTOP's weighted mark is blank for the quiz: worked out.
        assert_eq!(course.assessments[2].weighted, Some(5.0));
        assert_eq!((course.weighted, course.max_weighted), (14.0, 25.0));
        assert!(!is_reevaluation("Prerevaluation") && is_reevaluation("re evaluation CAT2"));
        assert!(is_reevaluation("Report Re-Evaluation"));
        assert_eq!(normalized(&strip_reevaluation("CAT2 reevaluation")), "cat2");
    }

    #[test]
    fn the_latest_is_the_last_seen_posted() {
        let marks = data(vec![record(
            "X",
            "Theory Only",
            vec![
                mark("CAT1", "50", "15", "Present", "40", "12"),
                mark("QUIZ1", "10", "10", "Present", "8", "8"),
            ],
        )]);
        let mut seen = HashMap::new();
        assert!(note_posted(&marks, &mut seen));
        assert!(!note_posted(&marks, &mut seen), "seen already");
        // One fetch saw both: the last in order.
        assert_eq!(Marks::new(marks.clone(), &seen).courses[0].latest, Some(1));
        seen.insert("X.theory/CAT1".into(), 2_000);
        let course = &Marks::new(marks, &seen).courses[0];
        assert_eq!(course.latest, Some(0));
        assert_eq!(course.assessments[0].posted_at, Some(2_000));
    }

    #[test]
    fn a_remark_repeating_the_score_is_dropped() {
        let mut repeated = mark("CAT1", "50", "15", "Present", "35.0", "10.5");
        repeated.remark = "35".into();
        assert_eq!(Assessment::from(&repeated).remark, "");
        repeated.remark = "Good".into();
        assert_eq!(Assessment::from(&repeated).remark, "Good");
    }
}
