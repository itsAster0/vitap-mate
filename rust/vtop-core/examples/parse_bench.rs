//! Times each parser on the test fixtures (or on a directory of raw pages).
//!
//! ```sh
//! cargo run --release -p vtop-core --example parse_bench [dir-with-html]
//! ```

use std::path::PathBuf;
use std::time::Instant;

use vtop_core::parser::{
    attendance, biometric, exam_schedule, grade_history, grades, marks, timetable,
};

const ITERATIONS: u32 = 3000;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let read = |name: &str| std::fs::read_to_string(dir.join(format!("{name}.html"))).ok();

    type Parse = fn(&str);
    let parsers: [(&str, Parse); 10] = [
        ("attendance", |html| {
            std::hint::black_box(attendance::parse_attendance(html, "S"));
        }),
        ("full_attendance", |html| {
            std::hint::black_box(attendance::parse_full_attendance(html, "S", "C", "T"));
        }),
        ("semesters", |html| {
            std::hint::black_box(timetable::parse_semid_timetable(html));
        }),
        ("timetable", |html| {
            std::hint::black_box(timetable::parse_timetable(html, "S"));
        }),
        ("marks", |html| {
            std::hint::black_box(marks::parse_marks(html, "S"));
        }),
        ("exam_schedule", |html| {
            std::hint::black_box(exam_schedule::parse_schedule(html, "S"));
        }),
        ("grade_view", |html| {
            std::hint::black_box(grades::parse_grade_view(html, "S"));
        }),
        ("grade_details", |html| {
            std::hint::black_box(grades::parse_grade_view_details(html, "S", "C"));
        }),
        ("grade_history", |html| {
            std::hint::black_box(grade_history::parse_grade_history(html));
        }),
        ("biometric", |html| {
            std::hint::black_box(biometric::parse_biometric(html, "01/01/2026"));
        }),
    ];

    println!("{:<16} {:>8} {:>12}", "page", "bytes", "us/parse");
    for (name, parse) in parsers {
        let Some(html) = read(name) else {
            println!("{name:<16} {:>8} {:>12}", "-", "missing");
            continue;
        };
        for _ in 0..10 {
            parse(&html);
        }
        let started = Instant::now();
        for _ in 0..ITERATIONS {
            parse(&html);
        }
        let micros = started.elapsed().as_secs_f64() * 1e6 / f64::from(ITERATIONS);
        println!("{name:<16} {:>8} {micros:>12.1}", html.len());
    }
}
