# AGENTS.md

vitapmate is an unofficial client for VIT-AP's VTOP portal: timetable,
attendance, marks, grades, exams, outings, course pages and more, cached on the
phone. Flutter app over a Rust core. Android is the main target; iOS builds
from the same code.

The app is being restructured: read `docs/plans/flutter-refactor.md` first.

## Repo map

```
lib/                  Flutter app (see "Target layout" in the plan)
  src/                generated flutter_rust_bridge bindings (do not edit)
rust/                 Cargo workspace
  src/                rust_lib_vitapmate: the flutter_rust_bridge crate
  vtop-core/          VTOP client, session, parsers, inputs, types, errors
  vtop-shared/        formats shared by app, node and edge (wire_page, page dictionary, ...)
  vtop-wire/          vm1: Noise IK over UDP for slow, lossy links
  vtop-gateway/       UDP gateway; runs on the Oracle VPS (rust/vps)
  vtop-bridge/        the engine: sign-in, CAPTCHA, fetch+parse; Railway service engine
  vtop-mcp/           MCP tools served by the engine behind the edge's /mcp
  vtop-edge/          Cloudflare Worker + Durable Objects (TypeScript): all server state
  ARCHITECTURE.md
extension/            Chrome/Firefox extension (Svelte)
android/ ios/         platform runners
docs/plans/           plans
```

## Where a change goes

| Change | Place |
|---|---|
| New VTOP page | parser in `rust/vtop-core/src/parser/`, call in `rust/vtop-core/src/client/`, FFI in `rust/src/api/`, entry in `lib/vtop/` queries and in `rust/vtop-bridge/src/pages.rs` |
| Use a VTOP page in a feature | one line in the feature's public file: `vtopPage(VtopQueries.x)` (see `lib/vtop/vtop_page.dart`) |
| New screen | `lib/features/<name>/`; imports `app/ ui/ observability/ vtop/ platform/`, and other features only via their public `features/<other>/<other>.dart` |
| Shared widget | `lib/ui/`, one widget per file |
| Phone API (notifications, storage, background...) | `lib/platform/` |
| Logging | Dart: `lib/observability/` (`Log`, `flow()`); Rust: `tracing` |
| Server-side state | `rust/vtop-edge` Durable Objects; never on the engine or gateway |

## Rules

- Same UI and feel; visual changes only when asked.
- Reuse and move working code; small diffs. Rust may be refactored for better
  traces, logs and errors.
- Fewest lines that do the job: no single-caller wrappers or pass-through
  layers; stubs for unbuilt parts.
- Ask when something could be done better, with a default; don't block on it.
- Observability: every user action or background run is a `flow()`; its id is
  on every log line, sent as `x-request-id`, and passed into Rust. Log lines
  carry `regno`. Logs stay on the phone (share/export from Developer settings).
  Never log cookies, passwords, OTPs, CSRF tokens or keys.
- Background work: short timeouts (warmup 4 s), never blocks or interrupts the
  user, configurable in Developer settings.
- Rust: Google Rust style guide; new crates on edition 2024.

## Commands

Toolchains come from mise (`mise.toml`). JS uses pnpm.

```sh
flutter pub get
flutter analyze
flutter test
dart run build_runner build -d                 # after changing @riverpod / freezed code
flutter_rust_bridge_codegen generate           # after changing rust/src/api
flutter build apk --release --flavor prod --dart-define-from-file=.env.json

cd rust && cargo fmt && cargo clippy --workspace && cargo test --workspace
cd rust/vtop-edge && pnpm test && pnpm check
cd extension && pnpm test && pnpm build
```

Deploys only when asked: `pnpm exec wrangler deploy` in `rust/vtop-edge/`;
`railway up --service engine` in `rust/` (the engine, `Dockerfile.engine`).
The UDP gateway deploys from CI on push (`.github/workflows/gateway-deploy.yml`,
`rust/vps/`).

Never print or commit `.env*` files, keystores, or secrets.

## Version control

jj, colocated with git. Use `jj` for anything that changes history; no
mutating `git` commands. Commit only when asked; never push unless asked.
Conventional Commits, subject under 72 characters.

## Testing

Run on the emulator or a connected phone, not only a build. Rust parser and
client tests live in `rust/vtop-core/tests` and `src/client/tests.rs`; Dart
tests in `test/`.
