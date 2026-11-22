//! Outing tools that act on VTOP: applying, cancelling and the pass.
//!
//! Applying and cancelling take two calls. The first only checks the
//! request against a fresh copy of VTOP's form and hands back what would be
//! sent, with a confirmation code; nothing is sent. The second, with the
//! same arguments and that code, sends it. The code is bound to the caller's
//! key and the exact request and lapses after ten minutes, so the agent has
//! to show the student this request before it can file it. The server keeps
//! no state between the two calls, so the code alone cannot prove the
//! student said yes: the tool descriptions ask the agent to get that first.

use axum::http::request::Parts;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use vtop_core::client::{
    GeneralOutingApplication, OutingKind, WeekendOutingApplication, check_weekend_outing,
};
use vtop_core::dates::{self, WEEKDAYS, civil_date, minutes_in_india, today_in_india};
use vtop_core::inputs::{ContactNumber, OutingDate, OutingPassId, OutingTime};
use vtop_core::outing::{
    DayProblem, LeavingProblem, ReturningProblem, TextProblem, check_general, check_weekend, tidy,
    weekend_dates,
};
use vtop_core::spec::{self, Query};
use vtop_core::types::{GeneralOutingData, OutingStudent, WeekendOutingData};
use vtop_core::{VtopError, VtopResult, now_unix};

use crate::tools::{Caller, VtopTools, file_result, input, json_result, tool_error, with_retry};

/// How long a confirmation code stays good.
const CONFIRM_SECONDS: u64 = 600;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct GeneralOutingArgs {
    #[schemars(description = "Where the student is going")]
    pub place: String,
    #[schemars(description = "Why the student is going")]
    pub purpose: String,
    #[schemars(description = "Leaving date, DD-Mon-YYYY, e.g. 11-Oct-2026")]
    pub out_date: String,
    #[schemars(description = "Leaving time, 24-hour HH:MM, e.g. 09:30")]
    pub out_time: String,
    #[schemars(description = "Return date, DD-Mon-YYYY")]
    pub in_date: String,
    #[schemars(description = "Return time, 24-hour HH:MM")]
    pub in_time: String,
    #[schemars(
        description = "Leave out to check the request; nothing is sent. Pass the code that call returned, after the student has said yes, to send it."
    )]
    pub confirmation: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct WeekendOutingArgs {
    #[schemars(description = "A place option's value from get_weekend_outing (places)")]
    pub place: String,
    #[schemars(description = "Why the student is going")]
    pub purpose: String,
    #[schemars(description = "Outing date, DD-Mon-YYYY, e.g. 11-Oct-2026")]
    pub date: String,
    #[schemars(description = "A time slot option's value from get_weekend_outing (time_slots)")]
    pub time_slot: String,
    #[schemars(description = "The student's 10-digit mobile number")]
    pub contact_number: String,
    #[schemars(
        description = "Leave out to check the request; nothing is sent. Pass the code that call returned, after the student has said yes, to send it."
    )]
    pub confirmation: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CancelOutingArgs {
    #[schemars(description = "general or weekend")]
    pub kind: String,
    #[schemars(
        description = "The request's cancel_id from get_general_outing or get_weekend_outing; set only while VTOP lets it be cancelled"
    )]
    pub cancel_id: String,
    #[schemars(
        description = "Leave out to check the request; nothing is sent. Pass the code that call returned, after the student has said yes, to cancel it."
    )]
    pub confirmation: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct OutingPassArgs {
    #[schemars(description = "general or weekend")]
    pub kind: String,
    #[schemars(
        description = "The accepted request's pass_id from get_general_outing or get_weekend_outing"
    )]
    pub pass_id: String,
}

fn invalid(message: impl Into<String>) -> VtopError {
    VtopError::ConfigurationError(message.into())
}

fn outing_kind(raw: &str) -> VtopResult<OutingKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "general" => Ok(OutingKind::General),
        "weekend" => Ok(OutingKind::Weekend),
        _ => Err(invalid("kind must be general or weekend")),
    }
}

/// `HH:MM`, 24-hour.
fn outing_time(raw: &str) -> VtopResult<OutingTime> {
    let (hour, minute) = raw
        .trim()
        .split_once(':')
        .filter(|(hour, minute)| hour.len() <= 2 && minute.len() == 2)
        .and_then(|(hour, minute)| Some((hour.parse().ok()?, minute.parse().ok()?)))
        .ok_or_else(|| invalid("time must be 24-hour HH:MM, e.g. 09:30"))?;
    OutingTime::new(hour, minute)
}

fn weekday(days: i64) -> &'static str {
    WEEKDAYS[usize::from(dates::weekday(days))]
}

/// `YYYY-MM-DD`, as the outing records show dates.
fn iso_date(days: i64) -> String {
    let (year, month, day) = civil_date(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// `Sunday 11-Oct-2026`, the form the tools take dates in, with the
/// weekday.
fn date_label(day: i64) -> String {
    let (year, month, date) = civil_date(day);
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1).clamp(0, 11) as usize];
    format!("{} {date:02}-{month}-{year:04}", weekday(day))
}

/// `Sunday 11-Oct-2026 09:30`, for minutes since 1970 in India.
fn moment_label(at: i64) -> String {
    let minute = at.rem_euclid(1440);
    format!(
        "{} {:02}:{:02}",
        date_label(at.div_euclid(1440)),
        minute / 60,
        minute % 60
    )
}

fn text_message(label: &str, problem: TextProblem) -> String {
    match problem {
        TextProblem::Missing => format!("{label} must not be empty"),
        TextProblem::TooLong { max_length } => {
            format!("{label} must be at most {max_length} characters")
        }
    }
}

/// Why the app's rules refuse this general outing as of `now` (minutes
/// since 1970 in India), the first problem only.
fn general_problem(
    form: &GeneralOutingData,
    application: &GeneralOutingApplication,
    now: i64,
) -> Option<String> {
    let at = |date: &OutingDate, time: OutingTime| {
        date.day_number() * 1440 + i64::from(time.hour) * 60 + i64::from(time.minute)
    };
    let problems = check_general(
        &form.rules(),
        &application.place,
        &application.purpose,
        at(&application.out_date, application.out_time),
        at(&application.in_date, application.in_time),
        now,
    );
    let leaving = problems.leaving.map(|problem| match problem {
        LeavingProblem::TooFarAhead { last_day } => format!(
            "leaving date can be at most {} days ahead, so {} at the latest",
            form.max_days_ahead,
            date_label(last_day)
        ),
        LeavingProblem::OutsideHours => "leaving time is outside the allowed hours".into(),
        LeavingProblem::TooSoon { earliest } => format!(
            "VTOP needs 24 hours' notice: leave at or after {}",
            moment_label(earliest)
        ),
    });
    let returning = problems.returning.map(|problem| match problem {
        ReturningProblem::OutsideHours => "return time is outside the allowed hours".into(),
        ReturningProblem::NotAfterLeaving => "return must be after leaving".into(),
    });
    [
        problems.place.map(|problem| text_message("place", problem)),
        problems
            .purpose
            .map(|problem| text_message("purpose", problem)),
        leaving,
        returning,
    ]
    .into_iter()
    .flatten()
    .next()
}

/// Why the app's rules refuse this weekend outing as of `today` (a day
/// number in India), the first problem only.
fn weekend_problem(
    form: &WeekendOutingData,
    application: &WeekendOutingApplication,
    today: i64,
) -> Option<String> {
    let problems = check_weekend(
        &form.rules(),
        &application.purpose,
        Some(application.date.day_number()),
        application.contact_number.as_str(),
        today,
    );
    let day = problems.day.map(|problem| match problem {
        DayProblem::Missing => "pick a date".to_string(),
        DayProblem::NotOffered => {
            let offered: Vec<String> = weekend_dates(&form.rules(), today)
                .into_iter()
                .map(date_label)
                .collect();
            if offered.is_empty() {
                "VTOP offers no weekend outing day right now".into()
            } else {
                format!(
                    "date is not one VTOP offers; it offers {}",
                    offered.join(", ")
                )
            }
        }
    });
    [
        problems
            .purpose
            .map(|problem| text_message("purpose", problem)),
        day,
        problems
            .contact
            .map(|_| "contact number must be 10 digits".to_string()),
    ]
    .into_iter()
    .flatten()
    .next()
}

/// The code for `key` making exactly this request, good until `expires`.
fn confirmation_code(key: &str, action: &str, fields: &[&str], expires: u64) -> String {
    let mut hash = Sha256::new();
    for part in [key, action, &expires.to_string()]
        .into_iter()
        .chain(fields.iter().copied())
    {
        hash.update(part.as_bytes());
        hash.update([0]);
    }
    let digest: String = hash.finalize()[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{expires}-{digest}")
}

pub fn new_confirmation(key: &str, action: &str, fields: &[&str]) -> String {
    confirmation_code(key, action, fields, now_unix() + CONFIRM_SECONDS)
}

/// Whether `code` was handed out for `key` making exactly this request, and
/// is still good.
pub fn check_confirmation(
    code: &str,
    key: &str,
    action: &str,
    fields: &[&str],
) -> Result<(), String> {
    let expires = code
        .trim()
        .split_once('-')
        .and_then(|(expires, _)| expires.parse::<u64>().ok())
        .ok_or("confirmation is not one this tool gave; call without it first")?;
    if expires < now_unix() {
        return Err("confirmation expired; call without it to check the request again".into());
    }
    if confirmation_code(key, action, fields, expires) != code.trim() {
        return Err(
            "confirmation does not match this request: the arguments changed, so call \
             without it, show the student the new request and get their yes again"
                .into(),
        );
    }
    Ok(())
}

/// Who VTOP files the request for, as the form pre-fills it.
fn student_json(student: &OutingStudent) -> serde_json::Value {
    serde_json::json!({
        "name": student.name,
        "registration_number": student.registration_number,
        "hostel_block": student.hostel_block,
        "room_number": student.room_number,
    })
}

fn closed(notice: &str) -> String {
    if notice.is_empty() {
        "VTOP is not taking outing applications right now".to_string()
    } else {
        format!("VTOP is not taking outing applications: {notice}")
    }
}

fn hhmm(time: OutingTime) -> String {
    format!("{:02}:{:02}", time.hour, time.minute)
}

/// What an agent is told when the key may not act on outings.
pub const OUTINGS_OFF: &str = "outings_off: The student has not allowed this access key to \
apply for or cancel outings. They can turn it on in VITAP Mate → Settings → Connected \
apps → this key → Allow outing applications. Reading outings still works.";

impl VtopTools {
    fn caller(parts: &Parts) -> Result<String, CallToolResult> {
        parts
            .extensions
            .get::<Caller>()
            .map(|Caller(key)| key.clone())
            .ok_or_else(|| tool_error("no caller on the request"))
    }

    /// The caller's key, if the student let it act on outings. Asked fresh
    /// each time (no VTOP call), so turning it off takes effect at once.
    async fn outing_caller(&self, parts: &Parts) -> Result<String, CallToolResult> {
        let key = Self::caller(parts)?;
        match self.source.whoami(&key).await {
            Ok(Some(identity)) if identity.outings => Ok(key),
            Ok(Some(_)) => Err(tool_error(OUTINGS_OFF)),
            Ok(None) => Err(tool_error("key_unknown: Unknown or revoked access key.")),
            Err(message) => Err(tool_error(message)),
        }
    }
}

#[tool_router(router = outing_tools_router, vis = "pub(crate)")]
impl VtopTools {
    #[tool(
        description = "Applies for a general outing (a leave) on VTOP, in two calls. Needs the student's permission on this access key (whoami: outings). First call it without confirmation: it checks the request against VTOP's form and returns what would be sent and a confirmation code, and sends nothing. Show the student that request (place, purpose, leaving and return date with weekday and time) and ask them to confirm. Only after they say yes, call again with the same arguments and the confirmation. Never confirm on your own. The code lapses after 10 minutes. The request then goes to the student's mentor and warden for approval.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn apply_general_outing(
        &self,
        Parameters(args): Parameters<GeneralOutingArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let key = match self.outing_caller(&parts).await {
            Ok(key) => key,
            Err(error) => return error,
        };
        let parsed = (|| {
            Ok(GeneralOutingApplication {
                place: tidy(&args.place),
                purpose: tidy(&args.purpose),
                out_date: OutingDate::parse(args.out_date.trim())?,
                out_time: outing_time(&args.out_time)?,
                in_date: OutingDate::parse(args.in_date.trim())?,
                in_time: outing_time(&args.in_time)?,
            })
        })();
        let application = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let (out_time, in_time) = (hhmm(application.out_time), hhmm(application.in_time));
        let fields = [
            application.place.as_str(),
            application.purpose.as_str(),
            application.out_date.as_str(),
            out_time.as_str(),
            application.in_date.as_str(),
            in_time.as_str(),
        ];
        let action = "apply_general_outing";

        let Some(code) = args.confirmation else {
            let form = match with_retry(self.source.as_ref(), &key, |client| async move {
                client.fetch(&spec::GENERAL_OUTING, &Query::NONE).await
            })
            .await
            {
                Ok(form) => form,
                Err(message) => return tool_error(message),
            };
            let Some(student) = form.student.as_ref() else {
                return tool_error(closed(&form.notice));
            };
            if let Some(problem) = general_problem(&form, &application, minutes_in_india()) {
                return tool_error(format!("VTOP would not take this request: {problem}"));
            }
            let (out, back) = (
                application.out_date.day_number(),
                application.in_date.day_number(),
            );
            return json_result(&serde_json::json!({
                "sent": false,
                "request": {
                    "place": application.place,
                    "purpose": application.purpose,
                    "leaving": format!("{} {}, {out_time}", weekday(out), application.out_date.as_str()),
                    "returning": format!("{} {}, {in_time}", weekday(back), application.in_date.as_str()),
                },
                "for": student_json(student),
                "confirmation": new_confirmation(&key, action, &fields),
                "next": "Show the student this request and ask them to confirm. Only if they say yes, call apply_general_outing again with the same arguments and this confirmation.",
            }));
        };

        if let Err(message) = check_confirmation(&code, &key, action, &fields) {
            return tool_error(message);
        }
        let out_iso = iso_date(application.out_date.day_number());
        let result = with_retry(self.source.as_ref(), &key, |client| {
            let (application, out_iso) = (application.clone(), out_iso.clone());
            async move {
                // A second send of the same code (or a retry after VTOP
                // dropped the session mid-send) must not file it twice.
                let form = client.fetch(&spec::GENERAL_OUTING, &Query::NONE).await?;
                if form.records.iter().any(|record| {
                    record.from_date == out_iso
                        && record.place.eq_ignore_ascii_case(&application.place)
                }) {
                    return Err(invalid(
                        "a request for this place and leaving date is already on VTOP; see get_general_outing",
                    ));
                }
                client.apply_general_outing(&application).await
            }
        })
        .await;
        match result {
            Ok(result) => json_result(&result),
            Err(message) => tool_error(message),
        }
    }

    #[tool(
        description = "Applies for a weekend outing on VTOP, in two calls. Needs the student's permission on this access key (whoami: outings). First call it without confirmation: it checks the request against VTOP's form and returns what would be sent and a confirmation code, and sends nothing. Show the student that request (place, purpose, date with weekday, time slot, contact number) and ask them to confirm. Only after they say yes, call again with the same arguments and the confirmation. Never confirm on your own. The code lapses after 10 minutes. place and time_slot are option values from get_weekend_outing.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn apply_weekend_outing(
        &self,
        Parameters(args): Parameters<WeekendOutingArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let key = match self.outing_caller(&parts).await {
            Ok(key) => key,
            Err(error) => return error,
        };
        let parsed = (|| {
            Ok(WeekendOutingApplication {
                place: args.place.trim().to_string(),
                purpose: tidy(&args.purpose),
                date: OutingDate::parse(args.date.trim())?,
                time_slot: args.time_slot.trim().to_string(),
                contact_number: ContactNumber::parse(&args.contact_number)?,
            })
        })();
        let application = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let fields = [
            application.place.as_str(),
            application.purpose.as_str(),
            application.date.as_str(),
            application.time_slot.as_str(),
            application.contact_number.as_str(),
        ];
        let action = "apply_weekend_outing";

        let Some(code) = args.confirmation else {
            let form = match with_retry(self.source.as_ref(), &key, |client| async move {
                client.fetch(&spec::WEEKEND_OUTING, &Query::NONE).await
            })
            .await
            {
                Ok(form) => form,
                Err(message) => return tool_error(message),
            };
            let Some(student) = form.student.as_ref() else {
                return tool_error(closed(&form.notice));
            };
            if let Some(problem) = check_weekend_outing(&form, &application)
                .err()
                .map(|error| error.to_string())
                .or_else(|| weekend_problem(&form, &application, today_in_india()))
            {
                return tool_error(format!("VTOP would not take this request: {problem}"));
            }
            let label = |options: &[vtop_core::types::OutingOption], value: &str| {
                options
                    .iter()
                    .find(|option| option.value == value)
                    .map_or_else(|| value.to_string(), |option| option.label.clone())
            };
            let date = application.date.day_number();
            return json_result(&serde_json::json!({
                "sent": false,
                "request": {
                    "place": label(&form.places, &application.place),
                    "purpose": application.purpose,
                    "date": format!("{} {}", weekday(date), application.date.as_str()),
                    "time_slot": label(&form.time_slots, &application.time_slot),
                    "contact_number": application.contact_number.as_str(),
                },
                "for": student_json(student),
                "confirmation": new_confirmation(&key, action, &fields),
                "next": "Show the student this request and ask them to confirm. Only if they say yes, call apply_weekend_outing again with the same arguments and this confirmation.",
            }));
        };

        if let Err(message) = check_confirmation(&code, &key, action, &fields) {
            return tool_error(message);
        }
        let date_iso = iso_date(application.date.day_number());
        let result = with_retry(self.source.as_ref(), &key, |client| {
            let (application, date_iso) = (application.clone(), date_iso.clone());
            async move {
                let form = client.fetch(&spec::WEEKEND_OUTING, &Query::NONE).await?;
                if form.records.iter().any(|record| record.date == date_iso) {
                    return Err(invalid(
                        "a weekend outing for this date is already on VTOP; see get_weekend_outing",
                    ));
                }
                client.apply_weekend_outing(&application).await
            }
        })
        .await;
        match result {
            Ok(result) => json_result(&result),
            Err(message) => tool_error(message),
        }
    }

    #[tool(
        description = "Cancels (deletes) a general or weekend outing request on VTOP, in two calls; needs the student's permission on this access key (whoami: outings); only possible before the mentor approves it, while the request has a cancel_id. First call it without confirmation: it finds the request and returns it with a confirmation code, and cancels nothing. Show the student which request will be cancelled and ask them to confirm. Only after they say yes, call again with the same arguments and the confirmation. Never confirm on your own.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true
        )
    )]
    async fn cancel_outing(
        &self,
        Parameters(args): Parameters<CancelOutingArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let key = match self.outing_caller(&parts).await {
            Ok(key) => key,
            Err(error) => return error,
        };
        let parsed = (|| {
            Ok((
                outing_kind(&args.kind)?,
                OutingPassId::parse(&args.cancel_id)?,
            ))
        })();
        let (kind, id) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let kind_name = match kind {
            OutingKind::General => "general",
            OutingKind::Weekend => "weekend",
        };
        let fields = [kind_name, id.as_str()];
        let action = "cancel_outing";

        let Some(code) = args.confirmation else {
            let found = with_retry(self.source.as_ref(), &key, |client| {
                let id = id.clone();
                async move {
                    Ok(match kind {
                        OutingKind::General => client
                            .fetch(&spec::GENERAL_OUTING, &Query::NONE)
                            .await?
                            .records
                            .into_iter()
                            .find(|record| record.cancel_id == id.as_str())
                            .map(|record| serde_json::to_value(record).unwrap_or_default()),
                        OutingKind::Weekend => client
                            .fetch(&spec::WEEKEND_OUTING, &Query::NONE)
                            .await?
                            .records
                            .into_iter()
                            .find(|record| record.cancel_id == id.as_str())
                            .map(|record| serde_json::to_value(record).unwrap_or_default()),
                    })
                }
            })
            .await;
            return match found {
                Ok(Some(record)) => json_result(&serde_json::json!({
                    "cancelled": false,
                    "request": record,
                    "confirmation": new_confirmation(&key, action, &fields),
                    "next": "Show the student this request and ask them to confirm cancelling it. Only if they say yes, call cancel_outing again with the same arguments and this confirmation.",
                })),
                Ok(None) => tool_error(
                    "no request with that cancel_id that VTOP still lets be cancelled; it may be approved already",
                ),
                Err(message) => tool_error(message),
            };
        };

        if let Err(message) = check_confirmation(&code, &key, action, &fields) {
            return tool_error(message);
        }
        // vtop-core checks a fresh copy of the page before cancelling.
        let result = with_retry(self.source.as_ref(), &key, |client| {
            let id = id.clone();
            async move {
                match kind {
                    OutingKind::General => client.cancel_general_outing(&id).await,
                    OutingKind::Weekend => client.cancel_weekend_outing(&id).await,
                }
            }
        })
        .await;
        match result {
            Ok(result) => json_result(&result),
            Err(message) => tool_error(message),
        }
    }

    #[tool(
        description = "The PDF pass of an accepted general or weekend outing, as an embedded resource. Takes the request's pass_id, which is set once it is accepted.",
        annotations(read_only_hint = true)
    )]
    async fn get_outing_pass(
        &self,
        Parameters(args): Parameters<OutingPassArgs>,
        Extension(parts): Extension<Parts>,
    ) -> CallToolResult {
        let key = match Self::caller(&parts) {
            Ok(key) => key,
            Err(error) => return error,
        };
        let parsed = (|| {
            Ok((
                outing_kind(&args.kind)?,
                OutingPassId::parse(&args.pass_id)?,
            ))
        })();
        let (kind, id) = match input(parsed) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let result = with_retry(self.source.as_ref(), &key, |client| {
            let id = id.clone();
            async move { client.outing_pass(kind, &id).await }
        })
        .await;
        match result {
            Ok(bytes) => file_result(
                &format!("{}.pdf", id.as_str()),
                "application/pdf",
                &bytes,
                &format!("vtop://outing-pass/{}", id.as_str()),
            ),
            Err(message) => tool_error(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn general_form() -> GeneralOutingData {
        GeneralOutingData {
            student: None,
            notice: String::new(),
            records: Vec::new(),
            out_hours: (6..=22).collect(),
            in_hours: (6..=20).collect(),
            place_max_length: 50,
            purpose_max_length: 50,
            max_days_ahead: 30,
            max_days_away: 0,
            update_time: 0,
        }
    }

    fn trip(out: &str, out_time: &str, back: &str, in_time: &str) -> GeneralOutingApplication {
        GeneralOutingApplication {
            place: "Guntur".into(),
            purpose: "Visit".into(),
            out_date: OutingDate::parse(out).unwrap(),
            out_time: outing_time(out_time).unwrap(),
            in_date: OutingDate::parse(back).unwrap(),
            in_time: outing_time(in_time).unwrap(),
        }
    }

    #[test]
    fn general_outing_needs_24_hours_notice() {
        // Friday 06-Nov-2026, 10:00 in India.
        let now = 20_763 * 1440 + 600;
        let form = general_form();
        let check = |application| general_problem(&form, &application, now);
        assert_eq!(
            check(trip("07-Nov-2026", "10:00", "07-Nov-2026", "18:00")),
            None
        );
        let soon = check(trip("07-Nov-2026", "09:59", "07-Nov-2026", "18:00")).unwrap();
        assert_eq!(
            soon,
            "VTOP needs 24 hours' notice: leave at or after Saturday 07-Nov-2026 10:00"
        );
        // Later today was fine before ("not in the past"); now it is not.
        assert!(
            check(trip("06-Nov-2026", "18:00", "06-Nov-2026", "20:00"))
                .unwrap()
                .contains("24 hours' notice")
        );
        assert!(
            check(trip("07-Dec-2026", "10:00", "07-Dec-2026", "18:00"))
                .unwrap()
                .contains("at most 30 days ahead")
        );
        // No days ahead allows no day, where 0 used to mean no limit.
        let none_ahead = GeneralOutingData {
            max_days_ahead: 0,
            ..general_form()
        };
        assert!(
            general_problem(
                &none_ahead,
                &trip("08-Nov-2026", "10:00", "08-Nov-2026", "18:00"),
                now
            )
            .unwrap()
            .contains("at most 0 days ahead")
        );
    }

    fn weekend_form(weekdays: Vec<u8>) -> WeekendOutingData {
        WeekendOutingData {
            student: None,
            notice: String::new(),
            records: Vec::new(),
            places: Vec::new(),
            time_slots: Vec::new(),
            purpose_max_length: 50,
            max_days_ahead: 6,
            weekdays,
            update_time: 0,
        }
    }

    fn outing_on(date: &str) -> WeekendOutingApplication {
        WeekendOutingApplication {
            place: "P".into(),
            purpose: "Movie".into(),
            date: OutingDate::parse(date).unwrap(),
            time_slot: "S".into(),
            contact_number: ContactNumber::parse("9876543210").unwrap(),
        }
    }

    #[test]
    fn empty_weekdays_allow_no_day() {
        // Tuesday 03-Nov-2026: tomorrow is not a usual outing day either.
        let tuesday = 20_760;
        let none = weekend_form(vec![]);
        assert_eq!(
            weekend_problem(&none, &outing_on("08-Nov-2026"), tuesday).as_deref(),
            Some("VTOP offers no weekend outing day right now")
        );
        let sundays = weekend_form(vec![0]);
        assert_eq!(
            weekend_problem(&sundays, &outing_on("08-Nov-2026"), tuesday),
            None
        );
        assert_eq!(
            weekend_problem(&sundays, &outing_on("07-Nov-2026"), tuesday).as_deref(),
            Some("date is not one VTOP offers; it offers Sunday 08-Nov-2026")
        );
    }

    #[test]
    fn weekend_tomorrow_counts_when_it_is_a_usual_day() {
        // Saturday 07-Nov-2026: VTOP's picker blocks Sunday, the app adds it.
        let saturday = 20_764;
        assert_eq!(
            weekend_problem(&weekend_form(vec![1]), &outing_on("08-Nov-2026"), saturday),
            None
        );
    }
}
