//! Hostel outings: the general (day leave) and weekend requests with their
//! status, the forms' rules and checks, applying, cancelling and the pass.
//! Port of the Flutter app's `lib/features/outing/` (`outing_rules.dart`,
//! `outing_page.dart`, the apply sheets).
//!
//! Writes go to VTOP once (vtop-core never retries a post) and change
//! nothing here: the screen fetches the list again afterwards and shows what
//! VTOP has. Logs carry counts and outcomes only, never places, purposes,
//! phone numbers or ids.
//!
//! The records and the exported calls stay here, as UniFFI orders the Kotlin bindings by
//! module; [`view`] builds the records from VTOP's pages, [`form`] checks the forms and
//! [`time`] reads and labels days and times.

mod form;
pub(crate) mod time;
pub(crate) mod view;

use std::fs;

use tracing::info;
use vtop_core::VtopError;
use vtop_core::client::{
    GeneralOutingApplication, OutingKind as VtopOutingKind, WeekendOutingApplication,
};
use vtop_core::inputs::{ContactNumber, OutingPassId};
use vtop_core::outing::tidy;
use vtop_core::spec;
use vtop_core::types::{
    GeneralOutingData, OutingApplyResult, OutingCancelResult, WeekendOutingData,
};

use crate::pages::course_page::{FileLink, file_link};
use crate::pages::records;
use crate::{Core, CoreError, PAGES, traced, traced_sync};
use form::{default_return, earliest_leaving, general_check, weekend_check};
use time::{outing_date, outing_time};
use view::{logged_general, logged_weekend};

/// Outings are not per semester; under `pages/` so signing out drops them.
pub(crate) const GENERAL: &str = spec::GENERAL_OUTING.file;
pub(crate) const WEEKEND: &str = spec::WEEKEND_OUTING.file;

/// How far after leaving the return date picker goes (VTOP's own longest
/// trip is shown, not enforced).
const RETURN_PICKER_DAYS: i32 = 365;

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
    /// Apply, Leave, Back by.
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
                .read_kept::<GeneralOutingData>(&format!("{PAGES}/{GENERAL}"))
                .map(|data| logged_general(GeneralOuting::new(data, now()))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the general outing page (form and requests) and caches it.
    pub async fn fetch_general_outing(&self, flow: String) -> Result<GeneralOuting, CoreError> {
        traced("fetch_general_outing", &flow, async {
            let data = self.page(&spec::GENERAL_OUTING, None, &[]).await?;
            self.write_outing(GENERAL, &data);
            Ok(logged_general(GeneralOuting::new(data, now())))
        })
        .await
    }

    pub fn cached_weekend_outing(&self, flow: String) -> Option<WeekendOuting> {
        traced_sync("cached_weekend_outing", &flow, || {
            Ok(self
                .read_kept::<WeekendOutingData>(&format!("{PAGES}/{WEEKEND}"))
                .map(|data| logged_weekend(WeekendOuting::new(data, now()))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the weekend outing page (form and requests) and caches it.
    pub async fn fetch_weekend_outing(&self, flow: String) -> Result<WeekendOuting, CoreError> {
        traced("fetch_weekend_outing", &flow, async {
            let data = self.page(&spec::WEEKEND_OUTING, None, &[]).await?;
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
            if let Some(saved) = self.read_kept::<GeneralOutingData>(&format!("{PAGES}/{GENERAL}"))
            {
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
            if let Some(saved) = self.read_kept::<WeekendOutingData>(&format!("{PAGES}/{WEEKEND}"))
            {
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
    fn write_outing(&self, file: &str, data: &(impl serde::Serialize + 'static)) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "a failed mkdir shows up in the write that follows"
        )]
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

impl From<OutingApplyResult> for OutingResult {
    fn from(result: OutingApplyResult) -> Self {
        Self {
            done: result.applied,
            message: result.message,
        }
    }
}

#[cfg(test)]
mod tests;
