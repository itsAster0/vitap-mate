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
    check_general_outing, check_weekend_outing, GeneralOutingApplication, OutingKind,
    WeekendOutingApplication,
};
use vtop_core::inputs::{ContactNumber, OutingDate, OutingPassId, OutingTime};
use vtop_core::types::{GeneralOutingData, OutingStudent, WeekendOutingData};
use vtop_core::{now_unix, VtopError, VtopResult};

use crate::tools::{
    civil_date, file_result, input, json_result, tool_error, with_retry, Caller, VtopTools,
};

/// How long a confirmation code stays good.
const CONFIRM_SECONDS: u64 = 600;

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

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

/// Today's day number in India (UTC+5:30), where VTOP's dates are.
fn today_in_india() -> i64 {
    (now_unix() as i64 + 19_800).div_euclid(86_400)
}

fn weekday(days: i64) -> &'static str {
    WEEKDAYS[(days + 4).rem_euclid(7) as usize]
}

/// `YYYY-MM-DD`, as the outing records show dates.
fn iso_date(days: i64) -> String {
    let (year, month, day) = civil_date(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// The dates the general outing form would accept, which the app's date
/// pickers enforce. A limit of 0 is taken as no limit.
fn check_general_dates(
    form: &GeneralOutingData,
    application: &GeneralOutingApplication,
) -> VtopResult<()> {
    let (today, out, back) = (
        today_in_india(),
        application.out_date.day_number(),
        application.in_date.day_number(),
    );
    if out < today {
        return Err(invalid("leaving date is in the past"));
    }
    if form.max_days_ahead > 0 && out > today + i64::from(form.max_days_ahead) {
        return Err(invalid(format!(
            "leaving date can be at most {} days ahead",
            form.max_days_ahead
        )));
    }
    let leaves = (out, application.out_time.hour, application.out_time.minute);
    let returns = (back, application.in_time.hour, application.in_time.minute);
    if returns <= leaves {
        return Err(invalid("return must be after leaving"));
    }
    if form.max_days_away > 0 && back > out + i64::from(form.max_days_away) {
        return Err(invalid(format!(
            "return date can be at most {} days after leaving",
            form.max_days_away
        )));
    }
    Ok(())
}

/// [`check_general_dates`] for a weekend outing.
fn check_weekend_date(
    form: &WeekendOutingData,
    application: &WeekendOutingApplication,
) -> VtopResult<()> {
    let date = application.date.day_number();
    if form.max_days_ahead > 0 && date > today_in_india() + i64::from(form.max_days_ahead) {
        return Err(invalid(format!(
            "outing date can be at most {} days ahead",
            form.max_days_ahead
        )));
    }
    let day = (date + 4).rem_euclid(7) as u8;
    if !form.weekdays.is_empty() && !form.weekdays.contains(&day) {
        let allowed: Vec<_> = form
            .weekdays
            .iter()
            .map(|day| WEEKDAYS[usize::from(*day) % 7])
            .collect();
        return Err(invalid(format!(
            "weekend outings can only be on {}",
            allowed.join(", ")
        )));
    }
    Ok(())
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
                client.general_outing().await
            })
            .await
            {
                Ok(form) => form,
                Err(message) => return tool_error(message),
            };
            let Some(student) = form.student.as_ref() else {
                return tool_error(closed(&form.notice));
            };
            if let Err(error) = check_general_outing(&form, &application)
                .and_then(|_| check_general_dates(&form, &application))
            {
                return tool_error(format!("VTOP would not take this request: {error}"));
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
                let form = client.general_outing().await?;
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
                client.weekend_outing().await
            })
            .await
            {
                Ok(form) => form,
                Err(message) => return tool_error(message),
            };
            let Some(student) = form.student.as_ref() else {
                return tool_error(closed(&form.notice));
            };
            if let Err(error) = check_weekend_outing(&form, &application)
                .and_then(|_| check_weekend_date(&form, &application))
            {
                return tool_error(format!("VTOP would not take this request: {error}"));
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
                let form = client.weekend_outing().await?;
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
                            .general_outing()
                            .await?
                            .records
                            .into_iter()
                            .find(|record| record.cancel_id == id.as_str())
                            .map(|record| serde_json::to_value(record).unwrap_or_default()),
                        OutingKind::Weekend => client
                            .weekend_outing()
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
