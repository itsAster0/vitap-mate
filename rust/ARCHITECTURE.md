# VTOP code architecture

```
rust/                    Cargo workspace; the root package is the app's FFI crate
├── src/                 rust_lib_vitapmate: flutter_rust_bridge wrapper (cdylib/staticlib)
├── vtop-core/           all VTOP logic, no FRB and no server code
└── vtop-server/         axum HTTP service over vtop-core
lib/core/vtop_backend/   Dart: VtopBackend interface, local (FRB) and remote (HTTP)
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

## Local or server

```
repository → data source → VtopBackend ─┬─ LocalVtopBackend  → FRB → vtop-core → VTOP
                                        └─ RemoteVtopBackend → vtop-server → vtop-core → VTOP
```

Settings → VTOP Data → Data Source has a switch. When it's on and a URL is
saved, `vtopBackendProvider` returns the remote backend and
`vtopAuthenticatorProvider` logs in through the server's `/v1/auth/*` (so the
server sees the password). Switching off keeps the URL and key. The server's
session (cookies, CSRF, regno, login time) is loaded into the local client,
and the remote backend sends that trio with every request, so the server never
re-validates. Update VTOP Data and background sync fetch the stale pages in one
`/v1/refresh` call. A session is reused while its last real login (recorded by
Rust) is younger than the Session Reuse setting; an expired session triggers
one re-login and retry. Server errors map back to `VtopError`, so the
existing re-login handling works unchanged. The server answers in Rust serde
JSON; `server_json.dart` converts it (snake_case → camelCase, `update_time`
number → string, `"Theory"` → `"theory"`), and a test pushes every parser
snapshot through that path.

The API key lives in plain `SharedPreferences`, as decided for this project.
Anyone with access to the app's data directory can read it. Use one key per
install or user, so a leaked key can be revoked by removing it from
`VTOP_SERVER_API_KEYS`.

Server login is stateless: the server returns the session to the caller and
keeps nothing.

## Tests

- `vtop-core/tests/parsers.rs`: fixture HTML → JSON snapshots, plus truncated
  pages that must not panic. Fixtures are synthetic or redacted.
- `vtop-core/src/client/tests.rs`: session reuse against an in-process mock VTOP.
- `vtop-server/tests/api.rs`: API key, rate limit, cache, errors, login switch.
- `test/core/remote_vtop_backend_test.dart`: server JSON → Dart types, HTTP client.
- `src/live_bench.rs` (ignored): live timings; `VTOP_DUMP_DIR` saves raw pages.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
