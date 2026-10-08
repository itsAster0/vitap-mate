//! Hostel outings: general (day leave) and weekend.
//!
//! Applying is two requests, as in VTOP's own page: the form is loaded
//! first for the student details VTOP pre-fills (and expects back), then
//! posted. The choices are checked against what that form offers, so a
//! request VTOP's page would not let through is never sent.

use log::warn;

use super::{network_error, server_error, SendWith, VtopClient, NETWORK};
use crate::error::{VtopError, VtopResult};
use crate::inputs::{ContactNumber, OutingDate, OutingPassId, OutingTime};
use crate::parser::hostel;
use crate::types::{
    GeneralOutingData, OutingApplyResult, OutingCancelResult, OutingStudent, WeekendOutingData,
};

/// A general outing (day leave) application.
#[derive(Debug, Clone)]
pub struct GeneralOutingApplication {
    pub place: String,
    pub purpose: String,
    pub out_date: OutingDate,
    pub out_time: OutingTime,
    pub in_date: OutingDate,
    pub in_time: OutingTime,
}

/// A weekend outing application. `place` and `time_slot` are option values
/// from [`WeekendOutingData`].
#[derive(Debug, Clone)]
pub struct WeekendOutingApplication {
    pub place: String,
    pub purpose: String,
    pub date: OutingDate,
    pub time_slot: String,
    pub contact_number: ContactNumber,
}

/// Which outing a pass belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutingKind {
    General,
    Weekend,
}

impl VtopClient {
    pub async fn general_outing_page(&self) -> VtopResult<String> {
        self.hostel_menu_page("get_general_outing", "/vtop/hostel/StudentGeneralOuting")
            .await
    }

    pub async fn general_outing(&self) -> VtopResult<GeneralOutingData> {
        Ok(hostel::parse_general_outing(
            &self.general_outing_page().await?,
        ))
    }

    pub async fn weekend_outing_page(&self) -> VtopResult<String> {
        self.hostel_menu_page("get_weekend_outing", "/vtop/hostel/StudentWeekendOuting")
            .await
    }

    pub async fn weekend_outing(&self) -> VtopResult<WeekendOutingData> {
        Ok(hostel::parse_weekend_outing(
            &self.weekend_outing_page().await?,
        ))
    }

    pub async fn apply_general_outing(
        &self,
        application: &GeneralOutingApplication,
    ) -> VtopResult<OutingApplyResult> {
        let form = self.general_outing().await?;
        let Some(student) = form.student.as_ref() else {
            return Ok(form_closed(&form.notice));
        };
        let place = limited_text(&application.place, "place", form.place_max_length)?;
        let purpose = limited_text(&application.purpose, "purpose", form.purpose_max_length)?;
        offered_time(application.out_time, &form.out_hours, "leaving")?;
        offered_time(application.in_time, &form.in_hours, "return")?;

        let auth = self.authorize("apply_general_outing").await?;
        let url = self.config.url("/vtop/hostel/saveGeneralOutingForm");
        let mut fields = student_fields(auth.registration_number.as_str(), student, "LeaveId");
        fields.extend([
            ("placeOfVisit", place),
            ("purposeOfVisit", purpose),
            ("outDate", application.out_date.as_str().to_string()),
            ("outTimeHr", application.out_time.hour_field()),
            ("outTimeMin", application.out_time.minute_field()),
            ("inDate", application.in_date.as_str().to_string()),
            ("inTimeHr", application.in_time.hour_field()),
            ("inTimeMin", application.in_time.minute_field()),
            ("_csrf", auth.csrf.clone()),
            ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
        ]);
        Self::log_request("apply_general_outing.send", "POST", &url);
        // Not retried: a retry after a timeout could file the request twice.
        let page = self
            .send_once("apply_general_outing", self.http.post(&url).form(&fields))
            .await?;
        Ok(hostel::parse_apply_response(&page, form.records.len()))
    }

    pub async fn apply_weekend_outing(
        &self,
        application: &WeekendOutingApplication,
    ) -> VtopResult<OutingApplyResult> {
        let form = self.weekend_outing().await?;
        let Some(student) = form.student.as_ref() else {
            return Ok(form_closed(&form.notice));
        };
        let purpose = limited_text(&application.purpose, "purpose", form.purpose_max_length)?;
        if application.date.day_number() <= today_in_india() {
            return Err(invalid(
                "a weekend outing cannot be applied for on its own day",
            ));
        }
        if !form.places.iter().any(|o| o.value == application.place) {
            return Err(invalid("place is not one VTOP offers"));
        }
        if !form
            .time_slots
            .iter()
            .any(|o| o.value == application.time_slot)
        {
            return Err(invalid("time slot is not one VTOP offers"));
        }

        let auth = self.authorize("apply_weekend_outing").await?;
        let url = self.config.url("/vtop/hostel/saveOutingForm");
        let mut fields = student_fields(auth.registration_number.as_str(), student, "BookingId");
        fields.extend([
            ("outPlace", application.place.clone()),
            ("purposeOfVisit", purpose),
            ("outingDate", application.date.as_str().to_string()),
            ("outTime", application.time_slot.clone()),
            (
                "contactNumber",
                application.contact_number.as_str().to_string(),
            ),
            ("_csrf", auth.csrf.clone()),
            ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
        ]);
        Self::log_request("apply_weekend_outing.send", "POST", &url);
        let page = self
            .send_once("apply_weekend_outing", self.http.post(&url).form(&fields))
            .await?;
        Ok(hostel::parse_apply_response(&page, form.records.len()))
    }

    /// Cancels a general outing request. Only while VTOP shows a delete
    /// button for it (before the mentor approves); checked against a fresh
    /// copy of the page first, so an accepted request is never touched.
    pub async fn cancel_general_outing(&self, id: &OutingPassId) -> VtopResult<OutingCancelResult> {
        let page = self.general_outing().await?;
        if !page.records.iter().any(|r| r.cancel_id == id.as_str()) {
            return Err(invalid("VTOP no longer lets this request be cancelled"));
        }
        self.cancel_outing(
            "cancel_general_outing",
            "/vtop/hostel/deleteGeneralOutingInfo",
            "LeaveId",
            id,
        )
        .await
    }

    /// [`Self::cancel_general_outing`] for a weekend outing request.
    pub async fn cancel_weekend_outing(&self, id: &OutingPassId) -> VtopResult<OutingCancelResult> {
        let page = self.weekend_outing().await?;
        if !page.records.iter().any(|r| r.cancel_id == id.as_str()) {
            return Err(invalid("VTOP no longer lets this request be cancelled"));
        }
        self.cancel_outing(
            "cancel_weekend_outing",
            "/vtop/hostel/deleteBookingInfo",
            "BookingId",
            id,
        )
        .await
    }

    /// What VTOP's `deleteStudentBookingInfo` posts.
    async fn cancel_outing(
        &self,
        context: &str,
        path: &str,
        id_field: &'static str,
        id: &OutingPassId,
    ) -> VtopResult<OutingCancelResult> {
        let auth = self.authorize(context).await?;
        let url = self.config.url(path);
        let form = [
            ("_csrf", auth.csrf.clone()),
            (id_field, id.as_str().to_string()),
            ("authorizedID", auth.registration_number.to_string()),
            ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
        ];
        Self::log_request(&format!("{context}.send"), "POST", &url);
        let page = self
            .send_once(context, self.http.post(&url).form(&form))
            .await?;
        Ok(hostel::parse_cancel_response(&page, id.as_str()))
    }

    /// The PDF pass of an accepted outing.
    pub async fn outing_pass(&self, kind: OutingKind, id: &OutingPassId) -> VtopResult<Vec<u8>> {
        let context = "get_outing_pass";
        let auth = self.authorize(context).await?;
        let path = match kind {
            OutingKind::General => "/vtop/hostel/downloadLeavePass/",
            OutingKind::Weekend => "/vtop/hostel/downloadOutingForm/",
        };
        let url = self.config.url(&format!("{path}{}", id.as_str()));
        let query = [
            ("authorizedID", auth.registration_number.to_string()),
            ("_csrf", auth.csrf.clone()),
            ("x", httpdate::fmt_http_date(std::time::SystemTime::now())),
        ];
        Self::log_request(context, "GET", &url);
        let response = self
            .http
            .get(&url)
            .query(&query)
            .send_with(self)
            .await
            .map_err(|error| network_error(&format!("{context}.send"), error))?;

        let final_url = response.url().to_string();
        if Self::is_login_url(&final_url) {
            self.mark_session_expired(context, &format!("VTOP redirected to {final_url}"));
            return Err(VtopError::SessionExpired);
        }
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| network_error(&format!("{context}.bytes"), error))?;
        if !status.is_success() {
            return Err(server_error(
                context,
                format!("VTOP returned HTTP {status}"),
            ));
        }
        if !bytes.starts_with(b"%PDF") {
            warn!(target: NETWORK, "{context}: VTOP answered with a page, not a PDF");
            return Err(VtopError::VtopServerError(
                "VTOP did not return the pass".to_string(),
            ));
        }
        Ok(bytes.to_vec())
    }

    /// Both outing pages are menu entries: the form plus the student's
    /// requests in `#BookingRequests`.
    async fn hostel_menu_page(&self, context: &str, path: &str) -> VtopResult<String> {
        let auth = self.authorize(context).await?;
        let url = self.config.url(path);
        let body = format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            auth.registration_number, auth.csrf,
        );
        Self::log_request(&format!("{context}.send"), "POST", &url);
        self.send_authenticated(context, || self.http.post(&url).body(body.clone()))
            .await
    }

    /// Sends a signed-in request once, with no retry.
    async fn send_once(
        &self,
        context: &str,
        request: reqwest::RequestBuilder,
    ) -> VtopResult<String> {
        let response = request
            .send_with(self)
            .await
            .map_err(|error| network_error(&format!("{context}.send"), error))?;
        self.read_authenticated_response_text(context, response)
            .await
    }
}

/// The pre-filled fields VTOP's form posts, plus the empty id of a new
/// request (`LeaveId` or `BookingId`).
fn student_fields(
    authorized_id: &str,
    student: &OutingStudent,
    id_field: &'static str,
) -> Vec<(&'static str, String)> {
    vec![
        ("authorizedID", authorized_id.to_string()),
        (id_field, String::new()),
        ("regNo", student.registration_number.clone()),
        ("name", student.name.clone()),
        ("applicationNo", student.application_no.clone()),
        ("gender", student.gender.clone()),
        ("hostelBlock", student.hostel_block.clone()),
        ("roomNo", student.room_number.clone()),
        ("parentContactNumber", student.parent_contact_number.clone()),
    ]
}

fn form_closed(notice: &str) -> OutingApplyResult {
    OutingApplyResult {
        applied: false,
        message: if notice.is_empty() {
            "VTOP is not taking outing applications right now.".to_string()
        } else {
            notice.to_string()
        },
    }
}

/// Today's day number (days since 1970-01-01) in India (UTC+5:30), where
/// VTOP's dates are.
fn today_in_india() -> i64 {
    (crate::now_unix() as i64 + 19_800).div_euclid(86_400)
}

fn invalid(message: &str) -> VtopError {
    VtopError::ConfigurationError(message.to_string())
}

fn limited_text(value: &str, label: &str, max_length: u32) -> VtopResult<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        return Err(invalid(&format!("{label} must not be empty")));
    }
    if value.chars().count() > max_length as usize {
        return Err(invalid(&format!(
            "{label} must be at most {max_length} characters"
        )));
    }
    Ok(value)
}

/// The form offers whole hours from a list; the last one only at :00.
fn offered_time(time: OutingTime, hours: &[u8], label: &str) -> VtopResult<()> {
    let Some(last) = hours.last() else {
        return Ok(());
    };
    if !hours.contains(&time.hour) || (time.hour == *last && time.minute != 0) {
        return Err(invalid(&format!(
            "{label} time is outside the allowed hours"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offered_time_follows_the_form() {
        let out_hours: Vec<u8> = (6..=22).collect();
        assert!(offered_time(OutingTime::new(6, 0).unwrap(), &out_hours, "x").is_ok());
        assert!(offered_time(OutingTime::new(21, 59).unwrap(), &out_hours, "x").is_ok());
        assert!(offered_time(OutingTime::new(22, 0).unwrap(), &out_hours, "x").is_ok());
        assert!(offered_time(OutingTime::new(22, 1).unwrap(), &out_hours, "x").is_err());
        assert!(offered_time(OutingTime::new(5, 59).unwrap(), &out_hours, "x").is_err());
    }

    #[test]
    fn limited_text_counts_characters_after_tidying() {
        assert_eq!(limited_text("  a   b ", "x", 3).unwrap(), "a b");
        assert!(limited_text("   ", "x", 3).is_err());
        assert!(limited_text("abcd", "x", 3).is_err());
    }
}
