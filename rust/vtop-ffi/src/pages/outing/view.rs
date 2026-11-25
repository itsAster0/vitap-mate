//! The screen's records from VTOP's outing pages: each request dated, labelled and sorted,
//! the rules panel's facts, and the counts the log carries.

use tracing::info;
use vtop_core::outing;
use vtop_core::types::{
    GeneralOutingData, GeneralOutingRecord, OutingStudent, WeekendOutingData, WeekendOutingRecord,
};

use super::form::{weekend_dates, window};
use super::time::{
    clock, day_of, format_day, list_days, minute_of, moment, short_clock, short_date, slot_label,
    time_slot,
};
use super::{
    GeneralOuting, GeneralOutingRules, OutingChoice, OutingFact, OutingKind, OutingRequest,
    OutingStanding, OutingWhen, WeekendOuting, WeekendOutingRules,
};
use crate::pages::records;

pub(super) fn logged_general(outing: GeneralOuting) -> GeneralOuting {
    info!(
        open = outing.open,
        requests = outing.requests.len(),
        pending = count(&outing.requests, OutingStanding::Pending),
        approved = count(&outing.requests, OutingStanding::Approved),
        cancellable = outing
            .requests
            .iter()
            .filter(|r| r.cancel_id.is_some())
            .count(),
        "general outing"
    );
    outing
}

pub(super) fn logged_weekend(outing: WeekendOuting) -> WeekendOuting {
    info!(
        open = outing.open,
        dates = outing.dates.len(),
        slots = outing.slots.len(),
        places = outing.places.len(),
        requests = outing.requests.len(),
        pending = count(&outing.requests, OutingStanding::Pending),
        approved = count(&outing.requests, OutingStanding::Approved),
        "weekend outing"
    );
    outing
}

fn count(requests: &[OutingRequest], standing: OutingStanding) -> usize {
    requests.iter().filter(|r| r.standing == standing).count()
}

impl From<&GeneralOutingData> for GeneralOutingRules {
    fn from(data: &GeneralOutingData) -> Self {
        Self {
            out_hours: data.out_hours.clone(),
            in_hours: data.in_hours.clone(),
            place_max_length: data.place_max_length,
            purpose_max_length: data.purpose_max_length,
            max_days_ahead: data.max_days_ahead,
            max_days_away: data.max_days_away,
        }
    }
}

impl From<&WeekendOutingData> for WeekendOutingRules {
    fn from(data: &WeekendOutingData) -> Self {
        Self {
            purpose_max_length: data.purpose_max_length,
            max_days_ahead: data.max_days_ahead,
            weekdays: data.weekdays.clone(),
        }
    }
}

impl GeneralOuting {
    pub(super) fn new(data: GeneralOutingData, now: (i32, i32)) -> Self {
        let rules = GeneralOutingRules::from(&data);
        let today = now.0;
        let out = window(&rules.out_hours);
        let back = window(&rules.in_hours);
        let mut facts = vec![fact(
            "Apply",
            format!("24 h – {} days ahead", rules.max_days_ahead),
        )];
        if let Some((earliest, latest)) = out {
            facts.push(fact(
                "Leave",
                format!("{} – {}", short_clock(earliest), short_clock(latest)),
            ));
        }
        if let Some((_, latest)) = back {
            facts.push(fact("Back by", short_clock(latest)));
        }
        let mut requests: Vec<(i64, OutingRequest)> = data
            .records
            .iter()
            .enumerate()
            .map(|(i, record)| general_request(i, record, now))
            .collect();
        requests.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        Self {
            updated_at: data.update_time as i64,
            open: data.student.is_some(),
            notice: data.notice.trim().to_string(),
            hostel: data.student.as_ref().map(hostel),
            facts,
            out_window: out.map(|(a, b)| format!("{} – {}", clock(a), clock(b))),
            in_window: back.map(|(a, b)| format!("{} – {}", clock(a), clock(b))),
            first_day: today,
            last_day: today + rules.max_days_ahead as i32,
            rules,
            requests: requests.into_iter().map(|(_, r)| r).collect(),
        }
    }
}

impl WeekendOuting {
    pub(super) fn new(data: WeekendOutingData, now: (i32, i32)) -> Self {
        let rules = WeekendOutingRules::from(&data);
        let today = now.0;
        let dates = weekend_dates(&rules, today);
        let taking = data.student.is_some();
        let mut requests: Vec<(i64, OutingRequest)> = data
            .records
            .iter()
            .enumerate()
            .map(|(i, record)| weekend_request(i, record, now))
            .collect();
        requests.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        let days = outing::weekend_days(&rules.weekdays, i64::from(today));
        Self {
            updated_at: data.update_time as i64,
            open: taking && !dates.is_empty(),
            notice: if !taking {
                data.notice.trim().to_string()
            } else if dates.is_empty() {
                "No outing day in the coming week.".to_string()
            } else {
                String::new()
            },
            hostel: data.student.as_ref().map(hostel),
            parent_contact: data
                .student
                .as_ref()
                .map(|s| s.parent_contact_number.trim().to_string())
                .filter(|n| !n.is_empty()),
            facts: vec![
                fact("Outing days", list_days(&days, "&", true)),
                fact(
                    "Next",
                    dates
                        .first()
                        .map_or("—".to_string(), |d| format_day(*d, "EEE d MMM")),
                ),
                fact("Time slots", data.time_slots.len().to_string()),
                fact("Places", data.places.len().to_string()),
            ],
            days_label: list_days(&days, "and", false),
            dates,
            places: data
                .places
                .iter()
                .map(|o| OutingChoice {
                    value: o.value.clone(),
                    label: o.label.clone(),
                })
                .collect(),
            slots: data
                .time_slots
                .iter()
                .map(|o| OutingChoice {
                    value: o.value.clone(),
                    label: slot_label(&o.value),
                })
                .collect(),
            rules,
            requests: requests.into_iter().map(|(_, r)| r).collect(),
        }
    }
}

fn fact(label: &str, value: String) -> OutingFact {
    OutingFact {
        label: label.to_string(),
        value,
    }
}

fn hostel(student: &OutingStudent) -> String {
    format!(
        "{} · Room {}",
        student.hostel_block.trim(),
        student.room_number.trim()
    )
}

fn non_empty(raw: &str) -> Option<String> {
    let raw = raw.trim();
    (!raw.is_empty()).then(|| raw.to_string())
}

/// The request and its start (minutes since 1970 in India; 0 when undated)
/// to sort by.
pub(super) fn general_request(
    index: usize,
    record: &GeneralOutingRecord,
    now: (i32, i32),
) -> (i64, OutingRequest) {
    let start = moment(&record.from_date, &record.from_time);
    let end = moment(&record.to_date, &record.to_time);
    let has_back_time = records::minutes(&record.to_time).is_some();
    let standing = standing(&record.status);
    let when = match (start, end) {
        (Some(start), Some(end)) => {
            let time = |when: i64, raw: &str| {
                if records::minutes(raw).is_some() {
                    format!(", {}", clock(minute_of(when)))
                } else {
                    String::new()
                }
            };
            let away = away_label(start, end, has_back_time).map(|text| OutingWhen {
                label: Some("Away".into()),
                text,
            });
            if day_of(start) == day_of(end) {
                let back = if has_back_time {
                    format!(" – {}", clock(minute_of(end)))
                } else {
                    String::new()
                };
                std::iter::once(OutingWhen {
                    label: None,
                    text: format!(
                        "{}{}{back}",
                        short_date(day_of(start), now.0),
                        time(start, &record.from_time)
                    ),
                })
                .chain(away)
                .collect()
            } else {
                [
                    OutingWhen {
                        label: Some("Out".into()),
                        text: format!(
                            "{}{}",
                            short_date(day_of(start), now.0),
                            time(start, &record.from_time)
                        ),
                    },
                    OutingWhen {
                        label: Some("Back".into()),
                        text: format!(
                            "{}{}",
                            short_date(day_of(end), now.0),
                            time(end, &record.to_time)
                        ),
                    },
                ]
                .into_iter()
                .chain(away)
                .collect()
            }
        }
        _ => vec![
            OutingWhen {
                label: Some("Out".into()),
                text: format!("{} {}", record.from_date.trim(), record.from_time.trim()),
            },
            OutingWhen {
                label: Some("Back".into()),
                text: format!("{} {}", record.to_date.trim(), record.to_time.trim()),
            },
        ],
    };
    let highlight = match (start, end) {
        (Some(start), Some(end)) if standing != OutingStanding::Rejected => {
            // Without a return time the trip lasts the day.
            let end = if has_back_time {
                end
            } else {
                end + 23 * 60 + 59
            };
            upcoming(start, end, now)
        }
        _ => None,
    };
    let request = OutingRequest {
        key: format!("general.{index}.{}", record.serial.trim()),
        kind: OutingKind::General,
        place: record.place.trim().to_string(),
        purpose: record.purpose.trim().to_string(),
        status: record.status.trim().to_string(),
        status_label: status_label(standing, &record.status),
        standing,
        when,
        highlight,
        pass_id: non_empty(&record.pass_id),
        cancel_id: non_empty(&record.cancel_id),
    };
    (start.unwrap_or(0), request)
}

fn weekend_request(
    index: usize,
    record: &WeekendOutingRecord,
    now: (i32, i32),
) -> (i64, OutingRequest) {
    let day = records::epoch_day(&record.date);
    let standing = standing(&record.status);
    let date = day.map_or_else(|| record.date.trim().to_string(), |d| short_date(d, now.0));
    let highlight = match (day, time_slot(&record.time_slot)) {
        (Some(day), Some((start, end))) if standing != OutingStanding::Rejected => {
            let base = i64::from(day) * 1440;
            upcoming(base + i64::from(start), base + i64::from(end), now)
        }
        _ => None,
    };
    let request = OutingRequest {
        key: format!("weekend.{index}.{}", record.serial.trim()),
        kind: OutingKind::Weekend,
        place: record.place.trim().to_string(),
        purpose: record.purpose.trim().to_string(),
        status: record.status.trim().to_string(),
        status_label: status_label(standing, &record.status),
        standing,
        when: vec![OutingWhen {
            label: None,
            text: format!("{date} · {}", slot_label(&record.time_slot)),
        }],
        highlight,
        pass_id: non_empty(&record.pass_id),
        cancel_id: non_empty(&record.cancel_id),
    };
    (day.map_or(0, |d| i64::from(d) * 1440), request)
}

pub(crate) fn standing(status: &str) -> OutingStanding {
    let t = status.to_lowercase();
    if t.contains("accepted") || t.contains("approved") {
        OutingStanding::Approved
    } else if t.contains("waiting") || t.contains("pending") {
        OutingStanding::Pending
    } else if t.contains("reject") || t.contains("declin") || t.contains("cancel") {
        OutingStanding::Rejected
    } else {
        OutingStanding::Other
    }
}

fn status_label(standing: OutingStanding, raw: &str) -> String {
    match standing {
        OutingStanding::Approved => "APPROVED".into(),
        OutingStanding::Pending => "PENDING".into(),
        OutingStanding::Rejected => "REJECTED".into(),
        OutingStanding::Other => raw.trim().to_uppercase(),
    }
}

/// "OUT NOW", "TODAY", "TOMORROW" or "IN 3 DAYS" for a trip from `start`
/// to `end` (minutes since 1970) that is not over.
pub(super) fn upcoming(start: i64, end: i64, (today, minute): (i32, i32)) -> Option<String> {
    let now = i64::from(today) * 1440 + i64::from(minute);
    if end < now {
        return None;
    }
    if start <= now {
        return Some("OUT NOW".into());
    }
    Some(match day_of(start) - today {
        0 => "TODAY".into(),
        1 => "TOMORROW".into(),
        days => format!("IN {days} DAYS"),
    })
}

/// "4 days" for a trip across days, "8 h 30 min" for one inside a day;
/// none for a same-day trip without a return time.
fn away_label(start: i64, end: i64, has_back_time: bool) -> Option<String> {
    let days = day_of(end) - day_of(start);
    if days > 0 {
        return Some(if days == 1 {
            "1 day".into()
        } else {
            format!("{days} days")
        });
    }
    if !has_back_time || end <= start {
        return None;
    }
    let span = end - start;
    let parts: Vec<String> = [
        (span / 60 > 0).then(|| format!("{} h", span / 60)),
        (span % 60 > 0).then(|| format!("{} min", span % 60)),
    ]
    .into_iter()
    .flatten()
    .collect();
    Some(parts.join(" "))
}
