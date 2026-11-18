# Wrapped New Slides Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four slides to Semester Wrapped (Close call, Exam crunch, Hostel life, On the move) without changing the existing slides.

**Architecture:** Each slide is one optional sub-record on `WrappedReport`, worked out in `rust/vtop-ffi/src/wrapped.rs` from pages the app already caches, tested in `rust/vtop-ffi/src/wrapped/tests.rs`. Kotlin adds one `Slide` builder per record in the existing slide files, in the existing visual style, and one summary tile each.

**Tech Stack:** Rust (UniFFI records), Kotlin Compose Multiplatform (M3 Expressive), Kotlin Toolchain (`./kotlin`), mise.

**Spec:** `docs/superpowers/specs/2026-11-16-semester-wrapped-design.md`, section "New slides (2026-11-18)". Mockups: `docs/superpowers/specs/wrapped-mockups/new-slides.html`.

## Global Constraints

- Don't change any existing slide, its tiles, or the order of existing slides.
- Theme colours only, through `Ink` / `inks()`; shapes from `MaterialShapes` or rounded rects, as the existing slides do.
- Logs say which facts exist, never their values (as in `wrapped()`'s `info!`).
- A fact without data is `None`, and the slide is skipped. No "N/A".
- Rust follows the Google Rust style guide; `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` stay clean.
- Attendance maths uses the existing `periods`, `is_attended`, `is_counted` and `percent` (rounded up, as VTOP does).
- Only complete cached biometric days (`BiometricDay::complete`) count, the same list `campus()` gets.
- jj repo: don't commit unless the user asks. No new dependencies.

## Review Focus

- A day scholar (no `MH`/`LH` punches): `hostel` is `None` and the slide is skipped, while On the move still shows. Test in Task 3.
- A venue that's neither form (`AB1-FACE-OUT-1`, `LIB-FACE`, empty): not a class punch, not a hostel punch, no panic. Test in Task 3.
- A day starting inside the hostel with an IN first (back from an overnight leave): no out interval before it. Test in Task 3.
- A course whose attendance was below 75% at its 10th counted class and never recovered: still a close call, spare `None`. Test in Task 1.
- Exam papers not dated yet, or dated today and later: left out of the crunch. Test in Task 2.

---

### Task 1: Close call (Rust)

**Files:**
- Modify: `rust/vtop-ffi/src/wrapped.rs` (record next to `WrappedComeback`, fn next to `comeback`, field on `WrappedReport` after `comeback`, `info!` flag)
- Test: `rust/vtop-ffi/src/wrapped/tests.rs`

**Interfaces:**
- Produces: `pub close_call: Option<WrappedCloseCall>` on `WrappedReport`;
  ```rust
  #[derive(Debug, Clone, Default, PartialEq, uniffi::Record)]
  pub struct WrappedCloseCall {
      pub course: String,      // AttendanceCourse::name
      pub day: i32,            // the class that set the moment
      pub spare: Option<i32>,  // further absences keeping percent() >= 75; None when under 75
      pub percent: i32,        // the course's percent at that moment
      pub final_percent: i32,  // the course's percent after its last counted class
  }
  ```
  `fn close_call(classes: &[Taken]) -> Option<WrappedCloseCall>`; consts `CLOSE_CALL_FROM: i32 = 10` (counted classes, per course), `CLOSE_CALL_SPARE: i32 = 3`, `ATTENDANCE_FLOOR: i32 = 75`.

- [ ] **Step 1: Write the failing tests** (use `Fixture`, `course`, `history` as `comeback_needs_twenty_classes_and_five_points` does)
  - `close_call_finds_the_tightest_moment`: one ETH course, 12 present then 4 absent then 8 present (24 classes). After 16 classes it is 12/16 = 75%, spare 0; assert `(course, day, spare, percent, final_percent) == ("CSE1001 name", d0() + 15, Some(0), 75, 84)` (20/24 = 83.3 → 84 rounded up). Name comes from `course()`'s `course_name` as `AttendanceCourse::name` parses it; read the parsed name in the test with `report.attendance` if unsure.
  - `close_call_skipped_with_room_to_spare`: 30 present → `close_call == None`.
  - `close_call_below_75_has_no_spare`: 6 present then 6 absent then 2 present → `spare == None`, `percent` = the lowest % from the 10th class on (`percent(6, 12) = 50`).
  - `close_call_counts_a_lab_as_two`: an ELA course, one history row per session (slot "L1+L2"): 5 present, 1 absent, 6 present. Spare counts sessions of 2 units: at the 6th session 10/12, and one more absent session makes 10/14 = 72%, so spare is `Some(0)` on `d0() + 5`, `final_percent == 92` (22/24). Counting single units would wrongly give 1 (10/13 = 77%).
- [ ] **Step 2: Run** `cd rust && cargo test -p vtop-ffi close_call`. Expected: FAIL (no field `close_call`).
- [ ] **Step 3: Implement.** Group `classes` by course key in time order (`day`, then `start`). Per course, walk counted classes adding `periods(course)` to counted (and to attended when `is_attended`); from counted ≥ `CLOSE_CALL_FROM` on, spare = the largest k ≥ 0 with `percent(attended, counted + k * periods) >= 75`, or `None` when `percent(attended, counted) < 75`. Rank moments by: `None` spare first (lowest percent first), then smallest spare, then earliest day. Return `None` when the best spare is `Some(s)` with `s > CLOSE_CALL_SPARE`. Set `report.close_call = close_call(&classes)` in `wrapped()`, and add `close_call = report.close_call.is_some()` to the `info!`.
- [ ] **Step 4: Run** `cd rust && cargo test -p vtop-ffi wrapped`. Expected: all pass, the old ones too.

### Task 2: Exam crunch (Rust)

**Files:**
- Modify: `rust/vtop-ffi/src/wrapped.rs` (record next to `WrappedExams`, fn next to `exams`, field after `exams`)
- Test: `rust/vtop-ffi/src/wrapped/tests.rs`

**Interfaces:**
- Produces: `pub crunch: Option<WrappedCrunch>` on `WrappedReport`;
  ```rust
  pub struct WrappedCrunch {   // same derives as the others
      pub papers: i32,         // papers in the busiest 7-day window
      pub first_day: i32,      // window start (the first paper's day in it)
      pub last_day: i32,       // first_day + 6
      pub most_in_a_day: i32,
      pub most_day: i32,       // the earliest day with most_in_a_day
      pub exam_days: i32,      // distinct days with a paper sat
  }
  ```
  `fn crunch(exams: &Exams, today: i32) -> Option<WrappedCrunch>`; const `CRUNCH_MIN: i32 = 3`.

- [ ] **Step 1: Write the failing tests** (paper JSON as in `exams_count_only_papers_already_sat`, with `exam_date` per paper; use `calendar_ending_after_today` where papers must be before today)
  - `crunch_finds_the_busiest_week`: papers on d, d+2, d+2, d+5, d+20 → `(papers, first_day, last_day, most_in_a_day, most_day, exam_days) == (4, d, d+6, 2, d+2, 4)`.
  - `crunch_needs_three_papers`: papers on d and d+1 → `None`.
  - `crunch_skips_undated_and_future_papers`: two sat papers plus one undated and one on today → `None`.
- [ ] **Step 2: Run** `cd rust && cargo test -p vtop-ffi crunch`. Expected: FAIL.
- [ ] **Step 3: Implement.** Days of every paper in every group with `day < today`, sorted. Window from each paper's day d: count days in `d..=d + 6`; best count, earliest d on a tie. `None` under `CRUNCH_MIN`. Wire it into `wrapped()` from the same `cached_exams` value `exams` uses (read it once, into a local), and add the `info!` flag.
- [ ] **Step 4: Run** `cd rust && cargo test -p vtop-ffi wrapped`. Expected: PASS.

### Task 3: Punch kinds and Hostel life (Rust)

**Files:**
- Modify: `rust/vtop-ffi/src/wrapped.rs` (record and fns next to `campus`, field after `campus`)
- Test: `rust/vtop-ffi/src/wrapped/tests.rs`

**Interfaces:**
- Produces:
  ```rust
  /// A class punch's block, room and floor: "CB-214-FACE" is ("CB", "214", Some(2)).
  struct ClassPunch { block: String, room: String, floor: Option<i32> }
  fn class_punch(venue: &str) -> Option<ClassPunch>
  fn is_hostel(punch: &Punch) -> bool   // place starts with "MH" or "LH", kind Entry or Exit
  ```
  `pub hostel: Option<WrappedHostel>` on `WrappedReport`;
  ```rust
  pub struct WrappedHostel {
      pub days: i32,                  // class days with at least one OUT→IN pair
      pub out_tenths: i32,            // average hours out, in tenths (94 = 9.4 h)
      pub out_by_half_hour: Vec<bool>,// 36 long, 6:00 to 24:00: out on at least half of `days`
      pub naps: i32,
      pub nap_weekday: Option<i32>,   // 0 = Monday, most naps; earliest weekday on a tie
  }
  ```
  `fn hostel(days: &[BiometricDay]) -> Option<WrappedHostel>`; const `BIOMETRIC_MIN_DAYS: i32 = 5`.

`class_punch` rules (spec, "Biometric punches"): split on `-`. Block = first part. Room = the first later part that is digits of length ≥ 3, or `G` followed by digits, or such a value inside brackets (`(101)`). Floor: room `G…` → 0; else the room's number / 100 (214 → 2, 105 → 1); `GF` / `<n>F` parts also give the floor when present. No room → `None`. Hostel venues (`MH2-FACE-IN-7`) have no room, so they are never class punches.

- [ ] **Step 1: Write the failing tests**
  - `class_punch_reads_block_room_floor` (a unit test on `class_punch`): `"CB-214-FACE"` → ("CB", "214", Some(2)); `"AB1-G05-FACE"` → ("AB1", "G05", Some(0)); `"AB1-GF-(101)"` → ("AB1", "101", Some(0)); `"AB2-105-FACE"` → floor 1; `"MH2-FACE-IN-7"`, `"AB1-FACE-OUT-1"`, `"LIB-FACE"`, `""` → `None`.
  - `hostel_out_time_and_naps`: write biometric days as `missing_days_skip_complete_cache` does (complete: `update_time` the next day), using the live 15 Oct 2026 shape on 5 class days: OUT 6:27, IN 7:27, OUT 9:55, class `CB-214-FACE` 10:48, IN 10:58, OUT 13:37, IN 14:14, OUT 17:37, IN 19:01 (`"19:1"` as VTOP sends it). Out = 1:00 + 1:03 + 0:37 + 1:24 = 4:04 → `out_tenths == 41`. Naps: IN 10:58 follows class 10:48, but the next OUT 13:37 has no class punch after it, so 0 naps; add a class punch `CB-214-FACE` at 14:30 on two days → `naps == 2`, `nap_weekday` that weekday. Assert `days == 5` and `out_by_half_hour[8]` (10:00–10:30) is true, `out_by_half_hour[1]` (6:30–7:00, midpoint 6:45) true, `out_by_half_hour[0]` (midpoint 6:15, before the 6:27 OUT) false, `out_by_half_hour[30]` (21:00) false.
  - `hostel_none_for_a_day_scholar`: 5 days of only class punches → `hostel == None`.
  - `hostel_ignores_an_in_before_any_out`: a day of IN 7:00, OUT 8:00, IN 9:00 → that day's out is 1:00, not 2:00.
- [ ] **Step 2: Run** `cd rust && cargo test -p vtop-ffi hostel class_punch`. Expected: FAIL.
- [ ] **Step 3: Implement.** Punches are newest first in `BiometricDay`; reverse to time order and skip punches without `minute`. Per day: pair each hostel Exit with the next hostel Entry. A nap: a hostel Entry with a class punch earlier that day and the following hostel Exit with a class punch later that day. A half hour counts as out on a day when any out interval covers its midpoint. `None` when `days < BIOMETRIC_MIN_DAYS`. Wire with `hostel(&biometric)` in `wrapped()` plus the `info!` flag.
- [ ] **Step 4: Run** `cd rust && cargo test -p vtop-ffi wrapped`. Expected: PASS.

### Task 4: On the move (Rust)

**Files:**
- Modify: `rust/vtop-ffi/src/wrapped.rs`, `rust/vtop-ffi/src/wrapped/tests.rs`

**Interfaces:**
- Consumes: `class_punch`, `BIOMETRIC_MIN_DAYS` (Task 3).
- Produces: `pub moves: Option<WrappedMoves>` on `WrappedReport`;
  ```rust
  pub struct WrappedMoves {
      pub floors: i32,
      pub rooms: i32,              // distinct (block, room)
      pub most_hops: i32,          // block changes in one day
      pub most_hops_day: i32,      // earliest on a tie
      pub blocks: Vec<WrappedBlock>, // up to 3, most rooms first, then name
  }
  pub struct WrappedBlock { pub block: String, pub rooms: i32 }
  ```
  `fn moves(days: &[BiometricDay]) -> Option<WrappedMoves>`.

- [ ] **Step 1: Write the failing test** `moves_counts_floors_rooms_hops`: 5 class days. Day 1 class punches in time order: `CB-214-FACE`, `CB-105-FACE`, `AB1-G05-FACE`, `AB1-301-FACE` → floors 2 + 0 + 0 + 3 = 5, hops 2. Days 2–5: one `CB-214-FACE` each → 2 floors each. Assert `floors == 13`, `rooms == 4`, `(most_hops, most_hops_day) == (2, day1)`, `blocks == [("AB1", 2), ("CB", 2)]` (a tie goes by name). Also `moves_none_under_five_days` with 4 days → `None`.
- [ ] **Step 2: Run** `cd rust && cargo test -p vtop-ffi moves`. Expected: FAIL.
- [ ] **Step 3: Implement** per the spec: floors = first floor + every rise; punches without a floor are skipped for floors but count for rooms and hops. `None` when fewer than `BIOMETRIC_MIN_DAYS` days have a class punch. Wire it in and add the `info!` flag.
- [ ] **Step 4: Run** the full Rust checks: `cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`. Expected: clean, all pass.
- [ ] **Step 5: Rebuild the bindings:** `mise run rust-android`. Expected: `core/src/vitapmate/core/vtop_ffi.kt` gains `WrappedCloseCall`, `WrappedCrunch`, `WrappedHostel`, `WrappedMoves`, `WrappedBlock`, and the `WrappedReport` fields `closeCall`, `crunch`, `hostel`, `moves`.

### Task 5: The four slides (Kotlin)

**Files:**
- Modify: `features/wrapped/src/vitapmate/features/wrapped/SlidesClasses.kt` (`closeCallSlide`), `SlidesResults.kt` (`crunchSlide`), `SlidesCampus.kt` (`hostelSlide`, `movesSlide`, the shared `BiometricNote`), `Slides.kt` (`slides()` order only)

**Interfaces:**
- Consumes: the Task 1–4 records via `WrappedReport`.
- Produces: `internal fun closeCallSlide(r: WrappedReport): Slide?`, `crunchSlide`, `hostelSlide`, `movesSlide`, the same shape as the others.

Order in `slides()`: `coursesSlide, closeCallSlide, timeSlide, …, examsSlide, crunchSlide, gradesSlide, …, campusSlide, hostelSlide, movesSlide, previousSlide, titleSlide`.

Layouts are the mockup's (`new-slides.html`), positions in the 360 × 640 frame, built from `Txt`, `H`, `P`, `Big`, `Label`, `Chip`, `Panel`, `Minis`, `Shaped`, `SplitBar`:

| Slide | Ink | Copy and parts | Tile |
|---|---|---|---|
| Close call | Danger | Label "Close call"; `Big(spare)` + H "class(es) to spare" (spare `None`: `Big("${percent}%")` + H "your lowest"); P "<Course> on <day>: you could miss just <n> more class(es) and stay at 75%." (n = 0: "…any miss and it was under 75%."; None: "You dipped to <p>% in <Course> on <day>. You finished at <final>%."); the 50–100% bar with a 75% line and a `Cookie9Sided` dot at `percent`; Minis "<percent>% its lowest point", "<final>% where it finished" | Danger, span 3: "<n> class(es)" / "to spare in <Course>", or "<p>%" / "low in <Course>" |
| Exam crunch | Accent | `Big(papers)` + H "exams in\none week"; P "Your busiest week, <first> to <last>"; a row of 7 day columns from `first_day`, a `Cookie9Sided` per paper, the busiest day's column on a tinted pill; Minis "<most> in a day / <weekday day month>", "<exam_days> days / with an exam" | Accent, span 2: "<papers>" / "exams in one week" |
| Hostel life | Success | Label "Hostel life"; `Big("9.4")` + H "h"; H(26) "out of the hostel on a class day"; the 36-part bar (out runs merged into `ink.shape` segments, in runs `ink.text` at 0.16 alpha) labelled 6 AM, noon, 6 PM, midnight; Panel with `Big(naps)` + "trips back to your room between classes, most on <Weekday>" (no naps: leave the Panel out); `BiometricNote(hostel = true)` | Success, span 3: "<x.x> h" / "out of the hostel a day, <n> naps" |
| On the move | Lab | 3 stepped rounded rects at the top right, as the mockup; Label "On the move"; `Big(floors)`; H "floors climbed"; P "Between classes, about <floors / 163, one decimal> Burj Khalifas" (under 0.1: leave the clause out); Minis "<rooms> rooms / you punched in", "<hops> hops / between blocks on <day>"; a Chip per block "<block> · <n> rooms"; `BiometricNote(hostel = false)` | Lab, span 3: "<floors>" / "floors climbed, <rooms> rooms" |

`BiometricNote(y: Int, hostel: Boolean)`: an info icon (Lucide, as the Share icon is imported) and "Worked out from your [hostel and ]class punches on <campus.daysCached> of <campus.daysTotal> days, so it can be off." at 12 sp, `ink.text` 0.78 alpha. It shows on the share image too.

Use the existing helpers `count`, `day`, `titled`, `WEEKDAYS`. Plurals through `count`.

- [ ] **Step 1: Build:** `./kotlin build`. Expected: success.
- [ ] **Step 2: Run on the emulator** (`emulator-5554`; it needs unlocking with the PIN first). Install with `./kotlin run -d emulator-5554`. Drive it through the T3 device tools (`device_open` with host `local` and device `phone`, then the `agent-device` command it returns), not adb input. In More, open Wrapped for a semester whose classes are over, step to each new slide and take a screenshot of each. Press Share once on Hostel life and check the note is on the image. Expected: each slide matches its mockup; slides without data are skipped; the summary shows the new tiles in slide order.
- [ ] **Step 3: Check the existing slides by eye** in the same run: they're unchanged.

### Task 6 (optional, only if the user says yes): New titles

**Files:** `rust/vtop-ffi/src/wrapped.rs` (`WrappedTitle`, `title()`), `features/wrapped/.../Slides.kt` (`label()`), `SlidesSummary.kt` (`reason`, `short`), tests.

- New variants after `Regular`, before `FreeSpirit`: `StairClimber` (`moves.floors >= 150`), `Homebody` (`hostel.naps >= 20`), `Survivor` (`close_call.spare.is_none() && close_call.final_percent >= 75`). Labels "The Stair Climber", "The Homebody", "The Survivor". Reasons: "<floors> floors between classes, <rooms> rooms"; "<naps> trips back to your room between classes"; "Down to <p>% in <Course>, finished at <final>%". Test: extend `title_follows_the_order`.

---

## Batch 2: On Duty, Full marks, Assessments, Morning or afternoon

Mockups: `docs/superpowers/specs/wrapped-mockups/more-slides.html`. Same constraints as above. Order in `slides()`: `attendanceSlide, odSlide, streakSlide, …, habitsSlide, dayHalfSlide, coursesSlide, closeCallSlide, …, marksSlide, fullMarksSlide, assessmentsSlide, examsSlide, crunchSlide, …`.

### Task 7: On Duty and Morning or afternoon (Rust)

**Interfaces:**
- `pub on_duty: Option<WrappedOnDuty>` on `WrappedReport`:
  ```rust
  pub struct WrappedOnDuty {
      pub classes: i32,          // OD classes, VTOP units (a lab session is two)
      pub days: i32,             // distinct days with an OD class
      pub present: i32,          // present (not OD), VTOP units
      pub absent: i32,           // VTOP units
      pub top: Vec<WrappedCourse>, // courses with the most OD, up to 2 when tied, `percent` holds the OD count
  }
  ```
  `fn on_duty(classes: &[Taken]) -> Option<WrappedOnDuty>`; const `ON_DUTY_MIN: i32 = 10`; `None` under it.
- `pub day_half: Option<WrappedDayHalf>`:
  ```rust
  pub struct WrappedDayHalf {
      pub morning: bool,         // true: before noon wins; false: from 2 PM wins
      pub percent: i32,          // that side's share of attended classes with a start time
      pub classes: i32,          // classes on that side (count each attended class once, a lab session once)
      pub total: i32,            // attended classes with a start time
      pub by_hour: Vec<i32>,     // 11 long: attended classes starting in hours 8..=18
  }
  ```
  `fn day_half(classes: &[Taken]) -> Option<WrappedDayHalf>`; const `DAY_HALF_PERCENT: i32 = 60`. Before noon = start < 12:00; afternoon = start >= 14:00. `None` when neither side reaches 60% (rounded down), or with no start times.

- [ ] Tests in `wrapped/tests.rs`: `on_duty_counts_classes_days_and_top_course` (two courses, 8 and 4 OD classes over 9 distinct days, plus present and absent → `(12, 9, …)`, top = the 8-OD course); `on_duty_needs_ten`; `day_half_afternoon` (3 classes at 08:00, 7 at 14:00 → `morning == false, percent == 70, classes == 7, total == 10, by_hour[0] == 3, by_hour[6] == 7`); `day_half_skipped_when_mixed` (5 and 5 → `None`). Run `cd rust && cargo test -p vtop-ffi wrapped`: FAIL, implement, PASS. Wire into `wrapped()` and its `info!`.

### Task 8: Full marks and Assessments (Rust)

**Interfaces:**
- `pub full_marks: Option<WrappedFullMarks>`:
  ```rust
  pub struct WrappedFullMarks {
      pub count: i32,
      pub of: i32,                     // posted assessments (scored is Some), re-evaluations left out
      pub by_kind: Vec<WrappedKindCount>, // kinds with at least one, most first
      pub top_course: Option<String>,  // the course with the most, None when that is 1
      pub top_count: i32,
  }
  pub struct WrappedKindCount { pub kind: String, pub count: i32 }
  ```
- `pub assessments: Option<WrappedAssessments>`: `{ pub total: i32, pub by_kind: Vec<WrappedKindCount> }`, posted assessments, re-evaluations left out, most first, ties in the order of `KINDS` below.
- `fn assessment_kind(title: &str) -> &'static str`, by case-insensitive match, first hit wins:
  `"quiz"` → "quizzes"; `"cat"` (as `cat_of` reads it, or the title starts with "CAT") → "CATs"; `"fat"` → "FATs"; `"exp"`, `"experiment"`, `"lab"` → "lab experiments"; `"digital assessment"`, `"da"` as a word → "digital assessments"; `"review"`, `"project"`, `"report"` → "reviews and reports"; `"assignment"` → "assignments"; else "other". `KINDS` lists them in that order.
- Full = `scored >= max` with `max > 0`. Both `None` with no posted assessment; `full_marks` `None` with count 0.
- `fn full_marks(marks: &Marks) -> Option<WrappedFullMarks>`, `fn assessments(marks: &Marks) -> Option<WrappedAssessments>`.

- [ ] Tests: `assessment_kinds_read_vtop_titles` (unit: "QUIZ1", "CAT1", "Experiment-3", "Exp1+2", "Final LAB Report", "Digital Assessment-2", "First Review", "Project Report", "ASSIGNMENT", "Viva" → the kinds above, "Viva" → "other"); `full_marks_counts_max_scores` (marks fixture with 3 posted of which 2 at max in course A, 1 at max in B, 1 unposted → `count 3, of 4… ` adjust to the fixture, top_course A, top_count 2); `assessments_group_by_kind`. Use the marks JSON shape `cat_change_averages_courses_with_both` writes. FAIL, implement, PASS. Read `cached_marks` once in `wrapped()` for `marks`, `full_marks` and `assessments`.
- [ ] `cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`, then `mise run rust-android`.

### Task 9: The four slides (Kotlin)

| Slide | Ink | Parts (mockup) | Tile |
|---|---|---|---|
| On Duty | Warning | `Big(classes)` + H "on\nOD"; P "<share> of your classes, across <days> days" (share: "A third" for 30–36%, "A quarter" 23–27%, "Half" 47–53%, else "<n>%"); a 3-part bar Present / OD (ink.shape) / Absent with labels; Label "Most OD in", `Cookie9Sided` with the count, the course name, a second line "and <n> in <Course>" when two tie | Warning, span 2: "<classes>" / "on OD, <days> days" |
| Full marks | Lab, `grades = true`, `shareable = { !it }` | `SoftBurst` with the count; H "full\nmarks"; P "<count> of <of> assessments, top score"; Minis of `by_kind` (up to 3, 3 columns); Wide-style panel "<top_count> full marks, the most in one course / <Course>" when `top_course` | Lab, key, grades: "<count>" / "full marks" |
| Assessments | Danger | `Big(total)` + H "tests\ntaken"; one row per kind (up to 6): a pill whose width is count / max of the bar span (min 16%), the first in ink.shape, the next two `mid`, the rest faint, the count inside, the kind beside | Danger, span 2: "<total>" / "assessments" |
| Morning or afternoon | Accent | A 16-point `Cookie` sun (Warning tone base) at the top right; `Big("<percent>%")`; H "An afternoon person" / "A morning person"; P "<classes> of <total> classes started at 2 PM or later" / "before noon"; 11 bars by hour, the winning side in ink.shape, the rest faint; labels 8 AM, noon, 2 PM, 6 PM | Accent, span 2: "<percent>%" / "classes after 2 PM" or "classes before noon" |

- [ ] `./kotlin build`; on the emulator (unlocked), step through the new slides and screenshot each; check that Hide grades drops the Full marks tile from the summary.
