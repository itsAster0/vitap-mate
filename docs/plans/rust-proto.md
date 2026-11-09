# Rust core without I/O (vtop-proto) + Kotlin transport

Status (2026-11-09): phases 1–3 and 5 done, phase 4 done except HttpEngine; see Linear KRY-24…KRY-30. Goal: faster cold start and a much
smaller native library in the KMP app. Rust thinks (state machines, parsing,
session rules); Kotlin moves bytes (OkHttp) and runs the loop.

## Now

```
Kotlin core.fetchX(flow) (suspend) ─JNA─▶ UniFFI async (tokio)
  └─ vtop-core VtopClient: reqwest + rustls + hyper + h2, ResettableJar (cookies),
     SessionManager (CSRF, reg no), relogin (login page → CAPTCHA via `image`
     → creds → OTP → CoreException.OtpRequired), scraper parsers,
     cache pages/<sem>/*.json + session.json; tracing → obs.rs → Kotlin Log
```

- libvtop_ffi.so ≈ 5.1 MB arm64 (HTTP/TLS stack, webpki roots, tokio, image, serde_json).
- Start-up: load 5 MB via JNA, start tokio, Rust TLS roots, restore session.
- Networking bypasses Android (system proxy/CA, HTTP/3, HTTP Toolkit toggle).
- Web view cookies copied from Rust (`web.rs`).
- Users of vtop-core: vtop-ffi (KMP), vtop-bridge, vtop-mcp (rust/src, the Flutter crate, was removed on 2026-11-09).

## After

```
vtop-proto   sync, no async/tokio/network/TLS. Deps: scraper, serde, uniffi (no async runtime).
             Own: baseline JPEG decoder, base64, form/url encoding, CAPTCHA model as consts.
             Machines: Login, one per page/action. Session state: CSRF, reg no, semester,
             stage, rejected. Detects expiry / wrong password / OTP / VTOP down. Parses.
             Writes the cache (std::fs). Never stores cookies.
Kotlin       OkHttp (redirects off; CookieJar backed by android.webkit.CookieManager, shared
             with the web view; interceptor adds x-request-id, logs without secrets, timeouts).
             Generic driver loop; coroutines give concurrency, cancellation, single-flight login.
Server       same vtop-proto + ~40-line Rust driver on reqwest/tokio with its own jar.
Other        vm1/UDP or any custom protocol = another driver.
```

```kotlin
suspend fun <T> Vtop.run(m: Machine<T>, flow: Flow): T {
    var step = m.start()
    while (true) step = when (step) {
        is Send       -> m.feed(http.send(step.request, flow))
        is NeedOtp    -> m.feedOtp(OtpPrompt.await(step.info))
        is NeedSignIn -> { signIn.once(flow); m.restart() }
        is Done       -> return step.result.getOrThrow()
    }
}
```

Contract: `Request {method, url, headers, body, stream, flow, timeout}`,
`Response {status, headers incl. every Set-Cookie, final url, body | file}`.
Transport never follows redirects; downloads stream to a file, Rust checks
headers and names it.

Expected: lib ≈ 1.5–2.5 MB (estimate, measure), no Rust threads at start,
Android TLS/proxy/CA, one cookie store, no async across FFI, tests replay
recorded responses with no server. JNA stays (UniFFI).

## Risks

- Login/OTP as a state machine: biggest job; record real sequences first
  (sign-in, OTP, expired session, rejected password) and replay them in tests.
- Own JPEG decoder must give the same answers as today on ~50 saved CAPTCHAs
  (confirm VTOP serves baseline JPEG).
- CookieManager jar: flush after sign-in; usable off the main thread.
- Sync Rust calls run on Dispatchers.IO (parse-only, fast).

## Phases (each leaves the app working)

0. Measure .so, APK, cold start (Flutter + KMP release). Save ~50 CAPTCHAs
   with answers. Record real page + auth sequences (redacted).
1. vtop-proto: parsers in, Request/Response types, cookie/form helpers;
   vtop-core re-exports (no caller breaks).
2. Page machines one by one; vtop-core keeps its API via an internal reqwest driver.
3. Login + OTP machine and own CAPTCHA decoder, tested on the recordings.
4. App: OkHttp + CookieManager jar + Kotlin driver; move pages one at a time.
5. Drop tokio/reqwest/image/serde_json from the app build; delete web.rs cookie copying.
6. Server (bridge, MCP) onto the Rust driver; retire the old client.
7. Measure again.

## Decided (2026-11-08)

1. OkHttp directly (no Ktor).
2. Cookie jar backed by android.webkit.CookieManager (shared with the web view).
3. Cache stays in Rust (std::fs, same files and format as today).
4. Server: no migration concern. The app and the server use completely separate
   endpoints, so the server keeps its old client; phase 6 is optional cleanup only.
   Later: redesign the server properly as its own project (likely on vtop-proto).

## Still open (my defaults)

5. Gmail OTP reading becomes its own glue crate on the transport.
6. Errors: thiserror (decided in phase 1).
7. Work on its own jj branch, separate from the KMP UI work (default yes).

## Transport for the proxy route (decided 2026-11-09)

The proxy route uses okhttp-httpengine on Android 14+ (plain OkHttp below).
It must work for any HTTPS server: HTTP/3 when offered, falling back to
HTTP/2 and HTTP/1.1 on its own (campus Wi-Fi blocks QUIC). Direct VTOP
requests stay on plain OkHttp, since VTOP speaks only HTTP/1.1.

## As built (2026-11-09)

- Machines live in `rust/vtop-proto` (pages, outing, course pages, login, OTP, CAPTCHA).
  vtop-core still drives them for the app over async (no tokio since KRY-29), with the hop going to Kotlin:
  the Kotlin driver loop described above was not needed for the size and TLS goals.
- Transport: vtop-core `Transport` (one hop; Rust follows redirects and decides retries).
  App: OkHttp via the UniFFI foreign trait `HttpTransport` (`VtopHttp.kt`); server: reqwest
  behind vtop-core's `reqwest` feature. Downloads go straight to a file (`download_to`).
- Cookies: not an OkHttp CookieJar (it would overwrite the explicit Cookie header the session
  check sends, and Rust needs cookies for forwarding and dev tools). Rust's `CookieStore` trait
  reads/writes android CookieManager through the foreign trait `SharedCookies`
  (`VtopCookies.kt`); only the installed session uses it. The web view cookie copy is gone.
  Old session.json cookies migrate once into CookieManager.
- libvtop_ffi.so arm64: 5,102,312 B (phase 0) → 2,652,144 B → 2,610,592 B without tokio (KRY-29).
- No tokio in the app build: UniFFI async exports run on Kotlin's polling; `vtop_core::sync`
  has the async `RwLock` (session slot, validation, per-course history lock) and `sleep` (a
  timer thread; reqwest's transport overrides `Transport::pause` with tokio's). The flow id
  for `x-request-id` is a thread-local set around each poll (`transport::with_flow`).
- HttpEngine (KRY-30): `VtopHttpEngine.kt`, the same `HttpTransport` on `android.net.http.HttpEngine`
  with no dependency, behind Developer → HttpEngine (off by default; Android 14+, and OkHttp while
  Inspect Traffic is on). Needs `ACCESS_NETWORK_STATE`. It honours `network_security_config.xml`,
  which adds VTOP's intermediate to the system CAs. VTOP negotiates HTTP/1.1 (no ALPN) on both
  stacks with the same timings; the edge goes h2, then h3 after Alt-Svc.
