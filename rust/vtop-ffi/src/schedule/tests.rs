//! Ports of Kotlin's `FetchQueueTest`, `SingleFlightTest` and `ActivityTest`. Times come from
//! the test's clock; nothing sleeps.

use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use smol::future::{block_on, poll_once};

use super::activity::{Activity, ActivityEntry, ActivityRun, ActivityState, KEEP};
use super::flight::{Flight, FlightError, Run, RunOpts};
use super::queue::{FetchQueue, QueueConfig, QueueEffect, QueueEvent, Slot, Ticket};
use crate::CoreError;
use crate::problem::{Problem, ProblemKind};
use crate::runtime::{Clock, ManualClock, Runtime};

// FetchQueue: its decisions, event by event.

const CONFIG: QueueConfig = QueueConfig {
    fetch_lanes: 1,
    queue_drop_ms: 10_000,
    slow_fetch_ms: 5_000,
};

#[derive(Default)]
struct Q(FetchQueue);

impl Q {
    fn on_with(&mut self, event: QueueEvent, now: i64, config: QueueConfig) -> Vec<QueueEffect> {
        let step = std::mem::take(&mut self.0).next(event, now, config);
        self.0 = step.queue;
        step.effects
    }

    fn on(&mut self, event: QueueEvent, now: i64) -> Vec<QueueEffect> {
        self.on_with(event, now, CONFIG)
    }

    fn start_at(
        &mut self,
        key: &str,
        at: i64,
        user: bool,
        has_cache: bool,
    ) -> (u64, Vec<QueueEffect>) {
        let ticket = Ticket {
            id: self.0.next_id,
            key: key.into(),
            flow: format!("f.{key}"),
            user,
            has_cache,
            background: false,
            at,
            logged: false,
        };
        let id = ticket.id;
        (id, self.on(QueueEvent::Start(ticket), at))
    }

    fn start(&mut self, key: &str) -> (u64, Vec<QueueEffect>) {
        self.start_at(key, 0, false, true)
    }
}

fn finished(id: u64, key: &str, succeeded: bool) -> QueueEvent {
    QueueEvent::Finished {
        ticket_id: id,
        key: key.into(),
        flow: "f".into(),
        succeeded,
    }
}

fn shown(page: &str, on: bool) -> QueueEvent {
    QueueEvent::Shown {
        page: page.into(),
        on,
    }
}

fn fetched(ms: i64, udp: bool) -> QueueEvent {
    QueueEvent::Fetched {
        flow: "f".into(),
        ms,
        udp,
    }
}

/// `"<msg> <page>"` per log effect (`null` without a page, as Kotlin printed it).
fn logs(effects: &[QueueEffect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            QueueEffect::Log { msg, fields, .. } => {
                let page = fields
                    .iter()
                    .find(|(key, _)| *key == "page")
                    .map_or("null", |(_, value)| value.as_str());
                Some(format!("{msg} {page}"))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_free_lane_starts_the_run_at_once_and_the_next_waits_with_a_queued_line() {
    let mut q = Q::default();
    let (a, first) = q.start("attendance//AP1");
    assert_eq!(first, vec![QueueEffect::Go(a)]);
    let (b, second) = q.start("marks//AP1");
    assert!(!second.iter().any(|e| matches!(e, QueueEffect::Go(_))));
    assert_eq!(logs(&second), ["queued marks"]);
    assert_eq!(q.0.running_pages(), ["attendance"]);
    assert_eq!(q.0.active("marks//AP1").map(|t| t.id), Some(b));
    // The lane frees: marks starts, and says it waited.
    let done = q.on(finished(a, "attendance//AP1", true), 300);
    assert!(done.contains(&QueueEffect::Go(b)));
    assert_eq!(logs(&done), ["started marks"]);
    assert_eq!(q.0.running_pages(), ["marks"]);
}

#[test]
fn success_is_remembered_across_the_next_run_and_a_failure_keeps_the_old_one() {
    let mut q = Q::default();
    let (a, _) = q.start("timetable//AP1");
    q.on(finished(a, "timetable//AP1", true), 1_000);
    assert!(matches!(
        q.0.slots.get("timetable//AP1"),
        Some(Slot::Cooldown { .. })
    ));
    assert!(q.0.succeeded_within("timetable//AP1", 60_000, 2_000));
    // Fetched again and failed: the success before it still counts.
    let (b, _) = q.start_at("timetable//AP1", 2_000, false, true);
    assert!(q.0.succeeded_within("timetable//AP1", 60_000, 2_000));
    q.on(finished(b, "timetable//AP1", false), 3_000);
    assert_eq!(
        q.0.slots.get("timetable//AP1"),
        Some(&Slot::Cooldown {
            succeeded_at: 1_000
        })
    );
    // A failure with no success before leaves nothing.
    let (c, _) = q.start("marks//AP1");
    q.on(finished(c, "marks//AP1", false), 0);
    assert_eq!(q.0.slots.get("marks//AP1"), None);
    assert!(!q.0.succeeded_within("marks//AP1", 60_000, 0));
}

#[test]
fn a_late_end_of_an_older_run_leaves_the_new_one_alone() {
    let mut q = Q::default();
    let (a, _) = q.start("marks//AP1");
    q.on(QueueEvent::CancelAll, 0);
    let (b, _) = q.start("marks//AP1");
    q.on(finished(a, "marks//AP1", false), 0);
    assert_eq!(q.0.active("marks//AP1").map(|t| t.id), Some(b));
}

#[test]
fn leaving_a_page_drops_its_stale_automatic_run_at_once() {
    let mut q = Q::default();
    q.start("profile//AP1");
    q.on(shown("marks", true), 0);
    let (marks, _) = q.start("marks//AP1");
    // Shown: never dropped, however long it waits.
    assert!(
        !q.on(fetched(100, false), 20_000)
            .iter()
            .any(|e| matches!(e, QueueEffect::Drop(_)))
    );
    // The student leaves marks long after it queued: dropped now, not when a lane frees.
    let left = q.on(shown("marks", false), 20_000);
    assert!(left.contains(&QueueEffect::Drop(marks)));
    assert_eq!(logs(&left), ["dropped marks"]);
    assert!(q.0.active("marks//AP1").is_none());
}

#[test]
fn asking_for_a_waiting_run_moves_it_ahead_and_keeps_it_from_being_dropped() {
    let mut q = Q::default();
    let (blocker, _) = q.start("profile//AP1");
    let (exams, _) = q.start("exams//AP1");
    let (marks, _) = q.start("marks//AP1");
    q.on(
        QueueEvent::Ask {
            key: "marks//AP1".into(),
        },
        0,
    );
    assert!(q.0.active("marks//AP1").unwrap().user);
    // Past the drop time: exams goes, marks (asked) stays and gets the lane first.
    let freed = q.on(finished(blocker, "profile//AP1", true), 20_000);
    assert!(freed.contains(&QueueEffect::Drop(exams)));
    assert!(freed.contains(&QueueEffect::Go(marks)));
}

#[test]
fn slow_fetches_narrow_to_one_lane_and_say_when_the_count_changes() {
    let two = QueueConfig {
        fetch_lanes: 2,
        ..CONFIG
    };
    let mut q = Q::default();
    q.on_with(fetched(100, false), 0, two);
    assert_eq!(q.0.lanes, 2);
    q.on_with(fetched(9_000, false), 0, two);
    let slow = q.on_with(fetched(9_000, false), 0, two);
    assert_eq!(q.0.lanes, 1);
    assert_eq!(logs(&slow), ["lanes null"]);
    assert!(q.0.slow(two));
    // Over UDP: one lane however fast.
    for _ in 0..3 {
        q.on_with(fetched(100, false), 0, two);
    }
    assert!(!q.0.slow(two));
    q.on_with(fetched(100, true), 0, two);
    assert!(q.0.slow(two));
}

#[test]
fn sign_out_cancels_every_run_and_forgets_successes() {
    let mut q = Q::default();
    let (a, _) = q.start("profile//AP1");
    q.on(finished(a, "profile//AP1", true), 0);
    let (b, _) = q.start("marks//AP1");
    let (c, _) = q.start("exams//AP1");
    assert_eq!(
        q.on(QueueEvent::CancelAll, 0),
        vec![QueueEffect::Cancel(b), QueueEffect::Cancel(c)]
    );
    assert!(q.0.slots.is_empty());
    assert!(q.0.running_pages().is_empty());
    // Ids keep counting, so a late end of a cancelled run can't match a new one.
    assert_eq!(q.0.next_id, c + 1);
}

// Flight: runs on the executor; the clock is the test's and log lines are collected.

/// Kotlin's tunable defaults: 60 s queue drop, 4 s slow fetch; lanes set per test.
struct Lab {
    flight: Flight,
    clock: ManualClock,
    lanes: Arc<AtomicI32>,
    logged: Arc<Mutex<Vec<String>>>,
}

fn lab() -> Lab {
    let clock = ManualClock::default();
    clock.advance(1_000);
    let lanes = Arc::new(AtomicI32::new(2));
    let logged = Arc::new(Mutex::new(Vec::new()));
    let flight = Flight::new(
        Arc::new(Runtime::new(Arc::new(clock.clone()))),
        Arc::new({
            let lanes = lanes.clone();
            move || QueueConfig {
                fetch_lanes: lanes.load(Ordering::SeqCst),
                queue_drop_ms: 60_000,
                slow_fetch_ms: 4_000,
            }
        }),
        Arc::new({
            let logged = logged.clone();
            move |msg, _flow, fields| {
                let page = fields
                    .iter()
                    .find(|(key, _)| *key == "page")
                    .map_or("null", |(_, value)| value.as_str());
                logged.lock().unwrap().push(format!("{msg} {page}"));
            }
        }),
    );
    Lab {
        flight,
        clock,
        lanes,
        logged,
    }
}

impl Lab {
    fn logged(&self, line: &str) -> bool {
        self.logged.lock().unwrap().iter().any(|l| l == line)
    }

    fn lines_starting(&self, start: &str) -> usize {
        self.logged
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.starts_with(start))
            .count()
    }

    /// A run that holds its lane until `gate` closes; gives its key's page id.
    fn hold(&self, key: &str, gate: &Gate, opts: RunOpts) -> Run<String> {
        let wait = gate.wait.clone();
        let page = key.split('/').next().unwrap_or_default().to_owned();
        self.flight
            .start(key, &format!("f.{key}"), opts, async move {
                // Closed is the signal.
                wait.recv().await.ok();
                Ok(page)
            })
    }
}

/// Opens once, for every run waiting on it.
struct Gate {
    open: smol::channel::Sender<()>,
    wait: smol::channel::Receiver<()>,
}

impl Gate {
    fn new() -> Self {
        let (open, wait) = smol::channel::bounded(1);
        Self { open, wait }
    }

    fn opened() -> Self {
        let gate = Self::new();
        gate.open();
        gate
    }

    fn open(&self) {
        self.open.close();
    }
}

const AUTO: RunOpts = RunOpts {
    user: false,
    has_cache: true,
    background: false,
};

/// Whether `key`'s run holds a lane. (Polling `Run::started` races the run's own poll of it.)
fn started(lab: &Lab, key: &str) -> bool {
    let page = key.split('/').next().unwrap_or_default();
    lab.flight.running_pages().iter().any(|p| p == page)
}

#[test]
fn a_caller_leaving_does_not_cancel_the_run_and_the_next_one_joins_it() {
    let lab = lab();
    let gate = Gate::new();
    let runs = Arc::new(AtomicUsize::new(0));
    let work = |gate: &Gate| {
        let (wait, runs) = (gate.wait.clone(), runs.clone());
        async move {
            runs.fetch_add(1, Ordering::SeqCst);
            wait.recv().await.ok();
            Ok("fresh".to_owned())
        }
    };
    // The tab that started the fetch goes away while it runs.
    let first = lab
        .flight
        .start("attendance//AP1", "f1", RunOpts::default(), work(&gate));
    assert!(!first.joined);
    drop(first);
    // Shown again: it joins the same run, named by the flow that runs it.
    let again = lab
        .flight
        .start("attendance//AP1", "f2", RunOpts::default(), work(&gate));
    assert!(again.joined);
    assert_eq!(again.flow, "f1");
    // Another semester or page is its own run.
    let other = lab
        .flight
        .start("attendance//AP2", "f3", RunOpts::default(), async {
            Ok("other".to_owned())
        });
    assert!(!other.joined);

    gate.open();
    assert_eq!(block_on(again.result), Ok("fresh".into()));
    assert_eq!(block_on(other.result), Ok("other".into()));
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // Done: the next start runs again.
    let next = lab
        .flight
        .start("attendance//AP1", "f4", RunOpts::default(), work(&gate));
    assert!(!next.joined);
    block_on(next.result).unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 2);
}

#[test]
fn a_run_that_just_succeeded_is_known_for_a_while() {
    let lab = lab();
    let gate = Gate::new();
    let run = lab.hold("timetable//AP1", &gate, RunOpts::default());
    assert!(lab.flight.is_running("timetable//AP1"));
    assert!(!lab.flight.succeeded_within("timetable//AP1", 60_000));
    gate.open();
    block_on(run.result).unwrap();
    assert!(!lab.flight.is_running("timetable//AP1"));
    // The screen comes back: the fetch that finished meanwhile counts, for a while.
    lab.clock.advance(59_000);
    assert!(lab.flight.succeeded_within("timetable//AP1", 60_000));
    assert!(!lab.flight.succeeded_within("timetable//AP2", 60_000));
    lab.clock.advance(2_000);
    assert!(!lab.flight.succeeded_within("timetable//AP1", 60_000));
    // A failure doesn't count.
    let failed = lab
        .flight
        .start::<()>("marks//AP1", "f2", RunOpts::default(), async {
            Err(CoreError::Offline)
        });
    assert_eq!(
        block_on(failed.result),
        Err(FlightError::Failed(CoreError::Offline))
    );
    assert!(!lab.flight.succeeded_within("marks//AP1", 60_000));
    // Sign-out forgets.
    block_on(
        lab.flight
            .start("timetable//AP1", "f3", RunOpts::default(), async { Ok(()) })
            .result,
    )
    .unwrap();
    lab.flight.cancel_all();
    assert!(!lab.flight.succeeded_within("timetable//AP1", 60_000));
}

#[test]
fn sign_out_cancels_every_run() {
    let lab = lab();
    let gate = Gate::new();
    let run = lab.hold("marks//AP1", &gate, RunOpts::default());
    lab.flight.cancel_all();
    assert_eq!(block_on(run.result), Err(FlightError::Cancelled));
    let again = lab
        .flight
        .start("marks//AP1", "f2", RunOpts::default(), async { Ok(()) });
    assert!(!again.joined);
}

#[test]
fn at_most_the_lane_count_runs_at_once() {
    let lab = lab();
    let gates: Vec<Gate> = (0..3).map(|_| Gate::new()).collect();
    let runs: Vec<Run<String>> = gates
        .iter()
        .enumerate()
        .map(|(i, gate)| lab.hold(&format!("page{i}//AP1"), gate, AUTO))
        .collect();
    assert_eq!(
        ["page0//AP1", "page1//AP1", "page2//AP1"].map(|key| started(&lab, key)),
        [true, true, false]
    );
    // Waiting counts as running: a page shown again joins it.
    assert!(lab.flight.is_running("page2//AP1"));
    assert!(lab.hold("page2//AP1", &gates[2], AUTO).joined);
    assert_eq!(lab.flight.running_pages(), ["page0", "page1"]);
    gates[0].open();
    block_on(runs[0].result.clone()).unwrap();
    assert!(started(&lab, "page2//AP1"));
    assert!(lab.logged("queued page2") && lab.logged("started page2"));
    for gate in &gates {
        gate.open();
    }
    for run in runs {
        block_on(run.result).unwrap();
    }
}

#[test]
fn on_screen_beats_left_beats_background_and_user_beats_automatic() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    lab.flight.shown("attendance", true);
    let gate = Gate::new();
    let blocker = lab.hold("profile//AP1", &gate, AUTO);
    let order = Arc::new(Mutex::new(Vec::<String>::new()));
    let queue = |key: &str, user: bool, background: bool| {
        let (order, key) = (order.clone(), key.to_owned());
        let opts = RunOpts {
            user,
            has_cache: true,
            background,
        };
        lab.flight
            .start(&key.clone(), &format!("f.{key}"), opts, async move {
                order.lock().unwrap().push(key);
                Ok(())
            })
    };
    let runs = [
        queue("marks/stale/", false, true),
        queue("timetable//AP1", false, false),
        queue("exams//AP1", true, false),
        queue("attendance//AP1", false, false),
    ];
    gate.open();
    block_on(blocker.result).unwrap();
    for run in runs {
        block_on(run.result).unwrap();
    }
    assert_eq!(
        *order.lock().unwrap(),
        [
            "attendance//AP1",
            "exams//AP1",
            "timetable//AP1",
            "marks/stale/"
        ]
    );
}

#[test]
fn a_waiting_run_moves_with_its_page() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    let gate = Gate::new();
    let blocker = lab.hold("profile//AP1", &gate, AUTO);
    let order = Arc::new(Mutex::new(Vec::<&str>::new()));
    let push = |name: &'static str| {
        let order = order.clone();
        async move {
            order.lock().unwrap().push(name);
            Ok(())
        }
    };
    let user_off = RunOpts {
        user: false,
        ..RunOpts::default()
    };
    lab.flight.shown("marks", true);
    let marks = lab
        .flight
        .start("marks//AP1", "f1", user_off, push("marks"));
    let exams = lab
        .flight
        .start("exams//AP1", "f2", user_off, push("exams"));
    // The student leaves marks for exams before a lane frees: nothing is cancelled, only moved.
    lab.flight.shown("marks", false);
    lab.flight.shown("exams", true);
    assert!(lab.flight.is_shown("exams") && !lab.flight.is_shown("marks"));
    assert!(lab.flight.is_running("marks//AP1"));
    gate.open();
    block_on(blocker.result).unwrap();
    block_on(marks.result).unwrap();
    block_on(exams.result).unwrap();
    assert_eq!(*order.lock().unwrap(), ["exams", "marks"]);
}

#[test]
fn a_stale_automatic_run_off_screen_is_dropped_but_not_a_user_or_uncached_one() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    let gate = Gate::new();
    let blocker = lab.hold("profile//AP1", &gate, AUTO);
    let done = Gate::opened();
    let auto = lab.hold("marks//AP1", &done, AUTO);
    let user = lab.hold("exams//AP1", &done, RunOpts { user: true, ..AUTO });
    let uncached = lab.hold(
        "grades//AP1",
        &done,
        RunOpts {
            has_cache: false,
            ..AUTO
        },
    );
    // Joined by the student: never dropped either.
    let asked = lab.hold("timetable//AP1", &done, AUTO);
    lab.flight.ask("timetable//AP1");
    lab.clock.advance(60_000 + 1);
    gate.open();
    block_on(blocker.result).unwrap();
    assert_eq!(block_on(auto.result), Err(FlightError::Dropped));
    assert!(!block_on(auto.started));
    assert_eq!(block_on(user.result), Ok("exams".into()));
    assert_eq!(block_on(uncached.result), Ok("grades".into()));
    assert_eq!(block_on(asked.result), Ok("timetable".into()));
    // Dropped is not "just fetched", and the next open fetches again.
    assert!(!lab.flight.succeeded_within("marks//AP1", 60_000));
    assert!(!lab.flight.is_running("marks//AP1"));
    assert!(!lab.hold("marks//AP1", &done, AUTO).joined);
    assert!(lab.logged("dropped marks"));
}

#[test]
fn sign_out_cancels_waiting_runs_too() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    let gate = Gate::new();
    let running = lab.hold("profile//AP1", &gate, AUTO);
    let waiting = lab.hold("marks//AP1", &gate, AUTO);
    lab.flight.cancel_all();
    assert_eq!(block_on(running.result), Err(FlightError::Cancelled));
    assert_eq!(block_on(waiting.result), Err(FlightError::Cancelled));
    assert!(!lab.flight.is_running("marks//AP1"));
    assert!(lab.flight.running_pages().is_empty());
    let again = lab
        .flight
        .start("marks//AP1", "f2", RunOpts::default(), async {
            Ok("fresh".to_owned())
        });
    assert!(!again.joined);
    assert_eq!(block_on(again.result), Ok("fresh".into()));
}

#[test]
fn slow_recent_fetches_run_one_at_a_time() {
    let lab = lab();
    let slow = 4_000 + 1;
    for _ in 0..2 {
        lab.flight.fetched("f0", slow, false);
    }
    assert!(lab.flight.slow_link());
    let gates: Vec<Gate> = (0..2).map(|_| Gate::new()).collect();
    let runs: Vec<Run<String>> = gates
        .iter()
        .enumerate()
        .map(|(i, gate)| lab.hold(&format!("page{i}//AP1"), gate, AUTO))
        .collect();
    assert_eq!(
        ["page0//AP1", "page1//AP1"].map(|key| started(&lab, key)),
        [true, false]
    );
    // Quick again: back to two lanes, and the waiting one starts.
    for _ in 0..2 {
        lab.flight.fetched("f0", 500, false);
    }
    assert!(!lab.flight.slow_link());
    assert!(started(&lab, "page1//AP1"));
    assert_eq!(lab.lines_starting("lanes"), 1);
    // Over UDP: one lane however fast.
    lab.flight.fetched("f0", 500, true);
    assert!(lab.flight.slow_link());
    assert_eq!(lab.lines_starting("lanes"), 2);
    for gate in &gates {
        gate.open();
    }
    for run in runs {
        block_on(run.result).unwrap();
    }
}

#[test]
fn cancel_all_cancels_waiting_and_running_and_forgets_success() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    block_on(lab.hold("timetable//AP1", &Gate::opened(), AUTO).result).unwrap();
    assert!(lab.flight.succeeded_within("timetable//AP1", 60_000));
    let gate = Gate::new();
    let running = lab.hold("profile//AP1", &gate, AUTO);
    let waiting = lab.hold("marks//AP1", &gate, AUTO);
    assert!(started(&lab, "profile//AP1") && !started(&lab, "marks//AP1"));
    lab.flight.cancel_all();
    assert_eq!(block_on(running.result), Err(FlightError::Cancelled));
    assert_eq!(block_on(waiting.result), Err(FlightError::Cancelled));
    // A cancelled waiting run never gets a lane.
    assert!(!block_on(waiting.started));
    assert!(!lab.flight.succeeded_within("timetable//AP1", 60_000));
    assert!(!lab.flight.is_running("profile//AP1"));
}

#[test]
fn joined_user_ask_promotes_and_never_drops() {
    let lab = lab();
    lab.lanes.store(1, Ordering::SeqCst);
    let gate = Gate::new();
    let blocker = lab.hold("profile//AP1", &gate, AUTO);
    let done = Gate::opened();
    let exams = lab.hold("exams//AP1", &done, AUTO);
    let marks = lab.hold("marks//AP1", &done, AUTO);
    // The student opens marks: joining with `user` asks for it.
    let joined = lab.hold("marks//AP1", &done, RunOpts::default());
    assert!(joined.joined);
    lab.clock.advance(60_000 + 1);
    gate.open();
    block_on(blocker.result).unwrap();
    assert_eq!(block_on(exams.result), Err(FlightError::Dropped));
    assert_eq!(block_on(marks.result), Ok("marks".into()));
    assert_eq!(block_on(joined.result), Ok("marks".into()));
}

// Activity.

fn entry(n: u64) -> ActivityEntry {
    ActivityEntry {
        id: n,
        flow: format!("f{n}"),
        label: format!("Refreshing {n}"),
        started_at: n as i64,
        run: ActivityRun::Running,
    }
}

fn done(n: u64, run: ActivityRun) -> ActivityEntry {
    ActivityEntry { run, ..entry(n) }
}

#[test]
fn a_finished_run_leaves_running_and_tops_recent() {
    let started = ActivityState::default().started(entry(1)).started(entry(2));
    assert_eq!(started.running, [entry(1), entry(2)]);
    let finished = started.finished(1, done(1, ActivityRun::Done { ms: 5 }));
    assert_eq!(finished.running, [entry(2)]);
    assert_eq!(finished.recent, [done(1, ActivityRun::Done { ms: 5 })]);
}

#[test]
fn recent_keeps_the_newest_only() {
    let mut state = ActivityState::default();
    for n in 0..(KEEP as u64 + 5) {
        state = state.started(entry(n)).finished(n, entry(n));
    }
    assert_eq!(state.recent.len(), KEEP);
    assert_eq!(state.recent.first(), Some(&entry(KEEP as u64 + 4)));
    assert!(state.running.is_empty());
}

#[test]
fn offline_is_the_latest_runs_problem() {
    let offline = Problem {
        kind: ProblemKind::Offline,
        title: "Offline".into(),
        message: "Connect and try again.".into(),
        flow: "f1".into(),
        retryable: true,
        user_initiated: true,
    };
    let state = ActivityState::default().started(entry(1)).finished(
        1,
        done(
            1,
            ActivityRun::Failed {
                ms: 0,
                problem: offline,
            },
        ),
    );
    assert!(state.offline());
    assert!(
        !state
            .started(entry(2))
            .finished(2, done(2, ActivityRun::Done { ms: 0 }))
            .offline()
    );
}

#[test]
fn track_records_done_failed_and_cancelled_and_pushes_each_change() {
    let clock = ManualClock::default();
    let pushed = Arc::new(Mutex::new(Vec::<Vec<ActivityEntry>>::new()));
    let activity = Activity::new(Arc::new(clock.clone()), {
        let pushed = pushed.clone();
        Arc::new(move |entries| pushed.lock().unwrap().push(entries))
    });
    let ok = block_on(activity.track("Refreshing marks", "f1", async {
        clock.advance(40);
        Ok(3)
    }));
    assert_eq!(ok, Ok(3));
    let failed =
        block_on(activity.track::<()>("Refreshing exams", "f2", async { Err(CoreError::Offline) }));
    assert_eq!(failed, Err(CoreError::Offline));
    assert!(activity.state().offline());
    // Dropped while it runs: cancelled.
    let mut pending =
        Box::pin(activity.track::<()>("Refreshing grades", "f3", std::future::pending()));
    assert!(block_on(poll_once(&mut pending)).is_none());
    assert_eq!(activity.state().running.len(), 1);
    drop(pending);
    let state = activity.state();
    assert!(state.running.is_empty());
    let runs: Vec<_> = state
        .recent
        .iter()
        .map(|e| (e.label.as_str(), &e.run))
        .collect();
    assert!(matches!(
        runs.as_slice(),
        [
            ("Refreshing grades", ActivityRun::Cancelled { ms: 0 }),
            ("Refreshing exams", ActivityRun::Failed { .. }),
            ("Refreshing marks", ActivityRun::Done { ms: 40 }),
        ]
    ));
    // Started and finished, three times; the last push lists what the app shows.
    let pushed = pushed.lock().unwrap();
    assert_eq!(pushed.len(), 6);
    assert_eq!(pushed.last().map(Vec::len), Some(3));
    assert_eq!(clock.now_ms(), 40);
}
