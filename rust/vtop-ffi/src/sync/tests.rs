//! Port of Kotlin's `VtopSyncTest`, and runs over a fake source on a manual clock.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::future::BoxFuture;
use smol::future::block_on;

use super::{
    SyncKind, SyncOtp, SyncOutcome, SyncPage, SyncResult, SyncSource, biometric_days, next_after,
};
use crate::events::Events;
use crate::runtime::{ManualClock, Runtime};
use crate::settings::{SettingChange, TunableKey};
use crate::sync_health::{PauseReason, SyncHealth};
use crate::{Core, CoreError};

#[test]
fn biometric_days_picks_the_newest_incomplete_days_with_today() {
    let set = |days: &[i32]| days.iter().copied().collect::<HashSet<_>>();
    assert_eq!(vec![100, 98, 96], biometric_days(100, &set(&[99, 97]), 3));
    let mut all: Vec<i32> = (87..=99).collect();
    all.push(100);
    assert_eq!(vec![100], biometric_days(100, &set(&all), 3));
    assert_eq!(
        vec![100, 99, 98, 97, 96, 95, 94],
        biometric_days(100, &HashSet::new(), 7)
    );
    let mut most: Vec<i32> = (88..=99).collect();
    most.push(86);
    assert_eq!(vec![100, 87], biometric_days(100, &set(&most), 3));
    assert_eq!(Vec::<i32>::new(), biometric_days(100, &HashSet::new(), 0));
}

#[test]
fn next_after_takes_courses_in_turn() {
    let ids: Vec<String> = ["a", "b", "c"].map(String::from).into();
    assert_eq!(Some("a".into()), next_after(&ids, None));
    assert_eq!(Some("b".into()), next_after(&ids, Some("a")));
    assert_eq!(Some("a".into()), next_after(&ids, Some("c")));
    assert_eq!(Some("a".into()), next_after(&ids, Some("gone")));
    assert_eq!(None, next_after(&[], Some("a")));
}

#[test]
fn page_keys_and_ids_match_kotlin() {
    let history = SyncPage::History("CSE1001/TH".into());
    assert_eq!("history.CSE1001/TH", history.id());
    assert_eq!("attendance_detail/sync/CSE1001/TH", history.key());
    assert_eq!("biometric/sync/20000", SyncPage::Biometric(20000).key());
    assert_eq!("attendance/sync/", SyncPage::Attendance.key());
}

/// What a fake page does when fetched.
#[derive(Clone)]
enum Answer {
    Fail(CoreError),
    /// Never answers.
    Hang,
}

#[derive(Default)]
struct FakeState {
    answers: HashMap<String, Answer>,
    /// Pages that want the OTP until one is accepted.
    otp_pages: HashSet<String>,
    fetched: Vec<String>,
    /// Whether each fetch's flow was unattended when it ran.
    unattended: Vec<bool>,
    cancelled_otp: u32,
}

#[derive(Clone)]
struct Fake {
    state: Arc<Mutex<FakeState>>,
    accepted: Arc<AtomicBool>,
    events: Arc<Events>,
}

impl Fake {
    fn answer(&self, page: &str, answer: Answer) {
        self.state
            .lock()
            .unwrap()
            .answers
            .insert(page.into(), answer);
    }

    fn fetched(&self) -> Vec<String> {
        self.state.lock().unwrap().fetched.clone()
    }
}

impl SyncSource for Fake {
    fn fetch(&self, flow: String, page: SyncPage) -> BoxFuture<'static, Result<(), CoreError>> {
        let id = page.id();
        let mut state = self.state.lock().unwrap();
        state.unattended.push(self.events.is_flow_unattended(&flow));
        if state.otp_pages.contains(&id) && !self.accepted.load(Ordering::SeqCst) {
            return futures::future::ready(Err(CoreError::OtpRequired {
                reason: "otp".into(),
                issued_at: Some(1),
            }))
            .boxed();
        }
        let answer = state.answers.get(&id).cloned();
        state.fetched.push(id);
        match answer {
            None => futures::future::ready(Ok(())).boxed(),
            Some(Answer::Fail(error)) => futures::future::ready(Err(error)).boxed(),
            Some(Answer::Hang) => futures::future::pending().boxed(),
        }
    }

    fn read_semesters(&self, _flow: &str) -> Option<String> {
        Some("AP1".into())
    }

    fn mismatched(&self, _flow: &str) -> Vec<String> {
        vec!["CSE1001".into()]
    }

    fn complete_days(&self, _flow: &str, _from: i32, _to: i32) -> HashSet<i32> {
        HashSet::new()
    }

    fn course_ids(&self, _flow: &str) -> Vec<String> {
        vec!["c1".into(), "c2".into()]
    }

    fn pending_outing(&self, _flow: &str) -> bool {
        false
    }

    fn cancel_otp(&self, _flow: String) -> BoxFuture<'static, ()> {
        self.state.lock().unwrap().cancelled_otp += 1;
        futures::future::ready(()).boxed()
    }
}

/// Counts asks; accepts the code (`gives`) or not.
struct Otp {
    asked: AtomicU32,
    gives: bool,
    accepted: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl SyncOtp for Otp {
    async fn get(&self, _issued_at: Option<u64>, _flow: String) -> bool {
        self.asked.fetch_add(1, Ordering::SeqCst);
        if self.gives {
            self.accepted.store(true, Ordering::SeqCst);
        }
        self.gives
    }
}

struct Lab {
    core: Arc<Core>,
    clock: ManualClock,
    fake: Fake,
}

fn lab(name: &str) -> Lab {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-sync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let clock = ManualClock::default();
    let core = Core::for_test(&dir);
    let runtime = Arc::new(Runtime::new(Arc::new(clock.clone())));
    let (events, schedule) = crate::events::wired_on(runtime, core.settings.clone());
    // One biometric day a run keeps the page lists short.
    core.settings.set(SettingChange::Tunable {
        key: TunableKey::SyncBiometricDays,
        value: Some(1),
    });
    let fake = Fake {
        state: Arc::default(),
        accepted: Arc::default(),
        events: events.clone(),
    };
    Lab {
        core: Arc::new(Core {
            events,
            schedule,
            ..core
        }),
        clock,
        fake,
    }
}

impl Lab {
    fn otp(&self, gives: bool) -> Arc<Otp> {
        Arc::new(Otp {
            asked: AtomicU32::new(0),
            gives,
            accepted: self.fake.accepted.clone(),
        })
    }

    fn sync(&self, kind: SyncKind, timeout: Option<u64>, otp: Arc<Otp>) -> SyncResult {
        block_on(self.core.clone().sync_with(
            "f.sync".into(),
            kind,
            timeout,
            otp,
            Arc::new(self.fake.clone()),
        ))
    }

    /// [`Lab::sync`] on its own thread, to act on it meanwhile.
    fn spawn(
        &self,
        kind: SyncKind,
        timeout: Option<u64>,
        otp: Arc<Otp>,
    ) -> std::thread::JoinHandle<SyncResult> {
        let (core, fake) = (self.core.clone(), self.fake.clone());
        std::thread::spawn(move || {
            block_on(core.sync_with("f.sync".into(), kind, timeout, otp, Arc::new(fake)))
        })
    }

    /// Waits until `key` holds a lane.
    fn wait_running(&self, key: &str) {
        let watch = self.core.schedule.flight.watch_running();
        while !self.core.schedule.flight.is_running(key) {
            let _ = block_on(watch.recv());
        }
    }
}

#[test]
fn background_runs_in_order_with_one_course() {
    let lab = lab("order");
    let result = lab.sync(SyncKind::Background, None, lab.otp(true));
    assert_eq!(SyncOutcome::Done, result.outcome);
    let fetched = lab.fake.fetched();
    assert_eq!(
        &fetched[..3],
        ["semesters", "attendance", "history.CSE1001"]
    );
    assert!(fetched.contains(&"grades".to_owned()), "{fetched:?}");
    assert!(fetched.contains(&"course_files".to_owned()), "{fetched:?}");
    assert!(!fetched.contains(&"profile".to_owned()), "{fetched:?}");
    assert!(
        !fetched.contains(&"general_outing".to_owned()),
        "{fetched:?}"
    );
    assert_eq!(result.fetched as usize, fetched.len());
    assert_eq!(Some("c1".into()), lab.core.settings().sync_course);
    lab.sync(SyncKind::Background, None, lab.otp(true));
    assert_eq!(Some("c2".into()), lab.core.settings().sync_course);
}

#[test]
fn fetch_all_adds_the_rest() {
    let lab = lab("all");
    let result = lab.sync(SyncKind::FetchAll, None, lab.otp(true));
    assert_eq!(SyncOutcome::Done, result.outcome);
    let fetched = lab.fake.fetched();
    for page in ["profile", "course_page", "general_outing", "weekend_outing"] {
        assert!(fetched.contains(&page.to_owned()), "{page}: {fetched:?}");
    }
    assert!(!fetched.contains(&"course_files".to_owned()));
}

#[test]
fn otp_asked_once_for_many_pages() {
    let lab = lab("otp-once");
    {
        let mut state = lab.fake.state.lock().unwrap();
        for page in ["timetable", "marks", "exams", "calendar"] {
            state.otp_pages.insert(page.into());
        }
    }
    let otp = lab.otp(true);
    let result = lab.sync(SyncKind::FetchAll, None, otp.clone());
    assert_eq!(SyncOutcome::Done, result.outcome);
    assert_eq!(1, otp.asked.load(Ordering::SeqCst));
    assert_eq!(0, result.failed);
    for page in ["timetable", "marks", "exams", "calendar"] {
        assert!(lab.fake.fetched().contains(&page.to_owned()), "{page}");
    }
}

#[test]
fn rejected_password_stops_whole_run() {
    let lab = lab("rejected");
    lab.fake
        .answer("attendance", Answer::Fail(CoreError::InvalidCredentials));
    let result = lab.sync(SyncKind::Background, None, lab.otp(true));
    assert_eq!(SyncOutcome::Rejected, result.outcome);
    assert_eq!(vec!["semesters", "attendance"], lab.fake.fetched());
    assert_eq!(
        SyncHealth::Paused {
            reason: PauseReason::PasswordRejected
        },
        lab.core.sync_health()
    );
}

#[test]
fn stop_at_otp_cancels_pending_otp() {
    let lab = lab("otp-stop");
    lab.fake
        .state
        .lock()
        .unwrap()
        .otp_pages
        .insert("semesters".into());
    let otp = lab.otp(false);
    let result = lab.sync(SyncKind::FetchAll, None, otp.clone());
    assert_eq!(SyncOutcome::StoppedAtOtp, result.outcome);
    assert_eq!(1, otp.asked.load(Ordering::SeqCst));
    assert_eq!(1, lab.fake.state.lock().unwrap().cancelled_otp);
    assert!(lab.fake.fetched().is_empty());
}

#[test]
fn page_timeout_counts_failed_and_continues() {
    let lab = lab("timeout");
    lab.fake.answer("marks", Answer::Hang);
    let running = lab.spawn(SyncKind::Background, Some(20_000), lab.otp(true));
    lab.clock.wait_for_sleepers(1);
    lab.clock.advance(20_000);
    let result = running.join().unwrap();
    assert_eq!(SyncOutcome::Done, result.outcome);
    assert_eq!(1, result.failed);
    assert_eq!(lab.fake.fetched().len() as u32 - 1, result.fetched);
    assert!(lab.fake.fetched().contains(&"timetable".to_owned()));
}

#[test]
fn background_never_prompts() {
    let lab = lab("unattended");
    lab.sync(SyncKind::Background, None, lab.otp(true));
    let seen = std::mem::take(&mut lab.fake.state.lock().unwrap().unattended);
    assert!(!seen.is_empty() && seen.iter().all(|u| *u), "{seen:?}");
    assert!(!lab.core.events.is_flow_unattended("f.sync"));
    lab.sync(SyncKind::FetchAll, None, lab.otp(true));
    let seen = std::mem::take(&mut lab.fake.state.lock().unwrap().unattended);
    assert!(seen.iter().all(|u| !*u), "{seen:?}");
}

#[test]
fn sign_out_mid_sync_cancels() {
    let lab = lab("sign-out");
    lab.fake.answer("attendance", Answer::Hang);
    let running = lab.spawn(SyncKind::FetchAll, None, lab.otp(true));
    lab.wait_running("attendance/sync/");
    lab.core.schedule.cancel_all();
    let result = running.join().unwrap();
    assert_eq!(SyncOutcome::Cancelled, result.outcome);
    assert_eq!(1, result.fetched);
}

#[test]
fn background_done_clears_a_pause() {
    let lab = lab("health");
    lab.core.change_sync_health("f", |_| SyncHealth::Paused {
        reason: PauseReason::NoCode,
    });
    lab.sync(SyncKind::Background, None, lab.otp(true));
    assert_eq!(SyncHealth::OK, lab.core.sync_health());
}
