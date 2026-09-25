//! Live VTOP timing bench. Ignored by default; run it by hand:
//!
//! ```sh
//! cd rust
//! VTOP_USERNAME=... VTOP_PASSWORD=... cargo test --release live_bench -- --ignored --nocapture
//! ```
//!
//! If VTOP asks for a security OTP, the bench prompts for it on stdin.
//! Results go to `target/bench/live.json` (override with `VTOP_BENCH_OUT`).
//! Set `VTOP_DUMP_DIR` to also save each raw page. Results and pages are
//! local only: never commit them (they are gitignored; pages hold personal
//! data).
//! Credentials and cookies are never printed or written to the output file.

use std::io::Write as _;
use std::time::{Duration, Instant};

use crate::api::vtop::vtop_client::{VtopClient, VtopError};
use crate::api::vtop_get_client as api;

const ROUNDS: usize = 3;

struct Sample {
    name: String,
    runs: Vec<Duration>,
}

impl Sample {
    fn json(&self) -> String {
        let mut ms: Vec<f64> = self.runs.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
        ms.sort_by(f64::total_cmp);
        let median = ms.get(ms.len() / 2).copied().unwrap_or_default();
        let min = ms.first().copied().unwrap_or_default();
        format!(
            "{{\"name\":\"{}\",\"runs_ms\":{:?},\"min_ms\":{min:.1},\"median_ms\":{median:.1}}}",
            self.name, ms
        )
    }
}

/// Today's UTC date as DD/MM/YYYY (Howard Hinnant's civil_from_days).
fn today_ddmmyyyy() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or_default() as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{day:02}/{month:02}/{year:04}")
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

async fn login_with_otp_prompt(client: &mut VtopClient) -> Result<(), VtopError> {
    match api::vtop_client_login(client).await {
        Err(VtopError::OTPRequired(..)) => {
            print!("VTOP sent a security OTP to your email. Enter it: ");
            std::io::stdout().flush().ok();
            let mut otp = String::new();
            std::io::stdin().read_line(&mut otp).ok();
            api::vtop_client_submit_security_otp(client, otp.trim().to_string()).await
        }
        other => other,
    }
}

macro_rules! time_rounds {
    ($samples:ident, $name:expr, $call:expr) => {{
        let mut runs = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let started = Instant::now();
            let result = $call.await;
            runs.push(started.elapsed());
            if let Err(error) = result {
                println!("{} failed: {error}", $name);
                break;
            }
        }
        $samples.push(Sample {
            name: $name.to_string(),
            runs,
        });
    }};
}

async fn dump_pages(
    client: &VtopClient,
    dir: &std::path::Path,
    semester: &str,
    course: Option<&vtop_core::types::AttendanceRecord>,
    grade_course: Option<&str>,
    date: &str,
) {
    use vtop_core::inputs::{BiometricDate, CourseId, CourseType, SemesterId};
    std::fs::create_dir_all(dir).expect("create dump dir");
    let core = &client.inner;
    let Ok(sem) = SemesterId::parse(semester) else {
        return;
    };
    let mut pages = vec![
        ("semesters", core.semesters_page().await),
        ("timetable", core.timetable_page(&sem).await),
        ("attendance", core.attendance_page(&sem).await),
        ("marks", core.marks_page(&sem).await),
        ("exam_schedule", core.exam_schedule_page(&sem).await),
        ("grade_view", core.grade_view_page(&sem).await),
        ("grade_history", core.grade_history_page().await),
    ];
    if let Ok(date) = BiometricDate::parse(date) {
        pages.push(("biometric", core.biometric_page(&date).await));
    }
    if let Some(course) = course {
        if let (Ok(id), Ok(kind)) = (
            CourseId::parse(&course.course_id),
            CourseType::parse(&course.course_type),
        ) {
            pages.push((
                "full_attendance",
                core.full_attendance_page(&sem, &id, &kind).await,
            ));
        }
    }
    if let Some(Ok(id)) = grade_course.map(CourseId::parse) {
        pages.push((
            "grade_details",
            core.grade_view_details_page(&sem, &id).await,
        ));
    }
    for (name, page) in pages {
        match page {
            Ok(html) => {
                let path = dir.join(format!("{name}.html"));
                std::fs::write(&path, html).expect("write page");
            }
            Err(error) => println!("dump {name} failed: {error}"),
        }
    }
    println!("raw pages saved in {}", dir.display());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "hits live VTOP; needs VTOP_USERNAME and VTOP_PASSWORD"]
async fn live_bench() {
    let (Some(username), Some(password)) = (env("VTOP_USERNAME"), env("VTOP_PASSWORD")) else {
        panic!("set VTOP_USERNAME and VTOP_PASSWORD");
    };
    let mut samples = Vec::new();

    let started = Instant::now();
    let mut client = api::get_vtop_client(username.clone(), password.clone(), None)
        .await
        .expect("client");
    login_with_otp_prompt(&mut client).await.expect("login");
    samples.push(Sample {
        name: "login.cold".into(),
        runs: vec![started.elapsed()],
    });

    let semesters = api::fetch_semesters(&client).await.expect("semesters");
    let semester = env("VTOP_SEMESTER")
        .or_else(|| semesters.semesters.first().map(|s| s.id.clone()))
        .expect("no semester available");
    let attendance = api::fetch_attendance(&client, semester.clone())
        .await
        .expect("attendance");
    let first_course = attendance.records.first().cloned();
    let grades = api::fetch_grade_view(&client, semester.clone()).await.ok();
    let first_grade_course = grades
        .as_ref()
        .and_then(|g| g.courses.first().map(|c| c.course_id.clone()));

    time_rounds!(samples, "fetch.semesters", api::fetch_semesters(&client));
    time_rounds!(
        samples,
        "fetch.attendance",
        api::fetch_attendance(&client, semester.clone())
    );
    time_rounds!(
        samples,
        "fetch.timetable",
        api::fetch_timetable(&client, semester.clone())
    );
    time_rounds!(
        samples,
        "fetch.marks",
        api::fetch_marks(&client, semester.clone())
    );
    time_rounds!(
        samples,
        "fetch.exam_schedule",
        api::fetch_exam_schedule(&client, semester.clone())
    );
    time_rounds!(
        samples,
        "fetch.grade_view",
        api::fetch_grade_view(&client, semester.clone())
    );
    if let Some(course_id) = first_grade_course.clone() {
        time_rounds!(
            samples,
            "fetch.grade_details",
            api::fetch_grade_view_details(&client, semester.clone(), course_id.clone())
        );
    }
    time_rounds!(
        samples,
        "fetch.grade_history",
        api::fetch_grade_history(&client)
    );
    let today = env("VTOP_BIOMETRIC_DATE").unwrap_or_else(today_ddmmyyyy);
    time_rounds!(
        samples,
        "fetch.biometric",
        api::fetch_biometric_history(&client, today.clone())
    );
    if let Some(course) = first_course.clone() {
        time_rounds!(
            samples,
            "fetch.full_attendance",
            api::fetch_full_attendance(
                &client,
                semester.clone(),
                course.course_id.clone(),
                course.course_type.clone()
            )
        );
    }

    if let Some(dir) = env("VTOP_DUMP_DIR") {
        dump_pages(
            &client,
            std::path::Path::new(&dir),
            &semester,
            first_course.as_ref(),
            first_grade_course.as_deref(),
            &today,
        )
        .await;
    }

    // What a pull-to-refresh of the main screens costs: the semester-scoped
    // fetches back to back on one client.
    let started = Instant::now();
    let _ = api::fetch_attendance(&client, semester.clone()).await;
    let _ = api::fetch_timetable(&client, semester.clone()).await;
    let _ = api::fetch_marks(&client, semester.clone()).await;
    let _ = api::fetch_exam_schedule(&client, semester.clone()).await;
    let _ = api::fetch_grade_view(&client, semester.clone()).await;
    samples.push(Sample {
        name: "refresh.five_endpoints".into(),
        runs: vec![started.elapsed()],
    });

    // The same five fetches at once. Fetches borrow the client immutably now,
    // so they no longer queue behind each other.
    let started = Instant::now();
    let _ = tokio::join!(
        api::fetch_attendance(&client, semester.clone()),
        api::fetch_timetable(&client, semester.clone()),
        api::fetch_marks(&client, semester.clone()),
        api::fetch_exam_schedule(&client, semester.clone()),
        api::fetch_grade_view(&client, semester.clone()),
    );
    samples.push(Sample {
        name: "refresh.five_endpoints_concurrent".into(),
        runs: vec![started.elapsed()],
    });

    // Cold start from a saved session: new client, restored cookie, first fetch.
    let snapshot = api::export_session_snapshot(&client, 0);
    let started = Instant::now();
    let restored = api::get_vtop_client(username, password, Some(snapshot))
        .await
        .expect("restored client");
    let restored_result = api::fetch_attendance(&restored, semester.clone()).await;
    samples.push(Sample {
        name: "restore.first_fetch".into(),
        runs: vec![started.elapsed()],
    });
    if let Err(error) = restored_result {
        println!("restore.first_fetch failed: {error}");
    }

    let body = format!(
        "[\n  {}\n]\n",
        samples
            .iter()
            .map(Sample::json)
            .collect::<Vec<_>>()
            .join(",\n  ")
    );
    println!("{body}");
    let out = env("VTOP_BENCH_OUT").unwrap_or_else(|| "target/bench/live.json".to_string());
    {
        if let Some(parent) = std::path::Path::new(&out).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&out, &body).expect("write bench output");
        println!("wrote {out}");
    }
}
