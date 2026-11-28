//! Wrapped's titles: every title whose facts hold, scored by how far past its bar they are, so
//! a strong streak beats a small comeback. One table of rules; drop new ones into [`RULES`].

use super::*;

/// The Iron Streak: a run of this share (percent) of all posted classes, and this many.
pub(super) const IRON_STREAK_PERCENT: i32 = 15;
pub(super) const IRON_STREAK_MIN: i32 = 15;

/// One title: its score from 0 to 1 when the facts earn it. `grades` titles go with Hide grades;
/// `hostel` ones with Hide hostel and outings.
pub(super) struct Rule {
    pub(super) title: WrappedTitle,
    pub(super) grades: bool,
    pub(super) hostel: bool,
    pub(super) score: fn(&WrappedReport) -> Option<f64>,
}

use WrappedTitle as T;

/// In the enum's order, which breaks ties.
pub(super) const RULES: &[Rule] = &[
    Rule {
        title: T::Topper,
        grades: true,
        hostel: false,
        score: |r| {
            let gpa = r.grades.as_ref()?.gpa;
            (gpa >= 9.0).then_some(0.55 + (gpa - 9.0) * 0.45)
        },
    },
    Rule {
        title: T::GhostGenius,
        grades: true,
        hostel: false,
        score: |r| {
            let gpa = r.grades.as_ref()?.gpa;
            let att = f64::from(r.attendance.as_ref()?.percent);
            (att < 82.0 && gpa >= 8.5)
                .then_some(0.6 + (82.0 - att) / 12.0 * 0.2 + (gpa - 8.5) / 1.5 * 0.2)
        },
    },
    Rule {
        title: T::ExamClutch,
        grades: true,
        hostel: false,
        score: |r| {
            let change = r.marks.as_ref()?.cat_change.filter(|&c| c >= 8)?;
            Some(0.5 + (f64::from(change) / 25.0).min(0.45))
        },
    },
    Rule {
        title: T::Perfectionist,
        grades: true,
        hostel: false,
        score: |r| {
            let f = r.full_marks.as_ref()?;
            (f.count >= 5 && f.count * 100 >= f.of * 25)
                .then(|| 0.5 + (f64::from(f.count) / 20.0).min(0.45))
        },
    },
    Rule {
        title: T::Comeback,
        grades: false,
        hostel: false,
        score: |r| {
            let c = r.comeback.as_ref()?;
            let gain = c.final_percent - c.low_percent;
            (gain >= 10).then(|| 0.45 + (f64::from(gain - 10) / 25.0).min(0.5))
        },
    },
    Rule {
        title: T::EdgeWalker,
        grades: false,
        hostel: false,
        score: |r| {
            let spare = r.close_call.as_ref()?.spare.filter(|&s| s <= 1)?;
            (r.attendance.as_ref()?.percent <= 82).then_some(if spare == 0 { 0.9 } else { 0.75 })
        },
    },
    Rule {
        title: T::IronStreak,
        grades: false,
        hostel: false,
        score: |r| {
            let s = r.streak.as_ref()?;
            (s.length >= IRON_STREAK_MIN && s.length * 100 >= s.total * IRON_STREAK_PERCENT)
                .then(|| 0.5 + (f64::from(s.length - IRON_STREAK_MIN) / 40.0).min(0.45))
        },
    },
    Rule {
        title: T::EarlyRegular,
        grades: false,
        hostel: false,
        score: |r| {
            let h = r.habits.as_ref().filter(|h| h.early_classes >= 15)?;
            let p = h.early_percent.filter(|&p| p >= 90)?;
            Some(0.55 + f64::from(p - 90) / 10.0 * 0.35)
        },
    },
    Rule {
        title: T::Regular,
        grades: false,
        hostel: false,
        score: |r| {
            let att = r.attendance.as_ref()?.percent;
            (att >= 90 || r.never_missed.len() >= 2)
                .then(|| 0.45 + f64::from((att - 90).max(0)) / 10.0 * 0.4)
        },
    },
    Rule {
        title: T::ClubCaptain,
        grades: false,
        hostel: false,
        score: |r| {
            let o = r.on_duty.as_ref()?;
            (o.classes >= 12 || o.days >= 5)
                .then(|| 0.5 + (f64::from((o.classes - 12).max(0)) / 24.0).min(0.45))
        },
    },
    Rule {
        title: T::LabRat,
        grades: false,
        hostel: false,
        score: |r| {
            let t = r.time.as_ref().filter(|t| t.minutes > 0)?;
            let lab = f64::from(t.lab_minutes) * 100.0 / f64::from(t.minutes);
            (lab >= 40.0).then(|| 0.45 + ((lab - 40.0) / 30.0).min(0.45))
        },
    },
    Rule {
        title: T::Explorer,
        grades: false,
        hostel: true,
        score: |r| {
            let o = r.outings.as_ref()?;
            let trips = o.leaves + o.weekends;
            (trips >= 8 || o.days_away >= 18)
                .then(|| 0.45 + (f64::from((trips - 8).max(0)) / 12.0).min(0.45))
        },
    },
    Rule {
        title: T::Stairmaster,
        grades: false,
        hostel: false,
        score: |r| {
            let m = r.moves.as_ref()?;
            (m.floors >= 250 || m.most_hops >= 4)
                .then(|| 0.45 + (f64::from((m.floors - 250).max(0)) / 400.0).min(0.4))
        },
    },
    Rule {
        title: T::NightOwl,
        grades: false,
        hostel: false,
        score: |r| {
            let d = r
                .day_half
                .as_ref()
                .filter(|d| !d.morning && d.percent >= 65)?;
            Some(0.45 + (f64::from(d.percent - 65) / 30.0).min(0.4))
        },
    },
];

/// Every title earned, the highest score first (rule order among equals), then The Steady
/// Cruiser.
pub(super) fn traits(report: &WrappedReport) -> Vec<WrappedTrait> {
    let mut earned: Vec<WrappedTrait> = RULES
        .iter()
        .filter_map(|rule| {
            let score = (rule.score)(report)?;
            Some(WrappedTrait {
                title: rule.title,
                score: (score.clamp(0.0, 1.0) * 100.0).round().max(1.0) as i32,
                grades: rule.grades,
                hostel: rule.hostel,
            })
        })
        .collect();
    // Stable, so rule order breaks ties.
    earned.sort_by_key(|t| std::cmp::Reverse(t.score));
    earned.push(WrappedTrait {
        title: WrappedTitle::SteadyCruiser,
        score: 1,
        grades: false,
        hostel: false,
    });
    earned
}

/// The first of `traits` that `grades` and `hostel` (false: hidden) leave.
pub(super) fn title(traits: &[WrappedTrait], grades: bool, hostel: bool) -> WrappedTitle {
    traits
        .iter()
        .find(|t| (grades || !t.grades) && (hostel || !t.hostel))
        .map_or(WrappedTitle::SteadyCruiser, |t| t.title)
}
