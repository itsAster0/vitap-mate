# AGENTS.md

vitapmate is an unofficial Android app for VIT-AP's VTOP portal. It shows the
timetable, attendance, marks, grades, exams, outings, course pages and more, and
caches them on the phone. The UI is Compose Multiplatform, built with the Kotlin
Toolchain (no Gradle), over a Rust core through UniFFI. It runs on Android only;
iOS would need other Kotlin bindings for UniFFI.

Read `rust/ARCHITECTURE.md` (the crates, the routes from the phone, UDP mode,
deploys) and `rust/vtop-edge/API.md` (every edge route) before you change the
Rust core or the server.

## Repo map

```
project.yaml          Kotlin Toolchain project: one module per layer and feature
kotlin kotlin.bat     toolchain wrapper (./kotlin build, run, test)
app/                  shell: feature registry (Features.kt), nav, More, Settings, Connection,
                      Server, Connected apps, About, Developer, Inspector, MainActivity;
                      res/, jniLibs/ (built, gitignored)
core/                 vitapmate.core: generated UniFFI bindings (vtop_ffi.kt, do not edit)
                      vitapmate.data: AppCore, pages (rememberPage, PageState), Settings,
                      the HTTP and UDP transports (VtopHttp, VtopHttpEngine, VtopUdp)
                      vitapmate.observability: Log, flow(), activity history, state dump
design/               vitapmate.ui: theme, shared components, PageScaffold, Feature type
features/<name>/      one feature per module (module.yaml + src/vitapmate/features/<name>/)
rust/                 Cargo workspace
  vtop-ffi/           the app's crate: Core, cache, session, records, routes (route.rs),
                      UDP (udp.rs), server page (server.rs), logs (obs.rs)
  vtop-proto/         parsers, types and HTTP shapes as page machines, with no I/O
  vtop-core/          VTOP client that runs vtop-proto's machines over a Transport
  vtop-shared/        formats the app, engine and edge share (device_sig, mail_seal, page_dictionary)
  vtop-wire/          vm1, Noise IK over UDP for slow, lossy links
  vtop-gateway/       UDP gateway on the Oracle VPS robin
  vtop-bridge/        the engine: sign-in, CAPTCHA, fetch and parse; Railway service engine-v2
  vtop-mcp/           MCP tools the engine serves behind the edge's /mcp
  vtop-edge/          Cloudflare Worker and Durable Objects (TypeScript), all server state
extension/            Chrome and Firefox extension (Svelte, bun)
fastlane/             Play Store lanes and release notes
keystore/             release keystore (gitignored; never read or print it)
```

## Where a change goes

| Change | Place |
|---|---|
| New VTOP page | Parser in `rust/vtop-proto/src/parser/` and its machine in `rust/vtop-proto/src/pages.rs`, call in `rust/vtop-core/src/client/`, FFI in the feature's file `rust/vtop-ffi/src/<feature>.rs` (its own `#[uniffi::export] impl Core` block, `cached_x` and `fetch_x`, flow id first) and an entry in `rust/vtop-bridge/src/pages.rs`. Read pages go through `page!` or `Core::fetch` with the bridge's page name, so the server routes can fetch them. Then run `mise run rust-android`. |
| Use a VTOP page in a feature | `rememberPage("<id>", { c, f -> c.cachedX(f) }, { c, f -> c.fetchX(f) }, ...)` in the screen, drawn with `PageScaffold` (see `core/src/vitapmate/data/Page.kt`). |
| New screen | `features/<name>/` with a `module.yaml` that depends on `//design` only, and `src/vitapmate/features/<name>/` exposing `val <name>Feature: Feature`. Add one line to `project.yaml` and one to `app/src/vitapmate/Features.kt`. A feature never imports another; code two features share moves to `design` (UI) or `core` (data). |
| Shared widget | `design/src/vitapmate/ui/`, one widget per file. |
| Shell, nav, Settings, Developer, Inspector UI | `app/src/vitapmate/`. |
| Data, settings store, downloads, files | `core/src/vitapmate/data/`. |
| How the app reaches VTOP | `rust/vtop-ffi/src/route.rs` (Direct, Proxy, Server, Udp, and the 5-minute skip after a failure). |
| Logging | Kotlin: `core/src/vitapmate/observability/` (`Log`, `flow()`). Rust: `tracing`, with the pipeline in `rust/vtop-ffi/src/obs.rs`. |
| Server-side state | Durable Objects in `rust/vtop-edge`, never the engine or the gateway. Document each route in `rust/vtop-edge/API.md`. |

## Rules

- Keep the same UI and feel. Change visuals only when asked. Use Material 3
  Expressive and theme colours only.
- Reuse and move working code, in small diffs. Rust may be refactored for
  better traces, logs and errors.
- Write the fewest lines that do the job: no single-caller wrappers or
  pass-through layers, and stubs for parts not built yet.
- When something could be done better, ask and say what you will do by
  default, then carry on.
- Call Rust off the main thread, once per screen state.
- Every user action or background run is a `flow()`. Its id is the first
  argument of every `Core` call, so Rust's log lines carry it too, and every
  line carries `regno`. Kotlin logs go through Rust into one file
  (`cache/logs/app.jsonl`) and logcat (tag `vitapmate`). Logs stay on the phone
  until the user shares them from Developer. Never log cookies, passwords,
  OTPs, CSRF tokens or keys.
- Background work uses short timeouts (the open-app sweep stops at a page that
  takes over 5 s), never blocks or interrupts the user, and is tunable in
  Settings → Developer (`Tunable` in `core/src/vitapmate/data/Settings.kt`).
- VTOP writes (outing apply and cancel) show a result only after VTOP confirms
  it.
- Rust follows the Google Rust style guide. New crates use edition 2024.

## Setup

The repo is `github.com/itsAster0/vitap-mate`. The `main` branch is this app.
The local `flutter` bookmark keeps the old Flutter app for reference; its server
(`vmate-api.aster0.dev`) still runs for old installs.

`.env.json` is gitignored and holds the Firebase app values (keys listed in
`.env.example.json`). `mise run firebase` turns it into
`app/res/values/firebase.xml`, which the app reads at start. CI writes
`.env.json` from the `ENV_JSON` secret. The server URL is `DEFAULT_SERVER` in
`rust/vtop-ffi/src/forwarding.rs` (`https://vmate.aster0.dev`). Phones sign
their requests with their own device key, so the app ships no server secret.

## Commands

mise installs the toolchains in `mise.toml`: Java, the Kotlin Toolchain and
bun. The edge (`rust/vtop-edge`) and the extension use bun.

```sh
mise run rust-android        # after changing rust/vtop-ffi: builds the .so into app/jniLibs and the Kotlin bindings
./kotlin build
./kotlin test -m core
./kotlin run -d emulator-5554
mise run release             # signed 64-bit APK at build/vitapmate-release.apk (--both: also -32bit.apk)
mise run logs                # also logs-pull, state, ui-dump and trace for the running app over adb

cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd rust/vtop-edge && bun run test && bun run check
cd extension && bun run test && bun run build
```

Deploy only when asked, and only from a laptop: CI builds the app and the
extension but deploys nothing. `bun run deploy` in `rust/vtop-edge/` deploys the
edge (`vmate-edge` on vmate.aster0.dev), the engine (Railway `engine-v2`,
`Dockerfile.engine`) and the UDP gateway (pushed to GHCR, run on robin as the
container `vitapmate-gateway`) from `alchemy.run.ts`, with the values in its
`.env.example`. The gateway needs `GHCR_TOKEN` (a GitHub token with
`write:packages`) and `GATEWAY_SSH=ssh://rocky@robin`; `SKIP_GATEWAY=1` deploys
without it. The README lists the CI workflows.

Never print or commit `.env*` files (`extension/.env` is the one tracked
exception, and it holds only the server URL), `keystore/`,
`app/keystore.properties`, `fastlane/key.json` or other secrets.

## Release

Google Play has the app as `com.vitap_pal.app`, the Flutter app's id, and takes
the AAB. To release:

1. Raise `versionCode` and `versionName` in `app/module.yaml`. Codes follow
   `M m pp bbb` from 3000000 (3.0.0); Play refuses a code it has seen.
2. Write `fastlane/metadata/android/en-GB/changelogs/<versionCode>.txt` (at most
   500 characters) and add the version to `RELEASES` in
   `app/src/vitapmate/UpdatesScreen.kt`.
3. Push the tag `v<versionName>`. `android-release.yml` builds the signed AAB
   (64-bit and 32-bit) and uploads it to the internal track.
4. Promote in the Play Console, or with `bundle exec fastlane android
   promote_all` (friends, alpha, beta). Production stays manual.

How hard the app asks to update is the release's in-app update priority, set
per versionCode in `UPDATE_PRIORITIES` in `fastlane/Fastfile` (add a line in the
release commit), `DEFAULT_UPDATE_PRIORITY` (3) when not listed: 0–1 nothing, 2 a pill above the nav bar, 3 Play's flexible
dialog on start, 4 Play's full-screen update (can be closed), 5 the app's own
blocking screen (`app/src/vitapmate/PlayUpdate.kt`). Promotions reuse it.

From a laptop, with the keystore and `keystore/play-service-account.json` in
place: `bundle install`, then `bundle exec fastlane android deploy_internal`.

## Version control

jj, colocated with git. Use `jj` for anything that changes history and no
mutating `git` commands. Commit only when asked and never push unless asked.
Use Conventional Commits with a subject under 72 characters.

## Testing

Test on the emulator or a connected phone, not only with a build. Parser tests
are in `rust/vtop-proto/tests`, client tests in
`rust/vtop-core/src/client/tests.rs`, the app crate's in
`rust/vtop-ffi/src/tests.rs` and next to each module, vm1's in
`rust/vtop-wire/src/tests.rs`, the edge's in `rust/vtop-edge/src/*.test.ts`,
and Kotlin's in `core/test/`.
