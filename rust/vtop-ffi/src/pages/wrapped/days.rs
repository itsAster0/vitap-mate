//! Wrapped's day-by-day slides: the attendance mosaic and the attendance arc.

use super::*;
use crate::pages::calendar::CalendarKind;

/// The mosaic needs this many posted classes (in VTOP's units).
pub(super) const MOSAIC_MIN: i32 = 15;
/// A month needs this many counted classes to be the best or worst.
pub(super) const MONTH_MIN: i32 = 10;
/// The worst month is called out only under this percent.
pub(super) const SLUMP_BELOW: i32 = 85;
/// The arc starts at this counted class, past the first days' swings, and needs this many
/// more after it.
pub(super) const ARC_FROM: i32 = 10;
pub(super) const ARC_MIN_AFTER: i32 = 10;
/// Points the arc is thinned to, before its peak and low are added back.
pub(super) const ARC_POINTS: usize = 20;
/// A fall the arc marks, in percentage points.
pub(super) const ARC_DROP: f64 = 3.0;

/// Counted classes by day, in day order: (attended, counted, on duty, absent) in VTOP's units.
fn by_day(classes: &[Taken]) -> BTreeMap<i32, (i32, i32, bool, bool)> {
    let mut days: BTreeMap<i32, (i32, i32, bool, bool)> = BTreeMap::new();
    for (course, class) in classes.iter().filter(|(_, class)| is_counted(class)) {
        let day = days.entry(class.day).or_default();
        let unit = periods(course);
        day.1 += unit;
        if is_attended(class) {
            day.0 += unit;
        }
        day.2 |= class.status == Some(ClassStatus::OnDuty);
        day.3 |= class.status == Some(ClassStatus::Absent);
    }
    days
}

/// None under [`MOSAIC_MIN`] posted classes.
pub(super) fn mosaic(
    classes: &[Taken],
    calendar: &SemesterCalendar,
    first_day: i32,
    window_end: i32,
) -> Option<WrappedMosaic> {
    let days = by_day(classes);
    if days.values().map(|day| day.1).sum::<i32>() < MOSAIC_MIN {
        return None;
    }
    let start = first_day - records::weekday(first_day);
    let end = window_end + 6 - records::weekday(window_end);
    let cells = (start..=end)
        .map(|day| {
            if day < first_day || day > window_end {
                return WrappedDay::Outside;
            }
            if let Some(&(attended, _, on_duty, absent)) = days.get(&day) {
                return match (attended, absent, on_duty) {
                    (0, _, _) => WrappedDay::AllAbsent,
                    (_, true, _) => WrappedDay::Mixed,
                    (_, false, true) => WrappedDay::OnDuty,
                    (_, false, false) => WrappedDay::AllPresent,
                };
            }
            match calendar.kind_on(day) {
                Some((CalendarKind::Exam { .. }, _)) => WrappedDay::Exam,
                Some((_, true)) => WrappedDay::Holiday,
                _ => WrappedDay::NoClass,
            }
        })
        .collect();
    // Each month with its attended classes, until the percent is worked out.
    let mut months: Vec<(WrappedMonth, i32)> = Vec::new();
    for day in first_day..=window_end {
        let month = month_of(day);
        let (attended, counted) = days.get(&day).map_or((0, 0), |d| (d.0, d.1));
        match months.last_mut() {
            Some((last, sum)) if last.month == month => {
                last.last_day = day;
                last.counted += counted;
                *sum += attended;
            }
            _ => months.push((
                WrappedMonth {
                    month,
                    first_day: day,
                    last_day: day,
                    counted,
                    percent: None,
                },
                attended,
            )),
        }
    }
    let months: Vec<WrappedMonth> = months
        .into_iter()
        .map(|(month, attended)| WrappedMonth {
            percent: percent(attended, month.counted),
            ..month
        })
        .collect();
    let ranked: Vec<(usize, i32)> = months
        .iter()
        .enumerate()
        .filter(|(_, month)| month.counted >= MONTH_MIN)
        .filter_map(|(at, month)| Some((at, month.percent?)))
        .collect();
    // The earliest of equals; none when every ranked month is equal.
    let best = ranked
        .iter()
        .max_by_key(|&&(at, p)| (p, std::cmp::Reverse(at)))
        .copied();
    let worst = ranked.iter().min_by_key(|&&(at, p)| (p, at)).copied();
    let (best_month, worst_month) = match (best, worst) {
        (Some(best), Some(worst)) if best.1 > worst.1 => (
            Some(best.0 as i32),
            // A month at 88% is no slump.
            (worst.1 < SLUMP_BELOW).then_some(worst.0 as i32),
        ),
        _ => (None, None),
    };
    Some(WrappedMosaic {
        first_day: start,
        days: cells,
        months,
        best_month,
        worst_month,
        class_days: days.len() as i32,
    })
}

/// The calendar's exams with a day from `first_day` to `window_end`, by first day.
fn exam_spans(calendar: &SemesterCalendar, first_day: i32, window_end: i32) -> Vec<WrappedSpan> {
    calendar
        .exam_spans()
        .into_iter()
        .filter(|&(_, from, to)| to >= first_day && from <= window_end)
        .map(|(name, first_day, last_day)| WrappedSpan {
            name,
            first_day,
            last_day,
        })
        .collect()
}

/// `day`'s month, 1 = January.
fn month_of(day: i32) -> i32 {
    records::civil_date(i64::from(day))
        .get(5..7)
        .and_then(|month| month.parse().ok())
        .unwrap_or(1)
}

/// None under [`ARC_FROM`] + [`ARC_MIN_AFTER`] counted classes.
pub(super) fn arc(
    classes: &[Taken],
    calendar: &SemesterCalendar,
    first_day: i32,
    window_end: i32,
) -> Option<WrappedArc> {
    let days = by_day(classes);
    let (mut attended, mut counted) = (0, 0);
    let mut line: Vec<WrappedPoint> = Vec::new();
    for (&day, &(a, c, _, _)) in &days {
        attended += a;
        counted += c;
        if counted >= ARC_FROM {
            line.push(WrappedPoint {
                day,
                percent: f64::from(attended) * 100.0 / f64::from(counted),
            });
        }
    }
    if counted < ARC_FROM + ARC_MIN_AFTER || line.len() < 2 {
        return None;
    }
    let (first, last) = (line.first()?.percent, line.last()?.percent);
    // The earliest peak and low of equals.
    let extreme = |better: fn(f64, f64) -> bool| {
        line.iter()
            .enumerate()
            .reduce(|best, at| {
                if better(at.1.percent, best.1.percent) {
                    at
                } else {
                    best
                }
            })
            .map(|(at, point)| (at, point.percent))
    };
    let ((peak, _), (low, low_percent)) = (extreme(|a, b| a > b)?, extreme(|a, b| a < b)?);
    let drop = biggest_drop(&line).filter(|&(_, _, fall)| fall >= ARC_DROP);
    let mut keep: Vec<usize> = (0..ARC_POINTS)
        .map(|i| i * (line.len() - 1) / (ARC_POINTS - 1))
        .chain([peak, low])
        .chain(drop.into_iter().flat_map(|(from, to, _)| [from, to]))
        .collect();
    keep.sort_unstable();
    keep.dedup();
    let shape = if last - low_percent >= f64::from(COMEBACK_POINTS) && low + 1 < line.len() {
        WrappedArcShape::Comeback
    } else if first - last >= f64::from(COMEBACK_POINTS) {
        WrappedArcShape::Slide
    } else if last - first >= f64::from(COMEBACK_POINTS) {
        WrappedArcShape::Climb
    } else {
        WrappedArcShape::Steady
    };
    let exams = exam_spans(calendar, first_day, window_end);
    Some(WrappedArc {
        peak: line.get(peak)?.clone(),
        low: line.get(low)?.clone(),
        drop: drop.and_then(|(from, to, _)| {
            Some(WrappedDrop {
                from: line.get(from)?.clone(),
                to: line.get(to)?.clone(),
            })
        }),
        points: keep
            .into_iter()
            .filter_map(|at| line.get(at).cloned())
            .collect(),
        phases: phases(&days, &exams),
        exams,
        shape,
    })
}

/// The indices of the largest fall from a point to a later one, and the fall; none on a line
/// that never falls.
fn biggest_drop(line: &[WrappedPoint]) -> Option<(usize, usize, f64)> {
    let mut best: Option<(usize, usize, f64)> = None;
    let mut top: Option<(usize, f64)> = None;
    for (at, point) in line.iter().enumerate() {
        let (from, high) = match top {
            Some(top) if top.1 >= point.percent => top,
            _ => {
                top = Some((at, point.percent));
                continue;
            }
        };
        let fall = high - point.percent;
        if fall > 0.0 && best.is_none_or(|(_, _, most)| fall > most) {
            best = Some((from, at, fall));
        }
    }
    best
}

/// Attendance between the CATs' first days: before the first, between each, after the last.
fn phases(
    days: &BTreeMap<i32, (i32, i32, bool, bool)>,
    exams: &[WrappedSpan],
) -> Vec<WrappedPhase> {
    let cats: Vec<&WrappedSpan> = exams
        .iter()
        .filter(|exam| exam.name == "CAT" || exam.name.starts_with("CAT-"))
        .collect();
    if cats.is_empty() {
        return Vec::new();
    }
    let mut phases: Vec<WrappedPhase> = Vec::new();
    for at in 0..=cats.len() {
        let after = at.checked_sub(1).and_then(|i| cats.get(i));
        let before = cats.get(at);
        let (from, to) = (
            after.map_or(i32::MIN, |cat| cat.first_day),
            before.map_or(i32::MAX, |cat| cat.first_day),
        );
        let (attended, counted) = days
            .range(from..to)
            .fold((0, 0), |(a, c), (_, day)| (a + day.0, c + day.1));
        if let Some(percent) = percent(attended, counted) {
            phases.push(WrappedPhase {
                after: after.map(|cat| cat.name.clone()),
                before: before.map(|cat| cat.name.clone()),
                percent,
                lowest: false,
            });
        }
    }
    let lowest = phases.iter().map(|phase| phase.percent).min();
    let ties = phases
        .iter()
        .filter(|phase| Some(phase.percent) == lowest)
        .count();
    if phases.len() > 1 && ties == 1 {
        for phase in &mut phases {
            phase.lowest = Some(phase.percent) == lowest;
        }
    }
    phases
}
