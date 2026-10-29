# VTOP code architecture

```
rust/                    Cargo workspace; the root package is the app's FFI crate
├── src/                 rust_lib_vitapmate: flutter_rust_bridge wrapper (cdylib/staticlib)
├── vtop-core/           all VTOP logic, no FRB and no server code
├── vtop-bridge/         the Engine: axum service over vtop-core, run on Railway
├── vtop-wire/           UDP mode's transport (Noise IK, Reed-Solomon), used by the app and the gateway
├── vtop-gateway/        UDP mode's gateway: vtop-wire in, the Engine's /engine/fetch out (on the VPS, rust/vps)
└── vtop-edge/           the Cloudflare Worker and Durable Objects in front of the Engine (TypeScript)
lib/vtop/backend/        Dart: VtopBackend interface, local (FRB) and edge (HTTP, NDJSON)
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

## UDP mode (experimental)

```
phone ──UDP (vtop-wire)──► gateway ──HTTPS──► Engine /engine/fetch ──► VTOP
             (Oracle VPS, Hyderabad)        (Railway, Singapore)
```

A switch under "Fetch on the server" (`lib/vtop/route/vtop_udp.dart`)
sends page fetches and sign-ins over UDP, for slow, lossy networks.
Changes to VTOP stay on HTTPS, and `ServerVtopApi.fetch` falls back to
HTTPS for five minutes when the gateway does not answer, has another key or
cannot read the request, and for any page too large for UDP. Where the
gateway is and its key come from the edge (`GET /v1/udp`), kept by the app
until the next Monday 00:00 IST.

- **Why not QUIC or TLS.** At 1 kbit/s a TLS handshake (~5 KB of
  certificates) takes about 40 s, and QUIC pads its first packets to 1200
  bytes. Measured on netem (100 ms each way): a 300-byte page at 1 kbit/s
  took 40 s over TLS/TCP and about 4 s on a saved UDP session; a 2 KB page
  at 8 kbit/s with 30% loss took 18 s against about 3 s.
- **Transport** (`vtop-wire`): Noise IK with the gateway's key from the
  edge, the request inside message 1 (one round trip). Later packets use
  the split keys with an explicit counter, so sessions survive restarts; the
  client saves the end of a 1024-counter block before using it, and each
  process locks its own session file. Responses go out as Reed-Solomon
  symbols, paced at the client's measured rate; the client spots losses from
  expected arrival times and asks for fresh symbols. Rate, round trip and
  loss are kept per network.
- **Gateway** (`vtop-gateway`): no VTOP logic. It calls `/engine/fetch`
  with the engine's relay secret, returning the answer byte for byte.
- **Keys**: the edge's `UdpKeys` object makes a random key pair every Monday
  00:00 IST and keeps the one before for a week; older ones are deleted. The
  gateway asks for them every 10 minutes through the engine
  (`/engine/udp-keys`); the edge seals them to the gateway's identity key
  (`GATEWAY_IDENTITY_KEY`), so the engine passes them on unopened. A
  handshake under an unknown key gets a 5-byte "wrong key" answer, and the
  app asks the edge again.
  It keeps each device's VTOP session for six hours so requests carry an
  8-byte hash instead of the cookies (`vtop_shared::wire_page`).
- **Abuse limits** (no rate limits: a campus shares one IP): message 1
  must be fresh and its timestamp unseen for the device; unproven
  addresses get at most 3x the bytes they sent, and a reset is never
  larger than the packet behind it; at most 4 requests open per session,
  50,000 sessions (the longest idle makes room) and 256 engine calls at once
  (more get "busy" and use HTTPS); sending stops after 30 s of silence.

Deploy: the gateway from CI (`.github/workflows/gateway-deploy.yml`) to the
VPS (`rust/vps`: compose file, its own resolvers, `.env` kept on the box).

## Tests

- `vtop-core/tests/parsers.rs`: fixture HTML → JSON snapshots, plus truncated
  pages that must not panic. Fixtures are synthetic or redacted.
- `vtop-core/src/client/tests.rs`: session reuse against an in-process mock VTOP.
- `vtop-bridge`: the engine routes (session, seal, sign-in with a forwarded
  or Gmail OTP).
- `vtop-wire`: framing, Reed-Solomon, the loss estimate, and client ↔
  gateway over localhost UDP (30% loss, restarts, replays, two processes).
- `vtop-gateway`: the client through the gateway to a mock engine.
- `vtop-edge` (`pnpm test`): accounts, devices, mail OTP, the `/vtop/` proxy.
- `src/live_bench.rs` (ignored): live timings; `VTOP_DUMP_DIR` saves raw pages.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
