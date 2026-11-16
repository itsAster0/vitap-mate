# Semester Wrapped Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An end-of-semester story about the selected semester. It opens from the top of
More, every screen shares as an image, and campus times are fetched only on request.

**Architecture:**
- The attendance maths still in Kotlin moves into `rust/vtop-ffi/src/records.rs`.
- A new `rust/vtop-ffi/src/wrapped.rs` builds a `WrappedReport` from the cached pages.
- A new `features/wrapped` module draws the story and shares screens as PNGs.
- Campus times come from a foreground WorkManager job in `core`. The background sync also adds
  recent biometric days.

**Tech Stack:** Rust (UniFFI 0.32), Kotlin, Compose Multiplatform (Android), WorkManager 2.11,
Kotlin Toolchain (`./kotlin`), mise.

**Spec:** `docs/superpowers/specs/2026-11-16-semester-wrapped-design.md`

## Global Constraints

- **Repo rules:** read `AGENTS.md` and `rust/ARCHITECTURE.md` first.
  - Use jj, and no mutating git commands.
  - **Don't commit unless the user asks.** Each task's commit step is "describe when told".
- **Calling Rust:** call it off the main thread, once per screen state. The flow id is the first
  argument of every `Core` call.
- **Flows:**
  - Every user action is a `flow()`: `wrapped.open`, `wrapped.share`, `wrapped.biometric`.
  - Background runs are `flow(name, unattended = true)`.
- **Logs:** never log cookies, passwords, OTPs, CSRF tokens or keys.
- **Style:**
  - Keep the same UI and feel elsewhere.
  - M3 Expressive and theme colours only.
  - Write the fewest lines that do the job.
- **Features:**
  - A feature depends on `//design` only and never imports another feature.
  - Shared data code goes in `core`.
- **Tunables:** live in `Tunable` in `core/src/vitapmate/data/Settings.kt`.
- **No new dependencies.** `androidx.work:work-runtime:2.11.2` moves from `app/module.yaml` to
  `core/module.yaml`, and that's all.
- **Rust:**
  - Google Rust style.
  - Run `cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
  - After changing `vtop-ffi`, run `mise run rust-android`.
- **Shared images:** 1080×1920 PNG. The registration number never appears; the first name does.
  The footer is "vitapmate" plus the Play link
  `https://play.google.com/store/apps/details?id=com.vitap_pal.app`.
- **Session age limit:** 900 s (15 minutes).
- **Biometric:**
  - Fetch at most 14 days per `fetch_biometric_range` call.
  - A cached day is final only when `is_complete` says so.
- **Testing:** on the emulator (`./kotlin run -d emulator-5554`), not only with a build.

## Review Focus

1. **A semester with no "Instructional Day" entries in the calendar** (an old semester, or a
   calendar fetch failed). Expected: `cached_wrapped` returns `None`, there's no More card, and
   nothing crashes. Test in Task 3.
2. **A course with zero counted classes,** or a lab with an odd period count. Expected: no
   divide-by-zero; the course is skipped from best/least and still counts in totals. Test in
   Task 3.
3. **Leave requests that span the semester edge, or have rejected or pending status.**
   Expected: only approved requests whose start falls inside the window count. Test in Task 3.
4. **The OTP needed during a background campus fetch.** Expected: the worker stops with "Open
   vitapmate to sign in and continue", with no retry loop. Test in Task 5 (the
   `CoreError.OtpRequired` path stops the work).
5. **The student switches semester while Wrapped is open, or while the fetch runs.** Expected:
   - the screen reloads for the new semester;
   - the worker keeps fetching its own semester's dates (they're passed in, not read from the
     current pick).
   Checked in Task 6 (on screen) and Task 5 (inputs).

---

### Task 1: Move the attendance maths leftovers to Rust

**Files:**
- Modify: `rust/vtop-ffi/src/records.rs`
  - `AttendanceCourse` gets `debar_line`.
  - The advice gets the lab suffix.
  - `AttendanceDetail` gets the counts and the streak.
  - Tests.
- Modify: `features/attendance/src/vitapmate/features/attendance/Standing.kt`
  - `advice()` and `debarLine()` read Rust's fields.
  - `projected()` stays as it is.
- Modify: `features/attendance/src/vitapmate/features/attendance/Insights.kt`
  - It takes the counts from `AttendanceDetail`.
- Modify: its caller (`CourseDetail.kt` or wherever `Insights(classes)` is called).
- Generated: `core/src/vitapmate/core/vtop_ffi.kt`, via `mise run rust-android`.

**Interfaces:**
- Produces, on `AttendanceCourse`:
  - `advice: String`, now with " lab"/" labs" after the count for labs, exactly as Kotlin's
    `labs(n)` does today. For example: "Can skip 2 labs", "Attend 1 lab", "Don't skip".
  - `debar_line: Option<String>`, same text as Kotlin's `debarLine()`, for example
    "Debarred for CAT-II · Permitted · Clear for FAT at 3 more".
- Produces, on `AttendanceDetail`:
  - `present: i32`, `on_duty: i32`, `absent: i32`, counting posted classes only;
  - `streak: i32`, the current run of present or OD from the newest posted class;
  - `last_missed: Option<i32>`, the day of the newest absence.
- Produces `pub(crate) fn longest_run(classes: &[(i32, Option<i32>, Option<ClassStatus>)]) -> Option<(i32, i32, i32)>`,
  taking `(day, start, status)` and returning `(length, first_day, last_day)`:
  - classes sorted by day, then start;
  - unposted classes (`status == None`) skipped, not breaking the run;
  - present and OD extend the run;
  - absent breaks it.
  Task 3 uses it.

- [ ] **Step 1: Write the failing tests** in `records.rs` `mod tests`:

```rust
#[test]
fn advice_names_labs() {
    // lab, 10 of 12 periods (5 of 6 sessions), safe: the count is in sessions
    let c = lab_course(10, 12);
    assert!(c.advice.ends_with(" labs") || c.advice.ends_with(" lab") || c.advice == "Don't skip");
}
#[test]
fn debar_line_matches_kotlin() {
    let c = debarred_course("CAT-II", true, Some("FAT"), /*safe*/ false, /*must*/ 3);
    assert_eq!(c.debar_line.as_deref(), Some("Debarred for CAT-II · Permitted · Clear for FAT at 3 more"));
}
#[test]
fn detail_counts_and_streak() {
    // newest first: P, OD, P, A, P, unposted
    let d = detail(&[Some(P), Some(OD), Some(P), Some(A), Some(P), None]);
    assert_eq!((d.present, d.on_duty, d.absent, d.streak), (3, 1, 1, 3));
    assert_eq!(d.last_missed, Some(day_of(3)));
}
#[test]
fn longest_run_skips_unposted_and_orders_by_time() {
    let run = longest_run(&[(1, Some(480), Some(P)), (1, Some(540), None), (2, Some(480), Some(OD)), (3, Some(480), Some(A)), (4, Some(480), Some(P))]);
    assert_eq!(run, Some((2, 1, 2)));
}
```

  Build the helpers (`lab_course`, `debarred_course`, `detail`, `day_of`) from the module's
  existing test builders.

- [ ] **Step 2: Run** `cd rust && cargo test -p vtop-ffi records`. Expected: FAIL. The fields
  and `longest_run` don't exist yet.
- [ ] **Step 3: Implement** the fields where `AttendanceCourse` and `AttendanceDetail` are
  built in `records.rs`, plus `longest_run`.
  - The debar text moves from `Standing.kt:71-79`.
  - The lab suffix moves from `Standing.kt:65`.
- [ ] **Step 4: Run** `cargo test -p vtop-ffi`. Expected: PASS. Then run fmt and clippy.
- [ ] **Step 5: Switch Kotlin over.**
  - `advice(showSkip)` becomes `when { total == 0 -> null; !safe -> advice; !showSkip -> null; else -> advice }`.
  - `debarLine()` and `labs()` are deleted where nothing else uses them (`rg -n "labs\(|debarLine" features`).
  - `Insights` takes `AttendanceDetail` (or its counts) instead of counting.
  - `projected()` is unchanged.
- [ ] **Step 6: Run** `mise run rust-android && ./kotlin build -m app -v debug`. Expected:
  "Build successful".
- [ ] **Step 7: Check on the emulator.** The attendance list, a theory course detail, a lab
  course detail, the calculator, the skip-day sheet and a debarred course (if any in the cache)
  must look the same as before. Compare against screenshots taken before Step 5.
- [ ] **Step 8: Commit when the user asks:** `refactor(attendance): advice, debar line and counts from Rust`.

---

### Task 2: Session age and a fresh sign-in on demand

**Files:**
- Modify: `rust/vtop-ffi/src/lib.rs`, next to `relogin` (around line 1050).
- Test: `rust/vtop-ffi/src/tests.rs`

**Interfaces:**
- Produces `#[uniffi::export] async fn ensure_fresh_session(&self, flow: String, max_age_secs: u64) -> Result<(), CoreError>`
  on `Core`:
  - reads the current slot's `session.client.session_state().logged_in_at`;
  - when that's `None` or older than `max_age_secs`, signs in again through the same code as
    `relogin` (same OTP handling: returns `CoreError::OtpRequired` when VTOP wants a code);
  - otherwise returns `Ok(())` without a VTOP call.
  - Extract the login body from `relogin` into one private `async fn sign_in_again(&self, session: &mut Session) -> Result<(), CoreError>`
    that both call. No copy.
- Kotlin name: `core.ensureFreshSession(flow, 900uL)`.

- [ ] **Step 1: Write the failing tests** in `tests.rs`, with the existing mock-VTOP helpers:
  - `fresh_session_skips_sign_in`: `logged_in_at` is now − 60, max 900, so there's no login
    request (the mock's login hit count stays 0).
  - `stale_session_signs_in_again`: `logged_in_at` is now − 1000, so the login count is 1 and
    `logged_in_at` is updated.
  - `stale_session_needing_otp_returns_otp_required`: the mock answers the login with the OTP
    page, so the call returns `Err(CoreError::OtpRequired { .. })`.
- [ ] **Step 2: Run** `cargo test -p vtop-ffi fresh_session stale_session`. Expected: FAIL.
- [ ] **Step 3: Implement** `ensure_fresh_session` and the extracted `sign_in_again`.
- [ ] **Step 4: Run** `cargo test -p vtop-ffi`. Expected: PASS. Then run clippy.
- [ ] **Step 5: Commit when the user asks:** `feat(session): sign in again when the session is older than asked`.

---

### Task 3: `wrapped.rs`, the facts

**Files:**
- Create: `rust/vtop-ffi/src/wrapped.rs`, with its own `#[uniffi::export] impl Core` block and
  tests in the file.
- Modify: `rust/vtop-ffi/src/lib.rs` to add `mod wrapped;`.

**Interfaces:**
- Consumes `records::longest_run` from Task 1.
- Reads the selected semester's cached pages directly:
  - `page_path` for attendance, attendance detail per course, timetable, calendar, marks and
    grades;
  - the raw outing page data (`GeneralOutingData` / `WeekendOutingData` records with ISO
    `from_date`, `to_date` and `date`). Use these, because `OutingRequest` only has display
    text.
- Produces these records (all `uniffi::Record`, every fact optional):

```rust
pub struct WrappedReport {
    pub semester_id: String,
    pub semester_name: String,
    pub first_day: i32,            // days since 1970-01-01, India
    pub last_day: i32,             // last "Instructional Day" in the calendar
    pub window_end: i32,           // the calendar's last entry: the outing window's end
    pub courses: i32,
    pub class_day_count: i32,
    pub holidays: i32,
    pub attendance: Option<WrappedAttendance>,
    pub streak: Option<WrappedStreak>,
    pub never_missed: Vec<String>,             // course names; empty skips the slide
    pub comeback: Option<WrappedComeback>,
    pub habits: Option<WrappedHabits>,
    pub time: Option<WrappedTime>,
    pub places: Option<WrappedPlaces>,
    pub marks: Option<WrappedMarks>,
    pub exams: Option<WrappedExams>,
    pub grades: Option<WrappedGrades>,
    pub cgpa: Option<WrappedCgpa>,
    pub outings: Option<WrappedOutings>,
    pub campus: Option<WrappedCampus>,
    pub previous: Option<WrappedPrevious>,
    pub title: WrappedTitle,
    pub title_without_grades: WrappedTitle,    // the title when Hide grades is on
    pub class_days: Vec<i32>,                  // every instructional day, for the campus fetch
}
pub struct WrappedAttendance { pub percent: i32, pub attended: i32, pub counted: i32, pub on_duty: i32, pub best: String, pub best_percent: i32, pub least: String, pub least_percent: i32 }
pub struct WrappedStreak { pub length: i32, pub first_day: i32, pub last_day: i32, pub perfect_weeks: i32 }
pub struct WrappedComeback { pub low_percent: i32, pub low_day: i32, pub final_percent: i32 }
pub struct WrappedHabits { pub worst_weekday: Option<i32>, pub worst_weekday_misses: i32, pub misses: i32, pub early_percent: Option<i32>, pub early_classes: i32, pub first_slot_days: i32, pub last_slot_days: i32 }
pub struct WrappedTime { pub classes: i32, pub minutes: i32, pub lab_minutes: i32, pub busiest_weekday: i32 }
pub struct WrappedPlaces { pub room: Option<String>, pub room_minutes: i32, pub faculty: Option<String>, pub faculty_classes: i32 }
pub struct WrappedMarks { pub best_cat: Option<WrappedCat>, pub best_course: Option<String>, pub best_course_percent: i32, pub cat_change: Option<i32>, pub most_improved: Option<String> }
pub struct WrappedCat { pub course: String, pub exam: String, pub percent: i32 }
pub struct WrappedExams { pub cats: i32, pub fats: i32, pub top_venue: Option<String> }
pub struct WrappedGrades { pub gpa: f64, pub mix: Vec<GradeCount>, pub s_count: i32, pub s_record: bool }
pub struct WrappedCgpa { pub before: Option<f64>, pub after: f64, pub credits_semester: f64, pub credits_total: f64 }
pub struct WrappedOutings { pub leaves: i32, pub weekends: i32, pub top_place: Option<String>, pub longest_leave_days: Option<i32>, pub days_away: i32, pub longest_stay_days: i32 }
pub struct WrappedCampus { pub days_cached: i32, pub days_total: i32, pub earliest_entry: Option<String>, pub latest_return: Option<String>, pub days_on_campus: i32 }
pub struct WrappedPrevious { pub semester_name: String, pub attendance_change: Option<i32>, pub gpa_change: Option<f64> }
pub enum WrappedTitle { Comeback, IronStreak, EarlyRegular, Topper, Explorer, LabRat, Regular, FreeSpirit }
```

- Produces exports:
  - `pub fn cached_wrapped(&self, flow: String) -> Option<WrappedReport>`, which is `None`
    when the semester's classes aren't over (today ≤ `last_day`) or there's no calendar;
  - `pub fn wrapped_missing_days(&self, flow: String) -> Vec<i32>`, the class days without a
    complete cached biometric copy (`is_complete`).
  - `pub async fn fetch_wrapped_previous(&self, flow: String) -> Result<(), CoreError>`, which
    fetches the next older semester's attendance into its own pages folder when it isn't
    cached. It makes one VTOP request, and none when there is no older semester.
- Kotlin names: `cachedWrapped(flow)`, `wrappedMissingDays(flow)`, `fetchWrappedPrevious(flow)`.
- The Files fact isn't in the report. Task 6 counts it in Kotlin from `Docs`.

The fact rules and the title table are in the spec's "How each fact is worked out" and "Title
rules". Copy the numbers from there exactly. Pinned here because tests use them:
- `percent` is `ceil(Σattended × 100 / Σcounted)`.
- The comeback starts at the 20th posted class and needs `final − low ≥ 5`.
- The title order: Comeback (≥10 points), IronStreak (≥40), EarlyRegular (≥90% over ≥15
  classes), Topper (GPA ≥ 9.0), Explorer (≥8 outings), LabRat (lab ≥40% of minutes), Regular
  (≥90%), FreeSpirit.
- `title_without_grades` runs the same list without Topper.
- Campus: the earliest entry is the minimum first `Entry` punch minute; the latest return is
  the maximum last `Entry` punch minute after 17:00.

- [ ] **Step 1: Write the failing tests** in `wrapped.rs` `mod tests`. Write the cached pages
  with `Core::write` into a temp core, as other modules' tests do:
  - `none_before_classes_end`: `last_day` is tomorrow, so the result is `None`.
  - `none_without_instructional_days` (Review Focus 1).
  - `pooled_percent_rounds_up`: two courses, 31/34 and 10/12, so `percent == 90` (41/46 = 89.1,
    rounded up to 90).
  - `zero_counted_course_skipped_from_best_least` (Review Focus 2).
  - `streak_runs_across_courses_in_time_order`.
  - `worst_weekday_and_early_percent`.
  - `outings_window_and_status` (Review Focus 3): a leave starting the day before `first_day`
    is not counted; a pending one isn't counted; one approved inside the window is; a weekend
    on `window_end` is.
  - `grades_only_when_posted`.
  - `perfect_weeks_need_a_posted_class_and_no_absence`.
  - `never_missed_lists_only_full_courses`.
  - `comeback_needs_twenty_classes_and_five_points`: a low of 70 and a final of 74 gives
    `None`; a low of 70 and a final of 92 gives `Some`.
  - `room_and_faculty_by_attended_classes`.
  - `cat_change_averages_courses_with_both`.
  - `cgpa_before_leaves_this_semester_out`.
  - `s_record_only_when_it_beats_every_earlier_semester`.
  - `days_away_clips_to_the_window`.
  - `title_follows_the_order`: a report matching both IronStreak and Regular gives
    IronStreak; GPA 9.2 alone gives Topper, and `title_without_grades` gives FreeSpirit.
  - `previous_semester_change`: with the older semester's attendance cached, the change is
    the difference of the two pooled percents; without it, `attendance_change` is `None`.
  - `missing_days_skip_complete_cache`: three class days, one cached complete and one cached
    incomplete, so `wrapped_missing_days` returns the other two.
- [ ] **Step 2: Run** `cargo test -p vtop-ffi wrapped`. Expected: FAIL.
- [ ] **Step 3: Implement** `wrapped.rs`. Keep one private builder per fact, so each test
  reads one function.
- [ ] **Step 4: Run** `cargo test -p vtop-ffi`. Expected: PASS. Then run fmt, clippy and
  `mise run rust-android`.
- [ ] **Step 5: Commit when the user asks:** `feat(wrapped): the semester's facts from the cached pages`.

---

### Task 4: Background sync adds recent biometric days

**Files:**
- Modify: `core/src/vitapmate/data/VtopSync.kt:108-124` (the page list).
- Modify: `core/src/vitapmate/data/Settings.kt`, to add a `Tunable`.

**Interfaces:**
- Produces `Tunable.SyncBiometricDays("dev_sync_bio_days", "Biometric days per sync", "days", 3, 0..7, "Recent days not final in the cache, newest first; 0 turns it off")`.
- Consumes the existing `cachedBiometricRange(flow, from, to)` and
  `fetchBiometricRange(flow, from, to)`.

- [ ] **Step 1: Add** the page entries.
  - Take today's day number (India) and look back 14 days with `cachedBiometricRange`.
  - Pick the newest N days whose `complete` is false (today is always picked), where N is the
    tunable.
  - Add one page per day: `add("biometric.$day" to { it.fetchBiometricRange(f, day, day) })`.
- [ ] **Step 2: Run** `./kotlin build -m app -v debug`. Expected: success.
- [ ] **Step 3: Check on the emulator.** Run a sync from Developer (or `mise run logs` while it
  runs).
  - The log shows `GET` lines for `biometric` with at most 3 days.
  - A second sync fetches only today.
- [ ] **Step 4: Commit when the user asks:** `feat(sync): fetch recent biometric days`.

---

### Task 5: Campus-times fetch worker

**Files:**
- Create: `core/src/vitapmate/data/BiometricFetch.kt`, containing
  `object BiometricFetch { fun start(context: Context, semester: String, days: List<Int>) }`,
  `class BiometricFetchWorker : CoroutineWorker` and `fun state(context, semester): Flow<WorkInfo?>`.
- Modify:
  - `core/module.yaml`: add `androidx.work:work-runtime:2.11.2`.
  - `app/module.yaml:21`: remove it.
- Modify: `app/src/AndroidManifest.xml`:
  - `FOREGROUND_SERVICE_DATA_SYNC`;
  - `<service android:name="androidx.work.impl.foreground.SystemForegroundService" android:foregroundServiceType="dataSync" tools:node="merge" />`.
- Modify: `Settings.kt`, to add `Tunable.CampusFetchGap("dev_campus_gap_s", "Campus fetch gap", "s", 3, 0..30, "Pause between biometric days in the Wrapped fetch")`.

**Interfaces:**
- Consumes `core.ensureFreshSession(flow, 900uL)` (Task 2), `fetchBiometricRange`, and
  `wrappedMissingDays` (Task 3, used by the caller).
- Produces:
  - unique work name `wrapped.biometric.$semester`, policy `KEEP`, constraint
    `NetworkType.CONNECTED`;
  - input `days: IntArray`;
  - progress `done: Int`, `total: Int`.
- Notification:
  - channel `campus_fetch` ("Campus times");
  - text `"Fetching campus times · $done of $total days"`;
  - a **Stop** action from `WorkManager.createCancelPendingIntent(id)`;
  - tapping it opens `MainActivity` with the Wrapped route.

- [ ] **Step 1: Implement** the worker.
  - It runs under `flow("wrapped.biometric")`, not unattended.
  - It calls `setForeground(ForegroundInfo(id, notification, FOREGROUND_SERVICE_TYPE_DATA_SYNC))`.
  - Then `ensureFreshSession`.
  - Then, for the missing days in chunks of ≤14, one `fetchBiometricRange(f, d, d)` per day,
    with the gap delay, updating progress.
  - Return:
    - `Result.success()` at the end;
    - on `CoreException.OtpRequired` or `InvalidCredentials`, post the notification "Open
      vitapmate to sign in and continue" and return `Result.failure()` (no retry; Review
      Focus 4);
    - on network errors, `Result.retry()`.
- [ ] **Step 2: Run** `./kotlin build -m app -v debug`. Expected: success, and `SyncWorker`
  still compiles (now through core).
- [ ] **Step 3: Check on the emulator.**
  - Start a fetch for a semester with ~20 missing days.
  - The notification counts up; Stop cancels it.
  - Starting again resumes with fewer days.
  - Background the app and it keeps going.
  - Run `adb shell cmd jobscheduler` if needed to see it's foreground.
- [ ] **Step 4: Check the OTP path.**
  - In Developer, drop the session (keep the account) on an account that needs an OTP.
  - Start a fetch. The notification says "Open vitapmate to sign in and continue" and the work
    is FAILED, not retrying.
- [ ] **Step 5: Commit when the user asks:** `feat(wrapped): fetch campus times in a foreground job`.

---

### Task 6: `features/wrapped`, the story and sharing

**Files:**
- Create: `features/wrapped/module.yaml`, copied from `features/attendance/module.yaml` with
  namespace `vitapmate.features.wrapped`.
- Create in `features/wrapped/src/vitapmate/features/wrapped/`:
  - `WrappedFeature.kt` (`val wrappedFeature: Feature`, hidden from nav groups and opened by
    route);
  - `WrappedScreen.kt` (loading, story pager, progress bar, taps, swipe down);
  - `Slides.kt` (one composable per screen and the summary card);
  - `ShareImage.kt` (capture and share).
- Modify: `project.yaml`, adding `features/wrapped`. Modify:
  `app/src/vitapmate/Features.kt`, adding `wrappedFeature`.
- Modify: `core/src/vitapmate/data/Settings.kt`:
  - `wrappedSeen: Set<String>` (semester ids opened);
  - `wrappedHideGrades: Set<String>` (semester ids with grades hidden).

**Interfaces:**
- Consumes `cachedWrapped`, `wrappedMissingDays` (Task 3), `BiometricFetch.start` and
  `BiometricFetch.state` (Task 5), plus the existing fetches.
- Produces the route id `WRAPPED`, which Task 7 uses.

Slide order: the 19 slides in the spec's table, in that order. A slide whose facts are all
missing is skipped, and the progress bar has one segment per slide that exists.

Titles shown for `WrappedTitle`: "The Comeback", "The Iron Streak", "The 8 AM Regular",
"The Topper", "The Explorer", "The Lab Rat", "The Regular", "The Free Spirit".

The Files slide: `Docs` entries with `addedAt` between `first_day` and `window_end`, their
count, and the `source` with the most. It's skipped when the count is 0.

The Summary slide has one tile per existing slide, with that slide's headline number.

Visual reference: the mockups in `docs/superpowers/specs/wrapped-mockups/` (open the HTML
files in a browser). New slides follow the same pattern: one tone per slide, one large shape holding
the number.

The Campus screen:
- shows the facts, plus "from <days_cached> of <days_total> days" when they differ;
- shows the row "<k> days missing · Fetch" (or "Add campus times · <m> days to fetch from VTOP")
  while `wrappedMissingDays` isn't empty;
- shows the worker's progress while it runs.

- [ ] **Step 1: Loading.**
  - Draw from `cachedWrapped(f)` at once.
  - Then fetch, one at a time inside `flow("wrapped.open")`: attendance, each course's
    attendance detail, timetable, calendar, marks, grades (with the selected semester), grade
    history, exams, general outing, weekend outing, then `fetchWrappedPrevious`.
  - Then read `cachedWrapped` again.
  - A failure logs `Log.warn` and continues.
  - Show "Putting your semester together…" only while there's nothing to draw.
  - Key everything on the selected semester id, so switching semester reloads (Review Focus 5).
- [ ] **Step 2: Story pager.** Tap the right third to go forward and the left third to go
  back. Use a segmented progress bar, M3 Expressive motion, theme colours only.
- [ ] **Step 3: Share.**
  - `ShareImage.kt` draws the current slide into a `rememberGraphicsLayer()` at 1080×1920.
  - It saves `share/wrapped-<semester>-<slide>.png` under the cache dir and shares through
    the existing `.logs` FileProvider (as `PdfExport.kt` does).
  - The image carries the first name from `cachedProfile` if present, never the registration
    number, and the footer.
  - Hide grades (on the Marks, Grades, CGPA, Last semester, Title and Summary slides) changes
    the image only, as the spec's rules say, and uses `title_without_grades`. It is remembered
    in `wrappedHideGrades`.
  - Flow: `wrapped.share`.
- [ ] **Step 4: Run** `./kotlin build -m app -v debug`. Expected: success.
- [ ] **Step 5: Check on the emulator** with a finished semester selected:
  - every slide shows;
  - Share opens the sheet with a 1080×1920 PNG (`adb pull` it and look at it);
  - Hide grades removes grades and marks from the image;
  - switching semester reloads;
  - a semester with no grades has no grades slide.
- [ ] **Step 6: Commit when the user asks:** `feat(wrapped): the semester story and share images`.

---

### Task 7: The More card

**Files:**
- Modify: `app/src/vitapmate/MoreScreen.kt:69-95`, adding an item above `changes.new`.

**Interfaces:**
- Consumes `cachedWrapped` (Task 3), `Settings.wrappedSeen` (Task 6) and the route `WRAPPED`
  (Task 6).

- [ ] **Step 1: Implement.**
  - Read `cachedWrapped(f)` off the main thread when More opens, keyed on the selected
    semester.
  - When it's present and the semester isn't in `wrappedSeen`, show the card:
    - the same style as the changes card;
    - icon `Lucide.Sparkles` in a `MaterialShapes.Sunny` badge;
    - title "Your <semester_name>, wrapped";
    - subtitle "<first_day d MMM> – <last_day d MMM> · tap to see";
    - test tag `more.wrapped_card`.
  - When it's present and the semester has been seen, show a one-line row "<semester_name>
    wrapped" with a chevron and test tag `more.wrapped_row`.
  - Opening adds the semester to `wrappedSeen`.
- [ ] **Step 2: Run** `./kotlin build -m app -v debug`, then check on the emulator:
  - the card shows for a finished semester and not for the current one;
  - after one open it becomes the row;
  - the changes card still shows below it.
- [ ] **Step 3: Commit when the user asks:** `feat(more): Semester Wrapped card`.

---

### Task 8: Docs and the full check

**Files:**
- Modify:
  - `rust/ARCHITECTURE.md`: a line on `wrapped.rs` and the campus fetch;
  - `AGENTS.md` repo map, only if a new top-level path needs it (it shouldn't).

- [ ] **Step 1: Run** the full checks from AGENTS.md:
  - `cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`;
  - `./kotlin test -m core`;
  - `./kotlin build`.
  Expected: all pass.
- [ ] **Step 2: Run** `mise run release` and compare the APK size with
  `build/vitapmate-release.apk` before this work. Report the change.
- [ ] **Step 3: Commit when the user asks:** `docs: Semester Wrapped in the architecture notes`.
