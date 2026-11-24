//! Attendance: debarments, each course's history, and the plan to stay above 75%.

use std::collections::HashMap;

use vtop_core::types::{self as raw, AttendanceData, FullAttendanceData};

use super::{
    Attendance, AttendanceCourse, AttendanceDetail, AttendancePlan, AttendedClass, ClassDay,
    ClassKind, ClassSlot, ClassStatus, Debar, PlanDay, THRESHOLD, TimetableDay, epoch_day, minutes,
    person_name, split_course_name,
};
use crate::pages::calendar::SemesterCalendar;

impl Debar {
    /// "CAT - II : Debarred & Permitted" → CAT-II, permitted, next FAT.
    pub(super) fn parse(raw: &str) -> Option<Self> {
        let lower = raw.to_lowercase();
        if !lower.contains("debar") {
            return None;
        }
        let exam = raw.split_once(':').map_or(String::new(), |(exam, _)| {
            crate::pages::calendar::exam_name(exam)
        });
        let next_exam = match exam.as_str() {
            "CAT-I" => Some("CAT-II".to_string()),
            "CAT-II" => Some("FAT".to_string()),
            _ => None,
        };
        Some(Self {
            exam,
            permitted: lower.contains("permit"),
            next_exam,
        })
    }

    /// [`AttendanceCourse::debar_line`] for a course standing at `plan`.
    fn line(&self, plan: &Standing, lab: bool) -> String {
        let mut parts = vec![if self.exam.trim().is_empty() {
            "Debarred".to_string()
        } else {
            format!("Debarred for {}", self.exam)
        }];
        if self.permitted {
            parts.push("Permitted".to_string());
        }
        if let Some(next) = &self.next_exam {
            parts.push(if plan.safe {
                format!("Clear for {next}")
            } else {
                let n = plan.must_attend;
                format!("Clear for {next} at {n}{} more", labs(lab, n))
            });
        }
        parts.join(" · ")
    }
}

/// Puts each held class's status from `histories` (by course code and
/// kind, as [`AttendanceDetail::classes`]) on `days`: the history row of
/// that date with the same slot, else the same start, else the next unused
/// one. Not posted stays none.
pub(crate) fn fill_posted(
    days: &mut [TimetableDay],
    histories: &HashMap<(String, ClassKind), Vec<AttendedClass>>,
) {
    for day in days {
        let mut used: HashMap<(String, ClassKind), Vec<usize>> = HashMap::new();
        for class in &mut day.classes {
            if class.state != ClassDay::Held {
                continue;
            }
            let key = (class.slot.code.clone(), class.slot.kind);
            let Some(rows) = histories.get(&key) else {
                continue;
            };
            let used = used.entry(key).or_default();
            let free: Vec<(usize, &AttendedClass)> = rows
                .iter()
                .enumerate()
                .filter(|(i, row)| row.day == day.day && !used.contains(i))
                .collect();
            let pick = free
                .iter()
                .find(|(_, row)| row.slot == class.slot.slot)
                .or_else(|| {
                    free.iter()
                        .find(|(_, row)| row.start == Some(class.slot.start))
                })
                .or_else(|| free.first());
            if let Some((i, row)) = pick {
                used.push(*i);
                class.posted = row.status;
            }
        }
    }
}

/// Whether a saved history matches `course`'s summary: as many classes
/// attended (present or on duty) and listed (posted or not) as VTOP counts,
/// labs twice. A history behind the summary is fetched again.
pub(crate) fn history_in_sync(course: &AttendanceCourse, classes: &[AttendedClass]) -> bool {
    let factor = if course.kind == ClassKind::Lab { 2 } else { 1 };
    let attended = classes
        .iter()
        .filter(|class| {
            matches!(
                class.status,
                Some(ClassStatus::Present | ClassStatus::OnDuty)
            )
        })
        .count() as i32;
    attended * factor == course.attended && classes.len() as i32 * factor == course.total
}

/// The last day VTOP's summary covers: the latest class in a history that
/// matches it (Flutter's `countedThrough`).
pub(crate) fn counted_through(course: &AttendanceCourse, classes: &[AttendedClass]) -> Option<i32> {
    history_in_sync(course, classes)
        .then(|| classes.iter().map(|class| class.day).max())
        .flatten()
}

/// Where a course stands against 75%, with the strict arithmetic of the
/// attendance calculator.
#[derive(Debug, PartialEq)]
pub(super) struct Standing {
    pub(super) safe: bool,
    pub(super) can_skip: i32,
    pub(super) must_attend: i32,
    pub(super) advice: String,
}

/// The standing for `attended` of `total` classes. VTOP's `reported`
/// percentage decides safe or not (it is what debars); the counts only say
/// by how much, and never contradict it.
/// `lab` names the unit in the advice ("Attend 2 labs").
pub(super) fn standing(attended: i32, total: i32, reported: Option<f64>, lab: bool) -> Standing {
    let (a, t) = (i64::from(attended), i64::from(total));
    let by_counts = t == 0 || a as f64 / t as f64 * 100.0 >= THRESHOLD;
    let safe = match reported {
        Some(percent) if t > 0 => percent >= THRESHOLD,
        _ => by_counts,
    };
    let (can_skip, must_attend) = if safe {
        let skip = if t == 0 {
            0
        } else {
            (4 * a - 3 * t).div_euclid(3)
        };
        (skip.max(0) as i32, 0)
    } else {
        (0, (3 * t - 4 * a).max(1) as i32)
    };
    let advice = match () {
        _ if t == 0 => "No classes yet".to_string(),
        _ if !safe => format!("Attend {must_attend}{}", labs(lab, must_attend)),
        _ if can_skip == 0 => "Don't skip".to_string(),
        _ => format!("Can skip {can_skip}{}", labs(lab, can_skip)),
    };
    Standing {
        safe,
        can_skip,
        must_attend,
        advice,
    }
}

/// " lab" / " labs" after a count for labs, which count in sessions;
/// nothing for theory.
fn labs(lab: bool, n: i32) -> &'static str {
    match (lab, n) {
        (false, _) => "",
        (true, 1) => " lab",
        (true, _) => " labs",
    }
}

/// The longest run of present or on duty in `classes` (day, start, status),
/// by time; only an absence breaks it, and classes not posted yet or
/// cancelled neither extend nor break it. Its length, first day and last day.
pub(crate) fn longest_run(
    classes: &[(i32, Option<i32>, Option<ClassStatus>)],
) -> Option<(i32, i32, i32)> {
    let mut classes = classes.to_vec();
    classes.sort_by_key(|&(day, start, _)| (day, start));
    let (mut best, mut run) = (None, None);
    for (day, _, status) in classes {
        match status {
            None | Some(ClassStatus::Other) => {}
            Some(ClassStatus::Present | ClassStatus::OnDuty) => {
                let (length, first, _) = run.unwrap_or((0, day, day));
                let next = (length + 1, first, day);
                run = Some(next);
                if best.is_none_or(|(most, _, _)| next.0 > most) {
                    best = Some(next);
                }
            }
            Some(ClassStatus::Absent) => run = None,
        }
    }
    best
}

/// The total VTOP's whole-number `reported` percentage of `attended` is
/// over: the largest from `attended` to `total` that gives it, rounded up as
/// VTOP rounds (31 of 34 is 92%). VTOP's total counts classes not posted
/// yet, its percentage does not. `total` when nothing matches.
pub(super) fn counted_total(attended: i32, total: i32, reported: Option<f64>) -> i32 {
    let Some(percent) = reported.filter(|percent| percent.fract() == 0.0) else {
        return total;
    };
    let a = i64::from(attended) * 100;
    (attended.max(1)..=total)
        .rev()
        .find(|&t| {
            let t = i64::from(t);
            ((a + t - 1) / t) as f64 == percent
        })
        .unwrap_or(total)
}

/// The course's plans from `today` at `minute` (local, India): classes of
/// its kind in `slots` on each day `calendar` holds them (a lab session is
/// one slot), today's only when not started; until the next exam and until
/// the last class day. Not posted: what VTOP lists as such, plus the classes
/// held after `counted_through` (the last day VTOP's summary covers) up to
/// now. Strict 75% arithmetic, like the calculator.
pub(crate) fn attendance_plans(
    course: &AttendanceCourse,
    slots: &[ClassSlot],
    calendar: &SemesterCalendar,
    today: i32,
    minute: i32,
    counted_through: Option<i32>,
) -> Vec<AttendancePlan> {
    let Some(last) = calendar.last_class_day() else {
        return Vec::new();
    };
    let lab = course.kind == ClassKind::Lab;
    let (attended, held, unposted) = if lab {
        (
            course.attended / 2,
            (course.counted + 1) / 2,
            (course.total - course.counted + 1) / 2,
        )
    } else {
        (
            course.attended,
            course.counted,
            course.total - course.counted,
        )
    };
    let mine: Vec<&ClassSlot> = slots
        .iter()
        .filter(|slot| slot.code == course.code && slot.kind == course.kind)
        .collect();
    let meetings = |day: i32| -> Vec<&&ClassSlot> {
        let weekday = calendar.weekday_of(day);
        mine.iter().filter(|slot| slot.day == weekday).collect()
    };
    // Held since the summary's last day, not on VTOP yet.
    let unposted = unposted
        + counted_through
            .filter(|&through| through < today)
            .map_or(0, |through| {
                (through + 1..=today.min(last))
                    .filter(|&day| calendar.holds_classes(day, lab))
                    .map(|day| {
                        meetings(day)
                            .into_iter()
                            .filter(|slot| day < today || slot.start <= minute)
                            .count() as i32
                    })
                    .sum()
            });
    let upcoming = |until_day: i32| -> Vec<PlanDay> {
        (today..until_day.min(last + 1))
            .filter(|&day| calendar.holds_classes(day, lab))
            .map(|day| {
                let weekday = calendar.weekday_of(day);
                let classes = mine
                    .iter()
                    .filter(|slot| slot.day == weekday && (day > today || slot.start > minute))
                    .count() as i32;
                PlanDay { day, classes }
            })
            .filter(|day| day.classes > 0)
            .collect()
    };
    let plan = |until: String, until_day: i32| {
        let days = upcoming(until_day);
        let upcoming = days.iter().map(|day| day.classes).sum();
        let left = i64::from(upcoming + unposted);
        let (a, t) = (i64::from(attended), i64::from(held));
        let total = t + left;
        let must = (3 * total - 4 * a + 3).div_euclid(4).max(0);
        AttendancePlan {
            until,
            until_day,
            attended,
            held,
            upcoming,
            unposted,
            can_miss: (4 * (a + left) - 3 * total).div_euclid(4).clamp(0, left) as i32,
            must_attend: (must <= left).then_some(must as i32),
            best_percent: if total == 0 {
                0.0
            } else {
                (a + left) as f64 / total as f64 * 100.0
            },
            days,
        }
    };
    let mut plans = Vec::new();
    if let Some((day, name)) = calendar
        .upcoming_exam(today)
        .filter(|(day, _)| *day <= last)
    {
        plans.push(plan(name, day));
    }
    plans.push(plan(String::new(), last + 1));
    plans
}

impl From<AttendanceData> for Attendance {
    fn from(data: AttendanceData) -> Self {
        let course = |record: raw::AttendanceRecord| {
            let attended = record.classes_attended.trim().parse().unwrap_or(0);
            let total: i32 = record.total_classes.trim().parse().unwrap_or(0);
            let lab = record.course_type.ends_with("LA") || record.course_type.ends_with("LO");
            let reported: Option<f64> = record
                .attendance_percentage
                .replace('%', "")
                .trim()
                .parse()
                .ok();
            let counted = counted_total(attended, total, reported);
            // Labs meet in two-period sessions VTOP counts twice; an odd total
            // rounds up and an odd attended count down.
            let plan = if lab {
                standing(attended / 2, (counted + 1) / 2, reported, true)
            } else {
                standing(attended, counted, reported, false)
            };
            let computed = if counted == 0 {
                0.0
            } else {
                f64::from(attended) / f64::from(counted) * 100.0
            };
            let (code, name) = split_course_name(&record.course_name, &record.course_code);
            let debar = Debar::parse(&record.debar_status);
            let debar_line = debar.as_ref().map(|debar| debar.line(&plan, lab));
            AttendanceCourse {
                key: format!("{code}.{}", if lab { "lab" } else { "theory" }),
                code,
                name,
                kind: if lab {
                    ClassKind::Lab
                } else {
                    ClassKind::Theory
                },
                category: record.category,
                faculty: person_name(&record.faculty_detail),
                attended,
                total,
                counted,
                percent: reported.unwrap_or(computed),
                safe: plan.safe,
                can_skip: plan.can_skip,
                must_attend: plan.must_attend,
                advice: plan.advice,
                debarred: debar.is_some(),
                debar,
                debar_line,
                course_id: record.course_id,
                course_type: record.course_type,
            }
        };
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            courses: data.records.into_iter().map(course).collect(),
        }
    }
}

impl AttendanceDetail {
    pub(crate) fn new(
        course: AttendanceCourse,
        data: FullAttendanceData,
        plans: Vec<AttendancePlan>,
    ) -> Self {
        let mut classes: Vec<AttendedClass> = data
            .records
            .into_iter()
            .filter_map(|record| {
                let day = history_day(&record.date)?;
                // "WED / 11:00-12:40"
                let time = record.day_time.rsplit('/').next().unwrap_or_default();
                let (start, end) = time.split_once('-').unzip();
                Some(AttendedClass {
                    day,
                    weekday: (day + 3).rem_euclid(7),
                    slot: record.slot,
                    start: start.and_then(minutes),
                    end: end.and_then(minutes),
                    status: match record.status.to_lowercase().replace(' ', "").as_str() {
                        "present" => Some(ClassStatus::Present),
                        "onduty" => Some(ClassStatus::OnDuty),
                        "absent" => Some(ClassStatus::Absent),
                        "notposted" | "" => None,
                        _ => Some(ClassStatus::Other),
                    },
                    remark: record.remark,
                })
            })
            .collect();
        classes.sort_by_key(|class| (std::cmp::Reverse(class.day), std::cmp::Reverse(class.start)));
        let posted = || classes.iter().filter_map(|class| class.status);
        let count = |status| posted().filter(|&posted| posted == status).count() as i32;
        Self {
            course,
            updated_at: data.update_time as i64,
            present: count(ClassStatus::Present),
            on_duty: count(ClassStatus::OnDuty),
            absent: count(ClassStatus::Absent),
            streak: posted()
                .filter(|&status| status != ClassStatus::Other)
                .take_while(|status| matches!(status, ClassStatus::Present | ClassStatus::OnDuty))
                .count() as i32,
            last_missed: classes
                .iter()
                .find(|class| class.status == Some(ClassStatus::Absent))
                .map(|class| class.day),
            classes,
            plans,
        }
    }
}

/// A history date, "19-11-2025" or "01-Jul-2026", as days since 1970-01-01.
pub(crate) fn history_day(raw: &str) -> Option<i32> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let mut parts = raw.trim().split('-').map(str::trim);
    let day: u32 = parts.next()?.parse().ok()?;
    let month = parts.next()?.to_lowercase();
    let month = match month.parse::<u32>() {
        Ok(month) => month,
        Err(_) => MONTHS.iter().position(|name| month.starts_with(name))? as u32 + 1,
    };
    let year: u32 = parts.next()?.parse().ok()?;
    epoch_day(&format!("{year:04}-{month:02}-{day:02}"))
}
