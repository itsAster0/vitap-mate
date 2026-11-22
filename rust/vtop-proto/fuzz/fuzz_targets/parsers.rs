//! Every VTOP parser on arbitrary input: the first byte picks the parser (see [`PARSERS`]),
//! the rest is its input, as lossy UTF-8 for the HTML ones. The property is no panic: a
//! parser answers garbage with empty or default data, never by taking the app down.
//!
//! `seed.sh` builds the starting corpus from `tests/fixtures` and `tests/captchas`, each
//! file prefixed with its parser's byte.
#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;
use vtop_proto::parser::{
    attendance, biometric, calendar, course_page, exam_schedule, grade_history, grades, hostel,
    marks, mentor, page, profile, timetable,
};
use vtop_proto::{captcha, gmail};

const SEM: &str = "AP2026271";

/// In selector order: byte `n` (mod the count) runs `PARSERS[n]`. Keep `seed.sh` in step.
const PARSERS: &[fn(&[u8])] = &[
    // 0..=12: the pages behind `spec::each`, then the calendar's two.
    |b| keep(timetable::parse_semid_timetable(&text(b))),
    |b| keep(attendance::parse_attendance(&text(b), SEM)),
    |b| {
        keep(attendance::parse_full_attendance(
            &text(b),
            SEM,
            "AP2026271000123",
            "ETH",
        ))
    },
    |b| keep(timetable::parse_timetable(&text(b), SEM)),
    |b| keep(marks::parse_marks(&text(b), SEM)),
    |b| keep(exam_schedule::parse_schedule(&text(b), SEM)),
    |b| keep(grades::parse_grade_view(&text(b), SEM)),
    |b| keep(calendar::parse_calendar_months(&text(b))),
    |b| keep(calendar::parse_calendar_month(&text(b), "01-OCT-2026")),
    |b| {
        keep(grades::parse_grade_view_details(
            &text(b),
            SEM,
            "AP2026271000123",
        ))
    },
    |b| keep(grade_history::parse_grade_history(&text(b))),
    |b| keep(biometric::parse_biometric(&text(b), "18/07/2026")),
    |b| keep(profile::parse_profile(&text(b))),
    // 13: the mentor's details and messages, split at the first NUL.
    |b| {
        let (details, messages) = b.split_at(b.iter().position(|&c| c == 0).unwrap_or(b.len()));
        let messages = messages.get(1..).unwrap_or_default();
        keep(mentor::parse_mentor(&text(details), &text(messages)));
    },
    // 14..=18: outings and course pages.
    |b| keep(hostel::parse_general_outing(&text(b))),
    |b| keep(hostel::parse_weekend_outing(&text(b))),
    |b| keep(course_page::parse_courses(&text(b), SEM)),
    |b| keep(course_page::parse_classes(&text(b), SEM, "CSE1001")),
    |b| keep(course_page::parse_detail(&text(b), SEM, "70123")),
    // 19..=20: the outing write answers.
    |b| keep(hostel::parse_apply_response(&text(b), 2)),
    |b| keep(hostel::parse_cancel_response(&text(b), "W123")),
    // 21: the sign-in page readers.
    |b| {
        let page = text(b);
        keep(page::extract_javascript_var(&page, "csrfValue"));
        keep(page::extract_csrf_token(&page));
        keep(page::extract_registration_number(&page));
        keep(page::extract_session_fields(&page));
        keep(page::extract_captcha_data(&page));
        keep(page::login_alert_message(&page));
        keep(page::login_page_error(&page));
        keep(page::is_invalid_credentials_response(&page));
        keep(page::is_security_otp_required_response(&page));
    },
    // 22: a Gmail API message (JSON), its body parts in URL-safe base64.
    |b| {
        if let Ok(message) = serde_json::from_slice::<gmail::Message>(b) {
            keep(gmail::otp(&message, 0));
        }
    },
    // 23: a CAPTCHA `data:` URL as text (the base64 decoder, then the JPEG one).
    |b| keep(captcha::solve_data_url(&text(b))),
    // 24: raw JPEG bytes, wrapped in a `data:` URL, for the JPEG decoder and the model.
    |b| {
        keep(captcha::solve_data_url(&format!(
            "data:image/jpeg;base64,{}",
            base64(b)
        )))
    },
];

fuzz_target!(|data: &[u8]| {
    if let Some((&selector, rest)) = data.split_first() {
        let parse = PARSERS[usize::from(selector) % PARSERS.len()];
        parse(rest);
    }
});

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize])
            } else {
                '='
            });
        }
    }
    out
}

/// Keeps a parser's answer from being optimised away.
fn keep<T>(value: T) {
    black_box(value);
}
