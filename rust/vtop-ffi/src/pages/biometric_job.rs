//! Wrapped's campus-times fetch job: picks a semester's missing biometric days, fetches them a
//! few VTOP requests at a time within a time budget, and reports whether the job finished, needs
//! another run, or failed. WorkManager scheduling and notifications stay in Kotlin
//! (`BiometricFetch.kt`).

use std::collections::{HashMap, HashSet};
use std::sync::PoisonError;

use futures::future::join_all;
use tracing::{info, warn};
use vtop_core::types::AcademicCalendarData;

use crate::pages::biometric::is_complete;
use crate::pages::calendar::SemesterCalendar;
use crate::problem::Problem;
use crate::settings::TunableKey;
use crate::{CALENDAR, Core, CoreError, PAGES, traced};

/// Maximum inclusive days per cached biometric range window (matches `biometric::MAX_RANGE_DAYS`).
#[cfg(test)]
const MAX_RANGE_DAYS: i32 = 14;
/// How many consecutive stalled attempts (fetching 0 days) before giving up.
const MAX_STALLS: u32 = 5;

/// Outcome of one [`Core::biometric_step`] slice.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum BiometricStep {
    /// Every missing day for the semester is now cached or skipped.
    Done { fetched: u32 },
    /// More days remain: the budget ran out, or VTOP dropped after some days came in.
    More { fetched: u32 },
    /// Stopped on an error (`fetched == 0` on unreachable lets WorkManager retry; sign-in errors stop).
    Failed { problem: Problem, fetched: u32 },
}

/// What a worker run that ended on Offline / VTOP unavailable does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum BiometricStallNext {
    Rerun,
    Retry,
    GiveUp,
}

/// Days that failed for non-network/non-sign-in reasons in the current job, keyed by
/// `(student, semester)` so switching accounts never leaks skips.
#[derive(Default)]
pub(crate) struct Skipped(HashMap<(String, String), HashSet<i32>>);

impl Skipped {
    fn get(&self, key: &(String, String)) -> HashSet<i32> {
        self.0.get(key).cloned().unwrap_or_default()
    }

    fn insert(&mut self, key: (String, String), day: i32) {
        self.0.entry(key).or_default().insert(day);
    }

    fn clear(&mut self, key: &(String, String)) {
        self.0.remove(key);
    }
}

/// Windows of at most [`MAX_RANGE_DAYS`] inclusive days covering `days` (oldest first).
#[cfg(test)]
fn windows(days: &[i32]) -> Vec<(i32, i32)> {
    let mut sorted = days.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut out: Vec<(i32, i32)> = Vec::new();
    for day in sorted {
        let covered = out
            .last()
            .is_some_and(|&(start, end)| day >= start && day <= end);
        if !covered {
            out.push((day, day + MAX_RANGE_DAYS - 1));
        }
    }
    out
}

/// Progress (`fetched > 0`) resets stalls to a fresh rerun; otherwise retry up to [`MAX_STALLS`].
#[uniffi::export]
pub fn biometric_after_stall(fetched: u32, attempt: u32) -> BiometricStallNext {
    if fetched > 0 {
        BiometricStallNext::Rerun
    } else if attempt + 1 >= MAX_STALLS {
        BiometricStallNext::GiveUp
    } else {
        BiometricStallNext::Retry
    }
}

fn is_sign_in(error: &CoreError) -> bool {
    matches!(
        error,
        CoreError::OtpRequired { .. } | CoreError::InvalidCredentials | CoreError::NotSignedIn
    )
}

fn is_unreachable(error: &CoreError) -> bool {
    matches!(
        error,
        CoreError::Offline | CoreError::VtopUnavailable { .. }
    )
}

fn today_ist_day(now_ms: i64) -> i32 {
    ((now_ms.div_euclid(1000) + 19_800).div_euclid(86_400)) as i32
}

enum DayOutcome {
    Done,
    SignIn(CoreError),
    Unreachable(CoreError),
    Skipped(i32),
}

#[uniffi::export]
impl Core {
    /// Fetches missing biometric days for `semester` in parallel chunks within `budget_ms`.
    /// Returns [`BiometricStep::More`] when the budget runs out or VTOP drops after partial
    /// progress, [`BiometricStep::Failed`] on sign-in errors or when VTOP is unreachable before
    /// any day succeeded, and [`BiometricStep::Done`] once no missing days remain.
    pub async fn biometric_step(
        &self,
        flow: String,
        semester: String,
        budget_ms: u64,
    ) -> BiometricStep {
        let outcome = traced("biometric_step", &flow, async {
            Ok(self.run_biometric_step(&flow, &semester, budget_ms).await)
        })
        .await;
        outcome.unwrap_or_else(|error| BiometricStep::Failed {
            problem: Problem::of(&error, &flow, false),
            fetched: 0,
        })
    }
}

impl Core {
    fn semester_missing_days(
        &self,
        semester: &str,
        today_ist: i32,
        skipped: &HashSet<i32>,
    ) -> Vec<i32> {
        let path = format!("{PAGES}/{semester}/{CALENDAR}");
        let class_days = self
            .read_kept::<AcademicCalendarData>(&path)
            .map_or_default(|data| SemesterCalendar::of(&data).class_days());
        class_days
            .into_iter()
            .filter(|&day| day <= today_ist && !skipped.contains(&day))
            .filter(|&day| {
                self.cached_day(day)
                    .is_none_or(|data| !is_complete(day, &data))
            })
            .collect()
    }

    async fn run_biometric_step(
        &self,
        flow: &str,
        semester: &str,
        budget_ms: u64,
    ) -> BiometricStep {
        let clock = self.shared.runtime.clock().clone();
        let started_ms = clock.now_ms();
        let today = today_ist_day(started_ms);
        let student = self.student().unwrap_or_default();
        let skip_key = (student, semester.to_owned());

        let skipped = self
            .shared
            .biometric_skipped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&skip_key);
        let missing = self.semester_missing_days(semester, today, &skipped);
        if missing.is_empty() {
            self.shared
                .biometric_skipped
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear(&skip_key);
            return BiometricStep::Done { fetched: 0 };
        }

        if let Err(error) = self.ensure_fresh_session(flow.to_owned(), 900).await {
            return BiometricStep::Failed {
                problem: Problem::of(&error, flow, false),
                fetched: 0,
            };
        }

        let settings = self.settings();
        let batch_size =
            usize::try_from(settings.tunable(TunableKey::CampusFetchAtOnce).max(1)).unwrap_or(1);
        let gap_ms =
            u64::try_from(settings.tunable(TunableKey::CampusFetchGap).max(0)).unwrap_or(0) * 1000;

        let mut fetched: u32 = 0;
        let chunks: Vec<&[i32]> = missing.chunks(batch_size).collect();
        let last_idx = chunks.len().saturating_sub(1);

        for (idx, chunk) in chunks.into_iter().enumerate() {
            if fetched > 0 && clock.now_ms().saturating_sub(started_ms) >= budget_ms as i64 {
                info!(fetched, "biometric step budget reached");
                return BiometricStep::More { fetched };
            }

            let outcomes = join_all(chunk.iter().map(|&day| async move {
                match self.fetch_day(day).await {
                    Ok(_) => DayOutcome::Done,
                    Err(error) if is_sign_in(&error) => DayOutcome::SignIn(error),
                    Err(error) if is_unreachable(&error) => DayOutcome::Unreachable(error),
                    Err(error) => {
                        warn!(day, error = error.kind(), "campus day skipped");
                        DayOutcome::Skipped(day)
                    }
                }
            }))
            .await;

            let mut sign_in_err: Option<CoreError> = None;
            let mut unreachable_err: Option<CoreError> = None;
            for outcome in outcomes {
                match outcome {
                    DayOutcome::Done => fetched = fetched.saturating_add(1),
                    DayOutcome::Skipped(day) => {
                        self.shared
                            .biometric_skipped
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .insert(skip_key.clone(), day);
                    }
                    DayOutcome::SignIn(error) => {
                        if sign_in_err.is_none() {
                            sign_in_err = Some(error);
                        }
                    }
                    DayOutcome::Unreachable(error) => {
                        if unreachable_err.is_none() {
                            unreachable_err = Some(error);
                        }
                    }
                }
            }

            if let Some(error) = sign_in_err {
                return BiometricStep::Failed {
                    problem: Problem::of(&error, flow, false),
                    fetched,
                };
            }
            if let Some(error) = unreachable_err {
                if fetched > 0 {
                    info!(fetched, "biometric step partial drop");
                    return BiometricStep::More { fetched };
                }
                return BiometricStep::Failed {
                    problem: Problem::of(&error, flow, false),
                    fetched: 0,
                };
            }

            if idx < last_idx {
                if clock.now_ms().saturating_sub(started_ms) >= budget_ms as i64 {
                    info!(fetched, "biometric step budget reached");
                    return BiometricStep::More { fetched };
                }
                if gap_ms > 0 {
                    clock.sleep(gap_ms).await;
                }
                if clock.now_ms().saturating_sub(started_ms) >= budget_ms as i64 {
                    info!(fetched, "biometric step budget reached");
                    return BiometricStep::More { fetched };
                }
            }
        }

        self.shared
            .biometric_skipped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear(&skip_key);
        info!(fetched, "biometric step done");
        BiometricStep::Done { fetched }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::sync::{Arc, Mutex};

    use vtop_core::SessionState;
    use vtop_core::types::{AcademicCalendarData, BiometricData, CalendarEntry};

    use super::*;
    use crate::problem::ProblemKind;
    use crate::runtime::ManualClock;
    use crate::settings::SettingChange;
    use crate::transport::{Foreign, HttpRequest, HttpResponse, HttpTransport, TransportError};
    use crate::{Account, Session, Shared};

    const SEM: &str = "AP2024253";
    /// 2026-07-10.
    const DAY0: i32 = 20_644;

    struct ScriptedHttp {
        replies: Mutex<VecDeque<Result<HttpResponse, TransportError>>>,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl HttpTransport for ScriptedHttp {
        async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
            self.calls.lock().unwrap().push(request.url.clone());
            self.replies.lock().unwrap().pop_front().unwrap_or_else(|| {
                Err(TransportError::Connect {
                    reason: "offline".into(),
                })
            })
        }
    }

    fn bio_html() -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            url: "https://vtop.vitap.ac.in/vtop/processViewBiometricDetails".into(),
            body: br#"<table><tr><th>S.No</th><th>Date</th><th>In Time</th><th>Location</th></tr>
                <tr><td>1</td><td>10/07/2026</td><td>09:00:00</td><td>AB1-FACE-IN-1</td></tr></table>"#
                .to_vec(),
            saved_size: None,
        }
    }

    fn setup(
        name: &str,
        class_days_dates: &[u32],
        replies: Vec<Result<HttpResponse, TransportError>>,
    ) -> (Core, ManualClock, Arc<ScriptedHttp>) {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-biojob-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(PAGES).join(SEM)).unwrap();
        fs::create_dir_all(dir.join(PAGES).join(vtop_core::spec::BIOMETRIC.file)).unwrap();

        let clock = ManualClock::default();
        // Set clock to 2026-07-20 12:00 IST (well after July 10..15).
        let now_s = i64::from(DAY0 + 10) * 86_400;
        clock.advance((now_s * 1000) as u64);

        let http = Arc::new(ScriptedHttp {
            replies: Mutex::new(replies.into()),
            calls: Mutex::new(Vec::new()),
        });
        let shared = Shared {
            http: Some(Arc::new(Foreign(http.clone()))),
            ..Shared::default()
        };
        let core = Core::for_test_with_clock(&dir, shared, Arc::new(clock.clone()));
        // Install a signed-in session logged in just now so ensure_fresh_session does not re-login.
        let account = Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            registration_number: Some("22BCE0001".into()),
        };
        let session = Session::new(account, &core.shared).unwrap();
        session
            .client
            .resume_session(&SessionState {
                cookies: "JSESSIONID=live".into(),
                csrf_token: Some("csrf".into()),
                registration_number: Some("22BCE0001".into()),
                otp_issued_at: None,
                logged_in_at: Some(crate::unix_now() as u64),
            })
            .unwrap();
        core.install(session);

        // Set batch size = 1 and gap = 0 for deterministic per-day tests.
        core.settings.set(SettingChange::Tunable {
            key: TunableKey::CampusFetchAtOnce,
            value: Some(1),
        });
        core.settings.set(SettingChange::Tunable {
            key: TunableKey::CampusFetchGap,
            value: Some(0),
        });

        let calendar = AcademicCalendarData {
            semester_id: SEM.into(),
            entries: class_days_dates
                .iter()
                .map(|&d| CalendarEntry {
                    date: format!("2026-07-{d:02}"),
                    kind: "Instructional Day".into(),
                    group: "General (Semester)".into(),
                    note: String::new(),
                })
                .collect(),
            update_time: 1_700_000_000,
        };
        core.write(&format!("{PAGES}/{SEM}/{CALENDAR}"), &calendar);
        (core, clock, http)
    }

    #[test]
    fn windows_cover_the_days_in_fourteen_day_ranges() {
        assert_eq!(windows(&[]), Vec::<(i32, i32)>::new());
        assert_eq!(windows(&[100]), vec![(100, 113)]);
        assert_eq!(
            windows(&[113, 100, 114, 127, 100]),
            vec![(100, 113), (114, 127)]
        );
        assert_eq!(windows(&[100, 105, 200]), vec![(100, 113), (200, 213)]);
        let many: Vec<i32> = (0..120).collect();
        assert_eq!(windows(&many).len(), 9);
    }

    #[test]
    fn progress_resets_the_stalls_and_the_fifth_stall_gives_up() {
        assert_eq!(biometric_after_stall(3, 4), BiometricStallNext::Rerun);
        assert_eq!(biometric_after_stall(0, 0), BiometricStallNext::Retry);
        assert_eq!(biometric_after_stall(0, 3), BiometricStallNext::Retry);
        assert_eq!(biometric_after_stall(0, 4), BiometricStallNext::GiveUp);
    }

    #[tokio::test]
    async fn skips_complete_days() {
        // July 10 (DAY0) and July 11 (DAY0 + 1). Mark July 10 as complete in cache.
        let (core, _clock, http) = setup("skips-complete", &[10, 11], vec![Ok(bio_html())]);
        let completed_at = (i64::from(DAY0 + 2) * 86_400) as u64;
        core.write(
            &format!("{PAGES}/{}/{DAY0}.json", vtop_core::spec::BIOMETRIC.file),
            &BiometricData {
                records: Vec::new(),
                requested_date: "10/07/2026".into(),
                update_time: completed_at,
            },
        );

        let step = core.biometric_step("f".into(), SEM.into(), 60_000).await;
        assert_eq!(step, BiometricStep::Done { fetched: 1 });
        assert_eq!(http.calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn more_after_partial_drop() {
        // July 10 succeeds, July 11 drops with Connect error -> More { fetched: 1 }.
        let (core, _clock, _http) = setup(
            "partial-drop",
            &[10, 11],
            vec![
                Ok(bio_html()),
                Err(TransportError::Connect {
                    reason: "reset".into(),
                }),
            ],
        );
        let step = core.biometric_step("f".into(), SEM.into(), 60_000).await;
        assert_eq!(step, BiometricStep::More { fetched: 1 });
    }

    #[tokio::test]
    async fn failed_only_when_nothing_fetched() {
        // July 10 drops immediately -> Failed { fetched: 0, kind: Offline }.
        let (core, _clock, _http) = setup(
            "nothing-fetched",
            &[10, 11],
            vec![Err(TransportError::Connect {
                reason: "offline".into(),
            })],
        );
        let step = core.biometric_step("f".into(), SEM.into(), 60_000).await;
        match step {
            BiometricStep::Failed { problem, fetched } => {
                assert_eq!(fetched, 0);
                assert_eq!(problem.kind, ProblemKind::Offline);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }
}
