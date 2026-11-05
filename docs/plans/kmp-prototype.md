# Plan: Compose prototype over a UniFFI Rust core

Status: in progress. Started as a test next to the Flutter app; the Flutter
app was removed on 2026-11-09 and this app moved to the repo root.

Goal: find out whether a Kotlin + Compose app with most logic in Rust is
faster and as good-looking as the Flutter app. Scope: sign-in (with the
security OTP), attendance, timetable (agenda + weekly). Android first.

## Stack

- Build: Kotlin Toolchain 0.13 (`kotlin` CLI, pinned in `mise.toml`), no Gradle.
  Compose Multiplatform 1.12, Kotlin 2.4 (toolchain defaults).
- Rust: new crate `rust/vtop-ffi` (edition 2024), UniFFI 0.32 proc macros,
  wraps `vtop-core`. Upstream `uniffi-bindgen` generates Kotlin for Android
  (JNA direct mapping). JNA 5.19.1 (`@aar`) on the app side.
- iOS later: upstream Kotlin bindings are Android/JVM only, so the UI code
  that touches Rust types lives in the Android source set for now. Moving to
  iOS = switch to Gobley's bindgen (common + native bindings) and move the UI
  to common code.

## Modules (the build enforces the boundaries)

```
rust/vtop-ffi/                 Rust: Core, cache, session, records, obs.rs
project.yaml                   Kotlin Toolchain project (repo root)
  core/      vitapmate.core    generated bindings (src/vitapmate/core/vtop_ffi.kt)
             vitapmate.data    AppCore, VtopPage/PageState, Problem, Settings, Inspector data
             vitapmate.observability  Log, flow()
  design/    vitapmate.ui      M3 Expressive theme, components, PageScaffold, Feature type
  features/<name>/             one feature per module; depends on design only;
             vitapmate.features.<name>   exposes `val <name>Feature: Feature`
  app/       vitapmate         registry, floating nav, More, Settings, Inspector UI,
                               MainActivity; jniLibs/ (built, gitignored)
```

Adding a feature = `features/<name>/module.yaml` + `src/` + one line in
`project.yaml` + one line in the registry (`app/src/vitapmate/Features.kt`).
A feature never imports another feature; what two features share moves to
`design` (UI) or `core` (data).

### Contracts between modules

```kotlin
// design: vitapmate.ui
data class Feature(
    val id: String,                 // stable: settings keys, routes, log src "ui.<id>"
    val label: String,
    val icon: ImageVector,
    val actions: @Composable RowScope.() -> Unit = {},          // tab app bar icons
    val detail: (@Composable (item: String) -> Unit)? = null,   // full screen per item
    val content: @Composable () -> Unit,
)
// A feature opens its detail with LocalOpenDetail.current.open(item, title); the app pushes
// Route.Detail(feature, item, title) (survives process death; predictive back).

// core: vitapmate.data — every screen's data goes through this
sealed interface PageState<out T> {
    data object Loading : PageState<Nothing>
    data class Ready<T>(val data: T, val updatedAt: Long, val refreshing: Boolean,
                        val problem: Problem?) : PageState<T>   // problem = last refresh failed, data kept
    data class Failed(val problem: Problem) : PageState<Nothing> // nothing cached
}
data class Problem(val kind: ProblemKind, val title: String, val message: String,
                   val flow: String, val retryable: Boolean)
enum class ProblemKind { Offline, VtopDown, SignInNeeded, OtpNeeded, OtpRejected, Unexpected }

// core: LocalInspector — app provides it; any "Details" button opens a flow
fun interface InspectorOpener { fun open(flow: String) }

// design: one place renders every state, for every screen
@Composable fun <T> PageScaffold(state: PageState<T>, onRefresh: suspend () -> Unit,
    isEmpty: (T) -> Boolean, empty: @Composable () -> Unit, content: @Composable (T) -> Unit)
```

## UI

- Material 3 Expressive (Compose Material 3 from Compose Multiplatform 1.12):
  expressive motion scheme, floating toolbar, loading indicators. Not a copy
  of the Flutter screens; better where M3 is better, same information.
- Typography: Inter with the Flutter app's tracking curve, mapped onto the M3
  type scale. Colours: the app palette (neutral + indigo accent, success /
  warning / danger / lab tones) mapped onto M3 colour roles; a setting
  switches to wallpaper colours (dynamic colour, Android 12+).
- Floating nav bar: up to 5 tabs the user picks and orders (Settings →
  Appearance → Tabs in the bar, or long-press a tab); default Timetable,
  Attendance, More. Tapping the current tab scrolls to the top. More lists
  every feature plus Settings and the Inspector.
- Tab app bar: today's date, or what runs now ("Refreshing attendance…",
  from core's `Activity`; tap for recent activity, each row opens its flow in
  the Inspector) and an Offline chip when offline; on the right, feature
  actions, a season icon only while VTOP lists a newer semester than the one
  shown (the Flutter app's "New semester data available!"; tap: the semester
  sheet), and the avatar (dot: OTP waiting, offline,
  last refresh failed).
- Colours: seed palettes only (MaterialKolor): Graphite (default, the Flutter
  look), Indigo, Ocean, Violet, or the wallpaper; status tones harmonised
  toward the accent. Theme System / Light / Dark, pure black for OLED.
- Errors: a failed refresh is a toast (Retry / Sign in) and the data stays;
  only "nothing cached" is a full-screen problem. The data's age is a footer
  (`UpdatedFooter`) at the end of each page.
- Fetching (as the Flutter app's Auto Refresh, Settings → Data, on by
  default): a page with nothing saved always fetches; with Auto Refresh on, a
  page fetches on open (once per screen and semester), when its data is over
  6 h old (on open or when the app comes back to the foreground), and after
  the OTP prompt. The academic calendar is fetched again when a day old
  (after each attendance refresh, as Flutter's `calendarMaxAgeOnAttendance`,
  and when the timetable shows). The open-app sweep (Flutter's
  `StaleRefresher`): after a page fetch under 2 s, at most every 15 min, pages
  not on screen older than 12 h (attendance, timetable, marks, semesters,
  histories behind their summary) are fetched one at a time while nothing
  else runs; it stops at a failure or a page over 5 s and never prompts for
  an OTP. Settings → Account → Semester refreshes the list on each visit.
  Not built: the periodic background sync (Flutter: WorkManager, off by
  default, change notifications) and the exam-day exam schedule refresh (no
  exams page yet).
- Motion: M3 Expressive springs for push/pop and predictive back.
- States and edge cases (PageScaffold): first load (skeleton), cached +
  refreshing (age shown), offline with cache (banner), VTOP down, sign-in
  needed, OTP needed (sheet), wrong/expired OTP, nothing posted yet (empty),
  unexpected (with "Details" → Inspector at that flow). Every problem shows
  what happened, what to do, and its flow id.

## Observability in the app

- Settings → Inspector (visible to everyone): live list of flows (name, ms,
  outcome, time), tap → every Kotlin and Rust line of that flow; filter by
  level/src; share/export the log file. Secrets are never logged.
- Agent state dump: `adb shell am broadcast -a vitapmate.DUMP -p com.vitapmate.kmp`
  writes `files/state.json` (current route, pinned tabs, each page's state,
  last problem with flow) and logs it; `mise run state` pulls it.

## Rust API (the contract)

```rust
#[derive(uniffi::Object)] pub struct Core { .. }

#[uniffi::export(async_runtime = "tokio")]
impl Core {   // every method below also takes `flow: String` first (see Observability)
    #[uniffi::constructor] fn new(data_dir: String) -> Result<Arc<Core>, CoreError>;
    fn signed_in_user(&self) -> Option<String>;          // saved account, sync
    async fn sign_in(&self, username: String, password: String) -> Result<SignIn, CoreError>;
    async fn submit_otp(&self, code: String) -> Result<(), CoreError>;
    async fn resend_otp(&self) -> Result<(), CoreError>;
    fn sign_out(&self);
    fn cached_attendance(&self) -> Option<Attendance>;   // sync, from disk
    async fn fetch_attendance(&self) -> Result<Attendance, CoreError>;
    fn cached_timetable(&self) -> Option<Timetable>;
    async fn fetch_timetable(&self) -> Result<Timetable, CoreError>;
}

enum SignIn { Done, NeedsOtp { message: String } }
#[derive(uniffi::Error)] enum CoreError {
    Offline, VtopUnavailable { message }, InvalidCredentials,
    OtpRequired { message },  // a re-login during a fetch wants the OTP
    NotSignedIn, Other { message },
}
```

- Semesters: `cached_semesters` / `fetch_semesters` / `select_semester`.
  Pages are fetched and cached per semester (`pages/<semester>/<page>.json`)
  for the student's pick; nothing is guessed. Without a pick (or when VTOP no
  longer lists it) a page fails with `SemesterNeeded` before any request, and
  the app shows "Choose your semester" (VTOP's list, newest first, nothing
  pre-picked, then Continue), as the Flutter app's onboarding step 2: after
  sign-in and on an install that has no pick. The pick stays until the
  student changes it in Settings → Account → Semester. Any picked semester
  works like the current one (agenda, today), as in the Flutter app.
- `Core` keeps one `VtopClient`; a fetch that hits `SessionExpired` signs in
  again with the saved credentials and retries once.
- Cache and session live in `data_dir` (app-private). Credentials too, for
  now (TODO: Android Keystore).
- Records are ready to draw: numbers parsed, days as indices, times as minutes,
  attendance standing (`can_skip`, `must_attend`, `advice`) computed in Rust
  with the same arithmetic as the Flutter app's `attendance_standing.dart`.
- Logging: `tracing` to `data_dir/logs/rust.log`, one span per call, `regno`
  on lines; `vtop-core`'s `log` lines bridged in. Never cookies, passwords,
  OTPs, CSRF tokens.

## Observability (a main goal: humans and agents can see what the app does)

One timeline for Kotlin and Rust, readable over plain `adb`, no IDE.

- **Rust owns the log pipeline** (`rust/vtop-ffi/src/obs.rs`, its own module):
  `tracing` with three layers: JSON lines to `files/logs/app.jsonl` (rolls
  to `.1` past 1 MB; fields `t l src msg flow regno` like the Flutter
  app's `log.dart` lines, plus span fields and `ms` on span
  close), logcat (tag `vitapmate`), and ATrace sections so Rust spans show
  in Perfetto next to Compose frames. `vtop-core`'s `log` lines bridged in.
- **Flows**: every user action or background run is a flow. Kotlin's
  `flow(name) { id -> ... }` makes the id and passes it as the first argument
  of every `Core` call (`flow: String`); Rust opens a span with it, so each
  line carries the flow, and the span close logs the duration and the
  outcome (ok / error kind).
- **Kotlin logs go through Rust** (`log_event(level, src, flow, msg, fields)`,
  `obs_init(data_dir)` before `Core`), so there is one file and one order.
  Uncaught exceptions are logged before the default handler runs.
- **UI is inspectable**: `testTagsAsResourceId = true` at the root and test
  tags on screens and key elements, so `uiautomator dump` shows them; each
  screen logs its state changes (`src: ui.<screen>`: loading / cached / fresh
  / error, item counts, ms).
- **Agent commands** (mise tasks): `logs` (`adb logcat -s vitapmate`),
  `logs-pull` (`run-as ... cat files/logs/app.jsonl`), `ui-dump`
  (`uiautomator dump`), `trace` (Perfetto capture of app + Rust sections).
- Never logged: cookies, passwords, OTPs, CSRF tokens, keys.

## Rules

- Material 3 Expressive; typography and palette from the Flutter app (see UI).
- Fewest lines; one widget per file; no pass-through layers.
- Calls into Rust off the main thread; one call per screen state.

## Release build

`mise run release` writes the signing file, builds and verifies
`build/vitapmate-release.apk`.

- Signing: `settings.android.signing: enabled` in `app/module.yaml` reads
  `app/keystore.properties` (`storeFile`, `storePassword`, `keyAlias`,
  `keyPassword`; gitignored). `mise run signing` writes it from
  `keystore/keystore.properties` (gitignored; the Flutter app's shape) with
  the keystore's absolute path, so the APK carries the Flutter app's upload
  key. The keystore stays in `keystore/`.
- The toolchain turns on R8 and resource shrinking for release and reads
  `app/proguard-rules.pro` (keeps JNA and the UniFFI bindings,
  `vitapmate.core.**`).
- `./kotlin build -v release` alone gives
  `build/tasks/_app_buildAndroidRelease/gradle-project-release.apk`
  (`-unsigned` when the properties file is missing).
- Phone: `adb install -r build/vitapmate-release.apk`. A debug build
  is signed with the debug key, so replacing it needs `adb uninstall
  com.vitapmate.kmp` first (clears the signed-in account).

## Mail forwarding

VTOP's sign-in OTP reaches the phone by forwarded mail (the Flutter app's
`mail_forwarding_service.dart` and `MailForwardingPage`; Gmail OAuth is not
ported). Rust: `rust/vtop-ffi/src/forwarding.rs`; UI: Settings → Account →
Mail forwarding (`ForwardingSheet` in `features/login`).

- **Address.** `set_up_forwarding` registers the install with the edge
  (`/v1/devices/register`), makes an X25519 key pair (`vtop_shared::mail_seal`)
  and, with a live VTOP session, first tries `/v1/mail-otp/reclaim` (the edge
  asks the engine whose session the cookies are and finds that student's
  address by its owner hash): a student who already has an address, e.g. from
  the Flutter app, joins it and Gmail needs no changes. Otherwise
  `/v1/mail-otp/register` makes `vtop-otp-<id>@aster0.dev` and `/owner` ties it
  to the student. The id, address, mailbox secret, private key and device
  secret go to `forwarding.json` in the data dir and are never logged.
- **Relay.** The edge's email handler (`vtop-edge/src/mail.ts`) keeps only
  mail from `noreply.sdc@vitap.ac.in` (the 6-digit OTP) and Gmail's forwarding
  confirmation (code, link, requester); anything else is dropped. It seals the
  JSON payload to each member's public key (AES-256-GCM, HKDF over X25519,
  mailbox id as AAD), so the server can't read it, and keeps the latest box
  for 5 minutes per member.
- **Pull.** The phone POSTs `/v1/mail-otp/inbox` with its id and secret and
  opens the box locally. This build has no FCM: the device is registered
  without a push token (edge 0.5.1 skips pushes for it) and the app polls
  instead of waiting for pushes or the event stream.
- **Setup check.** States: off, waiting (an address, no mail yet), on (any
  forwarded mail arrived, or the address was joined). While waiting, the sheet
  checks the inbox every 5 s; Gmail's confirmation mail arriving proves the
  address works and shows its code and "Confirm in Gmail" link.
- **Gmail steps shown.** 1. Gmail → Settings → Forwarding: add the address
  (Chrome, desktop site), leave forwarding disabled. 2. Confirm with the code
  or link the sheet shows. 3. Create a filter From `noreply.sdc@vitap.ac.in`,
  Forward it to the address.
- **OTP prompt.** With an address, `OtpChallenge.autofill` runs flow
  `forwarding.poll` while the code is live: `fetch_forwarded_otp(since)` every
  2 s (8 s timeout each), takes a 6-digit OTP sent no earlier than the round's
  start minus 60 s, skips codes VTOP turned down, fills it in, submits, and
  shows "From your forwarded mail". Resend starts a new round.
- **Turn off.** `/v1/mail-otp/delete` takes this install off the address (the
  edge deletes the address when no other install uses it), then the file goes.
  An address the edge no longer knows (`mailbox_not_found`) is forgotten.
- **Local edge.** A one-line `files/forwarding_server` (e.g.
  `http://127.0.0.1:8787` with `adb reverse tcp:8787 tcp:8787` and
  `wrangler dev --local`) points new setups at another server; mail is
  injected with `POST /cdn-cgi/handler/email?from=…&to=…`.
- TODO: FCM, the device list, a Developer setting for the poll interval.

## Measure (Flutter vs Compose, `phone` emulator, release builds)

Cold start to first timetable frame, APK size (arm64), RAM after opening
attendance, janky frames while scrolling the weekly grid (`dumpsys gfxinfo`).

Run with a local `bench.sh` (SurfaceFlinger frames, so both apps are measured
the same way; not checked in).

| 2026-11-05, emulator `phone` (arm64) | Flutter (main 061c5e2b, prod release) | Compose |
|---|---|---|
| APK, arm64 | 32.2 MB (libflutter 11.2, libapp 11.1, rust 4.6, dex 1.5) | |
| Cold start, median of 5 | 507 ms (426–686) | |
| PSS on timetable | 144.1 MB | |
| Weekly grid, 20 swipes | 63 frames, 1–2 over 20 ms | |

## TODO

- Changes inbox: a bell with a count next to the avatar listing what changed
  since the student last looked (attendance posted per class, marks out,
  timetable changed, a course dropping under 75%); tapping one opens that
  course. Rust keeps a diff of each fetch against the cached copy (the
  Flutter app's `change_alert_notification_service.dart` detects the same
  changes for notifications).
- Done (2026-11-06): installs register without a push token; the Worker (0.5.1) accepts that and skips pushes to them and to the old placeholder.
- Mail forwarding, emulator: on 2026-11-06 07:51 the emulator install was
  joined to the live forwarding address as an extra device. Turn it off from
  the sheet when done testing.

### Hand-off 2026-11-08

Built this round (all on emulator-5554 unless noted): Outing (lists, pass in
Docs; no apply/cancel sent), Docs (download folder, every download in Docs,
real file names, Nov mess menu + M3 Expressive menu), Biometric, Course page,
toasts (above the nav bar, user-initiated only; "Show background errors" in
Developer → Refresh), cloud-off offline icon, user Activity view (compact
cards, this-session default, "Show older", Settings → Activity), Account page
(username + password, device-auth gate only with a screen lock), Developer
"Set credentials without checking" (in place), password eye toggle, attendance
sheet (9/10, refresh beside the title, calendar, footer), timetable header chip
+ one-line subtitle, animated splash (API 31+), haptics + switch, profile padding.

Next:
- Rust core without I/O + Kotlin transport: docs/plans/rust-proto.md (decisions made; start with phase 0).
- Later: redesign the server (bridge, MCP, gateway) as its own project, likely on vtop-proto once the app uses it.
- Check on a phone: timetable scroll bounce (bar snap removed in Page.kt; fallback: fixed bar on Timetable), haptics, profile padding, splash in dark mode, Activity in light theme.
- Build the signed release (`mise run release`) and install on the phone (85ed7db1); the user builds it themselves.
- Small: haptic on a refused OTP code on the login screen and on VTOP reload done; Account save + OTP cancel keeps old creds on disk. (Accepted as is: SWE3003 own section not always detected, course title twice; files open in the phone's viewer by design.)
- Emulator: "Show background errors" is ON there (default off).
- Open questions: icon-only Grades tabs; tab bar letter from name or reg no. Decided 2026-11-08: Inspector only in Developer; Developer unlocked by tapping the version 7 times (About in More or Settings); version shows "name (code)"; no "prototype" text; activity-compose kept (photo and folder pickers use it).
