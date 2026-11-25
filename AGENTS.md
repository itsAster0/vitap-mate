# AGENTS.md

vitapmate is an unofficial Android app for VIT-AP's VTOP portal. It shows the
timetable, attendance, marks, grades, exams, outings, course pages and more, and
caches them on the phone. The UI is Compose Multiplatform, built with the Kotlin
Toolchain (no Gradle), over a Rust core through UniFFI. It runs on Android only;
iOS would need other Kotlin bindings for UniFFI.

Read `rust/ARCHITECTURE.md` (the crates, the routes from the phone, UDP mode,
deploys), `backend/README.md` (the Convex functions) and
`docs/plans/2026-11-23-backend-v3.md` before you change the Rust core or the
server. `backend/API.md` is the HTTP contract the extension, MCP clients and the
engine use.

## Ways to break things

- The connected real phone holds the user's own cache and accounts: never
  clear its data, uninstall, sign out or roll pages back on it. Use the emulator.
- Deploying prod (`bun run deploy`, hawk, the Workers) without an explicit OK
  for that step.
- Renaming a spec's name, args or file (`rust/vtop-proto/tests/spec_contract.txt`
  only grows at its end).
- Sending the VTOP session to our servers without `consent::allowed`.
- Release lanes: pushing a `v*` tag, `fastlane android upload_internal`,
  `upload_testing`, `promote_testing`, `promote_production` or `widen_rollout` upload to Play
  (`status` only reads).

Agents never open PRs or issues, push, or deploy unless asked.

## Repo map

```
project.yaml          Kotlin Toolchain project: one module per layer and feature
kotlin kotlin.bat     toolchain wrapper (./kotlin build, run, test)
app/                  shell: feature registry (Features.kt), nav, More, Settings, Connection,
                      Account, Connected apps, About, Developer, Inspector, MainActivity;
                      res/, jniLibs/ (built, gitignored)
core/                 vitapmate.core: generated UniFFI bindings (vtop_ffi.kt, do not edit)
                      vitapmate.data: AppCore, pages (rememberPage, PageState), Settings,
                      the HTTP and UDP transports (VtopHttp, VtopHttpEngine, VtopUdp)
                      vitapmate.observability: Log, flow(), activity history, state dump
design/               vitapmate.ui: theme, shared components, PageScaffold, Feature type
features/<name>/      one feature per module (module.yaml + src/vitapmate/features/<name>/)
rust/                 Cargo workspace
  vtop-ffi/           the app's crate: Core, cache, session, records, routes (remote/route.rs),
                      UDP (remote/udp.rs), server account (remote/server.rs), logs (obs.rs)
  vtop-proto/         parsers, types and HTTP shapes as page machines, with no I/O
  vtop-core/          VTOP client that runs vtop-proto's machines over a Transport
  vtop-shared/        formats the app, engine and backend share (device_sig, mail_seal, page_dictionary)
  vtop-tlsn/          TLSNotary account proof of /vtop/content (prover in the app, verifier in the engine)
  vtop-wire/          vm1, Noise IK over UDP for slow, lossy links
  vtop-gateway/       UDP gateway on the Linode fedora-in-maa
  vtop-bridge/        the engine: sign-in, CAPTCHA, fetch and parse; on hawk (deploy/hawk)
  vtop-mcp/           MCP tools the engine serves behind the backend's /mcp
backend/              self-hosted Convex (TypeScript, bun): all server state; the phone's one socket
mail/                 Cloudflare Email Worker: hands forwarded mail to the backend, keeps nothing
proxy/                Cloudflare Worker vmate-proxy: VTOP behind a compressing proxy (the Proxy route)
deploy/               alchemy.run.ts: hawk's compose (hawk/), Workers, then Convex
extension/            Chrome and Firefox extension (Svelte, bun)
fastlane/             Play Store lanes and release notes
keystore/             release keystore (gitignored; never read or print it)
```

## Where a change goes

| Change | Place |
|---|---|
| New VTOP page | Parser in `rust/vtop-proto/src/parser/`, its machine in `rust/vtop-proto/src/pages.rs`, and a `PageSpec` (name, args, cache file, machine) in `rust/vtop-proto/src/spec.rs`, listed in `spec::each`. vtop-core runs it with `VtopClient::fetch(&spec, &query)` and the engine serves everything in `spec::each`, so a read page needs nothing in vtop-core or vtop-bridge (writes and downloads keep a method in `rust/vtop-core/src/client/`). FFI in the feature's file `rust/vtop-ffi/src/pages/<feature>.rs` (its own `#[uniffi::export] impl Core` block, `cached_x` and `fetch_x`, flow id first): `fetch_x` calls `Core::fetch(&spec::X, ...)`, which caches it under the picked semester (`Core::page` for one not cached), and `cached_x` reads it back with `read_page`. Both go by the chosen route, so the server routes fetch it too. Never rename a spec's name, args or file: old apps and the deployed engine use them. Then run `mise run rust-android`. |
| Use a VTOP page in a feature | `rememberPage("<id>", { c, f -> c.cachedX(f) }, { c, f -> c.fetchX(f) }, ...)` in the screen, drawn with `PageScaffold` (see `core/src/vitapmate/data/Page.kt`). |
| New screen | `features/<name>/` with a `module.yaml` that depends on `//design` only, and `src/vitapmate/features/<name>/` exposing `val <name>Feature: Feature`. Add one line each to `project.yaml`, `app/module.yaml` (`- //features/<name>` under `dependencies`) and `app/src/vitapmate/Features.kt`. A feature never imports another; code two features share moves to `design` (UI) or `core` (data). |
| Shared widget | `design/src/vitapmate/ui/`, one widget per file. |
| Shell, nav, Settings, Developer, Inspector UI | `app/src/vitapmate/`. |
| Data, settings store, downloads, files | `core/src/vitapmate/data/`. |
| How the app reaches VTOP | `rust/vtop-ffi/src/remote/route.rs` (Direct, Proxy, Server, Udp); the policy (which route to try, the 5-minute skip after a failure, a server fetch's steps) is pure, in `rust/vtop-ffi/src/remote/route/plan.rs`. |
| Logging | Kotlin: `core/src/vitapmate/observability/` (`Log`, `flow()`). Rust: `tracing`, with the pipeline in `rust/vtop-ffi/src/obs.rs`. |
| Server-side state | Convex in `backend/convex/`, never the engine or the gateway. A read the app shows is a query it can watch live (`LiveTopic`); queries take no `flow` argument, so the same read shares the watch's subscription. List new functions in `backend/README.md`. |

### Touch every surface

A page or route change reaches more than one place. Check each:

- The four routes: Direct, Proxy, Server and Udp (`rust/vtop-ffi/src/remote/route.rs`).
- Cached and fresh data: `cached_x` with no cache yet, and `fetch_x` over it.
- The vtop-mcp tools (`rust/vtop-mcp/src/tools.rs`) for a read page.
- The extension, when a session or server route changes (`extension/AGENTS.md`).
- `"since": <versionCode>` in `backend/guides/updates.json` for a new feature.
- The demo data in `backend/guides/demo.json`, so the demo account shows it.

### Glossary

- flow: one user action or background run (`flow()`); its id goes first in every `Core` call.
- route: how the phone reaches VTOP: Direct, Proxy (the same requests through the
  `proxy/` Worker, whose URL Convex gives out as `proxy:info`), Server (the engine
  fetches) or Udp (Server over vm1).
- spec: a `PageSpec` in `rust/vtop-proto/src/spec.rs`: name, args, cache file, machine.
- page: one VTOP page, fetched by its spec and cached on the phone per semester.
- backend: the self-hosted Convex the phone talks to over one WebSocket (`backend/`);
  the app and the mail Worker use vmate.aster0.dev (prod; there is no test
  deployment, so point a debug build at a local Convex from Settings → Developer → Server).
- engine: `rust/vtop-bridge`, on hawk: signs in and fetches for the server routes.
- gateway: `rust/vtop-gateway`, the UDP end of vm1 on the Linode.
- vm1: Noise IK over UDP for slow, lossy links (`rust/vtop-wire`).
- consent kinds: `CookieKind::Route` (a server route fetching), `Account` (the server
  checking who the student is) and `Handover` (a connected app asking for a session).

## Rules

- Keep the same UI and feel. Change visuals only when asked. Use Material 3
  Expressive and theme colours only.
- Reuse and move working code, in small diffs. Rust may be refactored for
  better traces, logs and errors.
- Write the fewest lines that do the job: no pass-through layers that add
  nothing (Wire/Stored types and a flow's local driver are fine), and stubs
  for parts not built yet.
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
- A send of the VTOP session to our servers goes through `consent::allowed`
  (`rust/vtop-ffi/src/remote/consent.rs`); background flows are
  `flow(name, unattended = true)` and send it only over the chosen route
  with its "Don't ask again" ticked (else they fetch Direct).
- Rust follows the Google Rust style guide. New crates use edition 2024.
- No `let _ =` on a fallible call without a comment saying why the error
  doesn't matter. `#[expect(lint, reason = "...")]` over `#[allow]`. A
  `// SAFETY:` comment on every `unsafe` block.
- Comments say why, not what, and carry no session history ("now", "changed to").
- State has one owner. Server state lives in Convex and is watched live.
  Domain state (session, identity, pages, forwarding, consent, routes) and
  every calculation on it (attendance, grades, plans) live in Rust and cross
  UniFFI as enums and records. Kotlin keeps only temporary UI state (input
  being typed, a picked day, an open sheet, scroll) and Android-only state
  (permissions, Keystore, the password manager). A Kotlin screen never copies
  or recomputes domain state.
- Make invalid states unrepresentable: model mutually exclusive phases as Rust
  enums with data (Kotlin sealed classes through UniFFI) and Kotlin sealed
  interfaces, not parallel booleans, nullable pairs, empty-string sentinels or
  string-matched codes. Parse untrusted input (VTOP HTML, server JSON, disk
  files, user input) once at the boundary into domain types; keep wire and
  disk formats compatible with explicit Wire/Stored types and roundtrip tests,
  never `#[serde(untagged)]`. Multi-step flows keep transitions pure
  (`next(state, event)` returning the next state and a small closed Effect
  type) with a thin local driver, not a shared framework.

## Setup

The repo is `github.com/itsAster0/vitap-mate`. The `main` branch is this app.
The local `flutter` bookmark keeps the old Flutter app for reference; its server
(`vmate-api.aster0.dev`) still runs for old installs.

`.env.json` is gitignored and holds the Firebase app values (keys listed in
`.env.example.json`). `mise run firebase` turns it into
`app/res/values/firebase.xml`, which the app reads at start. CI writes
`.env.json` from the `ENV_JSON` secret. The server URL is `DEFAULT_SERVER` in
`rust/vtop-ffi/src/remote/edge.rs`. A phone proves the account once (TLSNotary),
then holds a short JWT and a refresh token signed by its own device key, so the
app ships no server secret.

## Commands

mise installs the toolchains in `mise.toml`: Java, the Kotlin Toolchain and
bun. The backend, the Workers, the deploy and the extension use bun.

```sh
mise run rust-android        # after changing rust/vtop-ffi: builds the .so into app/jniLibs and the Kotlin bindings
mise run rust-android-dev    # the same, arm64 only and unoptimised, for quick runs on an arm64 emulator or phone
./kotlin build
./kotlin test                # every module's tests (core, design, features)
./kotlin run -d emulator-5554
mise run release             # signed 64-bit APK at build/vitapmate-release.apk (--both: also -32bit.apk)
mise run logs                # also logs-pull, state, ui-dump and trace for the running app over adb
mise run detekt              # Kotlin lint (brew install detekt); CI runs the same
mise run notes [since]       # draft the release notes from the commits since the last release

cd rust && cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cd backend && bunx tsc -p . --noEmit && bunx vitest run
cd backend && bunx convex deploy -y && bun run smoke   # prod: needs an OK
cd extension && bun run test && bun run build
mise run check               # all of what CI runs; or check-rust, check-kotlin, check-backend, check-workers, check-deploy, check-ext
```

Run `mise run check` (or the part you touched) before saying you're done.
`ANDROID_HOME` and `ANDROID_SERIAL` pick the SDK and the device for the adb tasks.

Deploy only when asked, and only from a laptop: CI builds the app and the
extension but deploys nothing. `bun run deploy` in `deploy/` deploys, from
`alchemy.run.ts`: hawk's one compose, `deploy/hawk/compose.yaml` (Convex, the engine, the
proof verifier and the UDP gateway, in `/opt/vmate`; their images are built on nest and
pushed to GHCR), the verifier's standby on nest (hawk's Caddy falls back to it for
`/proof`), and the mail and proxy Workers; then the Convex functions
(`bunx convex deploy` in `backend/`). The v3 secrets come from the Convex
deployment's env, the rest from `deploy/.env` (`.env.example` lists them). The images need `GHCR_TOKEN`
(a GitHub token with `write:packages`); hawk logs in with it only for the pull. Caddy and DNS are in the infra repo. The README lists the CI workflows.

Never print or commit `.env*` files (`extension/.env` is the one tracked
exception, and it holds only the server URL), `keystore/`,
`app/keystore.properties`, `fastlane/key.json` or other secrets.

## Release

Google Play has the app as `com.vitap_pal.app`, the Flutter app's id, and takes
the AAB. To release:

1. Raise `versionCode` and `versionName` in `app/module.yaml`. Codes follow
   `M mmm p bb` from 3000000 (3.0.0 is 3000000, 3.0.4 is 3000410, 3.1.0 is 3001000); Play refuses a code it has seen.
2. Write `fastlane/metadata/android/en-GB/changelogs/<versionCode>.txt` (at most
   500 characters) and add the version with its `code` to `releases` (newest
   first) in `backend/guides/updates.json`; give features it adds
   `"since": <versionCode>`, so older apps leave them out. Then `bun
   dev/gen-guides.ts` and `bunx convex deploy` in `backend/`.
3. Push the tag `v<versionName>`. `android-release.yml` builds the signed AAB
   (64-bit and 32-bit) and uploads it to the internal track.
4. Promote in the Play Console, or with `bundle exec fastlane android
   promote_testing` (friends, alpha, beta). Production is its own lane:
   `promote_production` sends the beta build to production as a staged rollout
   (`rollout:0.2` by default) and `widen_rollout rollout:0.5` (or `1`) widens it.
   `status` shows the release on each track.

How hard the app asks to update is the release's in-app update priority, set
per versionCode in `UPDATE_PRIORITIES` in `fastlane/Fastfile` (add a line in the
release commit), `DEFAULT_UPDATE_PRIORITY` (2) when not listed: 0–1 nothing, 2 a pill above the nav bar, 3 Play's flexible
dialog on start, 4 Play's full-screen update (can be closed), 5 the app's own
blocking screen (`app/src/vitapmate/PlayUpdate.kt`). Promotions reuse it.

From a laptop, with the keystore and `keystore/play-service-account.json` in
place: `bundle install`, then `bundle exec fastlane android upload_internal`
(or `upload_testing`: upload to internal, then promote to friends, alpha and beta).

## Version control

jj, colocated with git. Use `jj` for anything that changes history and no
mutating `git` commands. Commit only when asked and never push unless asked.
Use Conventional Commits with a subject under 72 characters.

## Testing

Test on the emulator or a connected phone, not only with a build. Parser tests
are in `rust/vtop-proto/tests`, client tests in
`rust/vtop-core/src/client/tests.rs`, the app crate's in
`rust/vtop-ffi/src/tests.rs` and next to each module, vm1's in
`rust/vtop-wire/src/tests.rs`, the backend's in `backend/convex/*.test.ts`,
and Kotlin's in `core/test/` and `features/<name>/test/`.

- Extend an existing test before adding one, and test at the lowest layer that
  shows the bug (a parser test over a client test over an app test).
- Parser snapshots (`fixtures/<name>.expected.json`) are regenerated with
  `UPDATE_SNAPSHOTS=1 cargo test -p vtop-proto --test parsers`, never edited by hand.
- No sleeps or wall-clock time in tests: pass the time in.
- Never assume a failure was already there: check it on the parent change first.
- `mise run check-changed` runs only the checks for what you changed.

Without a VTOP account, sign in as the demo account (`DEMO2026`, password
`vitapmate-demo`, `rust/vtop-ffi/src/demo.rs`): it asks VTOP nothing, reads
every page from the backend's `/guides/demo.json`, and answers outing writes,
downloads, the web view and forwarding as unavailable. Debuggable builds take
`adb shell am broadcast -p com.vitap_pal.app -a vitapmate.DUMP` (the state dump
`mise run state` prints) and `-a vitapmate.SYNC_NOW` (a background sync now;
`--ez simulate true` rolls the saved pages back first). `rust/vtop-convex/` is
a local leftover, not part of the app: ignore it.
