# VTOP code architecture

```
rust/                    Cargo workspace
├── vtop-ffi/            the app's crate: UniFFI `Core`, cache, session, records, logs (obs.rs)
├── vtop-proto/          parsers, types and HTTP shapes as page machines, with no I/O
├── vtop-core/           the VTOP client: runs vtop-proto's machines over a Transport, no FFI
├── vtop-shared/         formats shared by app, engine and edge (wire_page, mail_seal, ...)
├── vtop-bridge/         the Engine: axum service over vtop-core, Railway service engine-v2
├── vtop-mcp/            MCP tools served by the Engine behind the edge's /mcp
├── vtop-wire/           UDP mode's transport (Noise IK, Reed-Solomon), used by the app and the gateway
├── vtop-gateway/        UDP mode's gateway: vtop-wire in, the Engine's /engine/fetch out; makes its own weekly keys (on the Oracle VPS)
└── vtop-edge/           the Cloudflare Worker and Durable Objects in front of the Engine (TypeScript)
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
| `types.rs`, `error.rs` | the data types and `VtopError` that the app and the server both use |

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

## The app's crate (vtop-ffi)

`vtop-ffi` wraps `vtop-core` for the app with UniFFI proc macros.
Each feature has its own file (sign-in, attendance and timetable in
`lib.rs`; `marks.rs`, `exams.rs`, `outing.rs`, ...) with its own
`#[uniffi::export] impl Core` block and ready-to-draw records. Every call
takes the caller's flow id first. `mise run rust-android` builds
`libvtop_ffi.so` into `app/jniLibs/` and generates the Kotlin bindings
(`core/src/vitapmate/core/vtop_ffi.kt`).

## VTOP from the phone

```
feature screen → rememberPage (core) → Core (UniFFI, JNA) → vtop-core → Transport → VTOP
```

- **Transport.** vtop-core sends one hop at a time through its `Transport` trait;
  Rust follows redirects and decides retries. In the app, Kotlin carries the hop
  (`HttpTransport`: OkHttp in `VtopHttp.kt`, or Android's `HttpEngine` on
  Android 14+ in `VtopHttpEngine.kt`, which speaks HTTP/3 to the edge). The
  server builds vtop-core with its `reqwest` feature. The app build has no tokio:
  UniFFI's async calls run on Kotlin's polling, and `vtop_core::sync` has the
  async lock and the timer the client needs. Downloads go straight to a file.
- **Cookies.** Rust's `CookieStore` reads and writes Android's `CookieManager`
  through the foreign trait `SharedCookies` (`VtopCookies.kt`), so the web view
  and Rust share one session.
- **Routes** (`vtop-ffi/src/route.rs`, Settings → Connection). Direct sends to
  VTOP. Proxy swaps VTOP's origin for the edge's `/vtop/` on each hop
  (`Routed`), so cookies, redirects and the parsers never see the edge. Server
  sends read pages to the edge's `/v1/vtop/fetch` with the phone's session and,
  when it expired, signs in through `/v1/vtop/login`; writes, downloads and the
  web view stay on the phone. Udp sends the same request to the gateway first
  (`udp.rs`, a Kotlin `DatagramSocket` in `VtopUdp.kt`). A route that fails is
  skipped for 5 minutes and the request goes on to Direct. The log line
  `route used` names the route each call took.

The edge also serves `POST /v1/app/status` (signed by the phone, with the
registration number, no VTOP cookies): the sealed session's and password's ages,
when VTOP last rejected the password, the forwarding address, the phone count,
the last 10 sign-ins and each key's last 10 uses (extension or MCP, from the
`x-vtop-client` header). Settings → Server shows these and the account's other
records, with a delete for each (`vtop-ffi/src/server.rs`). `/healthz` reports
`version` and `colo`.

## UDP mode

```
phone ──UDP (vtop-wire)──► gateway ──HTTPS──► Engine /engine/fetch ──► VTOP
             (Oracle VPS, Hyderabad)        (Railway, Singapore)
```

The app's Udp route sends read pages over UDP, for slow, lossy networks.
Sign-in and changes to VTOP stay on HTTPS. The client gives up after 5 s
without any answer from the gateway (longer on a slow link it has measured) and
after 30 s in all, and the route then falls back to HTTPS for five minutes.
Where the gateway is and its keys come from the edge (`GET /v1/udp`); the app
keeps them for 6 hours, or until the gateway says it has another key.

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
  (and the `/engine/vtop-login*` routes) with `INTERNAL_TOKEN`, returning
  the answer byte for byte. It keeps each device's VTOP session for six
  hours so requests carry an 8-byte hash instead of the cookies
  (`vtop_shared::wire_page`).
- **Keys** (`vtop-gateway/src/keys.rs`): the gateway makes its own key pair
  every Monday 00:00 IST, keeps the one before for a week and deletes
  older ones, in `$GATEWAY_DATA_DIR/udp-keys.json` (owner-only). At start
  and after each rotation it posts
  `{ host, ports, keys: [{ id, publicKey, from, until }] }` (public halves
  only, base64url) to the edge's `POST /internal/udp-keys` with
  `INTERNAL_TOKEN`, retrying with backoff (5 s doubling to 10 min). A
  handshake under an unknown key gets a 5-byte "wrong key" answer, and the
  app asks the edge again.
- **Abuse limits** (no rate limits: a campus shares one IP): message 1
  must be fresh and its timestamp unseen for the device; unproven
  addresses get at most 3x the bytes they sent, and a reset is never
  larger than the packet behind it; at most 4 requests open per session,
  50,000 sessions (the longest idle makes room) and 256 engine calls at once
  (more get "busy" and use HTTPS); sending stops after 30 s of silence.

Internal calls (edge → engine, engine → edge `/internal/*`, gateway →
engine, gateway → edge) all carry `Authorization: Bearer $INTERNAL_TOKEN`.

Deploy: the edge (`vmate-edge` on vmate.aster0.dev), the engine (Railway
`engine-v2`) and the gateway with `bun run deploy` in `rust/vtop-edge`
(`alchemy.run.ts`), from a laptop; CI deploys nothing.
The gateway image is built for x86, pushed to GHCR and run on the Oracle VPS
`robin` over SSH, with host networking, `rust/vps/resolv.conf` as its resolvers
and `~/vitapmate-gateway/data` for the keys.

## Tests

- `vtop-proto/tests/parsers.rs`: fixture HTML → JSON snapshots, plus truncated
  pages that must not panic. Fixtures are synthetic or redacted.
- `vtop-core/src/client/tests.rs`: session reuse against an in-process mock VTOP.
- `vtop-bridge`: the engine routes (session, seal, sign-in with a forwarded
  or Gmail OTP).
- `vtop-wire`: framing, Reed-Solomon, the loss estimate, and client ↔
  gateway over localhost UDP (30% loss, restarts, replays, two processes).
- `vtop-gateway`: the client through the gateway to a mock engine.
- `vtop-edge` (`bun run test`): accounts, devices, mail OTP, the `/vtop/` proxy.
- `vtop-ffi/src/tests.rs` and `outing/tests.rs`: the app's crate.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
