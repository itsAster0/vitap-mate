//! Hostel outings: the lists, apply and cancel, and the pass. Apply and
//! cancel first read the list (VTOP's form fields and what it still lets
//! be cancelled), then send the action once, never retried.

use crate::error::{VtopError, VtopResult};
use crate::http::{Request, Response, form_encode, http_date};
use crate::inputs::{ContactNumber, OutingDate, OutingPassId, OutingTime};
use crate::machine::{Machine, Signed, Step, is_login_url, signed_page};
use crate::now_unix;
use crate::parser::hostel;
use crate::types::{
    GeneralOutingData, OutingApplyResult, OutingCancelResult, OutingStudent, WeekendOutingData,
};

#[derive(Debug, Clone)]
pub struct GeneralOutingApplication {
    pub place: String,
    pub purpose: String,
    pub out_date: OutingDate,
    pub out_time: OutingTime,
    pub in_date: OutingDate,
    pub in_time: OutingTime,
}

#[derive(Debug, Clone)]
pub struct WeekendOutingApplication {
    pub place: String,
    pub purpose: String,
    pub date: OutingDate,
    pub time_slot: String,
    pub contact_number: ContactNumber,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutingKind {
    General,
    Weekend,
}

impl OutingKind {
    fn list_path(self) -> &'static str {
        match self {
            Self::General => "/vtop/hostel/StudentGeneralOuting",
            Self::Weekend => "/vtop/hostel/StudentWeekendOuting",
        }
    }

    fn id_field(self) -> &'static str {
        match self {
            Self::General => "LeaveId",
            Self::Weekend => "BookingId",
        }
    }
}

/// The outing list of `kind`, as the side menu opens it.
pub fn list(s: &Signed, kind: OutingKind) -> Request {
    Request::post(
        s.url(kind.list_path()),
        format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            s.registration_number, s.csrf,
        ),
    )
}

/// The pass PDF of an approved outing.
pub fn pass(s: &Signed, kind: OutingKind, id: &OutingPassId) -> Request {
    let path = match kind {
        OutingKind::General => "/vtop/hostel/downloadLeavePass/",
        OutingKind::Weekend => "/vtop/hostel/downloadOutingForm/",
    };
    let query = form_encode(&[
        ("authorizedID", &s.registration_number),
        ("_csrf", &s.csrf),
        ("x", &http_date(now_unix())),
    ]);
    Request::get(format!(
        "{}?{query}",
        s.url(&format!("{path}{}", id.as_str()))
    ))
}

/// The pass bytes from [`pass`]'s answer: a PDF, or an error.
pub fn read_pass(response: &Response) -> VtopResult<Vec<u8>> {
    if is_login_url(&response.url) || response.redirect().is_some_and(is_login_url) {
        return Err(VtopError::SessionExpired);
    }
    match response.status {
        504 => {
            return Err(VtopError::VtopUnavailable(
                "VTOP took too long to answer".into(),
            ));
        }
        502 | 503 => return Err(VtopError::VtopUnavailable("VTOP is down right now".into())),
        200..=299 => {}
        status => {
            return Err(VtopError::VtopServerError(format!(
                "VTOP returned HTTP {status}"
            )));
        }
    }
    if !response.body.starts_with(b"%PDF") {
        return Err(VtopError::VtopServerError(
            "VTOP did not return the pass".into(),
        ));
    }
    Ok(response.body.clone())
}

/// What the apply machine sends, once VTOP's form is read.
pub enum Application {
    General(GeneralOutingApplication),
    Weekend(WeekendOutingApplication),
}

impl Application {
    fn kind(&self) -> OutingKind {
        match self {
            Self::General(_) => OutingKind::General,
            Self::Weekend(_) => OutingKind::Weekend,
        }
    }
}

/// Reads the outing list, checks the application against VTOP's form,
/// then sends it once.
pub struct Apply {
    signed: Signed,
    application: Application,
    /// Requests on the list before applying, once it is read.
    before: Option<usize>,
}

impl Apply {
    pub fn new(signed: Signed, application: Application) -> Self {
        Self {
            signed,
            application,
            before: None,
        }
    }

    /// The save request for VTOP's form, or the answer without sending.
    fn save(&mut self, page: &str) -> Result<Request, VtopResult<OutingApplyResult>> {
        let s = &self.signed;
        let x = http_date(now_unix());
        let (path, mut fields) = match &self.application {
            Application::General(application) => {
                let form = hostel::parse_general_outing(page);
                let Some(student) = form.student.as_ref() else {
                    return Err(Ok(form_closed(&form.notice)));
                };
                let (place, purpose) = check_general_outing(&form, application).map_err(Err)?;
                self.before = Some(form.records.len());
                let mut fields = student_fields(&s.registration_number, student, "LeaveId");
                fields.extend([
                    ("placeOfVisit", place),
                    ("purposeOfVisit", purpose),
                    ("outDate", application.out_date.as_str().to_string()),
                    ("outTimeHr", application.out_time.hour_field()),
                    ("outTimeMin", application.out_time.minute_field()),
                    ("inDate", application.in_date.as_str().to_string()),
                    ("inTimeHr", application.in_time.hour_field()),
                    ("inTimeMin", application.in_time.minute_field()),
                ]);
                ("/vtop/hostel/saveGeneralOutingForm", fields)
            }
            Application::Weekend(application) => {
                let form = hostel::parse_weekend_outing(page);
                let Some(student) = form.student.as_ref() else {
                    return Err(Ok(form_closed(&form.notice)));
                };
                let purpose = check_weekend_outing(&form, application).map_err(Err)?;
                self.before = Some(form.records.len());
                let mut fields = student_fields(&s.registration_number, student, "BookingId");
                fields.extend([
                    ("outPlace", application.place.clone()),
                    ("purposeOfVisit", purpose),
                    ("outingDate", application.date.as_str().to_string()),
                    ("outTime", application.time_slot.clone()),
                    (
                        "contactNumber",
                        application.contact_number.as_str().to_string(),
                    ),
                ]);
                ("/vtop/hostel/saveOutingForm", fields)
            }
        };
        fields.extend([("_csrf", s.csrf.clone()), ("x", x)]);
        let pairs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        Ok(Request::form(s.url(path), &pairs).once())
    }
}

impl Machine for Apply {
    type Output = OutingApplyResult;

    fn start(&mut self) -> Step<OutingApplyResult> {
        Step::Send(list(&self.signed, self.application.kind()))
    }

    fn feed(&mut self, response: Response) -> Step<OutingApplyResult> {
        let page = match signed_page(&response) {
            Ok(page) => page,
            Err(error) => return Step::Done(Err(error)),
        };
        match self.before {
            None => match self.save(&page) {
                Ok(request) => Step::Send(request),
                Err(done) => Step::Done(done),
            },
            Some(before) => Step::Done(Ok(hostel::parse_apply_response(&page, before))),
        }
    }
}

/// Reads the outing list, checks VTOP still lets `id` be cancelled, then
/// cancels it once.
pub struct Cancel {
    signed: Signed,
    kind: OutingKind,
    id: OutingPassId,
    sent: bool,
}

impl Cancel {
    pub fn new(signed: Signed, kind: OutingKind, id: OutingPassId) -> Self {
        Self {
            signed,
            kind,
            id,
            sent: false,
        }
    }
}

impl Machine for Cancel {
    type Output = OutingCancelResult;

    fn start(&mut self) -> Step<OutingCancelResult> {
        Step::Send(list(&self.signed, self.kind))
    }

    fn feed(&mut self, response: Response) -> Step<OutingCancelResult> {
        let page = match signed_page(&response) {
            Ok(page) => page,
            Err(error) => return Step::Done(Err(error)),
        };
        if self.sent {
            return Step::Done(Ok(hostel::parse_cancel_response(&page, self.id.as_str())));
        }
        let id = self.id.as_str();
        let cancellable = match self.kind {
            OutingKind::General => hostel::parse_general_outing(&page)
                .records
                .iter()
                .any(|r| r.cancel_id == id),
            OutingKind::Weekend => hostel::parse_weekend_outing(&page)
                .records
                .iter()
                .any(|r| r.cancel_id == id),
        };
        if !cancellable {
            return Step::Done(Err(invalid(
                "VTOP no longer lets this request be cancelled",
            )));
        }
        self.sent = true;
        let path = match self.kind {
            OutingKind::General => "/vtop/hostel/deleteGeneralOutingInfo",
            OutingKind::Weekend => "/vtop/hostel/deleteBookingInfo",
        };
        let s = &self.signed;
        let x = http_date(now_unix());
        let request = Request::form(
            s.url(path),
            &[
                ("_csrf", &s.csrf),
                (self.kind.id_field(), id),
                ("authorizedID", &s.registration_number),
                ("x", &x),
            ],
        );
        Step::Send(request.once())
    }
}

pub fn check_general_outing(
    form: &GeneralOutingData,
    application: &GeneralOutingApplication,
) -> VtopResult<(String, String)> {
    let place = limited_text(&application.place, "place", form.place_max_length)?;
    let purpose = limited_text(&application.purpose, "purpose", form.purpose_max_length)?;
    offered_time(application.out_time, &form.out_hours, "leaving")?;
    offered_time(application.in_time, &form.in_hours, "return")?;
    Ok((place, purpose))
}

pub fn check_weekend_outing(
    form: &WeekendOutingData,
    application: &WeekendOutingApplication,
) -> VtopResult<String> {
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
    Ok(purpose)
}

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

fn today_in_india() -> i64 {
    (now_unix() as i64 + 19_800).div_euclid(86_400)
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

    fn signed() -> Signed {
        Signed {
            base_url: "https://vtop.example".into(),
            csrf: "c".into(),
            registration_number: "22BCE0001".into(),
        }
    }

    fn page(html: &str) -> Response {
        Response {
            status: 200,
            headers: Vec::new(),
            url: "https://vtop.example/vtop/x".into(),
            body: html.as_bytes().to_vec(),
        }
    }

    /// A general outing list with one request VTOP still lets be cancelled.
    const OUTING_PAGE: &str = r#"<table id="BookingRequests"><thead><tr>
<th>S.No</th><th>Registration Number</th><th>Place Of Visit</th><th>Purpose Of Visit</th>
<th>From Date</th><th>From Time</th><th>To Date</th><th>To Time</th><th>Action</th>
<th>Status</th><th>Download OutPass</th></tr></thead><tbody>
<tr><td>1</td><td>22BCE0001</td><td>Home</td><td>Holidays</td><td>2026-10-17 00:00:00.0</td>
<td>11:30 AM</td><td>2026-10-21 00:00:00.0</td><td>08:00 PM</td>
<td><button onclick="deleteStudentBookingInfo(this.getAttribute('data-bookingId'));"
data-bookingId="L1">x</button></td><td>Waiting</td><td></td></tr></tbody></table>"#;

    #[test]
    fn cancel_reads_the_list_then_sends_once() {
        let mut machine = Cancel::new(
            signed(),
            OutingKind::General,
            OutingPassId::parse("L1").unwrap(),
        );
        assert!(
            matches!(machine.start(), Step::Send(ref r) if r.url.ends_with("/StudentGeneralOuting"))
        );
        let Step::Send(cancel) = machine.feed(page(OUTING_PAGE)) else {
            panic!("expected the cancel")
        };
        assert!(!cancel.retry);
        assert!(cancel.url.ends_with("/deleteGeneralOutingInfo"));
        let body = String::from_utf8(cancel.body).unwrap();
        assert!(
            body.starts_with("_csrf=c&LeaveId=L1&authorizedID=22BCE0001&x="),
            "{body}"
        );
        assert!(matches!(
            machine.feed(page("<html></html>")),
            Step::Done(Ok(_))
        ));
    }

    #[test]
    fn cancel_refuses_what_vtop_no_longer_lists() {
        let mut machine = Cancel::new(
            signed(),
            OutingKind::General,
            OutingPassId::parse("L9").unwrap(),
        );
        machine.start();
        assert!(matches!(
            machine.feed(page(OUTING_PAGE)),
            Step::Done(Err(VtopError::ConfigurationError(_)))
        ));
    }

    #[test]
    fn apply_on_a_closed_form_sends_nothing() {
        let application = Application::General(GeneralOutingApplication {
            place: "Vijayawada".into(),
            purpose: "Shopping".into(),
            out_date: OutingDate::parse("05-Jan-2030").unwrap(),
            out_time: OutingTime::new(9, 0).unwrap(),
            in_date: OutingDate::parse("05-Jan-2030").unwrap(),
            in_time: OutingTime::new(18, 0).unwrap(),
        });
        let mut machine = Apply::new(signed(), application);
        assert!(
            matches!(machine.start(), Step::Send(ref r) if r.url.ends_with("/StudentGeneralOuting"))
        );
        assert!(
            matches!(machine.feed(page("<html></html>")), Step::Done(Ok(ref result)) if !result.applied)
        );
    }

    #[test]
    fn pass_reads_only_pdfs() {
        let request = pass(
            &signed(),
            OutingKind::Weekend,
            &OutingPassId::parse("B7").unwrap(),
        );
        assert!(request.url.starts_with("https://vtop.example/vtop/hostel/downloadOutingForm/B7?authorizedID=22BCE0001&_csrf=c&x="));
        let pdf = Response {
            body: b"%PDF-1.4".to_vec(),
            ..page("")
        };
        assert_eq!(read_pass(&pdf).unwrap(), b"%PDF-1.4");
        assert!(matches!(
            read_pass(&page("<html>")),
            Err(VtopError::VtopServerError(_))
        ));
    }
}
