//! Hostel outings: the general (day leave) and weekend requests with their
//! status, the forms' rules and checks, applying, cancelling and the pass.
//! Port of the Flutter app's `lib/features/outing/` (`outing_rules.dart`,
//! `outing_page.dart`, the apply sheets).
//!
//! Writes go to VTOP once (vtop-core never retries a post) and change
//! nothing here: the screen fetches the list again afterwards and shows what
//! VTOP has. Logs carry counts and outcomes only, never places, purposes,
//! phone numbers or ids.

use std::fs;

use tracing::info;
use vtop_core::client::{
    GeneralOutingApplication, OutingKind as VtopOutingKind, WeekendOutingApplication,
};
use vtop_core::inputs::{ContactNumber, OutingDate, OutingPassId, OutingTime};
use vtop_core::types::{
    GeneralOutingData, GeneralOutingRecord, OutingApplyResult, OutingCancelResult, OutingStudent,
    WeekendOutingData, WeekendOutingRecord,
};
use vtop_core::{VtopError, VtopResult};

use crate::course_page::{FileLink, file_link};
use crate::route::page;
use crate::{Core, CoreError, PAGES, records, traced, traced_sync};

/// Outings are not per semester; under `pages/` so signing out drops them.
const GENERAL: &str = "outing_general.json";
const WEEKEND: &str = "outing_weekend.json";

/// VTOP's general outing page refuses a leaving time less than this far off
/// (minutes).
const NOTICE: i64 = 24 * 60;
/// How far after leaving the return date picker goes (VTOP's own longest
/// trip is shown, not enforced).
const RETURN_PICKER_DAYS: i32 = 365;
/// The days weekend outings are usually on (0 = Sunday).
const USUAL_WEEKEND_DAYS: [u8; 2] = [0, 1];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAY_NAMES: [&str; 7] = [
    "Sundays",
    "Mondays",
    "Tuesdays",
    "Wednesdays",
    "Thursdays",
    "Fridays",
    "Saturdays",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OutingKind {
    General,
    Weekend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OutingStanding {
    Pending,
    Approved,
    Rejected,
    Other,
}

/// A wall-clock moment in India: days since 1970-01-01 and minutes since
/// midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct OutingMoment {
    pub day: i32,
    pub minute: i32,
}

/// A small label over a short value, for the rules panel.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutingFact {
    pub label: String,
    pub value: String,
}

/// A `<select>` option: what VTOP expects back and what to show.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutingChoice {
    pub value: String,
    pub label: String,
}

/// One line of when a request is: "Sat 10 Oct, 9:00 AM" with an optional
/// label ("Out", "Back", "Away").
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutingWhen {
    pub label: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutingRequest {
    /// Unique in its list.
    pub key: String,
    pub kind: OutingKind,
    pub place: String,
    pub purpose: String,
    /// VTOP's words, e.g. "Waiting for Mentor's Approval".
    pub status: String,
    pub standing: OutingStanding,
    /// "APPROVED", "PENDING", "REJECTED", else VTOP's status in capitals.
    pub status_label: String,
    pub when: Vec<OutingWhen>,
    /// "OUT NOW", "TODAY", "TOMORROW", "IN 3 DAYS" for a trip not over
    /// (and not rejected).
    pub highlight: Option<String>,
    /// Set once VTOP accepted the request.
    pub pass_id: Option<String>,
    /// Set while VTOP still lets the request be cancelled.
    pub cancel_id: Option<String>,
}

/// What the general outing form allows, from VTOP's page.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GeneralOutingRules {
    /// Hours (24 h) offered for leaving and for coming back; the last one
    /// only at :00. Empty: any time.
    pub out_hours: Vec<u8>,
    pub in_hours: Vec<u8>,
    pub place_max_length: u32,
    pub purpose_max_length: u32,
    pub max_days_ahead: u32,
    pub max_days_away: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GeneralOuting {
    /// Unix seconds.
    pub updated_at: i64,
    /// VTOP takes applications.
    pub open: bool,
    /// VTOP's reason while it does not; may be empty.
    pub notice: String,
    /// "MH-2 · Room 911" while open.
    pub hostel: Option<String>,
    pub rules: GeneralOutingRules,
    /// Apply, Leave, Back by, Longest trip.
    pub facts: Vec<OutingFact>,
    /// "6:00 AM – 10:00 PM", for the fields.
    pub out_window: Option<String>,
    pub in_window: Option<String>,
    /// The days the leaving date can be on (epoch days, India).
    pub first_day: i32,
    pub last_day: i32,
    /// Newest first.
    pub requests: Vec<OutingRequest>,
}

/// What the weekend outing form allows, from VTOP's page.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WeekendOutingRules {
    pub purpose_max_length: u32,
    pub max_days_ahead: u32,
    /// 0 = Sunday … 6 = Saturday.
    pub weekdays: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WeekendOuting {
    pub updated_at: i64,
    /// VTOP takes applications and a day is left in the coming week.
    pub open: bool,
    /// Why not, when not open; may be empty.
    pub notice: String,
    pub hostel: Option<String>,
    /// The parent number VTOP has on file.
    pub parent_contact: Option<String>,
    pub rules: WeekendOutingRules,
    /// Outing days, Next, Time slots, Places.
    pub facts: Vec<OutingFact>,
    /// "Sundays and Mondays".
    pub days_label: String,
    /// The days one can apply for (epoch days), from tomorrow.
    pub dates: Vec<i32>,
    pub places: Vec<OutingChoice>,
    /// Labels as "9:30 AM – 3:30 PM".
    pub slots: Vec<OutingChoice>,
    /// Newest first.
    pub requests: Vec<OutingRequest>,
}

/// A field's problem, or none. `away` ("Away for 1 d 3 h") while both
/// times are fine.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GeneralOutingCheck {
    pub place: Option<String>,
    pub purpose: Option<String>,
    pub leaving: Option<String>,
    pub returning: Option<String>,
    pub away: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WeekendOutingCheck {
    pub purpose: Option<String>,
    pub day: Option<String>,
    pub contact: Option<String>,
}

/// The form's starting times: just over a day ahead, in the offered hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct GeneralOutingTimes {
    pub leaving: OutingMoment,
    pub returning: OutingMoment,
}

/// A general outing to apply for.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GeneralOutingForm {
    pub place: String,
    pub purpose: String,
    pub leaving: OutingMoment,
    pub returning: OutingMoment,
}

/// A weekend outing to apply for; `place` and `slot` are option values.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WeekendOutingForm {
    pub place: String,
    pub purpose: String,
    pub day: i32,
    pub slot: String,
    pub contact: String,
}

/// What VTOP said to an application or a cancel.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutingResult {
    /// The request is in the list (applied) or gone from it (cancelled).
    pub done: bool,
    pub message: String,
}

#[uniffi::export]
impl Core {
    pub fn cached_general_outing(&self, flow: String) -> Option<GeneralOuting> {
        traced_sync("cached_general_outing", &flow, || {
            Ok(self
                .read::<GeneralOutingData>(&format!("{PAGES}/{GENERAL}"))
                .map(|data| logged_general(GeneralOuting::new(data, now()))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the general outing page (form and requests) and caches it.
    pub async fn fetch_general_outing(&self, flow: String) -> Result<GeneralOuting, CoreError> {
        traced("fetch_general_outing", &flow, async {
            let data = page!(self, "general_outing", None, [], |client| client
                .general_outing())?;
            self.write_outing(GENERAL, &data);
            Ok(logged_general(GeneralOuting::new(data, now())))
        })
        .await
    }

    pub fn cached_weekend_outing(&self, flow: String) -> Option<WeekendOuting> {
        traced_sync("cached_weekend_outing", &flow, || {
            Ok(self
                .read::<WeekendOutingData>(&format!("{PAGES}/{WEEKEND}"))
                .map(|data| logged_weekend(WeekendOuting::new(data, now()))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the weekend outing page (form and requests) and caches it.
    pub async fn fetch_weekend_outing(&self, flow: String) -> Result<WeekendOuting, CoreError> {
        traced("fetch_weekend_outing", &flow, async {
            let data = page!(self, "weekend_outing", None, [], |client| client
                .weekend_outing())?;
            self.write_outing(WEEKEND, &data);
            Ok(logged_weekend(WeekendOuting::new(data, now())))
        })
        .await
    }

    /// Sends a general outing application once. Checked first against the
    /// saved form as the app's sheet checks it, then by vtop-core against a
    /// fresh copy of VTOP's form; nothing is sent when either says no.
    pub async fn apply_general_outing(
        &self,
        flow: String,
        form: GeneralOutingForm,
    ) -> Result<OutingResult, CoreError> {
        traced("apply_general_outing", &flow, async {
            if let Some(saved) = self.read::<GeneralOutingData>(&format!("{PAGES}/{GENERAL}")) {
                let rules = GeneralOutingRules::from(&saved);
                general_check(
                    &rules,
                    &form.place,
                    &form.purpose,
                    form.leaving,
                    form.returning,
                    now(),
                )
                .first()
                .map_or(Ok(()), refused)?;
            }
            let application = GeneralOutingApplication {
                place: tidy(&form.place),
                purpose: tidy(&form.purpose),
                out_date: outing_date(form.leaving.day)?,
                out_time: outing_time(form.leaving.minute)?,
                in_date: outing_date(form.returning.day)?,
                in_time: outing_time(form.returning.minute)?,
            };
            let result = signed_in!(self, |client| client.apply_general_outing(&application))?;
            info!(applied = result.applied, "general outing application sent");
            Ok(OutingResult::from(result))
        })
        .await
    }

    /// [`Self::apply_general_outing`] for a weekend outing.
    pub async fn apply_weekend_outing(
        &self,
        flow: String,
        form: WeekendOutingForm,
    ) -> Result<OutingResult, CoreError> {
        traced("apply_weekend_outing", &flow, async {
            if let Some(saved) = self.read::<WeekendOutingData>(&format!("{PAGES}/{WEEKEND}")) {
                let rules = WeekendOutingRules::from(&saved);
                weekend_check(
                    &rules,
                    &form.purpose,
                    Some(form.day),
                    &form.contact,
                    now().0,
                )
                .first()
                .map_or(Ok(()), refused)?;
            }
            let application = WeekendOutingApplication {
                place: form.place.clone(),
                purpose: tidy(&form.purpose),
                date: outing_date(form.day)?,
                time_slot: form.slot.clone(),
                contact_number: ContactNumber::parse(&form.contact)?,
            };
            let result = signed_in!(self, |client| client.apply_weekend_outing(&application))?;
            info!(applied = result.applied, "weekend outing application sent");
            Ok(OutingResult::from(result))
        })
        .await
    }

    /// Cancels a request VTOP still lets the student delete (`cancel_id`
    /// from its list). vtop-core checks a fresh copy of the list first, so an
    /// approved request is never touched.
    pub async fn cancel_outing(
        &self,
        flow: String,
        kind: OutingKind,
        cancel_id: String,
    ) -> Result<OutingResult, CoreError> {
        traced("cancel_outing", &flow, async {
            let id = OutingPassId::parse(&cancel_id)?;
            let result: OutingCancelResult = signed_in!(self, |client| async {
                match kind {
                    OutingKind::General => client.cancel_general_outing(&id).await,
                    OutingKind::Weekend => client.cancel_weekend_outing(&id).await,
                }
            })?;
            info!(?kind, cancelled = result.cancelled, "outing cancel sent");
            Ok(OutingResult {
                done: result.cancelled,
                message: result.message,
            })
        })
        .await
    }

    /// An accepted request's PDF pass as a [`FileLink`] for Android's
    /// DownloadManager, named "Outing pass <id>.pdf"; signs in first if
    /// needed.
    pub async fn outing_pass_link(
        &self,
        flow: String,
        kind: OutingKind,
        pass_id: String,
    ) -> Result<FileLink, CoreError> {
        traced("outing_pass_link", &flow, async {
            let id = OutingPassId::parse(&pass_id)?;
            let vtop_kind = match kind {
                OutingKind::General => VtopOutingKind::General,
                OutingKind::Weekend => VtopOutingKind::Weekend,
            };
            let link = async { signed_in!(self, |client| client.outing_pass_link(vtop_kind, &id)) }
                .await?;
            info!(?kind, "outing pass link ready");
            Ok(FileLink {
                name: format!("Outing pass {}.pdf", id.as_str()),
                ..file_link(link, "outing-pass")
            })
        })
        .await
    }
}

impl Core {
    fn write_outing(&self, file: &str, data: &impl serde::Serialize) {
        let _ = fs::create_dir_all(self.at(PAGES));
        self.write(&format!("{PAGES}/{file}"), data);
    }
}

/// The problems with a general outing, by field, as of now in India.
#[uniffi::export]
pub fn check_general_outing(
    rules: GeneralOutingRules,
    place: String,
    purpose: String,
    leaving: OutingMoment,
    returning: OutingMoment,
) -> GeneralOutingCheck {
    general_check(&rules, &place, &purpose, leaving, returning, now())
}

/// The problems with a weekend outing, by field, as of today in India.
#[uniffi::export]
pub fn check_weekend_outing(
    rules: WeekendOutingRules,
    purpose: String,
    day: Option<i32>,
    contact: String,
) -> WeekendOutingCheck {
    weekend_check(&rules, &purpose, day, &contact, now().0)
}

/// Where the general outing form starts.
#[uniffi::export]
pub fn general_outing_times(rules: GeneralOutingRules) -> GeneralOutingTimes {
    let leaving = earliest_leaving(&rules, now());
    GeneralOutingTimes {
        leaving,
        returning: default_return(&rules, leaving),
    }
}

/// The last day the return date picker offers for a trip leaving on `day`.
#[uniffi::export]
pub fn general_return_last_day(day: i32) -> i32 {
    day + RETURN_PICKER_DAYS
}

fn now() -> (i32, i32) {
    records::ist_now()
}

fn refused(message: String) -> Result<(), CoreError> {
    info!("outing refused before sending");
    Err(CoreError::Other { reason: message })
}

fn logged_general(outing: GeneralOuting) -> GeneralOuting {
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

fn logged_weekend(outing: WeekendOuting) -> WeekendOuting {
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

impl From<OutingApplyResult> for OutingResult {
    fn from(result: OutingApplyResult) -> Self {
        Self {
            done: result.applied,
            message: result.message,
        }
    }
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
    fn new(data: GeneralOutingData, now: (i32, i32)) -> Self {
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
        facts.push(fact(
            "Longest trip",
            format!("{} days", rules.max_days_away),
        ));
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
    fn new(data: WeekendOutingData, now: (i32, i32)) -> Self {
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
        let days = weekend_days(&rules, today);
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
fn general_request(
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

fn standing(status: &str) -> OutingStanding {
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
fn upcoming(start: i64, end: i64, (today, minute): (i32, i32)) -> Option<String> {
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

/// VTOP's `YYYY-MM-DD` with an optional `hh:mm AM`, as minutes since 1970.
fn moment(date: &str, time: &str) -> Option<i64> {
    let day = records::epoch_day(date)?;
    Some(i64::from(day) * 1440 + i64::from(records::minutes(time).unwrap_or(0)))
}

fn at(moment: OutingMoment) -> i64 {
    i64::from(moment.day) * 1440 + i64::from(moment.minute)
}

fn day_of(at: i64) -> i32 {
    at.div_euclid(1440) as i32
}

fn minute_of(at: i64) -> i32 {
    at.rem_euclid(1440) as i32
}

/// The `h:mm AM` times in `text`, as minutes since midnight.
fn clock_times(text: &str) -> Vec<i32> {
    let bytes = text.as_bytes();
    let mut times = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let colon = j - i <= 2
            && bytes.get(j) == Some(&b':')
            && bytes.get(j + 1).is_some_and(u8::is_ascii_digit)
            && bytes.get(j + 2).is_some_and(u8::is_ascii_digit);
        if colon {
            let mut k = j + 3;
            while bytes.get(k) == Some(&b' ') {
                k += 1;
            }
            let suffix = text.get(k..k + 2).map(str::to_ascii_uppercase);
            if matches!(suffix.as_deref(), Some("AM" | "PM"))
                && let Some(minutes) = records::minutes(&text[i..k + 2])
            {
                times.push(minutes);
                i = k + 2;
                continue;
            }
        }
        i = j;
    }
    times
}

/// The leaving and return of a weekend slot like `9:30 AM- 3:30PM`.
fn time_slot(slot: &str) -> Option<(i32, i32)> {
    match clock_times(slot).as_slice() {
        [start, end] => Some((*start, *end)),
        _ => None,
    }
}

/// `9:30 AM – 3:30 PM` for VTOP's `9:30 AM- 3:30PM`.
fn slot_label(slot: &str) -> String {
    time_slot(slot).map_or_else(
        || slot.trim().to_string(),
        |(start, end)| format!("{} – {}", clock(start), clock(end)),
    )
}

/// The earliest and latest times a general outing form offers for `hours`.
fn window(hours: &[u8]) -> Option<(i32, i32)> {
    Some((
        i32::from(*hours.first()?) * 60,
        i32::from(*hours.last()?) * 60,
    ))
}

fn offered(hours: &[u8], minute: i32) -> bool {
    let Some(last) = hours.last() else {
        return true;
    };
    let hour = minute / 60;
    hours.iter().any(|h| i32::from(*h) == hour) && (hour != i32::from(*last) || minute % 60 == 0)
}

/// Collapses runs of whitespace, as VTOP's own form effectively does.
fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_error(value: &str, max_length: u32, missing: &str) -> Option<String> {
    let tidy = tidy(value);
    if tidy.is_empty() {
        Some(missing.to_string())
    } else if tidy.chars().count() > max_length as usize {
        Some(format!("At most {max_length} characters"))
    } else {
        None
    }
}

impl GeneralOutingCheck {
    fn first(&self) -> Option<String> {
        [&self.place, &self.purpose, &self.leaving, &self.returning]
            .into_iter()
            .flatten()
            .next()
            .cloned()
    }
}

impl WeekendOutingCheck {
    fn first(&self) -> Option<String> {
        [&self.purpose, &self.day, &self.contact]
            .into_iter()
            .flatten()
            .next()
            .cloned()
    }
}

fn general_check(
    rules: &GeneralOutingRules,
    place: &str,
    purpose: &str,
    leaving: OutingMoment,
    returning: OutingMoment,
    now: (i32, i32),
) -> GeneralOutingCheck {
    let last_day = now.0 + rules.max_days_ahead as i32;
    let now_at = i64::from(now.0) * 1440 + i64::from(now.1);
    let leaving_error = if leaving.day > last_day {
        Some(format!(
            "Up to {} days ahead ({})",
            rules.max_days_ahead,
            format_day(last_day, "d MMM")
        ))
    } else if !offered(&rules.out_hours, leaving.minute) {
        window(&rules.out_hours)
            .map(|(a, b)| format!("Leave between {} and {}", clock(a), clock(b)))
    } else if at(leaving) - now_at < NOTICE {
        let earliest = now_at + NOTICE;
        Some(format!(
            "Needs 24 h notice: leave after {}, {}",
            format_day(day_of(earliest), "EEE d MMM"),
            clock(minute_of(earliest))
        ))
    } else {
        None
    };
    let returning_error = if !offered(&rules.in_hours, returning.minute) {
        window(&rules.in_hours)
            .map(|(a, b)| format!("Be back between {} and {}", clock(a), clock(b)))
    } else if at(returning) <= at(leaving) {
        Some("Has to be after you leave".into())
    } else {
        None
    };
    let away = (leaving_error.is_none() && returning_error.is_none())
        .then(|| trip_length(at(returning) - at(leaving)))
        .filter(|length| !length.is_empty())
        .map(|length| format!("Away for {length}"));
    GeneralOutingCheck {
        place: text_error(place, rules.place_max_length, "Add where you're going"),
        purpose: text_error(purpose, rules.purpose_max_length, "Add why you're going"),
        leaving: leaving_error,
        returning: returning_error,
        away,
    }
}

/// "1 d 3 h", "5 h 30 min".
fn trip_length(span: i64) -> String {
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
fn earliest_leaving(rules: &GeneralOutingRules, (today, minute): (i32, i32)) -> OutingMoment {
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
fn default_return(rules: &GeneralOutingRules, leaving: OutingMoment) -> OutingMoment {
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

/// 0 = Sunday for days since 1970-01-01 (a Thursday).
fn sunday_weekday(day: i32) -> u8 {
    (day + 4).rem_euclid(7) as u8
}

/// The weekdays a weekend outing can be on. VTOP's picker also blocks the
/// day before an outing day, so on Saturday it leaves only Monday; tomorrow
/// is added back when it is a usual outing day.
fn weekend_days(rules: &WeekendOutingRules, today: i32) -> Vec<u8> {
    let tomorrow = sunday_weekday(today + 1);
    let mut days = rules.weekdays.clone();
    if USUAL_WEEKEND_DAYS.contains(&tomorrow) {
        days.push(tomorrow);
    }
    days.sort_unstable();
    days.dedup();
    days
}

/// Dates a weekend outing can be applied for: tomorrow up to
/// `max_days_ahead`, on [`weekend_days`]. Never today (the app's rule; VTOP's
/// picker allows it, vtop-core refuses it too).
fn weekend_dates(rules: &WeekendOutingRules, today: i32) -> Vec<i32> {
    let days = weekend_days(rules, today);
    (1..=rules.max_days_ahead as i32)
        .map(|i| today + i)
        .filter(|day| days.contains(&sunday_weekday(*day)))
        .collect()
}

fn weekend_check(
    rules: &WeekendOutingRules,
    purpose: &str,
    day: Option<i32>,
    contact: &str,
    today: i32,
) -> WeekendOutingCheck {
    let contact = contact.trim();
    WeekendOutingCheck {
        purpose: text_error(purpose, rules.purpose_max_length, "Add why you're going"),
        day: match day {
            None => Some("Pick a day".into()),
            Some(day) if weekend_dates(rules, today).contains(&day) => None,
            Some(_) => Some("Not a day VTOP offers".into()),
        },
        contact: (contact.len() != 10 || !contact.chars().all(|c| c.is_ascii_digit()))
            .then(|| "A 10-digit mobile number".into()),
    }
}

/// "Sundays and Mondays", or "Sun & Mon" when `short`.
fn list_days(days: &[u8], and: &str, short: bool) -> String {
    let names: Vec<&str> = days
        .iter()
        .filter_map(|d| DAY_NAMES.get(usize::from(*d)))
        .map(|name| if short { &name[..3] } else { name })
        .collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} {and} {last}", rest.join(", ")),
    }
}

/// (year, month 1–12, day) of an epoch day.
fn ymd(day: i32) -> (i32, usize, i32) {
    let iso = records::civil_date(i64::from(day));
    let mut parts = iso.split('-').map(|p| p.parse::<i32>().unwrap_or(1));
    let (year, month, day) = (
        parts.next().unwrap_or(1970),
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(1),
    );
    (year, month.clamp(1, 12) as usize, day)
}

/// `day` as "EEE d MMM" ("Sat 10 Oct"), "d MMM" or "EEE d MMM yyyy".
fn format_day(day: i32, pattern: &str) -> String {
    let (year, month, date) = ymd(day);
    let weekday = &DAY_NAMES[usize::from(sunday_weekday(day))][..3];
    let month = MONTHS[month - 1];
    match pattern {
        "d MMM" => format!("{date} {month}"),
        "EEE d MMM yyyy" => format!("{weekday} {date} {month} {year}"),
        _ => format!("{weekday} {date} {month}"),
    }
}

/// `Sat 10 Oct`, with the year when it is not this year.
fn short_date(day: i32, today: i32) -> String {
    if ymd(day).0 == ymd(today).0 {
        format_day(day, "EEE d MMM")
    } else {
        format_day(day, "EEE d MMM yyyy")
    }
}

/// 360 → "6:00 AM".
fn clock(minutes: i32) -> String {
    let (hour, minute) = (minutes / 60, minutes % 60);
    let half = if hour >= 12 { "PM" } else { "AM" };
    let hour = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{hour}:{minute:02} {half}")
}

/// 360 → "6 AM", 390 → "6:30 AM".
fn short_clock(minutes: i32) -> String {
    let full = clock(minutes);
    if minutes % 60 == 0 {
        full.replace(":00", "")
    } else {
        full
    }
}

/// `11-Oct-2026`, what the outing date fields post.
fn outing_date(day: i32) -> VtopResult<OutingDate> {
    let (year, month, date) = ymd(day);
    OutingDate::parse(&format!("{date:02}-{}-{year:04}", MONTHS[month - 1]))
}

fn outing_time(minute: i32) -> VtopResult<OutingTime> {
    let invalid = || VtopError::ConfigurationError("outing time must be a real time of day".into());
    let hour = u8::try_from(minute / 60).map_err(|_| invalid())?;
    let minute = u8::try_from(minute % 60).map_err(|_| invalid())?;
    OutingTime::new(hour, minute)
}

#[cfg(test)]
mod tests;
