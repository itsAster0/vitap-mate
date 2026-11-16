# Semester Wrapped

## Why

At the end of a semester, students get a story about their semester: attendance, streaks,
grades, outings and campus times, built from the VTOP data the app already keeps. Every screen
and a final summary card can be shared as an image (Instagram story, WhatsApp). Students who see
it ask what app made it, which is how new students find vitapmate.

## Goals

- A tap-through story in the app, one fact per screen, ending in one summary card with
  everything on it. Each screen and the summary card share as a 1080×1920 image.
- Built on the phone from data the app already caches or can fetch. Nothing about the student
  leaves the phone except the image they choose to share.
- Works for any semester the student has selected, once that semester's classes are over.
- One copy of the attendance maths, in Rust.

## Non-goals

- No server work: no rendering, no stats, no leaderboards.
- No "Past semesters" list. The student switches semester in the app's semester picker, as
  they do now, and More shows Wrapped for that semester.
- No MCP tool.

## Anti-goals

- A card that embarrasses the student. A bad GPA must be easy to hide before sharing.
- Fetching biometric days without being asked, beyond the few the background sync adds per run.
- Changing how the attendance screens look or behave while moving their maths.
- Blank or "N/A" facts. A fact that can't be worked out is left out of the story.

## Where it shows

- A card at the top of More, styled like the "new changes" card
  (`app/src/vitapmate/MoreScreen.kt`: `accentTone.subtle` background, `heroCookie()`, a shaped
  icon, chevron). It sits above the changes card.
  - Text: "Your <semester>, wrapped" / "<first class day> – <last class day> · tap to see".
- It shows from **the second day after the last instructional day** of the selected semester
  (last class day + 1 has passed), from that semester's academic calendar. The extra day lets
  the last day's attendance get posted. No calendar cached, no card. The open-app sweep fetches the
  calendar anyway.
- After the student has opened that semester's Wrapped once, the card shrinks to a one-line
  row at the top of More (remembered per semester id).

## The story

Story screens are drawn in Compose with theme colours and Inter, M3 Expressive. The steps:
- Tap right or left to move, with a progress bar at the top.
- A **Share** button on every screen.
- Swipe down to close.

Slides and their facts (all for the selected semester). There is no limit on the number of
slides.

| # | Slide | Facts | Source |
|---|---|---|---|
| 1 | Opening | Semester name, first and last class day, courses taken, class days and holidays | calendar, attendance |
| 2 | Attendance | Overall %, classes attended of total, classes on OD | attendance, attendance detail |
| 3 | Streak | Longest run of present or OD with its dates, against all posted classes (a bar of the semester's classes with the run where it fell, "59 of 336 classes, 18%"), perfect weeks | attendance detail |
| 4 | Never missed | Courses at 100%, by name | attendance |
| 5 | Comeback | Lowest attendance point and its month, against the final % | attendance detail |
| 6 | Habits | Worst weekday, 8 AM attendance %, days started at the first slot, days ended at the last slot | attendance detail, timetable |
| 7 | Courses | The top 3 best-attended courses and the least-attended course | attendance |
| 8 | Time | Classes attended, hours, busiest weekday, lab hours against theory hours | timetable, attendance detail |
| 9 | Places and people | The room with the most hours, the faculty with the most classes | timetable, attendance |
| 10 | Marks | Best CAT, best course by marks, CAT-1 to CAT-2 change (only with two CATs) | marks |
| 11 | Exams | CATs and FATs sat, the venue used most | exam schedule |
| 12 | Grades | Semester GPA, grade mix, S-grade record | grades, grade history |
| 13 | CGPA | CGPA before and after, credits earned this semester and in total | grade history |
| 14 | Outings | Leaves, weekend outings, most-visited place, longest leave, days away, longest stretch on campus | outing lists |
| 15 | Campus | Earliest entry, latest return, days on campus. A Fetch row when any day is missing (see Campus times) | biometric |
| 16 | Last semester | Attendance and GPA against the previous semester | previous semester's attendance, grade history |
| 17 | Title | One title for the semester, picked from the numbers | all |
| 18 | Summary | One tile per slide above that exists, with its headline number | all |

How each fact is worked out:
- **Overall %:** attended over counted, summed across every course, rounded up as VTOP does.
- **Longest run:** over every class of every course in time order (`day`, then `start`).
  Unposted classes are skipped, not breaks. It is drawn against the posted classes (present,
  OD or absent, a lab session once): their count, and how many came before the run. The
  summary tile reads "59 in a row, 18% of classes".
- **Perfect weeks:** Monday to Saturday weeks with at least one posted class and no absence.
- **Never missed:** courses with at least one counted class and no absence.
- **Courses:** the top 3 by attendance %, more attended classes first among equals, courses
  with no counted class skipped. A name shows once (an embedded course's theory and lab share
  one), and never the least-attended course. The summary tile names the first of them.
- **Comeback:** the pooled % after each class in time order, from the 20th posted class on. The
  slide shows only when the final % is at least 5 points above the lowest point.
- **8 AM %:** over classes that start before 9:00.
- **First and last slot:** days with an attended class in the timetable's earliest slot, and
  days with one in its latest slot.
- **Hours:** attended counted periods × 50 minutes. A lab session is two periods.
- **Room and faculty:** attended classes matched to the timetable by course and slot. Room by
  minutes, faculty by class count.
- **Best CAT:** highest scored ÷ max among the course's CATs, as the marks name them: "CAT"
  when a semester has only one (the freshers' first semester), else CAT-1, CAT-2. Titles such
  as "Continuous Assessment Test - I", "CAT1", "CAT - 1" and "CAT 2 (Theory)" match.
- **Best course by marks:** highest weighted marks as a share of the weightage assessed so far,
  among courses with at least half their weightage assessed.
- **CAT change:** the average of (CAT-2 % − CAT-1 %) over courses with both, and the course
  that rose the most. With a single CAT there is no change, and it's left out.
- **S-grade record:** S grades this semester. "Your most in a semester" only when it beats
  every earlier semester in the grade history.
- **CGPA before and after:** the CGPA as it was then. Before is credit-weighted over the
  grade history's dated sessions older than this semester's; a first semester has none, and the
  slide shows only after. The slide says up, down or the same, with the change, at two places.
  Without the semester's own grades page (an old semester), its GPA, grade mix and CGPA come
  from its session in the grade history, matched by the courses in attendance or the timetable. After is over those and this semester's session, so later semesters
  are left out of an old semester's story; while no later session exists it is VTOP's own CGPA
  (and total credits). A course taken more than once counts its latest attempt within those
  sessions only, as VTOP's CGPA does. N and grades without points are left out, as in a
  session's GPA. This semester's session is the newest one after the month classes started
  (or in it, when classes start and end in the same month, as a short summer semester can)
  that holds one of its courses.
- **S-grade record:** compared with earlier semesters only, not later ones.
- **Days away:** leave days inside the window, clipped to it. Students apply for more days
  than they take, so a leave is one trip: its longest run of days without a biometric punch.
  A day of the leave with a complete cached biometric copy that has a punch is a day on
  campus; days without a cached copy count as away, so without biometric the whole leave is
  the trip. A leave of the 10th to the 18th with punches on the 11th, 12th, 16th and 17th is
  3 days away, the 13th to the 15th. **Longest leave:** the longest trip. **Longest stretch on
  campus:** the longest gap between trips inside the window.
- **Exams:** papers in the exam schedule's CAT groups (a lone "CAT", or CAT-1, CAT-2) and its
  FAT group.
- **Last semester:** the next older semester in the list. GPA from the grade history. Its
  attendance comes from one extra fetch when it isn't cached. Shown as a difference ("up 4%");
  the GPA difference is taken at hundredths, so it is never "up 0.00".
  Only the halves that improved are shown, and the slide is skipped when neither did.

Title rules. The first that matches wins:

| Title | When |
|---|---|
| The Comeback | Final % is 10 or more points above the lowest point |
| The Iron Streak | Longest run of 15% or more of all posted classes, and 15 or more classes |
| The 8 AM Regular | 8 AM attendance 90% or more, over 15 or more such classes |
| The Topper | GPA 9.0 or more |
| The Explorer | 8 or more outings |
| The Lab Rat | Lab hours 40% or more of all hours |
| The Regular | Attendance 90% or more |
| The Free Spirit | Anything else |

Rules:
- A fact with no data is left out. A slide with no facts is skipped.
- Outings are picked by date range, from the semester calendar's first day to its last
  entry (which covers the exam weeks), not by list position. VTOP's leave list isn't in date
  order.
- Grades appear when VTOP posts them. Before that, the story has no Grades or CGPA slide.
- **Hide grades** switch next to Share on the Marks, Grades, CGPA, Last semester, Title and
  Summary slides, off by default. It changes only the shared image:
  - Marks, Grades and CGPA facts are left out;
  - Last semester keeps attendance and drops GPA;
  - The Topper is skipped, and the next matching title is used.
  It is remembered per semester.
- No title or fact says anything unkind. There is no title for low attendance.
- The registration number is never on a shared image. The student's given name is, if the
  profile is cached: VTOP gives "SURNAME GIVEN [OTHER]", so the second word, or the only one.
- Footer on every shared image: "vitapmate" and the Play link, small.

## Look

The mockups in `docs/superpowers/specs/wrapped-mockups/` are the visual reference: open the
HTML files in a browser. One tone per slide, one large Material shape holding the number, theme
colours only. Their numbers and names are made up.

## Loading

Opening Wrapped:

1. Calls `cachedWrapped(flow)` (the selected semester, like the other `cached_*`) off the main thread and draws at once from the cache.
2. Fetches what's missing for that semester, one at a time, with a "Putting your semester
   together…" screen until the first draw:
   - attendance
   - attendance detail per course
   - timetable, calendar, marks, grades, grade history, exam schedule
   - both outing lists
   - the previous semester's attendance
3. Calls `cachedWrapped` again when fetches finish.

A failed fetch leaves its facts out and logs a warning. It doesn't block the story. Flow:
`wrapped.open`.

## Campus times (biometric)

Biometric is one VTOP request per day (`rust/vtop-ffi/src/biometric.rs`), so a semester is
about 120 requests. It is never fetched automatically for Wrapped.

- **The Campus screen shows from whatever is cached.** Cached days are used as they are, and
  past days are never fetched again: `is_complete` already treats a day as final only when it
  was fetched after that day ended (India time).
  - Every class day cached: the facts alone.
  - Some cached, some missing: the facts from the cached days, labelled "from <n> of <m> days",
    and a row "<k> days missing · Fetch".
  - None cached: only the row "Add campus times · <m> days to fetch from VTOP · Fetch".
- **Fetch** queues a WorkManager job, `BiometricFetchWorker` in `core/src/vitapmate/data/`.
  `androidx.work:work-runtime` moves from `app/module.yaml` to `core/module.yaml`, so the
  feature can start it. It's the same dependency, not a new one, and `SyncWorker` keeps working.
  - unique work `wrapped.biometric.<semester>` with KEEP, so a second tap doesn't start a
    second run;
  - a network constraint;
  - it is an ordinary WorkManager job, not a foreground service. The manifest removes
    WorkManager's `SystemForegroundService` on purpose, because Play flags it as a foreground
    service started at boot (Android 15), and a `dataSync` service needs a Play Console
    declaration;
  - it shows its own ongoing notification "Fetching campus times · 34 of 118 days" with a
    **Stop** action that cancels the work.
  Android stops an ordinary job after about 10 minutes, so the job hands back at about 8
  minutes and WorkManager runs it again shortly after. It picks up where it left off, since
  done days are skipped. The same happens after a restart.
- **Before the first day**, it checks the VTOP session's sign-in time (`logged_in_at` in the
  persisted session). If it's older than 15 minutes, it signs in again first, using the same
  path as the sign-in after an expired session.
- **Mid-run**, an expired session goes through the existing sign-in-again path, and the run
  carries on. If signing in again needs an OTP the app can't fill, the job stops and the
  notification says "Open vitapmate to sign in and continue". Tapping it opens Wrapped, and the
  Fetch row resumes from there.
- It fetches only the missing days, through `fetchBiometricRange` in chunks of 14 days or fewer,
  with a pause of a few seconds between days (tunable in Settings → Developer). The notification
  counts only the missing days.
- It is a user action: `flow("wrapped.biometric")`, not unattended.
- When it finishes, Wrapped reloads from the cache and the Campus screen appears.

**Background sync addition** (`core/src/vitapmate/data/VtopSync.kt`):
- Each run also fetches up to 3 of the most recent days that aren't complete in the cache:
  today, yesterday, and any day missed while the phone was off. The cap is a `Tunable`.
- Each is one page under the sweep's existing per-page timeout.
- Over a semester this fills most days, so the Wrapped fetch usually has little left. The
  biometric screen also gets recent days instantly.

## Attendance maths moves to Rust

The standing, "Can skip" and plan are already in `rust/vtop-ffi/src/records.rs`. These
Kotlin leftovers move there, with tests, and Kotlin only draws them:

| Kotlin today | Moves to |
|---|---|
| `Standing.kt` `advice()` | Already in Rust as `AttendanceCourse.advice`. Kotlin's copy goes, keeping the `showSkip` setting and the " lab(s)" suffix by moving both into Rust's text or a field. |
| `Standing.kt` `debarLine()` | A `debar_line: Option<String>` on `AttendanceCourse` |
| `Insights.kt` counts, streak, last missed | Fields on `AttendanceDetail` (`present`, `on_duty`, `absent`, `streak`, `last_missed`) |
| `Standing.kt` `projected()` | **Stays in Kotlin**, the only copy |

`projected()` runs on every stepper tap. Rust has no copy of that sum, so keeping it in Kotlin
duplicates nothing and avoids a Rust call per tap.

The attendance screens must look and behave the same after the move.

## Panics

The app library now unwinds panics. A panic in `wrapped.rs` fails that call, and Wrapped shows
the usual "Unexpected" problem instead of closing the app.

## Approaches considered

**Where the facts are worked out**
- **Rust, `rust/vtop-ffi/src/wrapped.rs` (chosen).** One FFI call reads every cached page it
  needs (instead of ~15 from Kotlin), the attendance maths it needs is already in Rust, and it's
  easy to unit test from cached pages.
- Kotlin in `features/wrapped`. Faster to start, but the attendance maths would live in two
  languages.

**How share images are made**
- **Compose story screens saved as an image when Share is tapped (chosen):** one layout per
  screen, the same look as the app. Saved to the cache's share folder and sent through the
  existing `.logs` FileProvider, as `ShareLogs.kt` and `PdfExport.kt` do.
- A separate Canvas drawing per image would mean two layouts that drift apart.
- Server rendering sends the student's data off the phone.

**Shape**
- **Story plus a final summary card (chosen).**
- A single card only. Less to build, but less of the reveal that gets posted.

## Decided by the student (the user)

- Wrapped lives at the top of More, styled like the changes card.
- It is shown for the selected semester once its classes are over. No past-semesters list.
- Grades are shown when available.
- Old semesters fetch what they need.
- The outing lists come back whole, so no paging work is needed. This was checked against live
  VTOP data.
- Biometric shows from the cache. A Fetch button appears when any day is missing, and it runs
  as a WorkManager job with a progress notification. The session is signed in again when it's older than 15 minutes or expires,
  and cached past days are skipped.
- Background sync adds recent biometric days.
- Story plus summary card.
- The attendance maths moves to Rust.

## Calls made in this design (check these)

- The Wrapped card sits above the changes card, and shrinks to a row after the first open
  (it doesn't grow back).
- Overall % is pooled across courses; the streak runs across every class in time order.
- The outing window is the calendar's first to last entry.
- The fact rules and thresholds above (comeback from the 20th class and 5 points, the title
  list and its order, best course needing half its weightage, the Iron Streak at 15% and 15
  classes, the top 3 courses).
- A leave is one trip, its longest run of days without a punch.
- The CGPA after is VTOP's own while this semester's is the newest session in the history.
- The summary shows one tile per slide, not every fact.
- There is no Files slide: saved files don't record their course, and the count alone wasn't
  worth a slide (removed after seeing it on the phone).
- The Hide grades switch is off by default and remembered per semester.
- First name on images, never the registration number. A small vitapmate footer.
- Up to 3 recent biometric days per sync, tunable. The Wrapped fetch pace is tunable.
- `projected()` stays in Kotlin, the only copy of that sum.
- `BiometricFetchWorker` is an ordinary WorkManager job with its own notification, not a
  foreground service (Play flags WorkManager's foreground service). It runs in stretches of
  about 8 minutes.
- When signing in again needs an OTP the app can't fill, the job stops and asks the student to
  open the app.

## Left to the builder

- Exact screen layouts, motion and colours, within M3 Expressive and theme colours only.
- How the share image is captured: `rememberGraphicsLayer` then `toImageBitmap`, drawn at
  1080×1920.
- Whether `wrapped.rs` reads the cached records directly or through the existing `cached_*`.
- The `WrappedReport` record shape, with one optional field or sub-record per screen.
