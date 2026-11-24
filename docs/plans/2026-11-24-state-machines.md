# State Machine Audit

Stateful workflows across the app that currently spread their state across multiple booleans, `Option`s, mutexes, files, or Kotlin/Rust layers — and would benefit from the single-state, pure-transition pattern already used in `rust/vtop-ffi/src/remote/route/plan.rs` (`RoutePlan`, `ServerFetch`) and `rust/vtop-proto/` page machines.

## Done

### 0. Server Identity & Token Lifecycle (`rust/vtop-ffi/src/remote/backend/auth.rs`)
- **Status:** Refactored into `AuthMachine` (`Phase::{Unauthenticated, Ready(Jwt)}` + monotonic `generation` and `jwt_epoch` counters + explicit `Effect`s).
- **What it fixed:**
  - Stale `device_removed` / `unauthenticated` callbacks from earlier identities/JWTs can no longer wipe a newer refresh token or trigger double refreshes.
  - Expired/mismatched JWTs emit `SetSocketAuth(None)` before `auth:token` runs, avoiding `AuthError` reconnect storms during refresh.
  - `LiveWatch` `renew()` invalidates the failing `jwt_epoch` instead of short-circuiting on a cached `Ready` JWT.
  - `sign_out_last_student` persists rotated refresh tokens before `auth:signOut` and restores the active student's socket JWT afterwards.
  - Server URL comparison normalises trailing slashes, and `prove_in_background` checks grant availability inside `identity_turn`.
  - Block Store (`AccountBackup`) was removed so reinstalls never restore stale rotated refresh tokens.

---

## High Priority

### 1. VTOP Session, Login, OTP & Re-login Lifecycle
**Files:**
- `rust/vtop-ffi/src/lib.rs:206-276` (`Account`, `Session`, `Slot`)
- `rust/vtop-ffi/src/session.rs:58-298` (`sign_in`, `submit_otp`, `resend_otp`, `cancel_otp`, `fresh_challenge`, `relogin`, `sign_in_again`)
- `rust/vtop-ffi/src/remote/route.rs:455-636` (`on_server`, `server_relogin`, `server_otp`)
- `rust/vtop-ffi/src/remote/udp.rs:272-290` (`adopt_renewed`)
- `rust/vtop-core/src/session.rs:260-380` (`AuthState`, `SessionManager`)
- `rust/vtop-core/src/client/mod.rs:54-76, 650-696` (`AuthStage`, `ClientState`)
- `rust/vtop-core/src/client/auth.rs:10-105`, `rust/vtop-core/src/client/otp.rs:13-76`
- `core/src/vitapmate/data/Page.kt:372-429` (`OtpPrompt`), `core/src/vitapmate/data/AccountState.kt:13-18`, `core/src/vitapmate/data/VtopSync.kt:60-107`
- `features/login/src/vitapmate/features/login/OtpChallenge.kt:43-258`, `OtpSheet.kt:86-207`, `FetchOtpOverlay.kt:24-53`, `LoginScreen.kt:107-181, 314-343`, `OtpAutofill.kt:91-174`

**Current scattered state:**
Four parallel state models across three layers: `vtop-core` (`AuthState` + `AuthStage` + `logged_in_at`), `vtop-ffi` (`Session { otp, server_otp, logins, account.rejected }`), `core` Kotlin (`OtpPrompt { message, generation, asks }`, `AccountState`, `VtopSync`'s OTP coordinator), and `features/login` (`OtpChallenge`'s 12 mutable UI fields).

**Concrete issues:**
- **TOCTOU in `submit_otp` / `resend_otp` → `server_otp` (`session.rs:106-108, 138-140`, `route.rs:589-635`):** Checks `server_otp` under a read lock, drops it to await the consent dialog (up to 60 s), then takes a write lock and calls `resume_session(&state)` without re-checking `server_otp`, `otp.is_some()`, or `self.is_current(slot)`.
- **`udp::adopt_renewed` (`udp.rs:281-289`) ignores open OTP state:** Resuming a renewed session leaves `session.otp = Some(...)` while the client is authenticated.
- **`vtop-core` `AuthStage::LoggingIn` leak (`client/auth.rs:12, 68-77`):** Set before login, never reset to `Idle` if `run("login", ...)` fails with a network error or `InvalidCredentials`.
- **Asymmetric credential rejection:** `sign_in_again` (`session.rs:285-289`) clears cookies and checks `is_current(slot)`; `server_relogin` (`route.rs:553-558`) skips cookie clearing.
- **`LoginScreen` OTP cancel leak (`LoginScreen.kt:337-340`):** Clears local `otp = null` without calling `core.cancelOtp()`, leaving the installed `Session` with `otp = Some(...)`.
- **`FetchOtpOverlay` key collision (`FetchOtpOverlay.kt:29`):** `remember(message)` collides when two consecutive challenges share VTOP's default message.
- **`OtpChallenge` concurrent mutations (`OtpChallenge.kt:90-257`):** Countdown, autofill, and user callbacks mutate shared flags concurrently (`while (submitting) delay(100)`), while `OtpAutofill.solve` duplicates the same retry loop.

**Proposed state machine:**
- **Rust `SessionPhase`** (single owner in `Slot`, collapsing `vtop-core`'s `AuthStage` + `AuthState` as well):
  - `Active { logins, logged_in_at }`
  - `AwaitingOtp { via: Phone | Server, message, issued_at, challenge_id }`
  - `PasswordRejected`
  - Every OTP submission/resend carries `challenge_id`, and `UdpAdopted` transitions directly to `Active`.
- **Kotlin `OtpChallengeState`** keyed by `challengeId` instead of message text:
  - `Active(code, remaining, autofill: Watching | Filled | Off | Error, minimized)` | `Submitting(code)` | `Expired(autoResendLeft)` | `Resending` | `Done` | `Cancelled`.

---

### 2. `ConvexClient` Connection Lifecycle & `LiveWatch` Renewal Coordinator
**Files:**
- `rust/vtop-ffi/src/convex/client.rs:101-133, 254-390, 417-566, 644-661`
- `rust/vtop-ffi/src/remote/backend.rs:159-340` (`live_watch`, `keep_renewed`, `renew`)
- `core/src/vitapmate/data/Live.kt:34-129`
- `app/src/vitapmate/PushService.kt:130-147`

**Current scattered state:**
- `convex/client.rs:101-133` tracks connection state with 7 overlapping fields (`socket`, `epoch`, `live`, `open`, `reconnect_at`, `closed`, `auth_changed`) alongside `backoff_ms`, `last_server_ms`, `last_used_ms`, and `pending`.
- `backend.rs` runs a `std::thread::sleep` loop (`keep_renewed`) plus ad-hoc per-error OS threads (`convex-renew`).
- `PushService.kt:136` polls `hasServerIdentity` every 5 s in a Kotlin loop.

**Concrete issues:**
- **Combinatorial flag states in `ConvexClient`:** 32+ flag combinations for 5 real states, requiring repeated ad-hoc guards across handlers.
- **Backoff bump on `AuthError` (`client.rs:465-473`):** `drop_socket` doubles `backoff_ms` before `reconnect_at = Some(now)` overrides the delay, leaving `backoff_ms` inflated for the next real disconnect.
- **Per-error OS thread spawning in `renew()` (`backend.rs:307-328`):** Every failing watch spawns a new `"convex-renew"` thread instead of waking one `Condvar`-backed renewal coordinator.
- **`LiveWake` shutdown race (`Live.kt:111-124`):** `@Volatile live` + conflated `Channel` can race between shutdown and buffered wakes.

**Proposed state machine:**
- **Pure `Link` machine in `convex/client.rs`:**
  - `Idle` | `Backoff { at_ms, backoff_ms }` | `Opening { epoch, socket: Option<_>, deadline_ms }` | `Open { epoch, socket, last_server_ms, last_used_ms, auth_syncing }` | `Closed`.
  - Single `step(event, now_ms) -> (Vec<Io>, Option<next_tick_ms>)` function.
- **Single `Condvar`-driven watch renewal coordinator** replacing both `keep_renewed`'s `thread::sleep` and per-watch `convex-renew` threads: `Idle` | `Scheduled(due_ms)` | `Renewing`.

---

### 3. Mail Forwarding Lifecycle & Leave Queue
**Files:**
- `rust/vtop-ffi/src/remote/forwarding.rs:46-154, 239-501, 531-736`
- `rust/vtop-ffi/src/remote/server.rs:324-371, 457-485, 518-541`
- `rust/vtop-ffi/src/session.rs:207, 311`
- `features/login/src/vitapmate/features/login/MailForwarding.kt:29-157`

**Current scattered state:**
`Saved` encodes lifecycle across `joined`, `ready`, `owner_claimed`, `last_mail_at`, `last_otp_at`, and `confirmation`, plus a separate `forwarding_leave.json`, two process-global `static Mutex<()>` locks (`FILE_LOCK`, `LEAVE_LOCK`), and direct calls from `server.rs`, `backend.rs`, and `session.rs`. Kotlin's `MailForwarding.kt` mirrors it with `status`, `busy`, `error`, and `actions`.

**Concrete issues:**
- **False `ForwardingState::On` before v3 rejoin (`forwarding.rs:723-729`):** `view()` reports `On` when `ready == true` even if `joined == false`, causing `BackgroundSync` (`BackgroundSync.kt:65`) to run before `keep_forwarding` rejoins.
- **Unlocked write race in `set_up_forwarding` (`forwarding.rs:243-272`):** Writes `forwarding.json` without `FILE_LOCK` or checking whether sign-out occurred during `mail:register`.
- **`leave_queued` race (`forwarding.rs:455-501`):** Reads current mailbox ID once before the network loop; re-joining mid-loop still sends `mail:leave` for it.
- **Self-healing reported as error (`forwarding.rs:582-587`):** Successful `rejoin()` inside `pull()` still returns `Err(CoreError::Other(...))`.
- **Kotlin `MailForwarding.run` race (`MailForwarding.kt:89-118`):** `quiet = true` polls run concurrently with `turnOff`/`setUp` and `catch` ignores the `actions` guard.

**Proposed state machine:**
- **Rust `MailboxState`** owned by `Shared` (eliminating process-global locks):
  - `Off` | `NeedsRejoin { server, id, address }` | `WaitingSetup { server, id, address, confirmation, last_mail_at }` | `Active { server, id, address, last_mail_at, last_otp_at }` + `pending_leaves`.
  - Makes `Active` + `confirmation` and `NeedsRejoin` → `On` unrepresentable, and validates the session slot before writing.
- **Kotlin `MailForwarding`:** Serialized `Idle(Forwarding)` | `Busy(Action, Forwarding?)` so quiet polls cancel/defer around explicit user actions.

---

## Medium Priority

### 4. Page Fetch Queue (`SingleFlight`), `rememberPage` & `StaleSweep`
**Files:**
- `core/src/vitapmate/data/PageFetches.kt:25-201`
- `core/src/vitapmate/data/Page.kt:40-366`
- `core/src/vitapmate/data/Freshness.kt:60-237`

**Current scattered state:**
`SingleFlight.Entry` infers state from `go`, `result`, `queued`, `user`, and 7 external collections; `PageHandle` duplicates `fetching`, `running`, and `PageState<T>` across 3 `LaunchedEffect`s in `rememberPage`; `StaleSweep` uses unsynchronised `job` + `lastRun` + exception control flow (`class Stop`).

**Concrete issues:**
- `runningPages` (`mutableStateListOf`) is mutated from `Dispatchers.IO` inside `synchronized(runs)` (`PageFetches.kt:66, 93-97, 180-184`).
- `ask(key)` and `shown(id, on)` (`PageFetches.kt:108, 118-121`) change priority without calling `pump()`.
- Cancellation or `Dropped` while `state is PageState.Loading` leaves `PageHandle` stuck on `Loading` (`Page.kt:106-112`).
- `readCache` and `fetchNow` race across `LaunchedEffect(page)`, `OtpPrompt.generation`, and `Foreground.resumes` (`Page.kt:183-199, 300-328`).

**Proposed state machine:**
- Pure `FetchQueue` state per key (`Queued` | `Running` | `Cooldown(succeededAt)`) with `MutableStateFlow` for `runningPages`.
- Unified `PageHandle` reducer (`CacheLoaded`, `FetchStarted`, `FetchQueued`, `FetchSucceeded`, `FetchFailed`, `FetchDropped`, `FetchCancelled`).

---

### 5. Connected Apps Session-Request Handover
**Files:**
- `app/src/vitapmate/PushService.kt:54-199`
- `app/src/vitapmate/MainActivity.kt:93-106`
- `rust/vtop-ffi/src/remote/keys.rs:33-39, 358-403, 474-528`

**Current scattered state:**
Three entry points (`PushService.onMessageReceived`, `MainActivity.opened`, `Push.follow`) coordinate only through an unbounded `ConcurrentHashMap.newKeySet<String>()` (`Push.answered`).

**Concrete issues:**
- FCM arriving while `Foreground.shown == true` is dropped (`PushService.kt:67`) even if `Push.follow` is in backoff or the app immediately backgrounds.
- Timeout fallback (`PushService.kt:101-104`) can send a second `sessions:answer` while the first Rust call is still finishing.
- `SessionRequest.expiresAt` is never checked before prompting or calling `verify_session`.
- All session-request notifications share one notification ID (`ASKED.hashCode()`), overwriting concurrent requests.

**Proposed state machine:**
- Per-request `HandoverCoordinator` in `Push`: `Notified(expiresAt)` | `Answering(expiresAt, job)` | `Done(expiresAt)` (pruned on expiry, keyed per request ID).

---

### 6. In-App VTOP WebView Session & Recovery
**Files:**
- `features/vtop/src/vitapmate/features/vtop/VtopScreen.kt:92-228, 300-362`
- `features/vtop/src/vitapmate/features/vtop/VtopBrowser.kt:59-108, 185-227, 272-337, 377-487`

**Current scattered state:**
8 Compose states in `VtopScreen` (`session`, `problem`, `reconnecting`, `autoTried`, `askRelogin`, `viewGeneration`, `lastLoss`, `wasReloading`) plus 15 mutable fields in `VtopBrowser`.

**Concrete issues:**
- **Infinite spinner after renderer crash (`VtopScreen.kt:175-187`, `VtopBrowser.kt:186-196`):** `onLost` destroys the `WebView` (`view = null`) and launches `prepare()` immediately; if `prepare()` completes before `AndroidView` attaches the new `WebView`, `browser.open(fresh)` silently returns early and leaves the loading spinner forever.
- Multiple `prepare()` triggers (`vtop.open`, `relogin`, `onLost`, `OtpPrompt.generation`, `onLoginPage`) run without cancelling prior jobs.

**Proposed state machine:**
- `VtopWebState`: `Preparing(force, job)` → `WaitingForView(session)` → `Active(session, recovery: Ready | AutoRelogin | PromptRelogin)` → `Failed(problem)` | `Crashed(lastLossMs)`.

---

## Low / Targeted Priority

### 7. Background Sync Pause State & Gmail Connect UI
**Files:** `core/src/vitapmate/data/Gmail.kt:189-214`, `features/login/src/vitapmate/features/login/GmailContent.kt:149-270`, `app/src/vitapmate/BackgroundSync.kt:63-67, 193-250`, `app/src/vitapmate/AccountScreen.kt:151`.
- Replace magic strings in `Settings.syncPaused` + `SharedPreferences("sync")` `MISSES` with a small `SyncHealth` state (`Active(otpMisses)` | `Paused(Reason)`).
- Guard `Gmail.save` / `setDeleteAfterReading` with `Gmail.lock`, and hoist `rememberConnecting()` above the `Setup`/`Connected` branch in `GmailContent.kt`.

### 8. App Root Boot & Account Teardown
**Files:** `app/src/vitapmate/App.kt:49-151`, `app/src/vitapmate/SettingsScreen.kt:312-333`, `app/src/vitapmate/AccountScreen.kt:153-165`.
- Consolidate `signedIn`, `user`, `semesterRead`, `onboarding`, and `SemesterChoice.selected` into one `AppSession` enum (`Booting` | `BootFailed` | `SignedOut` | `LoadingAccount` | `PickingSemester` | `Onboarding` | `Ready`), and share one `teardownAccount()` helper between sign-out and account switch.

### 9. Cookie Consent (Targeted Fixes Only)
**Files:** `rust/vtop-ffi/src/remote/consent.rs:79-211`, `core/src/vitapmate/data/CookieConsent.kt:18-62`.
- `consent.rs::decide` is already concise and fail-closed; it does not need a full state machine.
- Two targeted fixes worth making: (1) update `CookieConsent.pending` via `MutableStateFlow` (or `Dispatchers.Main`) instead of mutating `mutableStateListOf` from `Dispatchers.IO`, and (2) coalesce concurrent `ask` calls for the same `(flow, CookieKind)` inside `consent.rs` so parallel fetches don't queue duplicate dialogs.

---

## Already Clean (No State Machine Needed)
- `rust/vtop-ffi/src/remote/route/plan.rs` (`RoutePlan`, `ServerFetch`) — pure, clock-injected, table-tested.
- `rust/vtop-proto/` page/login/OTP machines — pure I/O-free state machines.
- `rust/vtop-ffi/src/store/cookies.rs` (`Place::{Own, Shared, Gone}`).
- `core/src/vitapmate/data/BiometricFetch.kt` (`afterStall` / `Next`).
- `rust/vtop-ffi/src/remote/account_proof.rs` — linear async pipeline with RAII `Shut` drop guard.
- `core/src/vitapmate/data/WebSocket.kt` — clean reader/writer lifecycle.
- `app/src/vitapmate/PlayUpdate.kt`, `ClassMode.kt`, `LiveUpdates.kt`, `CalendarSync.kt`.
