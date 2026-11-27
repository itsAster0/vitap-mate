//! Wrapped's FFCS verdict: the week's timetable graded on four facts a student can check
//! against it, 25 points each.

use super::*;

/// Each part's points.
pub(super) const PART: i32 = 25;
/// The lunch break, kept free or not, and left out of the waiting between classes.
pub(super) const LUNCH_FROM: i32 = 12 * 60 + 30;
pub(super) const LUNCH_TO: i32 = 14 * 60;
/// Points off for each 8 AM class a week.
pub(super) const EARLY_COST: i32 = 6;
/// A week with this much waiting between classes scores no gap points.
pub(super) const GAPS_ZERO: i32 = 10 * 60;
/// A break between classes up to this long is the walk to the next one, not waiting.
pub(super) const WALK: i32 = 15;
/// Grades from these scores down, S first.
pub(super) const GRADES: [(i32, WrappedGrade); 5] = [
    (85, WrappedGrade::S),
    (70, WrappedGrade::A),
    (55, WrappedGrade::B),
    (40, WrappedGrade::C),
    (25, WrappedGrade::D),
];

/// None without a class in the timetable.
pub(super) fn ffcs(slots: &[ClassSlot]) -> Option<WrappedFfcs> {
    // Each class once, by its day and start.
    let mut week: Vec<(i32, i32, i32, bool, String)> = slots
        .iter()
        .filter(|slot| (0..7).contains(&slot.day) && slot.end > slot.start)
        .map(|slot| {
            let block = slot
                .block
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            (
                slot.day,
                slot.start,
                slot.end,
                slot.kind == ClassKind::Lab,
                block,
            )
        })
        .collect();
    week.sort_by_key(|&(day, start, end, _, _)| (day, start, end));
    week.dedup_by_key(|(day, start, _, _, _)| (*day, *start));
    if week.is_empty() {
        return None;
    }
    let days: Vec<i32> = (0..7)
        .filter(|day| week.iter().any(|slot| slot.0 == *day))
        .collect();
    let free_days: Vec<i32> = (0..6).filter(|day| !days.contains(day)).collect();
    let early = week.iter().filter(|slot| slot.1 < EARLY_BEFORE).count() as i32;
    let (mut gap_minutes, mut lunch_free_days, mut sprints) = (0, 0, 0);
    let mut pairs: Vec<(String, i32)> = Vec::new();
    for &day in &days {
        let today: Vec<&(i32, i32, i32, bool, String)> =
            week.iter().filter(|slot| slot.0 == day).collect();
        lunch_free_days += i32::from(
            !today
                .iter()
                .any(|slot| slot.1 < LUNCH_TO && slot.2 > LUNCH_FROM),
        );
        for pair in today.windows(2) {
            let [from, to] = pair else {
                continue;
            };
            let gap = to.1 - from.2;
            if gap > WALK {
                let lunch = (to.1.min(LUNCH_TO) - from.2.max(LUNCH_FROM)).max(0);
                gap_minutes += gap - lunch;
            } else if !from.4.is_empty() && !to.4.is_empty() && from.4 != to.4 {
                sprints += 1;
                pairs.push((format!("{}\u{1}{}", from.4, to.4), 1));
            }
        }
    }
    let class_days = days.len() as i32;
    let parts = [
        (WrappedFfcsPart::Early, (PART - EARLY_COST * early).max(0)),
        (
            WrappedFfcsPart::Gaps,
            (PART - PART * gap_minutes / GAPS_ZERO).max(0),
        ),
        (
            WrappedFfcsPart::Lunch,
            PART * lunch_free_days / class_days.max(1),
        ),
        (
            WrappedFfcsPart::FreeDays,
            match free_days.len() {
                0 => 0,
                1 => 15,
                _ => PART,
            },
        ),
    ];
    let score: i32 = parts.iter().map(|&(_, points)| points).sum();
    let best = parts
        .iter()
        .rev()
        .max_by_key(|&&(_, points)| points)
        .map_or(WrappedFfcsPart::Early, |&(part, _)| part);
    let worst = parts
        .iter()
        .min_by_key(|&&(_, points)| points)
        .map_or(WrappedFfcsPart::Early, |&(part, _)| part);
    let (sprint_from, sprint_to) = most(pairs)
        .and_then(|(pair, _)| {
            let (from, to) = pair.split_once('\u{1}')?;
            Some((from.to_string(), to.to_string()))
        })
        .unzip();
    Some(WrappedFfcs {
        score,
        grade: GRADES
            .iter()
            .find(|&&(at, _)| score >= at)
            .map_or(WrappedGrade::E, |&(_, grade)| grade),
        best,
        worst,
        free_days,
        early,
        gap_minutes,
        lunch_free_days,
        class_days,
        sprints,
        sprint_from,
        sprint_to,
        week: week
            .into_iter()
            .map(|(day, start, end, lab, _)| WrappedWeekSlot {
                day,
                start,
                end,
                lab,
            })
            .collect(),
    })
}
