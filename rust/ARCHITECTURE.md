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
├── vtop-gateway/        UDP mode's gateway: vtop-wire in, the Engine's /engine/fetch out; makes its own weekly keys (on a Linode in Chennai)
└── vtop-edge/           the Cloudflare Worker and Durable Objects in front of the Engine (TypeScript)
```

## vtop-core

| Module | Holds |
| --- | --- |
| `client/mod.rs` | `VtopClient`, its builder, `fetch(spec, query)` for every read page, shared response handling, session import/export |
| `client/auth.rs` | password + CAPTCHA login, session validation and restore |
| `client/otp.rs` | security OTP submit, redirect follow-up, resend |
| `client/email_otp.rs` | login that answers the OTP itself, from a code source or Gmail |
| `client/hostel.rs` | outing apply and cancel, and the pass |
| `client/course_page.rs` | the course page's files and the course plan |
| `client/transport.rs` | the `Transport` trait: one hop, no redirects, no cookies |
| `client/http.rs` | the reqwest client: headers, VTOP intermediate CA, timeouts, pool |
| `session.rs` | auth state, CSRF, registration number, resettable cookie jar, `SessionState` |
| `gmail.rs` | reads the OTP mail from the student's Gmail |
| `sync.rs` | the async lock and the timer, with no runtime |

The parsers, page machines, page registry, inputs, types and `VtopError` are
vtop-proto's, re-exported here at their old paths. `vtop_proto::spec` lists
every read page once as a `PageSpec`: its name and arguments (what the app
sends the engine), its cache file and its machine. `VtopClient::fetch` runs
any of them, and the engine serves everything in `spec::each`, so a new read
page is a parser, a machine and a spec.

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

- **Pages.** A feature's `fetch_x` calls `Core::fetch(&spec::X, ...)`,
  which fetches the page for the picked semester and saves it at
  `pages/<semester>/<spec file>`; `Core::page` fetches one without saving.
  Both ask the route first (the server or UDP) and fall back to
  `VtopClient::fetch` on the phone. `cached_x` reads the saved copy with
  `read_page`.
- **Decoded copies** (`decoded.rs`). The semester list, the pick and every
  page stay decoded in memory after the first read, keyed by path and
  type, so screens that read the same page many times parse it once. A
  copy is used only while the file's length, modified time and inode are
  what they were when it was read; a write drops it, and sign-out,
  another student, Developer's clears and a forgotten pick drop what they
  remove. At most 4 MB of JSON is kept, the least recently used going
  first.
- **Route policy** (`route/plan.rs`). Which route to try, the 5-minute
  skips and the steps of a server fetch (consent, the fetch, one sign-in
  again) are pure: no I/O and no clock, the caller passes `now`.
  `route.rs` keeps the one plan and runs its steps.

## VTOP from the phone

```
feature screen → rememberPage (core) → Core (UniFFI, JNA) → vtop-core → Transport → VTOP
```

- **Transport.** vtop-core sends one hop at a time through its `Transport` trait;
  Rust follows redirects and decides retries. In the app, Kotlin carries the hop
  (`HttpTransport`: HttpURLConnection in `VtopHttp.kt`, or Android's `HttpEngine` on
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
  `route used` names the route each call took. The open-app sweep sends its
  pages as one batch (`Core::fetch_pages`, `route/batch.rs`,
  `/v1/vtop/fetch-batch`), asked about once; on Direct, or a server without
  batches, they are fetched a few at a time on the phone.
- **Cookie consent** (`vtop-ffi/src/consent.rs`). Every path that sends the
  VTOP session (or the saved password) to our servers calls `consent::allowed`
  first: Proxy, Server and Udp, the server OTP, Connected apps, the Server page,
  mail forwarding and a connected app's session request. Kotlin's
  `CookieConsent` (the `CookieGate`) shows a dialog with what
  goes where and why: Send, Cancel and "Don't ask again" per kind (route,
  account, handover; kept in `consent.json`, dropped on
  sign-out or another student). It fails closed: no gate, an empty flow, the
  app not shown or a background flow (`flow(name, unattended = true)`: the
  open-app sweep and background sync) is a no, except a route send with the
  route's tick, so background runs follow the chosen route. A no on a
  route fetches Direct without the 5-minute skip; elsewhere it is
  `ConsentDenied`. Calls nobody started (the semester push)
  never ask and send only with the tick. A session request while the
  app is away only posts a notification; tapping it opens the app and asks.
- **Account token** (`vtop-ffi/src/account_token.rs`, API.md "Account
  tokens"). After the edge checks the session for an account call
  (Connected apps, the Server page, mail forwarding), linked phone or not, it
  answers with a 14-day token naming the student and this install; the
  student's `Account` object keeps the live token per install and revokes it
  when the install leaves the account or the forwarding address. It proves
  who the student is, not that the phone is linked. The app
  keeps it (`account_token.json`) and sends it in place of the session, so
  those calls skip the VTOP check and the consent dialog; a refused or
  expiring token is dropped and the session goes once more. Calls that also
  send the password, a connected app's session and the routes keep sending
  the session. Dropped on sign-out, another student, and deleting the
  server account or this install.

The edge also serves `POST /v1/app/status` (signed by the phone, with the
registration number, no VTOP cookies): the sealed session's and password's ages,
when VTOP last rejected the password, the forwarding address, the phone count,
the last 10 sign-ins and each key's last 10 uses (extension or MCP, from the
`x-vtop-client` header). Settings → Server shows these and the account's other
records, with a delete for each (`vtop-ffi/src/server.rs`). `/healthz` reports
`version` and `colo`.

MCP clients (Claude and others) connect to `/mcp` with OAuth (`vtop-edge/src/
oauth.ts`): `/mcp` answers 401 with the metadata URL, the client registers and
opens `/authorize`, which shows a pairing code and QR. The student scans it in
Connected apps → Pair a browser; the app makes an access key with that
`pairCode`, named after the client, and `/token` hands the client that key.

## UDP mode

```
phone ──UDP (vtop-wire)──► gateway ──HTTPS──► Engine /engine/fetch ──► VTOP
             (Linode, Chennai)              (Railway, Singapore)
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
  (`/engine/fetch-batch` for a batch, and the `/engine/vtop-login*` routes) with `INTERNAL_TOKEN`, returning
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
The gateway image is built for x86, pushed to GHCR and run on the Linode
`fedora-in-maa` (172.232.115.40, UDP 3478-3481) over SSH, with host networking, `rust/vps/resolv.conf` as its resolvers
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
- `vtop-ffi/src/tests.rs`, `outing/tests.rs` and next to each module: the app's crate.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
