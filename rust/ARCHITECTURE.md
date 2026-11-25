# VTOP code architecture

```
rust/                    Cargo workspace
├── vtop-ffi/            the app's crate: UniFFI `Core`, cache, session, records, logs (obs.rs)
├── vtop-proto/          parsers, types and HTTP shapes as page machines, with no I/O
├── vtop-core/           the VTOP client: runs vtop-proto's machines over a Transport, no FFI
├── vtop-shared/         formats shared by app, engine and edge (wire_page, mail_seal, ...)
├── vtop-bridge/         the Engine: axum service over vtop-core, on hawk
├── vtop-mcp/            MCP tools served by the Engine behind the edge's /mcp
├── vtop-tlsn/           TLSNotary proofs of VTOP's /vtop/content (the account proof): prover for the app, verifier for the Engine
├── vtop-wire/           UDP mode's transport (Noise IK, Reed-Solomon), used by the app and the gateway
└── vtop-gateway/        UDP mode's gateway: vtop-wire in, the Engine's /engine/fetch out; makes its own weekly keys (on a Linode in Chennai)
backend/                 self-hosted Convex in front of the Engine (TypeScript, at the repo root; backend/README.md)
mail/ proxy/             the Cloudflare Email Worker and the Proxy route's Worker
deploy/                  alchemy.run.ts: deploys the Engine, the gateway and the Workers
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

`vtop-ffi` wraps `vtop-core` for the app with UniFFI proc macros. Its
`src/` is grouped by what a module is about:

| Place | Holds |
| --- | --- |
| `lib.rs` | `Core`, `CoreError`, the shared types and the session slot |
| `session.rs` | sign-in, the OTP prompt, signing in again, sign-out |
| `otp.rs` | the OTP prompt the app's sheet shows (`OtpListener`); a re-login the student watches waits there for the code instead of failing |
| `pages/` | one file per feature (`attendance.rs`, `timetable.rs`, `marks.rs`, `exams.rs`, `outing.rs`, ...) with its own `#[uniffi::export] impl Core` block and ready-to-draw records (`records.rs`), the change inbox (`changes.rs`) and Gmail's OTP (`gmail.rs`) |
| `remote/` | everything that talks to our servers: `backend.rs` (its exported calls; + `backend/`: Convex calls in `rpc.rs`, auth state machine in `auth.rs`, identity lifecycle in `identity.rs`, live watches in `live.rs`), `edge.rs` (the edge's URL and the session cookies sent to it), `route.rs` (+ `route/`), `udp.rs`, `consent.rs`, `account_token.rs`, `keys.rs` (Connected apps), `handover.rs` (+ `handover/`: a connected app's session request, from arrival to its one answer), `forwarding.rs` and `server.rs` (the Server page) |
| `store.rs` (+ `store/`) | the file reads and writes; `decoded.rs` (decoded copies) and `cookies.rs` (the shared cookie store) |
| top level | `transport.rs` (Kotlin's HTTP hop), `device.rs` (this install's keys), `dev.rs` (Developer), `demo.rs`, `web.rs`, `obs.rs` (logs), `macros.rs` (`signed_in!`) |

Every call takes the caller's flow id first. `mise run rust-android` builds
`libvtop_ffi.so` into `app/jniLibs/` and generates the Kotlin bindings
(`core/src/vitapmate/core/vtop_ffi.kt`).

- **Pages.** A feature's `fetch_x` calls `Core::fetch(&spec::X, ...)`,
  which fetches the page for the picked semester and saves it at
  `pages/<semester>/<spec file>`; `Core::page` fetches one without saving.
  Both ask the route first (the server or UDP) and fall back to
  `VtopClient::fetch` on the phone. `cached_x` reads the saved copy with
  `read_page`.
- **Decoded copies** (`store/decoded.rs`). The semester list, the pick and every
  page stay decoded in memory after the first read, keyed by path and
  type, so screens that read the same page many times parse it once. A
  copy is used only while the file's length, modified time and inode are
  what they were when it was read; a write drops it, and sign-out,
  another student, Developer's clears and a forgotten pick drop what they
  remove. At most 4 MB of JSON is kept per core (on `Shared`, so a core's
  budget and clears are its own), the least recently used going first.
- **Route policy** (`remote/route/plan.rs`). Which route to try, the 5-minute
  skips and the steps of a server fetch (consent, the fetch, one sign-in
  again) are pure: no I/O and no clock, the caller passes `now`.
  `remote/route.rs` runs its steps on the core's plan.
- **Per-core state** (`Shared` in `lib.rs`). The route plan, cookie consent,
  the Developer config and the app's pieces live on `Core`, and every VTOP
  client's transport (`Routed`) holds the same `Arc`. Kotlin hands `Core::new`
  one `Platform` record (the HTTP transport, the shared cookie store, the UDP,
  Convex and proof sockets, the cookie gate, the password cipher, the Developer
  config and the phone's name), so leaving one out doesn't compile.
- **The saved password** (`store/account.rs`). `account.json` holds it sealed
  by `PasswordCipher` (Kotlin's `AccountPassword`: an AES-GCM key in the
  Android Keystore); a plain-text file from an older version is sealed once,
  when the core opens, and kept as it is while the Keystore fails. A copy restored
  on another phone, or after a reinstall, doesn't open (`Opening::WrongKey`): the
  password is gone and the student signs in again. A Keystore that fails for a
  moment (`Opening::Unavailable`) leaves the student signed in: `Core::current`
  opens it again on the next call (`CoreError::PasswordLocked` until then). Beside the
  password the file keeps whether VTOP rejected it, on this phone's sign-in
  (`rejected`, the session's `SessionPhase::PasswordRejected` in memory) or the
  server's (`server_rejected`, from the `credentials_rejected` push, until the
  student signs in again). Kotlin hears who is signed in and where their account
  stands (`AccountStatus`: signed out, or the installed student's username once
  their account is saved, with a `Standing`: good, demo, password rejected by the
  phone or the server) from `Core::watch_account` (`AccountWatch.kt`) and keeps no
  copy of its own: the app's phase follows it, and another username lets go of the
  last student's Kotlin things (`AccountTeardown.follow`). A sign-in still waiting
  for its first OTP changes nothing it reports. Kotlin gets the plain text only from `saved_password`, for the
  Account page. The log pipeline stays
  process-wide: `obs_init` runs before the core, so Kotlin's start-up lines
  (Gmail) reach the log file.

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
- **Routes** (`vtop-ffi/src/remote/route.rs`, Settings → Connection). Direct sends to
  VTOP. Proxy swaps VTOP's origin for the edge's `/vtop/` on each hop
  (`Routed`), so cookies, redirects and the parsers never see the edge. Server
  sends read pages to the edge's `/v1/vtop/fetch` with the phone's session and,
  when it expired, signs in through `/v1/vtop/login`; writes, downloads and the
  web view stay on the phone. Udp sends the same request to the gateway first
  (`remote/udp.rs`, a Kotlin `DatagramSocket` in `VtopUdp.kt`). A route that fails is
  skipped for 5 minutes and the request goes on to Direct. The log line
  `route used` names the route each call took. The open-app sweep sends its
  pages as one batch (`Core::fetch_pages`, `remote/route/batch.rs`,
  `/v1/vtop/fetch-batch`), asked about once; on Direct, or a server without
  batches, they are fetched a few at a time on the phone.
- **Cookie consent** (`vtop-ffi/src/remote/consent.rs`). Every path that sends the
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
  Each request's lifecycle (notified, answering, done) is Rust's
  (`remote/handover/board.rs`, pure); Kotlin's `Push.handover` only posts or
  cancels the notification and starts the answer as its effects say, and
  `Core::answer_handover` sends one answer within the deadline.
- **Registration number** (`vtop-ffi/src/remote/regno.rs`). A server identity names
  the student, so none is taken until they said Share on the "Share your registration
  number?" sheet (Kotlin's `RegnoSheet`, asked through `CookieGate::ask_regno`). The answer
  (Unasked, Shared, Declined) is kept per student in `regno_sharing.json`; a sign-out or
  another student empties it but keeps the file, and an install with no file (from before
  the question) counts each student it holds a refresh token, v2 token or forwarding
  address for as Shared. `take_identity` checks it first (`Core::regno_allowed`): until
  Shared there is no proof, no refresh, no push token sent and no server route
  (`ServerFetch::start` goes Direct, nothing skipped), and calls fail `RegnoNotShared`.
  A flow with the student on screen that needs the identity (`ask` true) shows the sheet;
  "Not now" holds for the rest of that flow. Background runs never ask. After a sign-in
  the app asks once (`RegnoAsk::FirstTime`), picking Server or UDP asks again
  (`ForFeature`), and Settings → Connections switches it; turning it off drops the JWT
  and refresh token here, goes back to Direct and signs this install out of the server
  (`auth:signOut` with the refresh token, so its chain is revoked and it leaves the
  forwarding address, kept here to rejoin). Offline, the token waits in
  `server_last_student.json` (`KeptSignOut`, as a switch or an offline Server sign-out
  does) and goes at the next call that reaches the server, before any new identity. The
  account, its address and keys stay (the Server page deletes those). The demo account is
  always Shared.
- **Account token** (`vtop-ffi/src/remote/account_token.rs`, API.md "Account
  tokens"). A 14-day token naming the student and this install, for the
  account calls (Connected apps, the Server page, mail forwarding), linked
  phone or not; the student's `Account` object keeps the live token per
  install and revokes it when the install leaves the account or the
  forwarding address. It proves who the student is, not that the phone is
  linked. The app keeps it (`account_token.json`) and sends it in place of
  the session. Without one (or after a refused or expired one) it gets one
  from the account proof below, so the session never goes and no consent is
  asked; any other call nobody started (`ask` false) never proves and
  stays `ConsentDenied`. The one unasked proof is the background one
  (`Core::prove_in_background`, Kotlin's `AppCore.proveInBackground`, an
  unattended flow): after a sign-in and at each open signed in, while the
  install has no refresh or v2 token, it proves so the identity is there
  before Connected apps, forwarding or a server route needs it. It runs
  only on an unmetered network or a validated one that isn't mobile data
  (Developer's "Background proof on mobile data" allows that), gives up
  after "Background proof timeout" (90 s) and tries again at the next
  open, and holds the identity's turn, so a call started meanwhile waits
  for its JWT instead of proving again. Saving the password goes with the token too: asked, the
  Server screen's dialog is the consent; unasked, it needs a remembered
  `Account` yes. A connected app's session and the routes still send the
  session. Dropped on sign-out,
  another student, and deleting the server account or this install. Old
  installs still send the session to get one (`withSession`), deprecated.
- **Account proof** (`vtop-ffi/src/remote/account_proof.rs`, `vtop-tlsn`,
  API.md "Account proof"). The phone proves its regno with TLSNotary in
  proxy mode: `POST /v1/account/prove/start` gives a single-use challenge
  (2 minutes, kept in the install's `Device` object), then the phone opens
  the signed WebSocket `GET /v1/account/prove?challenge=`. The edge uses
  the challenge up and relays binary messages to the engine's
  `/engine/proof?device=&challenge=` (`INTERNAL_TOKEN`, at most 4 at once).
  The engine dials VTOP itself and relays the phone's TLS to it; the phone
  sends one fixed `GET /vtop/content?n=<binding>`, binding =
  `hex(SHA-256("vitapmate-tlsn-v1:" + challenge + ":" + device))[..32]`,
  revealing all of it but the 32-hex-digit `JSESSIONID`, and reveals the
  response's head and the page the template covers, regno included (never a
  whole direction, and it reads to the end, capped at 1 MB). The verifier holds both to the template
  (`vtop-tlsn/assets/content.tmpl`, matched from body byte 0, strict
  framing, no `Set-Cookie`), checks the server name, the binding and the
  caps (16 MB prover I/O, 1 MB server I/O, reveal 4 KB sent, 16 KB
  received) and sends its verdict (regno, owner hash) as the last text
  message; the edge mints the token as after a cookie check and sends the
  phone `{ accountToken, expiresAt, registrationNumber }`. Kotlin carries
  the socket (`ProofNetwork`, `Platform::proof`). tlsn's
  sessions end on its mux's close handshake, not on EOF, and it does not
  always flush, so both ends' pipe sinks are channels drained by a writer
  loop (`vtop_tlsn::session::Pipe`); the phone's writer joins what the
  prover queued meanwhile into one message of at most 64 KiB, so a proof
  is a few hundred frames, not a thousand. The phone checks VTOP's session
  (`verify_session`, signing in again when it expired) while it gets the
  ticket and opens the socket. The verifier may queue a socket for up to a
  minute before it starts (pinging, sending nothing), so the prover's own
  limit is `PROVER_TIMEOUT` (150 s), the socket's idle timeout 90 s, and a
  proof someone started has 150 s from the ticket to the verdict. Each
  proof logs one "proof timings" line (ms per step: verify_session,
  prove_start, ws_open, tlsn_setup, tls, zk_prove, close; messages and
  bytes each way), and "server identity" has `auth:token`'s `token_ms`.
  The crypto crates (tlsn, mpz, aes, sha2, blake3, the curves) build at
  opt-level 3 in the release profile; the rest of the .so stays at "z".
  vtop-ffi builds it behind the `tlsn` feature (on by default).

The edge also serves `POST /v1/app/status` (signed by the phone, with the
registration number, no VTOP cookies): the sealed session's and password's ages,
when VTOP last rejected the password, the forwarding address, the phone count,
the last 10 sign-ins and each key's last 10 uses (extension or MCP, from the
`x-vtop-client` header). Settings → Server shows these and the account's other
records, with a delete for each (`vtop-ffi/src/remote/server.rs`). `/healthz` reports
`version` and `colo`.

MCP clients (Claude and others) connect to `/mcp` with OAuth (`backend/convex/
oauth.ts`): `/mcp` answers 401 with the metadata URL, the client registers and
opens `/authorize`, which shows a pairing code and QR. The student scans it in
Connected apps → Pair a browser; the app makes an access key with that
`pairCode`, named after the client, and `/token` hands the client that key.

## UDP mode

```
phone ──UDP (vtop-wire)──► gateway ──HTTP──► Engine /engine/fetch ──► VTOP
             (hawk, Chennai: the same compose, over NetBird)
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
  (`/engine/fetch-batch` for a batch, and the `/engine/vtop-login*` routes) with `GATEWAY_TOKEN`, returning
  the answer byte for byte. It keeps each device's VTOP session for six
  hours so requests carry an 8-byte hash instead of the cookies
  (`vtop_shared::wire_page`).
- **Keys** (`vtop-gateway/src/keys.rs`): the gateway makes its own key pair
  every Monday 00:00 IST, keeps the one before for a week and deletes
  older ones, in `$GATEWAY_DATA_DIR/udp-keys.json` (owner-only). At start
  and after each rotation it posts
  `{ host, ports, keys: [{ id, publicKey, from, until }] }` (public halves
  only, base64url) to the edge's `POST /internal/udp-keys` with
  `GATEWAY_TOKEN`, retrying with backoff (5 s doubling to 10 min). A
  handshake under an unknown key gets a 5-byte "wrong key" answer, and the
  app asks the edge again.
- **Abuse limits** (no rate limits: a campus shares one IP): message 1
  must be fresh and its timestamp unseen for the device; unproven
  addresses get at most 3x the bytes they sent, and a reset is never
  larger than the packet behind it; at most 4 requests open per session,
  50,000 sessions (the longest idle makes room) and 256 engine calls at once
  (more get "busy" and use HTTPS); sending stops after 30 s of silence.

Internal calls between the edge and the engine carry `Authorization: Bearer
$INTERNAL_TOKEN`. The gateway, on a public VPS, has its own `GATEWAY_TOKEN`:
the engine takes it only on the fetch and sign-in routes, the edge only on
`/internal/udp-keys`, so it can't ask whose a session is or wait for OTPs.

Deploy: `bun run deploy` in `deploy/` (`alchemy.run.ts`), from a laptop; CI deploys
nothing. Convex, the engine and the gateway run on hawk, the Linode `fedora-in-maa`
(172.232.115.40, 2 GB), from one compose, `deploy/hawk/compose.yaml` in `/opt/vmate`;
Convex is served at vmate.aster0.dev. The engine listens on hawk's NetBird address
(`ENGINE_URL` http://100.126.178.2:8090) for Convex and the gateway. Caddy sends `/proof`
to the verifier on hawk (`127.0.0.1:8091`), else to its standby on nest
(`100.126.222.47:8091`); the verifier's one secret is `VERIFIER_TOKEN`, which Convex
holds too (`bunx convex env set VERIFIER_TOKEN`) and the deploy reads from there. The engine and
gateway images are built for x86 on nest and pushed to GHCR. The gateway uses host
networking (UDP 3478-3481), `rust/vps/resolv.conf` as its resolvers and
`~/vitapmate-gateway/data` for the keys. The same deploy updates the mail and proxy
Workers and then the Convex functions.

## Tests

- `vtop-proto/tests/parsers.rs`: fixture HTML → JSON snapshots, plus truncated
  pages that must not panic. Fixtures are synthetic or redacted.
- `vtop-core/src/client/tests.rs`: session reuse against an in-process mock VTOP.
- `vtop-bridge`: the engine routes (session, seal, sign-in with a forwarded
  or Gmail OTP, the proof route over a real WebSocket).
- `vtop-tlsn/tests`: the content policy (`content.rs`) and real tlsn
  sessions against a fake VTOP (`e2e.rs`, feature `fake-vtop`): relays
  swapping the device or challenge, signed out, smuggled headers,
  `Set-Cookie`, planted copies, over-cap reveals, another server.
- `vtop-wire`: framing, Reed-Solomon, the loss estimate, and client ↔
  gateway over localhost UDP (30% loss, restarts, replays, two processes).
- `vtop-gateway`: the client through the gateway to a mock engine.
- `backend/` (`bunx vitest run`): accounts, devices, mail OTP, account proofs' challenges and relay; `mail/` and `proxy/` (`bun run test`).
- `vtop-ffi/src/tests.rs`, `<module>/tests.rs` (`pages/outing/`, `remote/forwarding/`, ...) and next to
  each module: the app's crate; `test_edge.rs` is the mock edge they share.
- `vtop-core/examples/parse_bench.rs`: parser timings on fixtures or raw pages.
