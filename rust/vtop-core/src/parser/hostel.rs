//! Hostel outing pages (general and weekend).
//!
//! Both pages are the request form followed by the student's requests in
//! `table#BookingRequests`. The form's limits (offered hours, slots, places,
//! date range, allowed weekdays) are read from the page itself so the app
//! follows VTOP when they change; the defaults below are what VTOP served
//! when this was written and only apply when a value cannot be read.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{selector, word_text, TD, TH, TR};
use crate::now_unix;
use crate::types::{
    GeneralOutingData, GeneralOutingRecord, OutingApplyResult, OutingCancelResult, OutingOption,
    OutingStudent, WeekendOutingData, WeekendOutingRecord,
};

static INPUT: LazyLock<Selector> = LazyLock::new(|| selector("input[id]"));
static REQUESTS: LazyLock<Selector> = LazyLock::new(|| selector("table#BookingRequests"));
static REQUEST_SPANS: LazyLock<Selector> = LazyLock::new(|| selector("table#BookingRequests span"));
static CANCEL_BUTTON: LazyLock<Selector> =
    LazyLock::new(|| selector("button[onclick*='deleteStudentBookingInfo']"));
static PASS_LINK: LazyLock<Selector> = LazyLock::new(|| selector("a[data-url], a[data-leave-url]"));
static OPTION: LazyLock<Selector> = LazyLock::new(|| selector("option"));
static STYLED_SPAN: LazyLock<Selector> = LazyLock::new(|| selector("span[style]"));

const DEFAULT_OUT_HOURS: std::ops::RangeInclusive<u8> = 6..=22;
const DEFAULT_IN_HOURS: std::ops::RangeInclusive<u8> = 6..=20;
const DEFAULT_TEXT_LENGTH: u32 = 20;
const DEFAULT_GENERAL_DAYS_AHEAD: u32 = 30;
const DEFAULT_GENERAL_DAYS_AWAY: u32 = 15;
const DEFAULT_WEEKEND_DAYS_AHEAD: u32 = 6;
const DEFAULT_WEEKEND_DAYS: [u8; 2] = [0, 1];

pub fn parse_general_outing(html: &str) -> GeneralOutingData {
    let document = Html::parse_document(html);
    let records = request_rows(&document)
        .into_iter()
        .map(|row| GeneralOutingRecord {
            serial: row.cell("s.no"),
            place: row.cell("place of visit"),
            purpose: row.cell("purpose of visit"),
            from_date: date_part(&row.cell("from date")),
            from_time: row.cell("from time"),
            to_date: date_part(&row.cell("to date")),
            to_time: row.cell("to time"),
            status: row.cell("status"),
            pass_id: row.pass_id,
            cancel_id: row.cancel_id,
        })
        .collect();

    GeneralOutingData {
        student: student(&document),
        notice: notice(&document),
        records,
        out_hours: hours(&document, "outTimeHr", DEFAULT_OUT_HOURS),
        in_hours: hours(&document, "inTimeHr", DEFAULT_IN_HOURS),
        place_max_length: max_length(&document, "placeOfVisit"),
        purpose_max_length: max_length(&document, "purposeOfVisit"),
        max_days_ahead: datepicker_number(html, "outDate", "maxDate")
            .unwrap_or(DEFAULT_GENERAL_DAYS_AHEAD),
        // `maxDate.setDate(maxDate.getDate() + 15)` once a leaving date is set.
        max_days_away: number_after(html, "getDate() +").unwrap_or(DEFAULT_GENERAL_DAYS_AWAY),
        update_time: now_unix(),
    }
}

pub fn parse_weekend_outing(html: &str) -> WeekendOutingData {
    let document = Html::parse_document(html);
    let records = request_rows(&document)
        .into_iter()
        .map(|row| WeekendOutingRecord {
            serial: row.cell("s.no"),
            hostel_block: row.cell("hostel block"),
            room_number: row.cell("room number"),
            place: row.cell("place of visit"),
            purpose: row.cell("purpose of visit"),
            time_slot: row.cell("time"),
            date: date_part(&row.cell("date")),
            status: row.cell("status"),
            pass_id: row.pass_id,
            cancel_id: row.cancel_id,
        })
        .collect();

    WeekendOutingData {
        student: student(&document),
        notice: notice(&document),
        records,
        places: options(&document, "outPlace"),
        time_slots: options(&document, "outTime"),
        purpose_max_length: max_length(&document, "purposeOfVisit"),
        max_days_ahead: datepicker_number(html, "outingDate", "maxDate")
            .unwrap_or(DEFAULT_WEEKEND_DAYS_AHEAD),
        weekdays: allowed_weekdays(html),
        update_time: now_unix(),
    }
}

/// Reads the page VTOP sends back after an application.
///
/// VTOP no longer shows a confirmation: the save handler just reloads the
/// outing page. So the request list is the evidence — [`requests_before`]
/// is how many requests there were before submitting, and one more now
/// means it went through. A message in the `success` / `jsonBom` inputs or a
/// coloured form-level span (outside the list, whose red "Waiting for ...
/// Approval" statuses are not errors) is passed on when present.
pub fn parse_apply_response(html: &str, requests_before: usize) -> OutingApplyResult {
    let document = Html::parse_document(html);
    let success = input_value(&document, "success");
    let error = notice(&document);
    let requests_now = request_rows(&document).len();

    if !error.is_empty() {
        return OutingApplyResult {
            applied: false,
            message: error,
        };
    }
    if !success.is_empty() || requests_now > requests_before {
        return OutingApplyResult {
            applied: true,
            message: if success.is_empty() {
                "Applied. It now waits for approval.".to_string()
            } else {
                success
            },
        };
    }
    OutingApplyResult {
        applied: false,
        message: if document.select(&REQUESTS).next().is_some() {
            "VTOP did not add the request. Check the list before trying again.".to_string()
        } else {
            "VTOP sent back an unexpected page. Check the list before trying again.".to_string()
        },
    }
}

/// Reads the page VTOP sends back after a delete: the outing page reloaded.
/// The request is cancelled when no row carries [cancel_id] any more.
pub fn parse_cancel_response(html: &str, cancel_id: &str) -> OutingCancelResult {
    let document = Html::parse_document(html);
    let error = notice(&document);
    if !error.is_empty() {
        return OutingCancelResult {
            cancelled: false,
            message: error,
        };
    }
    if document.select(&REQUESTS).next().is_none() {
        return OutingCancelResult {
            cancelled: false,
            message: "VTOP sent back an unexpected page. Check the list.".to_string(),
        };
    }
    let still_there = request_rows(&document)
        .iter()
        .any(|row| row.cancel_id == cancel_id);
    OutingCancelResult {
        cancelled: !still_there,
        message: if still_there {
            "VTOP did not cancel the request. Check the list.".to_string()
        } else {
            "Request cancelled.".to_string()
        },
    }
}

/// One row of `#BookingRequests`, its cells keyed by lower-cased header.
struct RequestRow {
    cells: Vec<(String, String)>,
    pass_id: String,
    cancel_id: String,
}

impl RequestRow {
    fn cell(&self, header: &str) -> String {
        self.cells
            .iter()
            .find(|(name, _)| name == header)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    }
}

fn request_rows(document: &Html) -> Vec<RequestRow> {
    let Some(table) = document.select(&REQUESTS).next() else {
        return Vec::new();
    };
    let headers: Vec<String> = table
        .select(&TH)
        .map(|header| word_text(&header).to_lowercase())
        .collect();
    table
        .select(&TR)
        .filter_map(|row| {
            let cells: Vec<ElementRef<'_>> = row.select(&TD).collect();
            if cells.is_empty() {
                return None;
            }
            // DataTables renders "No data available in table" as one cell.
            if cells.len() < headers.len() {
                return None;
            }
            let pass_id = row
                .select(&PASS_LINK)
                .next()
                .and_then(|link| {
                    link.value()
                        .attr("data-url")
                        .or_else(|| link.value().attr("data-leave-url"))
                })
                .and_then(|url| url.rsplit('/').next())
                .unwrap_or_default()
                .to_string();
            // `deleteStudentBookingInfo(this.getAttribute('data-bookingId'))`
            // with the id in `data-bookingId` (attribute names arrive
            // lower-cased), or the id written into the call itself.
            let cancel_id = row
                .select(&CANCEL_BUTTON)
                .next()
                .and_then(|button| {
                    button
                        .value()
                        .attr("data-bookingid")
                        .map(str::to_string)
                        .or_else(|| {
                            let call = button.value().attr("onclick")?;
                            let start = call.find("deleteStudentBookingInfo(")?
                                + "deleteStudentBookingInfo(".len();
                            let arg = call[start..].split(')').next()?.trim();
                            arg.starts_with('\'')
                                .then(|| arg.trim_matches('\'').to_string())
                        })
                })
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
                .unwrap_or_default();
            Some(RequestRow {
                cells: headers
                    .iter()
                    .cloned()
                    .zip(cells.iter().map(word_text))
                    .collect(),
                pass_id,
                cancel_id,
            })
        })
        .collect()
}

/// VTOP leaves the student fields out while it is not taking applications.
fn student(document: &Html) -> Option<OutingStudent> {
    let student = OutingStudent {
        registration_number: input_value(document, "regNo"),
        name: input_value(document, "name"),
        application_no: input_value(document, "applicationNo"),
        gender: input_value(document, "gender"),
        hostel_block: input_value(document, "hostelBlock"),
        room_number: input_value(document, "roomNo"),
        parent_contact_number: input_value(document, "parentContactNumber"),
    };
    (!student.registration_number.is_empty()).then_some(student)
}

/// VTOP's message about the form: the `jsonBom` input it raises as an error
/// popup (e.g. "You are eligible to fill this form from Tuesday 12:00AM to
/// Friday 11:59PM"), else a red form-level span.
fn notice(document: &Html) -> String {
    let popup = input_value(document, "jsonBom");
    if !popup.is_empty() {
        return popup;
    }
    let in_requests: Vec<_> = document.select(&REQUEST_SPANS).map(|el| el.id()).collect();
    document
        .select(&STYLED_SPAN)
        .filter(|span| !in_requests.contains(&span.id()))
        .filter(|span| {
            let style = span
                .value()
                .attr("style")
                .unwrap_or_default()
                .replace(' ', "");
            style.contains("color:red")
        })
        .map(|span| word_text(&span))
        .find(|text| !text.is_empty())
        .unwrap_or_default()
}

fn input_value(document: &Html, id: &str) -> String {
    document
        .select(&INPUT)
        .find(|input| input.value().attr("id") == Some(id))
        .and_then(|input| input.value().attr("value"))
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn max_length(document: &Html, id: &str) -> u32 {
    document
        .select(&INPUT)
        .find(|input| input.value().attr("id") == Some(id))
        .and_then(|input| input.value().attr("maxlength"))
        .and_then(|value| value.trim().parse().ok())
        .filter(|length| *length > 0)
        .unwrap_or(DEFAULT_TEXT_LENGTH)
}

fn select_options<'a>(document: &'a Html, id: &str) -> Vec<ElementRef<'a>> {
    let select = Selector::parse(&format!("select#{id}")).ok();
    select
        .and_then(|select| document.select(&select).next())
        .map(|select| select.select(&OPTION).collect())
        .unwrap_or_default()
}

/// The options of `select#id`, without the empty placeholder.
fn options(document: &Html, id: &str) -> Vec<OutingOption> {
    select_options(document, id)
        .into_iter()
        .filter_map(|option| {
            let value = option.value().attr("value")?.trim().to_string();
            if value.is_empty() {
                return None;
            }
            Some(OutingOption {
                // The label carries typos ("12:30 AM- 6:30PM" for the 12:30 PM
                // slot); the value is what VTOP stores and shows in the list.
                label: value.clone(),
                value,
            })
        })
        .collect()
}

fn hours(document: &Html, id: &str, default: std::ops::RangeInclusive<u8>) -> Vec<u8> {
    let hours: Vec<u8> = select_options(document, id)
        .into_iter()
        .filter_map(|option| option.value().attr("value")?.trim().parse().ok())
        .filter(|hour| *hour < 24)
        .collect();
    if hours.is_empty() {
        default.collect()
    } else {
        hours
    }
}

/// `2026-05-16 00:00:00.0` → `2026-05-16`.
fn date_part(value: &str) -> String {
    value
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// A number set for `key` in the `$("#id").datepicker({...})` call, e.g.
/// `maxDate : '6'`. The first call wins; later ones are commented out.
fn datepicker_number(html: &str, id: &str, key: &str) -> Option<u32> {
    let start = html.find(&format!("#{id}\").datepicker("))?;
    let call = &html[start..];
    let call = &call[..call.find("})").unwrap_or(call.len())];
    number_after(call, key)
}

/// The first unsigned number after `marker`, skipping `:`, quotes and spaces.
fn number_after(text: &str, marker: &str) -> Option<u32> {
    let rest = &text[text.find(marker)? + marker.len()..];
    let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ':' || c == '\'');
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Weekdays left open by `beforeShowDay`, which blocks days with
/// `day != 2 && day != 3 ...` (0 = Sunday).
fn allowed_weekdays(html: &str) -> Vec<u8> {
    let Some(start) = html.find("beforeShowDay") else {
        return DEFAULT_WEEKEND_DAYS.to_vec();
    };
    let body = &html[start..];
    let body = &body[..body.find(']').unwrap_or(body.len())];
    let blocked: Vec<u8> = body
        .match_indices("!=")
        .filter_map(|(at, _)| {
            body[at + 2..]
                .trim_start()
                .chars()
                .next()
                .and_then(|c| c.to_digit(10))
                .map(|day| day as u8)
        })
        .collect();
    if blocked.is_empty() {
        return DEFAULT_WEEKEND_DAYS.to_vec();
    }
    (0..7).filter(|day| !blocked.contains(day)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW_STYLE: &str = r##"style="border: 1px solid #195e86""##;

    fn general_page(form_open: bool, rows: &str) -> String {
        let student = if form_open {
            r##"<input value="24MIC7076" id="regNo" name="regNo" readonly />
               <input value="A STUDENT" id="name" />
               <input value="2024751891" id="applicationNo" />
               <input value="MALE" id="gender" />
               <input value="MH-2" id="hostelBlock" />
               <input value="911" id="roomNo" />
               <input value="9000000000" id="parentContactNumber" />"##
        } else {
            ""
        };
        format!(
            r##"<form id="outingForm">
              <input type="hidden" id="success" value="" />
              <input type="hidden" id="jsonBom" value="{notice}" />
              {student}
              <input id="placeOfVisit" maxlength="20" />
              <input id="purposeOfVisit" maxlength="25" />
              <select id="outTimeHr"><option value="">-- HH --</option>
                <option value="06">06</option><option value="22">22</option></select>
              <select id="inTimeHr"><option value="">-- HH --</option>
                <option value="06">06</option><option value="20">20</option>
                <!-- <option value="21">21</option> --></select>
            </form>
            <table id="BookingRequests"><thead><tr>
              <th>S.No</th><th>Registration Number</th><th>Place Of Visit</th>
              <th>Purpose Of Visit</th><th>From Date</th><th>From Time</th>
              <th>To Date</th><th>To Time</th><th>Action</th><th>Status</th>
              <th>Download OutPass</th></tr></thead><tbody>{rows}</tbody></table>
            <script>
              $("#outDate").datepicker({{ dateFormat: 'dd-M-yy', minDate: 0, maxDate: 30 }});
              maxDate.setDate(maxDate.getDate() + 15);
              /* $("#outDate").datepicker({{ minDate : '0', maxDate : 99 }}) */
            </script>"##,
            notice = if form_open {
                ""
            } else {
                "You are eligible to fill this form from Tuesday 12:00AM"
            },
        )
    }

    fn general_row(place: &str, status_colour: &str, status: &str, pass: &str) -> String {
        let link = if pass.is_empty() {
            String::new()
        } else {
            format!(r##"<a data-url="/vtop/hostel/downloadLeavePass/{pass}">x</a>"##)
        };
        format!(
            r##"<tr><td {ROW_STYLE}>1</td><td>24MIC7076</td><td>{place} </td>
            <td>ugadi
               trip</td><td>2026-03-18 00:00:00.0</td><td>01:00 PM</td>
            <td>2026-03-22 00:00:00.0</td><td>09:00 PM</td><td> </td>
            <td><span> <span style="color: {status_colour};">{status}</span></span></td>
            <td><span>{link}</span></td></tr>"##
        )
    }

    #[test]
    fn reads_general_requests_by_header() {
        let page = general_page(
            true,
            &format!(
                "{}{}",
                general_row("Home", "red", "Waiting for\n Warden's Approval", ""),
                general_row("Guntur", "green", "Leave Request Accepted", "L24790947224"),
            ),
        );
        let data = parse_general_outing(&page);
        assert_eq!(data.records.len(), 2);
        let pending = &data.records[0];
        assert_eq!(pending.place, "Home");
        assert_eq!(pending.purpose, "ugadi trip");
        assert_eq!(pending.from_date, "2026-03-18");
        assert_eq!(pending.to_time, "09:00 PM");
        assert_eq!(pending.status, "Waiting for Warden's Approval");
        assert_eq!(pending.pass_id, "");
        assert_eq!(data.records[1].pass_id, "L24790947224");
        // The pending status is red but lives in the list: not a notice.
        assert_eq!(data.notice, "");
    }

    #[test]
    fn reads_general_form_limits_from_the_page() {
        let data = parse_general_outing(&general_page(true, ""));
        let student = data.student.expect("form is open");
        assert_eq!(student.registration_number, "24MIC7076");
        assert_eq!(student.hostel_block, "MH-2");
        assert_eq!(data.out_hours, vec![6, 22]);
        // The commented-out 21 must not count.
        assert_eq!(data.in_hours, vec![6, 20]);
        assert_eq!(data.place_max_length, 20);
        assert_eq!(data.purpose_max_length, 25);
        assert_eq!(data.max_days_ahead, 30);
        assert_eq!(data.max_days_away, 15);
    }

    #[test]
    fn closed_form_has_no_student_and_carries_the_notice() {
        let data = parse_general_outing(&general_page(false, ""));
        assert!(data.student.is_none());
        assert!(data.notice.starts_with("You are eligible"));
    }

    #[test]
    fn falls_back_when_the_form_is_missing() {
        let data = parse_general_outing("<html></html>");
        assert!(data.records.is_empty());
        assert_eq!(data.out_hours.first(), Some(&6));
        assert_eq!(data.out_hours.last(), Some(&22));
        assert_eq!(data.in_hours.last(), Some(&20));
        assert_eq!(data.place_max_length, 20);
        assert_eq!(data.max_days_ahead, 30);
    }

    const WEEKEND_PAGE: &str = r##"
        <input type="hidden" id="jsonBom" value="" />
        <input value="24MIC7076" id="regNo" />
        <span style="font-size: 20px; color: red; text-align: center;"></span>
        <select name="outPlace" id="outPlace">
          <option value="Vijayawada">Vijayawada</option>
          <option value="Others">Others</option></select>
        <input id="purposeOfVisit" maxlength="20" />
        <select name="outTime" id="outTime">
          <!-- <option value="7 AM - 1PM">7 AM - 1PM</option> -->
          <option value="9:30 AM- 3:30PM">9:30 AM- 3:30PM</option>
          <option value="12:30 PM- 6:30PM">12:30 AM- 6:30PM</option></select>
        <table id="BookingRequests"><thead><tr>
          <th>S.No</th><th>Registration Number</th><th>Hostel Block</th>
          <th>Room Number</th><th>Place Of Visit</th><th>Purpose Of Visit</th>
          <th>Time</th><!-- <th>Contact Number</th> --><th>Date</th>
          <th>Action</th><th>Status</th><th>Download OutPass</th></tr></thead>
        <tbody><tr><td>1</td><td>24MIC7076</td><td>MH-4</td><td>211</td>
          <td>Vijayawada</td><td>Movie </td><td>9:30 AM- 3:30PM</td>
          <td>2026-03-29</td><td></td>
          <td><span><span style="color: green;">Outing
            Request Accepted</span></span></td>
          <td><a data-leave-url="/vtop/hostel/downloadOutingForm/W24859931194">
            Download</a></td></tr></tbody></table>
        <script>
          $("#outingDate").datepicker({ autoClose : true, dateFormat : 'dd-M-yy',
            minDate : '0', maxDate : '6',
            beforeShowDay : function(d) { var day = d.getDay();
              return [ day != 2 && day != 3 && day != 4
                && day != 5 && day != 6 ]; }, })
        </script>"##;

    #[test]
    fn reads_weekend_page() {
        let data = parse_weekend_outing(WEEKEND_PAGE);
        assert!(data.student.is_some());
        assert_eq!(data.notice, "");
        assert_eq!(data.places.len(), 2);
        assert_eq!(data.places[0].value, "Vijayawada");
        // Commented-out slots are ignored and the value wins over a typo'd label.
        assert_eq!(data.time_slots.len(), 2);
        assert_eq!(data.time_slots[1].label, "12:30 PM- 6:30PM");
        assert_eq!(data.purpose_max_length, 20);
        assert_eq!(data.max_days_ahead, 6);
        assert_eq!(data.weekdays, vec![0, 1]);

        let record = &data.records[0];
        assert_eq!(record.hostel_block, "MH-4");
        assert_eq!(record.purpose, "Movie");
        assert_eq!(record.time_slot, "9:30 AM- 3:30PM");
        assert_eq!(record.date, "2026-03-29");
        assert_eq!(record.status, "Outing Request Accepted");
        assert_eq!(record.pass_id, "W24859931194");
    }

    #[test]
    fn weekdays_default_without_a_rule() {
        assert_eq!(allowed_weekdays("no picker here"), vec![0, 1]);
    }

    #[test]
    fn apply_counts_a_new_request_as_applied() {
        let before = general_page(
            true,
            &general_row("Home", "green", "Leave Request Accepted", "L1"),
        );
        let after = general_page(
            true,
            &format!(
                "{}{}",
                general_row("Guntur", "red", "Waiting for Warden's Approval", ""),
                general_row("Home", "green", "Leave Request Accepted", "L1"),
            ),
        );
        let requests_before = parse_general_outing(&before).records.len();
        assert!(parse_apply_response(&after, requests_before).applied);
        let unchanged = parse_apply_response(&before, requests_before);
        assert!(!unchanged.applied);
    }

    #[test]
    fn apply_reports_vtop_error() {
        let page = r##"<input id="jsonBom" value="You have already applied for this weekend" />
            <table id="BookingRequests"></table>"##;
        let result = parse_apply_response(page, 0);
        assert!(!result.applied);
        assert_eq!(result.message, "You have already applied for this weekend");
    }

    /// The live markup of a request VTOP still lets the student delete,
    /// beside one it does not.
    fn cancellable_page(with_cancellable: bool) -> String {
        let cancellable = if with_cancellable {
            r##"<tr><td>1</td><td>24MIC7076</td><td>Home</td><td>Holidays</td>
              <td>2026-10-17 00:00:00.0</td><td>11:30 AM</td>
              <td>2026-10-21 00:00:00.0</td><td>08:00 PM</td>
              <td><span> <button id="eventsbtn" class="btn btn-danger btn-sm"
                onclick="deleteStudentBookingInfo(this.getAttribute('data-bookingId'));"
                data-bookingId="L25677109842"><span class="bi bi-trash-fill"></span></button></span></td>
              <td><span><span style="color: red;">Waiting for Mentor's Approval</span></span></td>
              <td></td></tr>"##
        } else {
            ""
        };
        format!(
            r##"<input id="jsonBom" value="" />
            <table id="BookingRequests"><thead><tr>
              <th>S.No</th><th>Registration Number</th><th>Place Of Visit</th>
              <th>Purpose Of Visit</th><th>From Date</th><th>From Time</th>
              <th>To Date</th><th>To Time</th><th>Action</th><th>Status</th>
              <th>Download OutPass</th></tr></thead><tbody>{cancellable}
              <tr><td>2</td><td>24MIC7076</td><td>Home</td><td>summer vacation</td>
              <td>2026-05-16 00:00:00.0</td><td>06:00 AM</td>
              <td>2026-07-16 00:00:00.0</td><td>08:00 PM</td><td> </td>
              <td><span><span style="color: red;">Waiting for Warden's Approval</span></span></td>
              <td></td></tr></tbody></table>"##
        )
    }

    #[test]
    fn reads_which_requests_can_be_cancelled() {
        let data = parse_general_outing(&cancellable_page(true));
        assert_eq!(data.records.len(), 2);
        assert_eq!(data.records[0].cancel_id, "L25677109842");
        assert_eq!(data.records[1].cancel_id, "");
    }

    #[test]
    fn cancel_is_confirmed_by_the_row_going() {
        let gone = parse_cancel_response(&cancellable_page(false), "L25677109842");
        assert!(gone.cancelled);
        let kept = parse_cancel_response(&cancellable_page(true), "L25677109842");
        assert!(!kept.cancelled);
        let odd = parse_cancel_response("<html></html>", "L25677109842");
        assert!(!odd.cancelled);
    }
}
