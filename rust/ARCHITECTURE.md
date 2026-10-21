# VTOP code architecture

```
rust/                    Cargo workspace; the root package is the app's FFI crate
├── src/                 rust_lib_vitapmate: flutter_rust_bridge wrapper (cdylib/staticlib)
├── vtop-core/           all VTOP logic, no FRB and no server code
├── vtop-bridge/         the Engine: axum service over vtop-core, run as a Cloudflare Container
└── vtop-edge/           the Cloudflare Worker and Durable Objects in front of the Engine (TypeScript)
lib/core/vtop_backend/   Dart: VtopBackend interface, local (FRB) and edge (HTTP, NDJSON)
```

## vtop-core

| Module | Holds |
| --- | --- |
| `client/mod.rs` | `VtopClient`, its builder, shared response handling, session import/export |
| `client/auth.rs` | password + CAPTCHA login, session validation and restore |
| `client/otp.rs` | security OTP submit, redirect follow-up, resend |
| `client/captcha.rs` | embedded CAPTCHA model, on `spawn_blocking` (there is no remote solver) |
| `client/http.rs` | the reqwest client: headers, VTOP intermediate CA, timeouts, pool |
| `client/{academics,exams,biometric}.rs` | one method per endpoint, plus `*_page` for raw HTML |
| `session.rs` | auth state, CSRF, registration number, resettable cookie jar, `SessionState` |
| `parser/` | one parser per page; `page.rs` for login/landing pages |
| `inputs.rs` | validated inputs (semester id, OTP, date, password...) |
| `types.rs`, `error.rs` | the data types and `VtopError` that Dart and the server both expose |

A few design points:

- **Fetchers take `&self`.** Mutable session data sits behind a mutex that is
  never held across an `.await`, so several fetches can run on one client.
  Login and OTP take `&mut self`. Restored-cookie validation is serialised by
  an async lock, so concurrent first fetches cost one `/vtop/content` hit.
- **One HTTP client per `VtopClient`, for its whole life.** Resetting a
  session empties the cookie jar in place instead of rebuilding the client, so
  keep-alive connections and TLS sessions survive re-logins.
- **`SessionState`** (cookies, CSRF token, registration number, pending OTP)
  is the portable form of a session. With all fields present a client resumes
  without asking VTOP; with only the cookie it validates once.
- **Logging** goes through the `log` facade (`rust.auth`, `rust.network`).
  Cookie values, passwords, OTPs, CSRF tokens and API keys are never logged,
  only cookie names and URLs.

## Dart API stays the same

`rust_lib_vitapmate` re-exports the core types and declares flutter_rust_bridge
**mirrors** (`#[frb(mirror(X))] struct _X`) with the same `freezed` and
`json_serializable` metadata as before. The generated `types.dart` and
`vtop_errors.dart` are unchanged. Keep a mirror's fields in sync when a core
type changes (the compiler does not check mirrors), then run
`flutter_rust_bridge_codegen generate`.

The Dart-visible changes: `fetchExamShedule` was renamed `fetchExamSchedule`,
and the unused parser bindings and `nowUnix()` were removed.

## VTOP from the phone

```
repository → data source → VtopBackend → LocalVtopBackend → FRB → vtop-core → VTOP
```

The phone always signs in and fetches itself. Settings → Server can route
those requests through the Worker's `/vtop/` proxy (`settings_vtop_proxy`,
off by default, never in On-device Mode): the same requests, but VTOP's pages
arrive compressed, and VTOP sees Cloudflare's address instead of the phone's.
The server URL has a built-in default (`VTOP_SERVER_URL` at build time) and
can be changed on the Server page after a `/healthz` check.

The Server page reads `POST /v1/app/status` (app secret + registration number, no
VTOP cookies): the sealed session's and password's ages, when VTOP last
rejected the password, the forwarding address, the phone count, the last 10
sign-ins and each key's last 10 uses (extension or MCP, from the
`x-vtop-client` header). `POST /v1/app/session/clear` drops the sealed
session. `/healthz` reports `version` and `colo`.

## Tests

- `vtop-core/tests/parsers.rs`: fixture HTML → JSON snapshots, plus truncated
  pages that must not panic. Fixtures are synthetic or redacted.
- `vtop-core/src/client/tests.rs`: session reuse against an in-process mock VTOP.
- `vtop-bridge`: the engine routes (session, seal, sign-in with forwarded OTP).
- `vtop-edge` (`pnpm test`): accounts, devices, mail OTP, the `/vtop/` proxy.
- `src/live_bench.rs` (ignored): live timings; `VTOP_DUMP_DIR` saves raw pages.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
