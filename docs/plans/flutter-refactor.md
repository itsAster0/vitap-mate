# Plan: refactor the Flutter app

Status: draft. Replaces the Compose rewrite, which was dropped.

Keep Flutter and the Rust core. Restructure the Dart code so it is lean,
linear and modular, make the app smaller, and make every layer observable,
with the same UI and feel. Android first; iOS keeps building from the same
code.

## Rules

- **Same UI.** Screens look and behave as today. Visual changes only when
  asked.
- **Reuse, don't rewrite.** Move working code into the new structure; change
  only what a step needs. Rust may be refactored for better traces, logs and
  errors.
- **Fewest lines that do the job.** No single-caller wrappers, no
  pass-through layers. Unbuilt parts are stubs.
- **Ask when something could be done better**, with a default, and keep
  working while waiting.
- Latest stable Flutter/Dart and Rust, and their features. Rust follows the
  Google Rust style guide.
- Each step leaves the app building, tests passing, and running on the
  emulator.

## Target layout

```
lib/
  main.dart
  app/            app widget, router, shell (bottom bar), theme
  ui/             design system: one widget per file (today's ui.dart split)
  observability/  Log, flow() (flow id + timing), crash handler, log viewer
  vtop/           backend (on-device, server, UDP, re-login), queries,
                  session, sign-in and OTP, VtopPage (cache + refresh)
  platform/       notifications, background work, calendar sync, updates,
                  secure storage, files, webview store
  features/
    timetable/ attendance/ marks/ grades/ grade_history/ exams/ gpa/
    biometric/ calendar/ course_page/ docs/ outing/ profile/
    vtop_browser/ account/ server/ settings/ developer/ onboarding/
```

Each feature folder has the same shape:

```
features/attendance/
  attendance.dart      PUBLIC: its providers (on VtopPage) and anything other
                       features may use, re-exported
  attendance_page.dart the screen(s)
  logic/               pure Dart (calculators, projections), with tests
  widgets/
```

Rules, checked by `test/architecture_test.dart` (scans imports):

- A feature imports `app/`, `ui/`, `observability/`, `vtop/`, `platform/`,
  itself, and other features only through their public file
  (`features/<name>/<name>.dart`), never their pages, widgets or logic.
- `app/` composes features and may import their pages.
- `ui/` imports nothing from the app.
- Only `vtop/` touches the generated Rust bindings (`lib/src/`).

## One path for VTOP data

Today each page goes through a data source, a repository, two providers
under `state/`, the page provider and `VtopController`: about 40 files for 10
pages.

After: one generic `VtopPage<T>` (Riverpod family) built from the existing
`VtopQueries` entry. It reads the cache first, refreshes through
`VtopBackend` (which already handles re-login), writes the cache, and logs
both under the caller's flow id. A feature's provider becomes one line:

```dart
final marksProvider = vtopPage(VtopQueries.marks);
```

Saved pages that no longer parse (older formats) read as missing and are
fetched again; no compatibility code. Credentials, sessions and tokens are
never touched.

Feature-specific logic (attendance history sync, projections) stays in the
feature's `logic/`.

## Observability, in every layer

- **Flow id**: each user action or background run starts a `flow()` that
  makes an id, keeps it in a Zone, times the block and logs start/end. Every
  log line inside carries it; it is sent as `x-request-id` to the server and
  passed into Rust calls.
- **Dart**: `Log` (one entry point for all Dart logging) writes JSON lines
  to the app's log file, labelled with `regno` and `flow_id`. Screens log
  opens and actions; providers log state changes; `vtop/` logs route, cache
  hit/miss and fallbacks.
- **Rust**: `tracing` everywhere (today `vtop-core` uses `log`); spans per
  VTOP request (context, regno, attempt, status, bytes, ms) and per sign-in
  step; errors carry context. Rust log lines go into the same file.
- **Logs stay on the phone.** Developer settings shows them (level, text and
  flow id filters) and can share/export them. No upload.
- **Server**: the node logs JSON (`fly logs`), the Worker logs one line per
  request with `regno` and `request_id`.
- Never logged: cookies, passwords, OTPs, CSRF tokens, keys.

## Background work

| Task | Default timeout | On failure |
|---|---|---|
| Connection warmup | 4 s | Logged only |
| Background refresh of stale pages | 5 s per page | Logged; cached data stays |
| Periodic sync | 30 s per run | Logged; next run retries |
| Foreground fetch | 25 s | Shown at once |

After 3 background failures in a row, a quiet hint on the next screen; never
a dialog. All timeouts and the threshold are editable in Developer settings,
along with log level and a route override.

## Smaller app

1. Measure first: `flutter build apk --release --analyze-size
   --target-platform android-arm64`.
2. Compress native libraries in the APK (`useLegacyPackaging = true`); the
   app ships as APKs from GitHub, so download size is what matters.
3. Release builds with `--split-debug-info` and `--obfuscate`.
4. Remove what the refactor makes unused (dependencies, desktop and web
   folders), each confirmed first.
5. Measure again after each step.

## Steps

1. **Skeleton**: create `app/`, `ui/`, `observability/`, `vtop/`,
   `platform/` and move `core/` into them (moves and import fixes only),
   plus the architecture test.
2. **VtopPage**: the generic cached page; move attendance and timetable onto
   it, delete their data source / repository / state files.
3. **Features**: split `features/more` (marks, grades, grade history, exams,
   biometric, GPA, VTOP browser) and `features/settings` (account, server,
   developer) into their own folders; move each remaining page onto
   VtopPage.
4. **Observability**: `Log`, `flow()`, the log file and viewer, Rust
   `tracing` and its bridge into the same file, Developer settings.
5. **Background**: timeouts, failure counting, quiet hint.
6. **Size**: the steps above, with numbers before and after.
7. **Cleanup**: file and name typos (`clinet_provider`, `extention`,
   `onbaording`...), large files split where a screen has clear parts,
   docs (`AGENTS.md`, `rust/ARCHITECTURE.md`).

## Next steps (decided 28 Oct 2026)

Order: logging foundation first, then `vtop/` cleanup with its logging in the
same pass, then background timeouts, then APK size.

1. **Logging foundation**: `observability/log.dart` (`Log` + `flow()`),
   JSON lines to a rolling file on the phone (about 2 MB, two files), flow id
   on every line and as `x-request-id`, regno on lines. Rust `vtop-core` on
   `tracing`, written to the same file. Developer → Logs reads the file with
   level / text / flow filters, share and export. No upload.
2. **`vtop/` cleanup, shape only**: modern Dart, dead code out, misspelled
   names fixed, remaining direct binding calls moved behind `vtop/`. Sign-in,
   OTP and session behaviour unchanged; storage keys and formats unchanged.
3. **Background**: existing timeouts become settings with defaults
   (background refresh 5 s, sync 30 s, foreground 25 s, failure threshold 3),
   editable in Developer settings; quiet hint after repeated failures. No
   connection warmup.
4. **APK size**: measure with `--analyze-size`; drop the Cronet stack
   (`cronet_http`, `third_party/cronet_http`, `cupertino_http`, platform HTTP;
   Rust does all HTTP) and dependencies the report shows are unused (each
   confirmed first). No native-lib compression, no obfuscation.
   Done for Cronet: arm64 release 32.8 → 32.0 MB; vtop-core's `Transport`
   hook went with it.

## Open questions

1. Riverpod: keep code generation (`riverpod_generator`, `freezed`) or write
   providers by hand? Default: keep, to avoid rewriting providers.
2. Desktop and web folders (`linux/ macos/ windows/ web/`): delete?
   Default: delete in step 6 after confirming.
