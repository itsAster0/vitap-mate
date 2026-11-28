# Rust-driven core: design

## Intent

Make the app portable without building iOS now. When this is done, Kotlin in `core/` holds no
domain logic and runs no domain scheduling. A future shell on another platform only draws
screens and implements a few platform services: HTTP, sockets, secure storage, files, opening
URLs, background jobs. Android must look and behave exactly as it does today.

This finishes the AGENTS.md rule "State has one owner": domain state and every calculation on
it live in Rust.

## Goals

- Rust owns and drives page state, the fetch queue, freshness and the open-app sweep,
  background sync and Fetch all, semester choice, sync health, problem mapping, settings and
  tunables, the activity tracker, Gmail OTP, the biometric fetch loop, the docs index, mess
  menu parsing, the consent gates' timeouts, and live-topic following.
- Kotlin keeps Compose UI, temporary UI state, and platform services behind UniFFI foreign
  traits.
- Ship as one release at the end. Every phase lands on `main` and is tested on the emulator.

## Non-goals

- No iOS shell, no Swift bindings, no `rust-ios` task.
- No UI changes. Same screens, copy, toasts and timings.
- No move of the platform I/O that is already behind traits: HTTP (Cronet), `WebSocket.kt`,
  UDP sockets, Keystore, SAF, WorkManager, notifications, widgets, PDF export.
- No audit of `features/` for domain logic. That is a follow-up.

## Anti-goals (a failure even if it works)

- Anyone loses their settings, saved password, Gmail link, docs index or cached pages on
  update.
- Fetch behaviour changes in a way the student can see: more requests, lost dedup, a tab switch
  that cancels a fetch, background work that jumps ahead of the student's page.
- The VTOP session goes to our servers without `consent::allowed`.
- Logs lose `flow` or `regno`, or start carrying secrets.

## Constraints

- No tokio in the app. Rust work that outlives its caller runs on smol (`smol = "2"`, the
  user's choice; measured at about +17 KB stripped). Its `Executor` runs on 2 threads, and a
  `smol::Task` cancels when dropped, so dropping a task's handle is how work gets cancelled.
  Timers go through the `Clock` trait: the real one uses `smol::Timer`, and tests use a manual
  clock. Blocking work (the Gmail loopback `accept`) runs in `smol::unblock`. vtop-core keeps
  `vtop_core::sync::sleep`. smol's socket I/O stays unused, because the network stays platform
  I/O.
- smol is the only new crate. Spreadsheet decoding (`Spreadsheet.kt`: zip and XML) stays as Kotlin platform
  code because the tree has no zip or XML crate. Its output, `Map<String, List<List<String>>>`,
  goes to Rust's mess menu parser.
- Spec names, args and cache files never change.
- Pure transitions follow the `next(state, event) -> (state, effects)` style. Wire and disk
  formats use explicit Wire/Stored types with roundtrip tests.
- Tests use no sleeps or wall-clock time. Pass `now` in, and drive the executor with a manual
  clock in tests.

## Approaches considered

- **A. Rules in Rust, scheduling in Kotlin.** About 1–1.5 weeks. A new platform would rewrite
  about 300 lines of driver. Rejected by the user: Kotlin would still drive.
- **B. Rust drives too (chosen).** Rust runs the queue, sweep and sync on its own executor and
  pushes state to Kotlin through listeners. About 2–3 weeks.
- **B with tokio.** Rejected: a bigger `.so`, and the crate avoids tokio on purpose.
- **B on the `futures` thread pool.** No new crates, but it needs hand-made cancellation
  (`Abortable`) and timeouts. The user picked smol instead, for its task handles, timer and
  `unblock`.

## Design

### Runtime and host services

- `Core` gains one `Runtime`: a `smol::Executor` run by 2 threads, plus `spawn(flow, future)
  -> smol::Task<T>`. Dropping the task cancels it, and `.detach()` lets it run on. The fetch
  queue's runs, the sweep, sync runs and consent timeouts use it.
- Platform services are UniFFI foreign traits. The existing ones stay as they are:
  `HttpTransport`, `ConvexNetwork`/socket, `UdpNetwork`, `PasswordCipher`, `CookieGate`,
  `OtpListener`, `LiveListener`. New ones:
  - `SecretBox`: `seal(name, bytes)`, `open(name) -> Option<bytes>`. Keystore AES-GCM, as in
    `Secrets.kt`.
  - `UrlOpener`: `open(url)`, for Gmail OAuth.
  - `AppEvents`, one listener for app-wide pushes:
    - `on_background_problem(Problem)`
    - `on_activity(Vec<ActivityEntry>)`
    - `on_settings(Settings)`
    - `on_semesters(SemesterState)`
    - `on_sync_health(SyncHealth)`
    - `on_consent(ConsentAsk)`, where `ConsentAsk` is an enum: `None`, `Cookie{..}`,
      `Regno{..}`, `Handover{..}`
- The app being on screen is told to Rust with `Core::app_shown(shown: bool)`, from the
  activity's `onStart`/`onStop`. Rust counts resumes, wakes page watches, and cancels open
  consent asks when the app leaves.

### Pages

- Every `rememberPage` id becomes a Rust enum `PageId`: `Attendance`, `AttendanceDetail`,
  `Timetable`, `Calendar`, `Marks`, `Exams`, `Grades`, `GradeHistory`, `Profile`,
  `GeneralOuting`, `WeekendOuting`, `Biometric`, `CoursePage`, `CoursePageCourse`.
  `PageId::name()` returns today's string ids. Logs, flows and queue keys keep them.
- `Core::page_watch(flow, page: PageId, item: String, listener: Arc<dyn PageListener>) ->
  Arc<PageWatch>` opens a watch:
  - Rust reads the cache first, then decides the open fetch with today's `refreshTrigger`
    rules (rejoin, just-fetched, auto refresh, stale, the 6 h max age).
  - It runs fetches through the queue, follows resumes, and logs `ui.<id>` with the same
    fields.
  - `PageWatch` has `refresh(trigger, user)`, `shown(on)` and `close()`.
  - `PageListener::on_status(PageStatus)`.
- `PageStatus` is today's `PageState` without the data: `Loading{waiting}`,
  `Ready{updated_at, activity}`, `Failed{problem}`. `PageActivity` is `Idle`,
  `Refreshing{waiting}` or `Stale{problem}`.
- UniFFI has no generics, so data does not ride the status. When `updated_at` changes, Kotlin
  reads it with the screen's typed `cached_x` call, which is how `readCache` works today.
  `rememberPage` keeps its signature minus the `fetch` and `updatedAt` lambdas, which Rust now
  owns per `PageId`. The 14 call sites change by one line each.
- "This screen fetched this semester already" was `rememberSaveable` state. It becomes a
  per-key set in Rust for the process's life. **My call:** after process death, a page can
  fetch once more on open when Auto Refresh is on. That is acceptable and simpler.
- The queue is a direct port of `FetchQueue` and `SingleFlight`: keys, lanes, priorities, drop
  after `QueueDrop`, slow-link single lane, cooldown and `succeeded_within`.
- Problem mapping moves to Rust: `Problem{kind, title, message, flow, retryable,
  user_initiated}` with the exact texts from `problemOf`, plus `Problem::toasts(settings)`.
  Kotlin's `problemOf` becomes a one-line call for `CoreException`; other throwables stay
  `Unexpected`.

### Freshness and sync

- `CalendarRefresh`, `ExamDayRefresh` and `StaleSweep` become Rust. The page engine calls them
  where Kotlin did: after a fetch, when the timetable watch opens, and after an attendance
  refresh.
- `VtopSync.run` becomes `Core::sync(flow, kind: SyncKind, page_timeout_ms: Option<u64>, otp:
  Arc<dyn SyncOtp>) -> SyncResult`.
  - `SyncKind` is `Background` or `FetchAll`.
  - `SyncOtp::get(challenge) -> bool` keeps today's split: auto-fill for background, the
    prompt for Fetch all.
  - `BackgroundSync.kt` (WorkManager) and `FetchAll.kt` just call it.
  - `Histories` moves the same way.
- `SemesterChoice` and `SyncHealth`, with their persistence, move to Rust and reach Kotlin
  through `AppEvents`.

### Settings

- Rust owns `settings.json` in the app files dir. It is not a student file, so it survives
  sign-out, as SharedPreferences does today.
  - `SettingsStore` is its own UniFFI object, opened synchronously in `Application.onCreate`
    before the first frame, so the theme never flashes. Core holds it.
  - `Settings` is one record. `SettingsStore::set(change: SettingChange)` takes one variant
    per field, persists, and pushes `on_settings`.
  - `Tunable` with key, title, unit, default, range and note moves to Rust
    (`tunables() -> Vec<TunableInfo>`).
- Import: when `settings.json` is missing, Kotlin reads every SharedPreferences key into
  `Map<String, PrefValue>` and calls `SettingsStore::import(map)`. Rust parses it with the
  Flutter-era keys, writes the file, and Kotlin clears the prefs only after the write succeeds.
- Kotlin's `Settings` object keeps its property API, so the 295 call sites don't change. It
  becomes a Compose-state mirror of the last pushed record, and setters call `set`. This mirror
  is a view, not a second owner.

### Sign-in, Gmail, biometric

- Gmail: Rust owns the client file parse, the OAuth PKCE flow, and the loopback listener on
  `std::net::TcpListener`, whose `accept` runs in `smol::unblock`. It also owns token refresh, the Gmail
  API calls over `HttpTransport`, OTP search and tidy-up, and delete-after-reading. Tokens are
  sealed through `SecretBox`. `SignInService` (the Android foreground service) stays and wraps
  the call.
- Biometric fetch: the day picking, chunking and "append a fresh run" decision become
  `Core::biometric_step(flow, semester, budget_ms) -> BiometricStep{Done, More{fetched},
  Failed{problem}}`. `BiometricFetchWorker` loops it.

### Docs, downloads, mess menu

- Docs index: Rust already has `StoredDocIndex`. It now owns the list and its operations: add,
  rename, set as mess menu, opened, save view, remove, query (`queryDocs`), and newest mess
  menu.
- SAF reads, writes and moves stay in Kotlin `Downloads`. Rust owns the saved-by-name
  bookkeeping that `Downloads` keeps for folder moves.
- The mess menu parser (`parseMessMenu`, `resolveMessItem`, meal windows) moves to Rust and
  takes the decoded tables.

### Consent and live

- `CookieConsent`, `RegnoConsent` and `HandoverConsent` keep only their sheets. Rust holds the
  pending ask, the `CookieAsk` timeout, "app left means deny", and the dedup of one action's
  asks. It pushes `ConsentAsk`, and Kotlin answers with `Core::consent_answer(id, answer)`.
- `Live`: Rust follows a topic only while `app_shown`, and counts changes. Kotlin's
  `rememberLive` becomes a listener on `Core::live_watch` that the existing count feeds.

## Left to the builder

- Module layout inside `rust/vtop-ffi/src`: suggested `runtime.rs`, `schedule/` (queue,
  single flight, sweep), `page_watch/`, `settings/`, `sync/`, `gmail/`, `docs.rs`,
  `mess_menu.rs`.
- Exact field names of the new records, as long as they match the Kotlin ones they replace.
- Which Kotlin tests port to Rust one to one. All of `FetchQueueTest`, `PageEventTest`,
  `PageActivityTest`, `RefreshTriggerTest`, `SemesterStateTest`, `SyncHealthTest`,
  `VtopSyncTest`, `ProblemTest`, `DocsTest`, `MessMenuTest`, `ActivityTest`,
  `BiometricFetchTest`, `CookieConsentTest`, `ForegroundTest` and `SingleFlightTest` should.
  Delete the Kotlin copy once the Rust one passes.
