//! The forms' rules: each field's problem as the student reads it, and where the general
//! form's times start.

use vtop_core::outing::{
    self, DayProblem, GeneralRules, LeavingProblem, NOTICE_MINUTES as NOTICE, ReturningProblem,
    TextProblem, WeekendRules, check_general, check_weekend,
};

use super::time::{at, clock, day_of, format_day, minute_of};
use super::{
    GeneralOutingCheck, GeneralOutingRules, OutingMoment, WeekendOutingCheck, WeekendOutingRules,
};

/// The earliest and latest times a general outing form offers for `hours`.
pub(super) fn window(hours: &[u8]) -> Option<(i32, i32)> {
    Some((
        i32::from(*hours.first()?) * 60,
        i32::from(*hours.last()?) * 60,
    ))
}

fn text_error(problem: Option<TextProblem>, missing: &str) -> Option<String> {
    problem.map(|problem| match problem {
        TextProblem::Missing => missing.to_string(),
        TextProblem::TooLong { max_length } => format!("At most {max_length} characters"),
    })
}

impl GeneralOutingCheck {
    pub(super) fn first(&self) -> Option<String> {
        [&self.place, &self.purpose, &self.leaving, &self.returning]
            .into_iter()
            .flatten()
            .next()
            .cloned()
    }
}

impl WeekendOutingCheck {
    pub(super) fn first(&self) -> Option<String> {
        [&self.purpose, &self.day, &self.contact]
            .into_iter()
            .flatten()
            .next()
            .cloned()
    }
}

pub(super) fn general_check(
    rules: &GeneralOutingRules,
    place: &str,
    purpose: &str,
    leaving: OutingMoment,
    returning: OutingMoment,
    now: (i32, i32),
) -> GeneralOutingCheck {
    let problems = check_general(
        &GeneralRules {
            out_hours: &rules.out_hours,
            in_hours: &rules.in_hours,
            place_max_length: rules.place_max_length,
            purpose_max_length: rules.purpose_max_length,
            max_days_ahead: rules.max_days_ahead,
        },
        place,
        purpose,
        at(leaving),
        at(returning),
        i64::from(now.0) * 1440 + i64::from(now.1),
    );
    let leaving_error = problems.leaving.and_then(|problem| match problem {
        LeavingProblem::TooFarAhead { last_day } => Some(format!(
            "Up to {} days ahead ({})",
            rules.max_days_ahead,
            format_day(last_day as i32, "d MMM")
        )),
        LeavingProblem::OutsideHours => window(&rules.out_hours)
            .map(|(a, b)| format!("Leave between {} and {}", clock(a), clock(b))),
        LeavingProblem::TooSoon { earliest } => Some(format!(
            "Needs 24 h notice: leave after {}, {}",
            format_day(day_of(earliest), "EEE d MMM"),
            clock(minute_of(earliest))
        )),
    });
    let returning_error = problems.returning.and_then(|problem| match problem {
        ReturningProblem::OutsideHours => window(&rules.in_hours)
            .map(|(a, b)| format!("Be back between {} and {}", clock(a), clock(b))),
        ReturningProblem::NotAfterLeaving => Some("Has to be after you leave".into()),
    });
    let away = (leaving_error.is_none() && returning_error.is_none())
        .then(|| trip_length(at(returning) - at(leaving)))
        .filter(|length| !length.is_empty())
        .map(|length| format!("Away for {length}"));
    GeneralOutingCheck {
        place: text_error(problems.place, "Add where you're going"),
        purpose: text_error(problems.purpose, "Add why you're going"),
        leaving: leaving_error,
        returning: returning_error,
        away,
    }
}

/// "1 d 3 h", "5 h 30 min".
pub(super) fn trip_length(span: i64) -> String {
    if span <= 0 {
        return String::new();
    }
    let (days, hours, minutes) = (span / 1440, span / 60 % 24, span % 60);
    [
        (days > 0).then(|| format!("{days} d")),
        (hours > 0).then(|| format!("{hours} h")),
        (days == 0 && minutes > 0).then(|| format!("{minutes} min")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

/// A leaving time VTOP will accept: just over 24 hours from now (ten
/// minutes of slack while the form is filled in), on the next half hour,
/// moved into the offered hours.
pub(super) fn earliest_leaving(
    rules: &GeneralOutingRules,
    (today, minute): (i32, i32),
) -> OutingMoment {
    let notice = i64::from(today) * 1440 + i64::from(minute) + NOTICE + 10;
    let half_hours = (notice % 60 + 29) / 30;
    let candidate = notice - notice % 60 + half_hours * 30;
    let (day, minute) = (day_of(candidate), minute_of(candidate));
    match window(&rules.out_hours) {
        Some((earliest, _)) if minute < earliest => OutingMoment {
            day,
            minute: earliest,
        },
        Some((earliest, latest)) if minute > latest => OutingMoment {
            day: day + 1,
            minute: earliest,
        },
        _ => OutingMoment { day, minute },
    }
}

/// The latest return VTOP offers on the leaving day, else the next day.
pub(super) fn default_return(rules: &GeneralOutingRules, leaving: OutingMoment) -> OutingMoment {
    let latest = window(&rules.in_hours).map_or(20 * 60, |(_, latest)| latest);
    let same_day = OutingMoment {
        day: leaving.day,
        minute: latest,
    };
    if at(same_day) > at(leaving) {
        same_day
    } else {
        OutingMoment {
            day: leaving.day + 1,
            minute: latest,
        }
    }
}

impl WeekendOutingRules {
    fn shared(&self) -> WeekendRules<'_> {
        WeekendRules {
            purpose_max_length: self.purpose_max_length,
            max_days_ahead: self.max_days_ahead,
            weekdays: &self.weekdays,
        }
    }
}

/// [`outing::weekend_dates`] as epoch days.
pub(super) fn weekend_dates(rules: &WeekendOutingRules, today: i32) -> Vec<i32> {
    outing::weekend_dates(&rules.shared(), i64::from(today))
        .into_iter()
        .map(|day| day as i32)
        .collect()
}

pub(super) fn weekend_check(
    rules: &WeekendOutingRules,
    purpose: &str,
    day: Option<i32>,
    contact: &str,
    today: i32,
) -> WeekendOutingCheck {
    let problems = check_weekend(
        &rules.shared(),
        purpose,
        day.map(i64::from),
        contact,
        i64::from(today),
    );
    WeekendOutingCheck {
        purpose: text_error(problems.purpose, "Add why you're going"),
        day: problems.day.map(|problem| match problem {
            DayProblem::Missing => "Pick a day".into(),
            DayProblem::NotOffered => "Not a day VTOP offers".into(),
        }),
        contact: problems.contact.map(|_| "A 10-digit mobile number".into()),
    }
}
