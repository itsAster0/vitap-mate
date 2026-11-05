# AGENTS.md

vitapmate is an unofficial client for VIT-AP's VTOP portal: timetable,
attendance, marks, grades, exams, outings, course pages and more, cached on the
phone. Compose Multiplatform app (Kotlin Toolchain, no Gradle) over a Rust core
(UniFFI). Android only for now; iOS needs other bindings (see the plan).

Read `docs/plans/kmp-prototype.md` first: it holds the app's status, modules,
contracts and the latest hand-off.

## Repo map

```
project.yaml          Kotlin Toolchain project: one module per layer and feature
kotlin kotlin.bat     toolchain wrapper (./kotlin build, run, test)
app/                  shell: registry (Features.kt), nav, More, Settings, Developer,
                      Inspector, MainActivity; res/, jniLibs/ (built, gitignored)
core/                 vitapmate.core: generated UniFFI bindings (vtop_ffi.kt, do not edit)
                      vitapmate.data: AppCore, pages (rememberPage, PageState), Problem, Settings
                      vitapmate.observability: Log, flow(), Inspector, state dump
design/               vitapmate.ui: theme, shared components, PageScaffold, Feature type
features/<name>/      one feature per module (module.yaml + src/vitapmate/features/<name>/)
rust/                 Cargo workspace
  vtop-ffi/           the app's crate: Core, cache, session, records, obs.rs (logs)
  vtop-core/          VTOP client, session, parsers, inputs, types, errors
  vtop-shared/        formats shared by app, engine and edge (wire_page, mail_seal, ...)
  vtop-wire/          vm1: Noise IK over UDP for slow, lossy links
  vtop-gateway/       UDP gateway; runs on the Oracle VPS (rust/vps)
  vtop-bridge/        the engine: sign-in, CAPTCHA, fetch+parse; Railway service engine
  vtop-mcp/           MCP tools served by the engine behind the edge's /mcp
  vtop-edge/          Cloudflare Worker + Durable Objects (TypeScript): all server state
  ARCHITECTURE.md
extension/            Chrome/Firefox extension (Svelte)
fastlane/             Play Store lanes and metadata
keystore/             release keystore (gitignored; never read or print it)
docs/plans/           plans
```

## Where a change goes

| Change | Place |
|---|---|
| New VTOP page | parser in `rust/vtop-core/src/parser/`, call in `rust/vtop-core/src/client/`, FFI in the feature's file `rust/vtop-ffi/src/<feature>.rs` (its own `#[uniffi::export] impl Core` block; `cached_x` / `fetch_x`, flow id first), entry in `rust/vtop-bridge/src/pages.rs`; then `mise run rust-android` |
| Use a VTOP page in a feature | `rememberPage("<id>", { c, f -> c.cachedX(f) }, { c, f -> c.fetchX(f) }, ...)` in the screen, drawn with `PageScaffold` (see `core/src/vitapmate/data/Page.kt`) |
| New screen | `features/<name>/`: `module.yaml` (depends on `//design` only) + `src/vitapmate/features/<name>/` exposing `val <name>Feature: Feature`; one line in `project.yaml` and one in `app/src/vitapmate/Features.kt`. A feature never imports another; what two share moves to `design` (UI) or `core` (data) |
| Shared widget | `design/src/vitapmate/ui/`, one widget per file |
| Shell, nav, Settings, Developer, Inspector UI | `app/src/vitapmate/` |
| Data, settings store, downloads, files | `core/src/vitapmate/data/` |
| Logging | Kotlin: `core/src/vitapmate/observability/` (`Log`, `flow()`); Rust: `tracing`, pipeline in `rust/vtop-ffi/src/obs.rs` |
| Server-side state | `rust/vtop-edge` Durable Objects; never on the engine or gateway |

## Rules

- Same UI and feel; visual changes only when asked. Material 3 Expressive,
  theme colours only.
- Reuse and move working code; small diffs. Rust may be refactored for better
  traces, logs and errors.
- Fewest lines that do the job: no single-caller wrappers or pass-through
  layers; stubs for unbuilt parts.
- Ask when something could be done better, with a default; don't block on it.
- Calls into Rust off the main thread; one call per screen state.
- Observability: every user action or background run is a `flow()`; its id is
  the first argument of every `Core` call, so Rust's lines carry it too. Log
  lines carry `regno`. Kotlin logs go through Rust into one file
  (`files/logs/app.jsonl`) and logcat (tag `vitapmate`); logs stay on the
  phone (share/export from Developer). Never log cookies, passwords, OTPs,
  CSRF tokens or keys.
- Background work: short timeouts (the open-app sweep stops at a page over
  5 s), never blocks or interrupts the user, configurable in Settings →
  Developer (`Tunable` in `core/src/vitapmate/data/Settings.kt`).
- Rust: Google Rust style guide; new crates on edition 2024.

## Commands

Toolchains come from mise (`mise.toml`): Java and the Kotlin Toolchain. JS
uses pnpm.

```sh
mise run rust-android        # after changing rust/vtop-ffi: .so into app/jniLibs + Kotlin bindings
./kotlin build
./kotlin test -m core
./kotlin run -d emulator-5554
mise run release             # signed APK at build/vitapmate-release.apk (runs `mise run signing`)
mise run logs                # also logs-pull, state, ui-dump, trace: the running app over adb

cd rust && cargo fmt && cargo clippy --workspace && cargo test --workspace
cd rust/vtop-edge && pnpm test && pnpm check
cd extension && pnpm test && pnpm build
```

Deploys only when asked: `pnpm exec wrangler deploy` in `rust/vtop-edge/`;
`railway up --service engine` in `rust/` (the engine, `Dockerfile.engine`).
The UDP gateway deploys from CI on push (`.github/workflows/gateway-deploy.yml`,
`rust/vps/`).

Never print or commit `.env*` files, `keystore/`, `app/keystore.properties`,
`fastlane/key.json`, or other secrets.

## Version control

jj, colocated with git. Use `jj` for anything that changes history; no
mutating `git` commands. Commit only when asked; never push unless asked.
Conventional Commits, subject under 72 characters.

## Testing

Run on the emulator or a connected phone, not only a build. Rust parser and
client tests live in `rust/vtop-core/tests` and `src/client/tests.rs`, the
app's crate in `rust/vtop-ffi/src/tests.rs`; Kotlin tests in `core/test/`.
