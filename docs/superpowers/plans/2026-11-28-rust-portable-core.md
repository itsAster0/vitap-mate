# Rust-driven core: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move all domain logic and scheduling out of Kotlin `core/` into `rust/vtop-ffi`, so
Kotlin only draws and supplies platform services, with no visible change on Android.

**Architecture:** Rust gets a smol executor and drives the fetch queue, page watches,
sweep, sync, settings, consent timeouts and Gmail. It pushes state to Kotlin through UniFFI
listeners. Kotlin keeps Compose, a thin mirror of pushed state, and platform traits.

**Tech Stack:** Rust 2024 edition crate `vtop-ffi`, UniFFI 0.32, smol 2, Kotlin Compose built with the Kotlin Toolchain, jj.

**Spec:** `docs/superpowers/specs/2026-11-28-rust-portable-core-design.md`

## Global Constraints

- No tokio in the app. The only new crate is `smol = "2"` in `rust/vtop-ffi/Cargo.toml`.
  Use its executor, `Task`, `Timer` and `unblock`, and not its sockets: the network stays
  platform I/O.
- Spec names, args and cache files never change. Page string ids, flow names (`<id>.open`,
  `<id>.refresh`, `sync.stale`, `sync`) and log srcs (`ui.<id>`, `fetch.queue`, `sync.stale`,
  `sync`) stay byte-identical.
- Every `Core` call takes the flow id first. Never log cookies, passwords, OTPs, CSRF tokens or
  keys.
- The VTOP session goes to our servers only through `consent::allowed`.
- Tests use no sleeps or wall-clock time. `now` is passed in, and executor tests use a manual
  clock.
- Kotlin tests that move to Rust are deleted once the Rust port passes. Port them as written:
  same cases, same expected values.
- No UI or copy changes. Every user-facing string moves verbatim.
- No commits, pushes or deploys by agents. The working copy is jj's. The lead describes changes
  when the user asks.
- Use the emulator only (`emulator-5554`). Never install on, clear, or sign out of the
  connected real phone. The lead gives agents the emulator PIN.
- After any change in `rust/vtop-ffi`, run `mise run rust-android-dev` for quick emulator runs
  and `mise run rust-android` before the final check. Never hand-edit or hand-merge
  `core/src/vitapmate/core/vtop_ffi.kt`; regenerate it.
- Done means `mise run check` passes. A task is done when `mise run check-changed` passes and
  its emulator step is checked.

## Review Focus

1. **Update with custom settings.** A student who changed theme, tabs, tunables or the OTP
   source updates and sees the same values. If the import fails, SharedPreferences stay
   untouched and the next start tries again. Owner: Task 2.
2. **Tab switch mid-fetch.** The fetch keeps running. Coming back joins it, or, if it finished
   within `JustFetched`, shows the cache with no second request. Owner: Task 5.
3. **Sign-out mid-fetch or mid-sync.** Every run is cancelled, each open page shows the "Not
   Loaded" problem with no toast, and no listener fires after `PageWatch::close()`. Owner:
   Tasks 4 and 5.
4. **Listener threading.** Listeners are called on Rust pool threads. Kotlin hops to the main
   thread before touching Compose state. A listener that throws is logged and never stops a
   Rust task. Owner: Task 1.
5. **App leaves during an ask.** An open consent ask resolves as deny. Unattended flows never
   prompt and fetch Direct unless "Don't ask again" is ticked. Owner: Task 10.

## Agents and lanes

Two agents work at once, each in its own jj workspace (`isolation: "worktree"`). The lead (this
session) runs integration.

| Lane | Agent | Tasks, in order |
|---|---|---|
| A | Claude Opus 5.5 (native `Agent`) | 1, 3, 4, 5, 6, 7 |
| B | Gemini 4 Neon (`delegate_task`, Antigravity, `thought_level: high`) | 2, 8, 9, 10\*, 11\*, 12 |

\* Tasks 10 and 11 need Task 1's `Runtime`, `AppEvents`, `app_shown`, `SecretBox` and
`UrlOpener`. Lane B starts them only after the lead has rebased lane B onto Task 1.

Lane A's Task 4 reads tunables from Task 2, so the lead rebases lane A onto Task 2 before Task
4 starts.

**Integration after each task (lead):**

1. Rebase the lane onto the other lane's finished tasks: `jj rebase`.
2. Resolve conflicts in `lib.rs` (mod lines) and `AppCore.kt` by hand.
3. Resolve `vtop_ffi.kt` by running `mise run rust-android`.
4. Run `mise run check-changed`.
5. Run one emulator smoke: sign in as `DEMO2026`, open every tab, pull to refresh.

Only one agent uses the emulator at a time. The lead hands it out.

**Brief for each agent:**

- the task text and this file's Global Constraints
- the spec path
- the files the task may touch, and that it must not touch the other lane's files
- "don't commit"
- the emulator PIN

---

### Task 1: Runtime, app events and host traits (lane A)

**Files:**
- Create: `rust/vtop-ffi/src/runtime.rs`
- Create: `rust/vtop-ffi/src/events.rs`
- Modify: `rust/vtop-ffi/Cargo.toml` (add `smol = "2"`, with a comment saying why)
- Modify: `rust/vtop-ffi/src/lib.rs` (Core holds `Runtime` and `AppEvents`)
- Modify: `core/src/vitapmate/data/AppCore.kt`
- Modify: `core/src/vitapmate/data/Secrets.kt` (implements `SecretBox`)
- Modify: `core/src/vitapmate/data/Freshness.kt` (`Foreground` calls `app_shown`)
- Create: `core/src/vitapmate/data/AppEventsBridge.kt`

**Interfaces:**
- Produces, in Rust:
  - `pub(crate) struct Runtime`
    - `fn spawn<T: Send + 'static>(&self, flow: &str, fut: impl Future<Output = T> + Send +
      'static) -> smol::Task<T>`. Dropping the task cancels it; callers `.detach()` work that
      must outlive them.
    - `fn timeout<F: Future>(&self, ms: u64, fut: F) -> Result<F::Output, TimedOut>`, which
      races `fut` against `clock.sleep(ms)` with `smol::future::or`
  - `pub(crate) trait Clock: Send + Sync`, with `fn now_ms(&self) -> i64` and `fn sleep(&self,
    ms: u64) -> Pin<Box<dyn Future<Output = ()> + Send>>`. The real one uses `SystemTime` and
    `smol::Timer::after`. `ManualClock` (tests) wakes sleepers on `advance(ms)`.
- Produces, exported:
  - `#[uniffi::export(with_foreign)] trait AppEvents`, with:
    - `on_background_problem(Problem)`
    - `on_activity(Vec<ActivityEntry>)`
    - `on_settings(Settings)`
    - `on_semesters(SemesterState)`
    - `on_sync_health(SyncHealth)`
    - `on_consent(ConsentAsk)`

    Methods whose types later tasks add are added by those tasks.
  - `trait SecretBox`, with `seal(name: String, bytes: Vec<u8>) -> Result<(), SecretError>` and
    `open(name: String) -> Result<Option<Vec<u8>>, SecretError>`
  - `trait UrlOpener`, with `open(url: String)`
  - `Core::set_events(events: Arc<dyn AppEvents>)`
  - `Core::app_shown(flow: String, shown: bool)`
  - `Core::resumes() -> u32`
- Produces, in Kotlin: `AppEventsBridge : AppEvents`. Each method posts to the main thread
  (`Handler(Looper.getMainLooper())`), then sets Compose state.

- [ ] **Step 1: Write the failing tests** in `runtime.rs`
  - `spawn_runs_detached_from_caller`: the work keeps going after the caller's future is dropped.
  - `dropped_task_stops_before_next_await`
  - `timeout_fires_on_manual_clock`
  - `app_shown_counts_resumes_only_after_leaving`: started, then stopped, then started gives
    `resumes == 1`; a second start without a stop doesn't count. This ports `ForegroundTest`.
  - `listener_panic_is_caught_and_logged`: an `AppEvents` mock that panics doesn't kill the
    pool thread.
- [ ] **Step 2:** Run `cd rust && cargo test -p vtop-ffi runtime`. Expected: FAIL (unresolved
  `Runtime`).
- [ ] **Step 3:** Implement `Runtime` as a `static smol::Executor<'static>` (or an `Arc`'d one
  in Core). Run it on 2 threads named `vtop-rt`, each running `smol::block_on(ex.run(
  smol::future::pending::<()>()))`. Wrap listener calls in `catch_unwind` plus a `warn!`.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5:** Wire up Kotlin:
  - `AppCore` passes `AppEventsBridge`, `Secrets` (as `SecretBox`) and a `UrlOpener` that
    starts `ACTION_VIEW`.
  - `Foreground.started()`/`stopped()` call `app_shown`.
  - Delete `ForegroundTest.kt`.
  - Run `mise run rust-android-dev && ./kotlin build && ./kotlin test`. Expected: PASS.
- [ ] **Step 6:** Emulator check: the app starts, and backgrounding it and coming back logs
  `app_shown` with `resumes` going up.

### Task 2: Settings store in Rust, with import (lane B)

**Files:**
- Create: `rust/vtop-ffi/src/settings.rs`
- Create: `rust/vtop-ffi/src/settings/tests.rs`
- Modify: `rust/vtop-ffi/src/lib.rs`
- Modify: `core/src/vitapmate/data/Settings.kt` (becomes a mirror; same public API)
- Modify: `app/src/vitapmate/MainActivity.kt` or the Application class (open the store
  synchronously before `setContent`)

**Interfaces:**
- Produces, exported:
  - `#[derive(uniffi::Object)] SettingsStore`, with:
    - `SettingsStore::open(dir: String) -> Arc<SettingsStore>` (sync)
    - `get() -> Settings`
    - `set(change: SettingChange) -> Settings`
    - `import(prefs: HashMap<String, PrefValue>) -> Result<Settings, CoreError>`
    - `needs_import() -> bool`
    - `set_listener(Arc<dyn SettingsListener>)`
  - `Settings` record: one field per Kotlin `Settings` property, today's defaults.
  - `SettingChange` enum: one variant per settable field, plus `Tunable{key: TunableKey,
    value: Option<i32>}`, `NavTabs{ids: Vec<String>}` and `Notifies{kind: ChangeKind, on:
    bool}`.
  - `PrefValue` enum: `Bool`, `Int`, `Long`, `Float`, `Str`, `StrSet`.
  - `TunableKey` enum, mirroring Kotlin `Tunable` entries.
  - `tunables() -> Vec<TunableInfo{key, pref_key, title, unit, default, min, max, note}>`
  - `Settings::tunable(key) -> i32`
- Produces, in Rust: `Core::settings(&self) -> Settings`, for Tasks 4–7, 10 and 11.
- Consumes: nothing.

- [ ] **Step 1: Write the failing tests** in `settings/tests.rs`
  - `defaults_match_kotlin`: assert each default against `Settings.kt`'s literal, every field
    and every tunable (key, default, range).
  - `import_reads_flutter_keys`: a map with every pref key from `Settings.kt` gives the same
    values.
  - `import_ignores_unknown_and_wrong_typed_keys`
  - `stored_roundtrip`: `StoredSettings` serialises and parses back equal.
  - `tunable_clamps_to_range`
  - `set_persists_then_reopen_reads_it`
  - `import_failure_leaves_no_file`: a write error returns `Err` and `needs_import()` stays
    true.
- [ ] **Step 2:** Run `cd rust && cargo test -p vtop-ffi settings`. Expected: FAIL.
- [ ] **Step 3:** Implement `settings.rs`:
  - Store `settings.json` in the app files dir, written atomically: temp file plus rename, the
    same as `writeAtomically`.
  - Keep `StoredSettings` as the disk Wire type with `#[serde(default)]` per field.
  - Pull the Flutter pref keys and defaults verbatim from `Settings.kt`.
- [ ] **Step 4:** Run the tests. Expected: PASS.
- [ ] **Step 5:** Rewrite `Settings.kt` as a mirror. Every property keeps its name and type:
  - The getter reads `mutableStateOf<Settings>`.
  - The setter calls `store.set(...)` and updates the state with the returned record.
  - `Tunable` stays a Kotlin enum for the Developer screen's code, but reads title, unit, range
    and note from `tunables()`.
  - On first open, if `needs_import()`, copy `prefs.all` into `PrefValue`s, call `import`, and
    clear the prefs only on `Ok`.
- [ ] **Step 6:** Run `mise run rust-android-dev && ./kotlin build && ./kotlin test`. Expected:
  PASS. `NavTabsTest` still passes.
- [ ] **Step 7:** Emulator check (Review Focus 1):
  1. Install the **current release** APK.
  2. Change theme, nav tabs, one tunable and the OTP source.
  3. Install this build over it.
  4. Check all four values kept and no theme flash on start.

### Task 3: Problems in Rust (lane A)

**Files:**
- Create: `rust/vtop-ffi/src/problem.rs`
- Modify: `core/src/vitapmate/data/PageState.kt` (`Problem`, `ProblemKind`, `problemOf`,
  `toasts`)
- Delete: `core/test/vitapmate/data/ProblemTest.kt`

**Interfaces:**
- Produces:
  - `Problem` record `{kind: ProblemKind, title, message, flow, retryable: bool,
    user_initiated: bool}`
  - `ProblemKind` enum: `Offline`, `VtopDown`, `SignInNeeded`, `OtpNeeded`, `OtpRejected`,
    `SemesterNeeded`, `ConsentDenied`, `Unexpected`
  - exported `problem_of(error: CoreError, flow: String, server: bool) -> Problem`
  - `Problem::toasts(show_background_errors: bool) -> bool`, exported as
    `problem_toasts(p, show)`
  - `Problem::not_loaded(flow) -> Problem`
- Kotlin: `typealias Problem = vitapmate.core.Problem`. `problemOf(e: Throwable, flow,
  server)` is `(e as? CoreException)?.let { problemOf(it.toCoreError(), ...) } ?: unexpected`.
  If UniFFI's exceptions don't carry the enum, add `CoreException.toCoreError()` in Kotlin.

- [ ] **Step 1:** Port `ProblemTest.kt` case for case into `problem.rs`'s tests. Add
  `vtop_down_trims_reason_and_falls_back` and `server_other_shows_reason`.
- [ ] **Step 2:** Run `cargo test -p vtop-ffi problem`. Expected: FAIL.
- [ ] **Step 3:** Implement with the texts copied verbatim from `problemOf` and `NoToast`.
- [ ] **Step 4:** Run `cargo test -p vtop-ffi problem`, then `mise run rust-android-dev &&
  ./kotlin test`. Expected: PASS.

### Task 4: Fetch queue, single flight and activity tracker (lane A, after Task 2)

**Files:**
- Create: `rust/vtop-ffi/src/schedule/queue.rs` (pure port of `FetchQueue.kt`)
- Create: `rust/vtop-ffi/src/schedule/flight.rs` (port of `SingleFlight`)
- Create: `rust/vtop-ffi/src/schedule/activity.rs` (port of `Activity.kt`)
- Create: `rust/vtop-ffi/src/schedule/tests.rs`
- Delete: `core/test/vitapmate/data/FetchQueueTest.kt`, `SingleFlightTest.kt`,
  `ActivityTest.kt`
- Modify: `core/src/vitapmate/data/Activity.kt` (mirror of `on_activity`)

**Interfaces:**
- Consumes: `Runtime`, `Clock` and `AppEvents` (Task 1); `Settings::tunable` (Task 2).
- Produces, in Rust:
  - `FetchQueue::next(self, event: QueueEvent, now_ms: i64, cfg: QueueConfig) -> QueueStep`,
    with the same `Ticket`, `Slot`, `QueueEvent`, `QueueEffect`, `RECENT = 3` and log messages
    and fields as Kotlin.
  - `Flight`:
    - `fn start<T: Send + 'static>(&self, key: &str, flow: &str, opts: RunOpts, work: impl
      Future<Output = Result<T, CoreError>> + Send + 'static) -> Run<T>`
    - `RunOpts{user, has_cache, background}`
    - `Run<T>{result: Shared<…>, flow, joined, started}`
    - `ask(key)`, `shown(page, on)`, `fetched(flow, ms, udp)`, `is_running(key)`,
      `is_shown(page)`, `succeeded_within(key, ms)`, `slow_link()`, `cancel_all()`,
      `running_pages() -> Vec<String>`
  - Errors: `FlightError::Dropped` and `FlightError::Cancelled`, kept apart from `CoreError`.
  - `Activity::track<T>(label, flow, fut) -> T`. It pushes `on_activity(Vec<ActivityEntry{id,
    label, flow, started_ms, running: bool, failed: bool}>)`, keeping today's history length.
- `Core::sign_out` and the account switch call `flight.cancel_all()`.

- [ ] **Step 1:** Port `FetchQueueTest`, `SingleFlightTest` and `ActivityTest` one to one into
  `schedule/tests.rs`, using `ManualClock`. Add:
  - `cancel_all_cancels_waiting_and_running_and_forgets_success` (Review Focus 3)
  - `joined_user_ask_promotes_and_never_drops`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi schedule`. Expected: FAIL.
- [ ] **Step 3:** Implement:
  - `queue.rs` is a line-for-line port (pure, `#[derive(Clone)]` state).
  - `flight.rs` holds `Mutex<(FetchQueue, HashMap<u64, Handle>)>`. It runs effects outside the
    lock, as `perform` does, and spawns work on `Runtime`.
- [ ] **Step 4:** Run the same tests. Expected: PASS.
- [ ] **Step 5:** Run `mise run rust-android-dev && ./kotlin test`. Expected: PASS. Kotlin still
  uses its own `PageFetches` until Task 5.

### Task 5: Page watches and `rememberPage` (lane A)

**Files:**
- Create: `rust/vtop-ffi/src/page_watch.rs`
- Create: `rust/vtop-ffi/src/page_watch/state.rs` (port of `PageEvent.next` and
  `refreshTrigger`)
- Create: `rust/vtop-ffi/src/page_watch/tests.rs`
- Modify: `core/src/vitapmate/data/Page.kt` (`PageHandle` becomes a thin listener;
  `rememberPage` keeps its shape)
- Modify: `core/src/vitapmate/data/PageState.kt` (`PageState<T>` stays the Kotlin type the UI
  draws, built from `PageStatus` plus the data read)
- Delete: `core/src/vitapmate/data/PageFetches.kt`, `FetchQueue.kt`, `PageEvent.kt`;
  `core/test/.../PageEventTest.kt`, `PageActivityTest.kt`, `RefreshTriggerTest.kt`
- Modify: the 14 `rememberPage` call sites in `features/*` (drop the `fetch` and `updatedAt`
  lambdas; pass `PageId`)

**Interfaces:**
- Consumes: `Flight`, `Activity::track` (Task 4); `problem_of` and `Problem::not_loaded` (Task
  3); `Core::app_shown` resumes (Task 1); `Settings` (`auto_refresh`, `PageMaxAge`,
  `JustFetched`).
- Produces, exported:
  - `enum PageId`: `Attendance`, `AttendanceDetail`, `Timetable`, `Calendar`, `Marks`,
    `Exams`, `Grades`, `GradeHistory`, `Profile`, `GeneralOuting`, `WeekendOuting`,
    `Biometric`, `CoursePage`, `CoursePageCourse`, with `page_id_name(PageId) -> String`
    returning today's ids
  - `enum PageStatus { Loading{waiting: Option<String>}, Ready{updated_at: i64, activity:
    PageActivity}, Failed{problem: Problem} }`
  - `enum PageActivity { Idle, Refreshing{waiting: Option<String>}, Stale{problem: Problem} }`
  - `trait PageListener`, with `on_status(status: PageStatus)`
  - `Core::page_watch(flow: String, page: PageId, item: String, listener: Arc<dyn
    PageListener>) -> Arc<PageWatch>`
  - `PageWatch`, with `refresh(trigger: String, user: bool)` (async, never errors),
    `shown(on: bool)`, `close()` and `key() -> String`
- Produces, in Rust:
  - `PageId::fetch(&self, core, flow, item) -> Result<i64 /*updated_at*/, CoreError>`
  - `PageId::cached_updated_at(&self, core, flow, item) -> Option<i64>`
  - Both dispatch to the existing `fetch_x`/`cached_x`. `item` is the course key for
    `AttendanceDetail`, the semester for `Grades`, the course id for `CoursePageCourse`.
- Kotlin:
  - `rememberPage(page: PageId, cached: (Core, String) -> T?, item: String = "", count: (T)
    -> Int? = { null }): PageHandle<T>`
  - `PageHandle.state: PageState<T>`. When `Ready.updated_at` changes, it re-reads `cached` on
    `Dispatchers.IO`.
  - `PageHandle.refresh(trigger, user)`
  - `DisposableEffect` calls `shown` and `close`.

- [ ] **Step 1:** Port `PageEventTest`, `PageActivityTest` and `RefreshTriggerTest` into
  `page_watch/tests.rs`. Add, using a fake fetch and `ManualClock`:
  - `open_reads_cache_then_fetches_when_none`
  - `open_rejoins_running_fetch_without_second_request`
  - `open_within_just_fetched_skips_request` (Review Focus 2)
  - `resume_fetches_only_when_stale_and_auto_refresh`
  - `waiting_text_after_300ms_names_running_page`
  - `close_stops_all_listener_calls` (Review Focus 3)
  - `cancel_all_turns_loading_into_not_loaded_without_toast` (Review Focus 3)
  - `semester_needed_rereads_semesters`
  - `log_fields_match_kotlin`: asserts the `ui.<id>` line keys `state`, `kind`, `ms`,
    `count`, `age_s`, `problem`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi page_watch`. Expected: FAIL.
- [ ] **Step 3:** Implement `page_watch.rs`, keeping the sequence of today's `LaunchedEffect`:
  read the cache, apply the just-fetched check, take the open trigger, then follow resumes.
  - Track "fetched this semester" per key in a `Mutex<HashSet<String>>` on Core.
  - After each successful fetch, call `Flight::fetched` and the sweep hook (a no-op until Task
    6), and re-read semesters if none are saved.
  - Port `OtpPrompt.mark`/`openedSince` into Rust; Rust's `otp.rs` already knows when the
    prompt opens.
  - Report to the state dump under `page.<id>` with today's keys.
- [ ] **Step 4:** Run `cargo test -p vtop-ffi page_watch`. Expected: PASS.
- [ ] **Step 5:** Rewrite `Page.kt` and the 14 call sites, and delete the files listed above.
  Run `mise run rust-android-dev && ./kotlin build && ./kotlin test && mise run detekt`.
  Expected: PASS.
- [ ] **Step 6:** Emulator check with the demo account:
  - Every tab loads, and pull to refresh works.
  - Switching tabs mid-fetch, then coming back, shows one fetch in `mise run logs`
    (`fetch.queue` started once).
  - Signing out mid-fetch shows no toast.
  - Compare `mise run state` against a run of the parent change: same keys.

### Task 6: Freshness and the open-app sweep (lane A)

**Files:**
- Create: `rust/vtop-ffi/src/schedule/freshness.rs` (CalendarRefresh, ExamDayRefresh)
- Create: `rust/vtop-ffi/src/schedule/sweep.rs` (StaleSweep)
- Modify: `core/src/vitapmate/data/Freshness.kt` (only `Foreground` is left, and it calls
  `app_shown`)
- Modify: `features/timetable/.../TimetableScreen.kt`,
  `features/attendance/.../AttendanceScreen.kt`, `CourseDetail.kt` (drop direct calls)

**Interfaces:**
- Consumes: `Flight`, `Activity`, `PageId::cached_updated_at`, the sweep hook (Task 5), and
  `Settings` tunables `SweepQuick`, `SweepCooldown`, `SweepMaxAge`, `SlowPage` and
  `CalendarMaxAge`.
- Produces:
  - in Rust: `calendar_if_old(core, flow, semester, src, anyway) -> bool`,
    `exams_if_exam_today(core, flow, semester, src) -> bool` and `Sweep::after_fetch(ms)`
  - Watches call these: opening a `Timetable` watch calls both; a finished `Attendance` fetch
    calls `calendar_if_old(anyway = true)`.
  - `RETRY_MS = 15 * 60_000` and `QUIET_MS = 2_000` copied verbatim.

- [ ] **Step 1: Write the failing tests** in `sweep.rs` and `freshness.rs`:
  - `sweep_waits_for_quiet_and_idle_activity`
  - `sweep_stops_at_first_failure_slow_or_dropped`
  - `sweep_skips_pages_on_screen`
  - `sweep_needs_quick_fetch_fast_link_auto_refresh_and_shown`
  - `calendar_retry_once_per_15_min_per_semester`
  - `calendar_cancel_forgets_try`
  - `exam_day_only_with_paper_today_and_auto_refresh`
  - `batch_uses_only_batched_pages`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi sweep freshness`. Expected: FAIL.
- [ ] **Step 3:** Implement as a direct port. Failures go to `on_background_problem` only when
  `Problem::toasts`.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check: open the timetable on the demo account. The logs show `ui.timetable`
  `calendar fetched`, or no fetch when it's fresh. A quick fetch elsewhere starts
  `sync.stale` at most once per cooldown.

### Task 7: Semester choice, sync health, VtopSync and histories (lane A)

**Files:**
- Create: `rust/vtop-ffi/src/sync.rs`
- Create: `rust/vtop-ffi/src/sync/tests.rs`
- Create: `rust/vtop-ffi/src/semester_choice.rs`
- Create: `rust/vtop-ffi/src/sync_health.rs`
- Modify: `core/src/vitapmate/data/SemesterChoice.kt`, `SyncHealth.kt` (mirrors of
  `on_semesters` and `on_sync_health`)
- Delete: `core/src/vitapmate/data/VtopSync.kt`, `Histories.kt`; `core/test/.../VtopSyncTest.kt`,
  `SemesterStateTest.kt`, `SyncHealthTest.kt`
- Modify: `app/src/vitapmate/BackgroundSync.kt`, `app/src/vitapmate/FetchAll.kt`

**Interfaces:**
- Consumes: `Flight` and `Activity` (Task 4); `Settings` (`sync_course`, `SyncBiometricDays`)
  (Task 2); `AppEvents` (Task 1).
- Produces, exported:
  - `enum SyncKind { Background, FetchAll }`
  - `trait SyncOtp`, with `async get(challenge: OtpChallenge, flow: String) -> bool`
  - `Core::sync(flow, kind, page_timeout_ms: Option<u64>, otp: Arc<dyn SyncOtp>) ->
    SyncResult{outcome: SyncOutcome, fetched: u32, failed: u32}`
  - `enum SyncOutcome { Done, StoppedAtOtp, Rejected, Failed }`
  - `SemesterState` (port of the Kotlin sealed interface), `Core::semester_pick(flow, id)` and
    `Core::semester_refresh(flow, trigger)`
  - `SyncHealth`/`PauseReason`, as today's sealed types, with `Core::sync_health_after_otp(why)`
    and `Core::sync_health_resume_if(...)` matching today's call sites
  - `Core::histories_refresh(flow, ...)`, matching `Histories.kt`'s entry points
- Unattended: `SyncKind::Background` runs as `flow(name, unattended = true)` does today. The
  flag passes into Rust's consent check.

- [ ] **Step 1:** Port `VtopSyncTest`, `SemesterStateTest` and `SyncHealthTest` into Rust. Add:
  - `otp_asked_once_for_many_pages`
  - `rejected_password_stops_whole_run`
  - `stop_at_otp_cancels_pending_otp`
  - `page_timeout_counts_failed_and_continues`
  - `background_never_prompts` (Review Focus 5)
  - `sign_out_mid_sync_cancels` (Review Focus 3)
  - `pause_reason_stored_roundtrip`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi sync semester_choice sync_health`. Expected: FAIL.
- [ ] **Step 3:** Implement as a port. Keep the order (semesters, then attendance, then the
  rest through the lanes), the biometric day picking, the course round robin and the
  pending-outing check.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin build &&
  ./kotlin test`. Expected: PASS.
- [ ] **Step 5:** Emulator check:
  - More → Fetch all on the demo account ends `Done`, with the same counts as on the parent
    change.
  - Running `adb shell am broadcast -p com.vitap_pal.app -a vitapmate.SYNC_NOW --ez simulate
    true` logs `sync` `done`.

### Task 8: Mess menu parser (lane B)

**Files:**
- Create: `rust/vtop-ffi/src/mess_menu.rs`
- Create: `rust/vtop-ffi/src/mess_menu/tests.rs`
- Modify: `core/src/vitapmate/data/MessMenu.kt` (only the types the UI draws are left, or a
  typealias to the UniFFI records)
- Delete: `core/test/vitapmate/data/MessMenuTest.kt`
- Keep: `Spreadsheet.kt` (platform decoding)

**Interfaces:**
- Produces:
  - `parse_mess_menu(tables: HashMap<String, Vec<Vec<String>>>, year: Option<i32>) ->
    Result<MessMenu, MessMenuError>`
  - `resolve_mess_item(raw, day, block) -> Option<MessItem>`
  - `MessMenu`, `MessSheet`, `MessItem`, `Meal` and `MealWindow` as UniFFI records/enums with
    today's fields
  - `yearOf` becomes the `year` value Kotlin works out from the doc's date, as today's caller
    does.

- [ ] **Step 1:** Port `MessMenuTest` one to one into `mess_menu/tests.rs`.
- [ ] **Step 2:** Run `cargo test -p vtop-ffi mess_menu`. Expected: FAIL.
- [ ] **Step 3:** Implement as a port. `MessMenuFormatException`'s message becomes
  `MessMenuError::Format{message}`, with the same text.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check: import a mess menu `.xlsx` into Docs, and the mess view shows
  the same meals as before.

### Task 9: Docs index and downloads bookkeeping (lane B)

**Files:**
- Create: `rust/vtop-ffi/src/docs.rs` (next to the existing `StoredDocIndex`; move it here if
  it lives elsewhere)
- Create: `rust/vtop-ffi/src/docs/tests.rs`
- Modify: `core/src/vitapmate/data/Docs.kt` (SAF and file I/O only; the list comes from Rust)
- Modify: `core/src/vitapmate/data/Downloads.kt` (`savedByName` from Rust)
- Delete: `core/test/vitapmate/data/DocsTest.kt`

**Interfaces:**
- Produces, exported:
  - `Doc` and `DocKind` (with `doc_kind_of(file_name)`) and `DocSort`, as today
  - `Core::docs(flow) -> Vec<Doc>`
  - `Core::docs_query(sort, query, kind: Option<DocKind>) -> Vec<Doc>`
  - `Core::doc_add(flow, doc: Doc)`
  - `doc_rename(id, name)`, `doc_set_mess_menu(id, on)`, `doc_opened(id, now_ms)`,
    `doc_save_view(id, page, zoom)` and `doc_remove(id) -> Option<Doc>`
  - `Core::docs_newest_mess_menu() -> Option<Doc>`
  - `Core::docs_sort_set(sort)`
  - `Core::downloads_saved(name, uri)`, `downloads_moved(old_uri, new_uri)` and
    `downloads_saved_by_name() -> HashMap<String, String>`
  - `index.json` keeps its exact format (`StoredDocIndex`), so existing files read unchanged.

- [ ] **Step 1:** Port `DocsTest` into Rust. Add:
  - `existing_index_json_reads_unchanged`: a fixture copied from a real `index.json` shape,
    with no personal names.
  - `rename_blank_keeps_old_name`
  - `remove_returns_doc_for_file_delete`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi docs`. Expected: FAIL.
- [ ] **Step 3:** Implement. Rust owns the in-memory list and writes `index.json` atomically.
  Kotlin calls Rust after each successful SAF write or delete, never before.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check:
  - Docs lists the files saved before the update.
  - Rename, sort, search and delete all work.
  - Changing the download folder moves only the app's files.

### Task 10: Consent gates and live topics (lane B, after Task 1)

**Files:**
- Modify: `rust/vtop-ffi/src/remote/consent.rs`, `remote/regno.rs`, `remote/handover.rs`
  (pending ask, timeout, app-left deny)
- Modify: `rust/vtop-ffi/src/remote/backend/live.rs` (follow only while shown; count changes)
- Modify: `core/src/vitapmate/data/CookieConsent.kt`, `RegnoConsent.kt`, `HandoverConsent.kt`
  (sheets read `ConsentAsk`; answers call Rust)
- Modify: `core/src/vitapmate/data/Live.kt` (`rememberLive` and `LiveWake` read Rust's count)
- Delete: `core/test/vitapmate/data/CookieConsentTest.kt` once it's ported

**Interfaces:**
- Consumes: `Runtime.timeout`, `app_shown` and `AppEvents::on_consent` (Task 1);
  `Settings::tunable(CookieAsk)` (Task 2).
- Produces:
  - `enum ConsentAsk { None, Cookie{id: u64, action: String, ...}, Regno{id: u64, ...},
    Handover{id: u64, client: String, ai: bool, ...} }`. Fields come from today's Kotlin
    `pending` types.
  - `Core::consent_answer(flow, id: u64, answer: ConsentAnswer)`
  - `enum ConsentAnswer { Send, Cancel, DontAskAgain, Share, NotNow, Approve, Deny }`.
    Only the variants that fit each kind are accepted; any other answer is logged and treated
    as deny.
  - `Core::live_changes(topic) -> u32`, plus a change push on the existing `LiveListener`.
- The Kotlin `CookieGate` foreign trait goes away. Rust no longer calls into Kotlin to ask; it
  pushes `on_consent` and awaits the answer.

- [ ] **Step 1:** Port `CookieConsentTest`. Add:
  - `asks_of_one_action_share_one_dialog`
  - `timeout_is_cancel`
  - `app_left_is_cancel` (Review Focus 5)
  - `unattended_without_dont_ask_again_goes_direct` (Review Focus 5)
  - `handover_expired_shows_expired`
  - `newer_regno_ask_closes_older`
  - `live_followed_only_while_shown`
- [ ] **Step 2:** Run `cargo test -p vtop-ffi consent regno handover live`. Expected: FAIL.
- [ ] **Step 3:** Implement. Existing `consent::allowed` stays the single gate, and the new
  code only replaces where the question is asked.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check:
  - Switch the route to Server; the cookie dialog shows. Background the app, and it is
    cancelled.
  - "Don't ask again" works.
  - The regno sheet works from Settings → Connections.

### Task 11: Gmail OTP in Rust (lane B, after Task 1)

**Files:**
- Create: `rust/vtop-ffi/src/gmail.rs`
- Create: `rust/vtop-ffi/src/gmail/tests.rs`
- Modify: `core/src/vitapmate/data/Gmail.kt` (becomes a call to Rust plus
  `SignInService` start and stop)
- Modify: `features/login/.../GmailContent.kt`, `GmailLinks.kt` (only if signatures change)

**Interfaces:**
- Consumes: `SecretBox` and `UrlOpener` (Task 1); `HttpTransport` (existing); `Settings`
  (`otp_source`, delete-after-reading).
- Produces, exported:
  - `gmail_parse_client(bytes: Vec<u8>) -> Result<GmailClient, CoreError>`
  - `Core::gmail_connect(flow, client: GmailClient, regno: String) -> GmailAccount`. It binds a
    `std::net::TcpListener` on `127.0.0.1:0` (`accept` inside `smol::unblock`), opens the browser through `UrlOpener` with PKCE
    S256 and a random state, and waits for the redirect with today's timeout.
  - `Core::gmail_disconnect(flow)`
  - `Core::gmail_find_otp(flow, since_ms: i64, tried: Vec<String>) -> Option<GmailOtp>`
  - `Core::gmail_tidy_up(flow, otp)`
  - `Core::gmail_set_delete_after_reading(on)`
  - `Core::gmail_account() -> Option<GmailAccount>`
  - The token is sealed under the **same `SecretBox` name and byte format** `Gmail.kt` uses
    today, so a linked account survives the update.

- [ ] **Step 1: Write the failing tests** with a fake `HttpTransport`:
  - `existing_sealed_token_reads_unchanged`
  - `refreshes_when_expired_and_resaves`
  - `refused_refresh_reports_gmail_refused_code`
  - `find_otp_skips_tried_and_older_than_since`
  - `redirect_with_wrong_state_is_rejected`
  - `tidy_up_deletes_only_when_on`
  - `never_logs_token_or_code`: capture the tracing output and assert it holds no token or
    code value.
- [ ] **Step 2:** Run `cargo test -p vtop-ffi gmail`. Expected: FAIL.
- [ ] **Step 3:** Implement as a port of `Gmail.kt`'s logic (lines 52–590): same endpoints,
  same query, same redirect page text.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check:
  - Link Gmail with a test account (the BYOK client in `memory/otp-testing-plan`), then
    background the app during Google sign-in. The link still finishes.
  - Update over the old build, and the account is still linked.

### Task 12: Biometric fetch step (lane B)

**Files:**
- Create: `rust/vtop-ffi/src/pages/biometric_job.rs`
- Modify: `core/src/vitapmate/data/BiometricFetch.kt` (the worker loops `biometric_step`;
  notifications and WorkManager stay)
- Delete: `core/test/vitapmate/data/BiometricFetchTest.kt` once it's ported

**Interfaces:**
- Produces:
  - `Core::biometric_step(flow, semester: String, budget_ms: u64) -> BiometricStep`
  - `enum BiometricStep { Done{fetched: u32}, More{fetched: u32}, Failed{problem: Problem,
    fetched: u32} }`
  - Rust picks the semester's missing days and fetches a few per request, as today. It returns
    `More` when the budget runs out or VTOP drops after some days came in, which tells the
    worker to append a fresh run. `Failed` with `fetched == 0` lets WorkManager retry.

- [ ] **Step 1:** Port `BiometricFetchTest`. Add `skips_complete_days`,
  `more_after_partial_drop` and `failed_only_when_nothing_fetched`.
- [ ] **Step 2:** Run `cargo test -p vtop-ffi biometric_job`. Expected: FAIL.
- [ ] **Step 3:** Implement. Rust's budget check uses `Clock`.
- [ ] **Step 4:** Run the same tests, then `mise run rust-android-dev && ./kotlin test`.
  Expected: PASS.
- [ ] **Step 5:** Emulator check: Wrapped → Fetch campus times on the demo account answers
  "unavailable" as today. Stop in the notification cancels the run.

### Task 13: Integration and final check (lead)

- [ ] **Step 1:** Rebase both lanes into one change stack on `main`, then run `mise run
  rust-android`.
- [ ] **Step 2:** Check that no domain logic is left in Kotlin. Run `rg -n
  "FetchQueue|SingleFlight|PageFetches|StaleSweep|VtopSync|problemOf\(.*->" core/src
  features app`. Expected: only thin calls into Rust.
- [ ] **Step 3:** Count what's left in `core/src/vitapmate/data`. Expected: Android glue only
  (Cronet, WebSocket, UDP, Keystore, SAF, WorkManager, PDF, services, mirrors).
- [ ] **Step 4:** Run `mise run check`. Expected: PASS.
- [ ] **Step 5:** Run a full emulator pass with the test-on-emulator skill:
  - Update over the current release APK: settings, docs, Gmail link and cached pages kept.
  - Demo account: every tab, pull, tab switch mid-fetch, Fetch all, `SYNC_NOW`, sign-out
    mid-fetch, the consent dialog, and app backgrounding.
  - Diff `mise run state` and the log lines against a run of the parent change.
- [ ] **Step 6:** Update `AGENTS.md`'s "Where a change goes" rows for data, settings and
  logging, plus `rust/ARCHITECTURE.md` (runtime, page watches, settings). Hand back to the user
  for release.
