use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vtop_core::sync::RwLock;
use vtop_core::{SessionState, VtopClient, VtopConfig};

use super::*;
use crate::{Account, Session};

fn day(date: &str) -> i32 {
    records::epoch_day(date).unwrap()
}

fn general_record(
    from: (&str, &str),
    to: (&str, &str),
    status: &str,
    pass: &str,
    cancel: &str,
) -> GeneralOutingRecord {
    GeneralOutingRecord {
        serial: "1".into(),
        place: "Home ".into(),
        purpose: "Visit".into(),
        from_date: from.0.into(),
        from_time: from.1.into(),
        to_date: to.0.into(),
        to_time: to.1.into(),
        status: status.into(),
        pass_id: pass.into(),
        cancel_id: cancel.into(),
    }
}

fn general_data(records: Vec<GeneralOutingRecord>) -> GeneralOutingData {
    GeneralOutingData {
        student: Some(OutingStudent {
            registration_number: "22BCE0001".into(),
            hostel_block: "MH-2".into(),
            room_number: "911".into(),
            ..OutingStudent::default()
        }),
        notice: String::new(),
        records,
        out_hours: (6..=22).collect(),
        in_hours: (6..=20).collect(),
        place_max_length: 20,
        purpose_max_length: 25,
        max_days_ahead: 30,
        max_days_away: 15,
        update_time: 5,
    }
}

fn rules() -> GeneralOutingRules {
    GeneralOutingRules::from(&general_data(Vec::new()))
}

fn when(lines: &[OutingWhen]) -> Vec<(Option<&str>, &str)> {
    lines
        .iter()
        .map(|w| (w.label.as_deref(), w.text.as_str()))
        .collect()
}

#[test]
fn general_requests_are_dated_labelled_and_sorted() {
    // Friday 6 Nov 2026, 10:00 in India.
    let now = (day("2026-11-06"), 600);
    let outing = GeneralOuting::new(
        general_data(vec![
            general_record(
                ("2026-10-17", "11:30 AM"),
                ("2026-10-21", "08:00 PM"),
                "Leave Request Accepted",
                "L2",
                "",
            ),
            general_record(
                ("2026-11-07", "09:00 AM"),
                ("2026-11-07", "06:00 PM"),
                "Waiting for Mentor's Approval",
                "",
                "L1",
            ),
            general_record(
                ("2026-11-09", "09:00 AM"),
                ("2026-11-09", ""),
                "Rejected by warden",
                "",
                "",
            ),
        ]),
        now,
    );
    assert!(outing.open);
    assert_eq!(outing.hostel.as_deref(), Some("MH-2 · Room 911"));
    assert_eq!((outing.first_day, outing.last_day), (now.0, now.0 + 30));
    assert_eq!(outing.out_window.as_deref(), Some("6:00 AM – 10:00 PM"));
    let facts: Vec<_> = outing
        .facts
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert_eq!(
        facts,
        [
            ("Apply", "24 h – 30 days ahead"),
            ("Leave", "6 AM – 10 PM"),
            ("Back by", "8 PM"),
            ("Longest trip", "15 days")
        ]
    );

    let [rejected, pending, accepted] = outing.requests.as_slice() else {
        panic!("three requests");
    };
    // Newest first.
    assert_eq!(rejected.standing, OutingStanding::Rejected);
    assert_eq!(rejected.highlight, None);
    assert_eq!(when(&rejected.when), [(None, "Mon 9 Nov, 9:00 AM")]);

    assert_eq!(
        (pending.standing, pending.status_label.as_str()),
        (OutingStanding::Pending, "PENDING")
    );
    assert_eq!(pending.highlight.as_deref(), Some("TOMORROW"));
    assert_eq!(pending.cancel_id.as_deref(), Some("L1"));
    assert_eq!(pending.pass_id, None);
    assert_eq!(pending.place, "Home");
    assert_eq!(
        when(&pending.when),
        [
            (None, "Sat 7 Nov, 9:00 AM – 6:00 PM"),
            (Some("Away"), "9 h")
        ]
    );

    assert_eq!(accepted.standing, OutingStanding::Approved);
    assert_eq!(accepted.highlight, None);
    assert_eq!(accepted.pass_id.as_deref(), Some("L2"));
    assert_eq!(
        when(&accepted.when),
        [
            (Some("Out"), "Sat 17 Oct, 11:30 AM"),
            (Some("Back"), "Wed 21 Oct, 8:00 PM"),
            (Some("Away"), "4 days")
        ]
    );
}

#[test]
fn trips_under_way_and_odd_dates() {
    let now = (day("2026-11-06"), 600);
    let (_, out) = general_request(
        0,
        &general_record(
            ("2026-11-05", "09:00 AM"),
            ("2026-11-08", ""),
            "Accepted",
            "",
            "",
        ),
        now,
    );
    assert_eq!(out.highlight.as_deref(), Some("OUT NOW"));
    let (at, odd) = general_request(
        1,
        &general_record(("soon", "x"), ("later", ""), "Something", "", ""),
        now,
    );
    assert_eq!(at, 0);
    assert_eq!(odd.status_label, "SOMETHING");
    assert_eq!(
        when(&odd.when),
        [(Some("Out"), "soon x"), (Some("Back"), "later ")]
    );
    assert_eq!(
        upcoming(
            i64::from(now.0 + 3) * 1440,
            i64::from(now.0 + 4) * 1440,
            now
        )
        .as_deref(),
        Some("IN 3 DAYS")
    );
    assert_eq!(short_date(day("2027-01-02"), now.0), "Sat 2 Jan 2027");
}

fn weekend_data(weekdays: Vec<u8>) -> WeekendOutingData {
    WeekendOutingData {
        student: Some(OutingStudent {
            registration_number: "22BCE0001".into(),
            parent_contact_number: "9000000000".into(),
            ..OutingStudent::default()
        }),
        notice: String::new(),
        records: vec![WeekendOutingRecord {
            serial: "1".into(),
            hostel_block: "MH-4".into(),
            room_number: "211".into(),
            place: "Vijayawada".into(),
            purpose: "Movie".into(),
            time_slot: "9:30 AM- 3:30PM".into(),
            date: "2026-11-08".into(),
            status: "Outing Request Accepted".into(),
            pass_id: "W24859931194".into(),
            cancel_id: String::new(),
        }],
        places: vec![vtop_core::types::OutingOption {
            value: "Vijayawada".into(),
            label: "Vijayawada".into(),
        }],
        time_slots: vec![vtop_core::types::OutingOption {
            value: "12:30 PM- 6:30PM".into(),
            label: "12:30 AM- 6:30PM".into(),
        }],
        purpose_max_length: 20,
        max_days_ahead: 6,
        weekdays,
        update_time: 9,
    }
}

#[test]
fn weekend_days_dates_and_requests() {
    // Saturday: VTOP's picker leaves only Monday; Sunday comes back.
    let saturday = day("2026-11-07");
    let outing = WeekendOuting::new(weekend_data(vec![1]), (saturday, 600));
    assert!(outing.open);
    assert_eq!(outing.dates, [day("2026-11-08"), day("2026-11-09")]);
    assert_eq!(outing.days_label, "Sundays and Mondays");
    assert_eq!(outing.facts[0].value, "Sun & Mon");
    assert_eq!(outing.facts[1].value, "Sun 8 Nov");
    assert_eq!(outing.slots[0].label, "12:30 PM – 6:30 PM");
    assert_eq!(outing.parent_contact.as_deref(), Some("9000000000"));
    let request = &outing.requests[0];
    assert_eq!(
        when(&request.when),
        [(None, "Sun 8 Nov · 9:30 AM – 3:30 PM")]
    );
    assert_eq!(request.highlight.as_deref(), Some("TOMORROW"));
    assert_eq!(request.standing, OutingStanding::Approved);

    // Never today; a week with no allowed day closes the form with a reason.
    let sunday = day("2026-11-08");
    assert_eq!(
        weekend_dates(&WeekendOutingRules::from(&weekend_data(vec![0])), sunday),
        [day("2026-11-09")]
    );
    let mut none = weekend_data(vec![]);
    none.max_days_ahead = 0;
    let closed = WeekendOuting::new(none, (sunday, 0));
    assert!(!closed.open);
    assert_eq!(closed.notice, "No outing day in the coming week.");
    let mut shut = weekend_data(vec![0, 1]);
    shut.student = None;
    shut.notice = "Applications open on Friday".into();
    let shut = WeekendOuting::new(shut, (sunday, 0));
    assert_eq!(
        (shut.open, shut.notice.as_str()),
        (false, "Applications open on Friday")
    );
}

#[test]
fn weekend_checks() {
    let rules = WeekendOutingRules::from(&weekend_data(vec![0, 1]));
    let friday = day("2026-11-06");
    let ok = weekend_check(
        &rules,
        "Movie",
        Some(day("2026-11-08")),
        "9876543210",
        friday,
    );
    assert_eq!(ok.first(), None);
    let bad = weekend_check(&rules, "  ", Some(day("2026-11-07")), "98765", friday);
    assert_eq!(bad.purpose.as_deref(), Some("Add why you're going"));
    assert_eq!(bad.day.as_deref(), Some("Not a day VTOP offers"));
    assert_eq!(bad.contact.as_deref(), Some("A 10-digit mobile number"));
    assert_eq!(
        weekend_check(&rules, "x", None, "9876543210", friday)
            .day
            .as_deref(),
        Some("Pick a day")
    );
}

#[test]
fn general_checks_follow_the_flutter_rules() {
    let rules = rules();
    let now = (day("2026-11-06"), 600);
    let m = |offset: i32, minute: i32| OutingMoment {
        day: now.0 + offset,
        minute,
    };
    let check =
        |leaving, returning| general_check(&rules, "Guntur", "Visit", leaving, returning, now);

    // Exactly a day ahead is enough notice.
    let fine = check(m(1, 600), m(1, 18 * 60));
    assert_eq!(fine.first(), None);
    assert_eq!(fine.away.as_deref(), Some("Away for 8 h"));
    assert_eq!(
        check(m(2, 600), m(3, 720)).away.as_deref(),
        Some("Away for 1 d 2 h")
    );
    assert_eq!(
        check(m(1, 590), m(1, 18 * 60)).leaving.as_deref(),
        Some("Needs 24 h notice: leave after Sat 7 Nov, 10:00 AM")
    );
    assert_eq!(
        check(m(2, 22 * 60 + 30), m(3, 600)).leaving.as_deref(),
        Some("Leave between 6:00 AM and 10:00 PM")
    );
    assert_eq!(
        check(m(31, 600), m(31, 700)).leaving.as_deref(),
        Some("Up to 30 days ahead (6 Dec)")
    );
    let late = check(m(2, 600), m(2, 21 * 60));
    assert_eq!(
        late.returning.as_deref(),
        Some("Be back between 6:00 AM and 8:00 PM")
    );
    assert_eq!(late.away, None);
    assert_eq!(
        check(m(2, 600), m(2, 540)).returning.as_deref(),
        Some("Has to be after you leave")
    );
    let texts = general_check(&rules, "   ", &"x".repeat(26), m(2, 600), m(2, 700), now);
    assert_eq!(texts.place.as_deref(), Some("Add where you're going"));
    assert_eq!(texts.purpose.as_deref(), Some("At most 25 characters"));
    assert_eq!(texts.first().as_deref(), Some("Add where you're going"));
}

#[test]
fn the_form_starts_just_over_a_day_ahead() {
    let rules = rules();
    let today = day("2026-11-06");
    let at = |day: i32, minute: i32| OutingMoment { day, minute };
    assert_eq!(earliest_leaving(&rules, (today, 600)), at(today + 1, 630));
    // Past the last hour: the next morning; before the first: that morning.
    assert_eq!(
        earliest_leaving(&rules, (today, 23 * 60)),
        at(today + 2, 360)
    );
    assert_eq!(earliest_leaving(&rules, (today, 120)), at(today + 1, 360));
    assert_eq!(
        default_return(&rules, at(today + 1, 630)),
        at(today + 1, 1200)
    );
    assert_eq!(
        default_return(&rules, at(today + 1, 1260)),
        at(today + 2, 1200)
    );
    let any = GeneralOutingRules {
        out_hours: vec![],
        in_hours: vec![],
        ..rules
    };
    assert_eq!(
        earliest_leaving(&any, (today, 23 * 60)),
        at(today + 1, 23 * 60 + 30)
    );
    assert!(offered(&[], 3));
}

#[test]
fn slots_clocks_and_dates() {
    assert_eq!(time_slot("9:30 AM- 3:30PM"), Some((570, 930)));
    assert_eq!(time_slot("12:30 PM- 6:30PM"), Some((750, 1110)));
    assert_eq!(time_slot("7 AM - 1PM"), None);
    assert_eq!(slot_label("7 AM - 1PM"), "7 AM - 1PM");
    assert_eq!(short_clock(18 * 60), "6 PM");
    assert_eq!(short_clock(390), "6:30 AM");
    assert_eq!(clock(0), "12:00 AM");
    assert_eq!(list_days(&[0], "and", false), "Sundays");
    assert_eq!(list_days(&[0, 1, 6], "&", true), "Sun, Mon & Sat");
    assert_eq!(
        outing_date(day("2026-10-11")).unwrap().as_str(),
        "11-Oct-2026"
    );
    assert_eq!(
        outing_date(day("2026-03-05")).unwrap().as_str(),
        "05-Mar-2026"
    );
    assert!(outing_time(24 * 60).is_err());
    assert_eq!(outing_time(9 * 60 + 5).unwrap().minute_field(), "05");
    assert_eq!(trip_length(0), "");
}

// Write paths, against a stand-in for VTOP: what is sent, how often, and
// that nothing is sent when a check fails.

const HEADERS: &str = "<th>S.No</th><th>Registration Number</th><th>Place Of Visit</th>
<th>Purpose Of Visit</th><th>From Date</th><th>From Time</th><th>To Date</th><th>To Time</th>
<th>Action</th><th>Status</th><th>Download OutPass</th>";

fn general_row(id: &str) -> String {
    format!(
        r#"<tr><td>1</td><td>22BCE0001</td><td>Home</td><td>Holidays</td><td>2026-10-17 00:00:00.0</td>
<td>11:30 AM</td><td>2026-10-21 00:00:00.0</td><td>08:00 PM</td>
<td><button onclick="deleteStudentBookingInfo(this.getAttribute('data-bookingId'));"
data-bookingId="{id}">x</button></td><td>Waiting</td><td></td></tr>"#
    )
}

fn general_page(rows: &str) -> String {
    format!(
        r#"<form><input type="hidden" id="success" value="" /><input type="hidden" id="jsonBom" value="" />
<input value="22BCE0001" id="regNo" /><input value="MH-2" id="hostelBlock" /><input value="911" id="roomNo" />
<input id="placeOfVisit" maxlength="20" /><input id="purposeOfVisit" maxlength="25" />
<select id="outTimeHr"><option value="06">06</option><option value="22">22</option></select>
<select id="inTimeHr"><option value="06">06</option><option value="20">20</option></select></form>
<table id="BookingRequests"><thead><tr>{HEADERS}</tr></thead><tbody>{rows}</tbody></table>"#
    )
}

fn weekend_page(rows: &str) -> String {
    format!(
        r#"<input type="hidden" id="jsonBom" value="" /><input value="22BCE0001" id="regNo" />
<select id="outPlace"><option value="Vijayawada">Vijayawada</option></select>
<input id="purposeOfVisit" maxlength="20" />
<select id="outTime"><option value="9:30 AM- 3:30PM">9:30 AM- 3:30PM</option></select>
<table id="BookingRequests"><thead><tr><th>S.No</th><th>Registration Number</th><th>Hostel Block</th>
<th>Room Number</th><th>Place Of Visit</th><th>Purpose Of Visit</th><th>Time</th><th>Date</th>
<th>Action</th><th>Status</th><th>Download OutPass</th></tr></thead><tbody>{rows}</tbody></table>"#
    )
}

const WEEKEND_ROW: &str = r#"<tr><td>1</td><td>22BCE0001</td><td>MH-4</td><td>211</td><td>Vijayawada</td>
<td>Movie</td><td>9:30 AM- 3:30PM</td><td>2026-11-08</td>
<td><button onclick="deleteStudentBookingInfo(this.getAttribute('data-bookingId'));"
data-bookingId="W1">x</button></td><td>Waiting for Approval</td><td></td></tr>"#;

/// Answers VTOP's outing paths with canned pages and counts the requests.
#[derive(Clone, Default)]
struct MockVtop {
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

impl MockVtop {
    fn hits(&self, prefix: &str) -> usize {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(_, n)| n)
            .sum()
    }

    async fn start(&self) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mock = self.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mock = mock.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 16 * 1024];
                    while let Ok(read) = socket.read(&mut buffer).await {
                        if read == 0 {
                            return;
                        }
                        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                        *mock.hits.lock().unwrap().entry(path.clone()).or_default() += 1;
                        let (status, body) = respond(&path);
                        let response = format!(
                            "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
                            body.len()
                        );
                        if socket.write_all(response.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        format!("http://{address}")
    }
}

fn respond(path: &str) -> (&'static str, String) {
    let ok = "200 OK";
    match path {
        "/vtop/hostel/StudentGeneralOuting" => (ok, general_page(&general_row("L1"))),
        // The list VTOP sends back with the new request in it.
        "/vtop/hostel/saveGeneralOutingForm" => (
            ok,
            general_page(&format!("{}{}", general_row("L7"), general_row("L1"))),
        ),
        "/vtop/hostel/StudentWeekendOuting" => (ok, weekend_page(WEEKEND_ROW)),
        "/vtop/hostel/deleteBookingInfo" => (ok, weekend_page("")),
        "/vtop/hostel/deleteGeneralOutingInfo" => ("503 Service Unavailable", String::new()),
        p if p.starts_with("/vtop/hostel/downloadOutingForm/W9") => (ok, "%PDF-1.4 pass".into()),
        _ => (ok, "<html>login</html>".into()),
    }
}

async fn core_on(mock: &MockVtop, name: &str) -> (Core, PathBuf) {
    let base = mock.start().await;
    let dir = std::env::temp_dir().join(format!("vtop-ffi-outing-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let client = VtopClient::builder()
        .config(VtopConfig {
            base_url: base,
            ..VtopConfig::default()
        })
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    let session = Session {
        client,
        cookies: crate::cookies::SessionCookies::new(),
        account: Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            rejected: false,
            registration_number: None,
        },
        logins: 0,
        otp: None,
        server_otp: false,
    };
    let core = Core {
        dir: dir.clone(),
        slot: std::sync::Mutex::new(Some(Arc::new(RwLock::new(session)))),
    };
    (core, dir)
}

#[tokio::test]
async fn a_general_application_is_checked_then_sent_once() {
    let mock = MockVtop::default();
    let (core, dir) = core_on(&mock, "general").await;
    let fetched = core.fetch_general_outing("f".into()).await.unwrap();
    assert_eq!(fetched.requests.len(), 1);
    assert_eq!(
        core.cached_general_outing("f".into()).unwrap().requests,
        fetched.requests
    );

    let (today, _) = records::ist_now();
    let form = |place: &str| GeneralOutingForm {
        place: place.into(),
        purpose: "Visit".into(),
        leaving: OutingMoment {
            day: today + 2,
            minute: 360,
        },
        returning: OutingMoment {
            day: today + 2,
            minute: 20 * 60,
        },
    };
    // The saved form's rules refuse it: nothing reaches VTOP.
    let refused = core.apply_general_outing("f".into(), form("  ")).await;
    assert_eq!(
        refused,
        Err(CoreError::Other {
            reason: "Add where you're going".into()
        })
    );
    assert_eq!(mock.hits("/vtop/hostel/saveGeneralOutingForm"), 0);

    let sent = core
        .apply_general_outing("f".into(), form("Guntur"))
        .await
        .unwrap();
    assert!(sent.done, "{}", sent.message);
    assert_eq!(mock.hits("/vtop/hostel/saveGeneralOutingForm"), 1);

    // A cancel VTOP fails is not sent again.
    let failed = core
        .cancel_outing("f".into(), OutingKind::General, "L1".into())
        .await;
    assert!(failed.is_err());
    assert_eq!(mock.hits("/vtop/hostel/deleteGeneralOutingInfo"), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn weekend_cancel_and_pass() {
    let mock = MockVtop::default();
    let (core, dir) = core_on(&mock, "weekend").await;

    // Only a request VTOP lists as cancellable is cancelled.
    let unknown = core
        .cancel_outing("f".into(), OutingKind::Weekend, "W2".into())
        .await;
    assert!(unknown.is_err());
    assert_eq!(mock.hits("/vtop/hostel/deleteBookingInfo"), 0);
    let gone = core
        .cancel_outing("f".into(), OutingKind::Weekend, "W1".into())
        .await
        .unwrap();
    assert!(gone.done, "{}", gone.message);
    assert_eq!(mock.hits("/vtop/hostel/deleteBookingInfo"), 1);
    let bad_id = core
        .cancel_outing("f".into(), OutingKind::Weekend, "../x".into())
        .await;
    assert!(bad_id.is_err());

    // A weekend application for a day the saved form doesn't offer is refused before sending.
    core.fetch_weekend_outing("f".into()).await.unwrap();
    let (today, _) = records::ist_now();
    let refused = core
        .apply_weekend_outing(
            "f".into(),
            WeekendOutingForm {
                place: "Vijayawada".into(),
                purpose: "Movie".into(),
                day: today,
                slot: "9:30 AM- 3:30PM".into(),
                contact: "9876543210".into(),
            },
        )
        .await;
    assert!(matches!(refused, Err(CoreError::Other { .. })));
    assert_eq!(mock.hits("/vtop/hostel/saveOutingForm"), 0);

    // The pass is downloaded once, then read from disk.
    let path = core
        .outing_pass("f".into(), OutingKind::Weekend, "W9".into())
        .await
        .unwrap();
    assert!(path.ends_with("pages/outing_passes/W9.pdf"));
    assert!(fs::read(&path).unwrap().starts_with(b"%PDF"));
    core.outing_pass("f".into(), OutingKind::Weekend, "W9".into())
        .await
        .unwrap();
    assert_eq!(mock.hits("/vtop/hostel/downloadOutingForm/W9"), 1);

    // Signing out drops the cached lists and passes.
    core.sign_out("f".into());
    assert!(!dir.join(PAGES).exists());
    let _ = fs::remove_dir_all(&dir);
}
