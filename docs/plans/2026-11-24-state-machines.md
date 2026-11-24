# State Machine Audit

Stateful workflows across the app that currently spread their state across multiple booleans, `Option`s, mutexes, files, or Kotlin/Rust layers — and would benefit from the single-state, pure-transition pattern already used in `rust/vtop-ffi/src/remote/route/plan.rs` (`RoutePlan`, `ServerFetch`) and `rust/vtop-proto/` page machines.

See also [`2026-11-24-architecture-audit.md`](2026-11-24-architecture-audit.md) for broader structural, concurrency, persistence, and type-safety improvements beyond state machines.

## Done

### 0. Server Identity & Token Lifecycle (`rust/vtop-ffi/src/remote/backend/auth.rs`)
- **Status:** Done — refactored into `AuthMachine` (`Phase::{Unauthenticated, Ready(Jwt)}` + monotonic `generation` and `jwt_epoch` counters + explicit `Effect`s).
- **What it fixed:**
  - Stale `device_removed` / `unauthenticated` callbacks from earlier identities/JWTs can no longer wipe a newer refresh token or trigger double refreshes.
  - Expired/mismatched JWTs emit `SetSocketAuth(None)` before `auth:token` runs, avoiding `AuthError` reconnect storms during refresh.
  - `LiveWatch` `renew()` invalidates the failing `jwt_epoch` instead of short-circuiting on a cached `Ready` JWT.
  - `sign_out_last_student` persists rotated refresh tokens before `auth:signOut` and restores the active student's socket JWT afterwards.
  - Server URL comparison normalises trailing slashes, and `prove_in_background` checks grant availability inside `identity_turn`.
  - Block Store (`AccountBackup`) was removed so reinstalls never restore stale rotated refresh tokens.

### 1. VTOP Session, Login, OTP & Re-login Lifecycle
- **Status:** Done — `SessionPhase` (`Active` | `AwaitingOtp { via, message, challenge_id }` | `PasswordRejected`) in `rust/vtop-ffi/src/lib.rs`, `LoggingInGuard` in `rust/vtop-core/src/client/auth.rs`, `OtpPhase` + `phaseFlow` in `features/login/src/vitapmate/features/login/OtpChallenge.kt`, `OtpPrompt.challengeId` in `core/src/vitapmate/data/Page.kt`, and `cancelOtp` on `LoginScreen` cancel.
- **What it fixed:** `AuthStage::LoggingIn` leak on login error/cancel, `server_otp` TOCTOU across consent await, `udp::adopt_renewed` clearing stale OTP phase, asymmetric cookie clearing on rejected password in `server_relogin`, `LoginScreen` OTP cancel leak, `FetchOtpOverlay` message key collision, and `while (submitting) delay(100)` spin-wait.

### 2. `ConvexClient` Connection Lifecycle & `LiveWatch` Renewal Coordinator
- **Status:** Done — `Link` enum (`Idle`, `Backoff`, `Opening`, `Open`, `Closed`) in `rust/vtop-ffi/src/convex/client.rs`, single `Condvar`-driven `Renewal` coordinator in `rust/vtop-ffi/src/remote/backend.rs`, and channel-close `LiveWake` in `core/src/vitapmate/data/Live.kt`.
- **What it fixed:** Combinatorial connection flags, `AuthError` backoff inflation, per-error `"convex-renew"` thread spawning, old client timer leak on `on_server_changed`, and `LiveWake` `@Volatile` shutdown race.

### 5. Connected Apps Session-Request Handover
- **Status:** Done — clock-injected `HandoverCoordinator` (`Notified` | `Answering` | `Done`) in `app/src/vitapmate/HandoverCoordinator.kt`, wired into `PushService.kt` and `MainActivity.kt` with unit tests in `app/test/vitapmate/HandoverCoordinatorTest.kt`.
- **What it fixed:** Unbounded `Push.answered` set, expiry checks before prompting/answering, FCM dropped while foregrounded, foreground/tap promotion without double-answering, per-request notification IDs, and timeout vs completion double-answer race.

### 6. In-App VTOP WebView Session & Recovery
- **Status:** Done — pure, clock-injected `VtopWebState` (`Preparing` | `WaitingForView` | `Active` | `Failed` | `Crashed`) in `features/vtop/src/vitapmate/features/vtop/VtopWebState.kt`, wired into `VtopScreen.kt` and `VtopBrowser.kt` with unit tests in `features/vtop/test/vitapmate/features/vtop/VtopWebStateTest.kt`.
- **What it fixed:** Infinite spinner after renderer crash (`WaitingForView` waits for `ViewAttached`), racing `prepare()` jobs cancelled by monotonic `requestId`, stale `WebView` callbacks after `destroy()`, and explicit login-page recovery.

### 9. Cookie Consent (Targeted Fixes)
- **Status:** Done — `in_flight` coalescing + out-of-mutex disk write in `rust/vtop-ffi/src/remote/consent.rs`, and `StateFlow<List<Request>>` for `CookieConsent.pending` in `core/src/vitapmate/data/CookieConsent.kt` (removing the `AppCore.kt` snapshot workaround).

---

## Remaining — High Priority

### 3. Mail Forwarding Lifecycle & Leave Queue (Deferred — active uncommitted edits in `forwarding.rs` / `Forwarding*.kt`)
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

## Remaining — Low / Targeted Priority

### 7. Background Sync Pause State & Gmail Connect UI (Deferred — active uncommitted edits in `BackgroundSync.kt`)
**Files:** `core/src/vitapmate/data/Gmail.kt:189-214`, `features/login/src/vitapmate/features/login/GmailContent.kt:149-270`, `app/src/vitapmate/BackgroundSync.kt:63-67, 193-250`, `app/src/vitapmate/AccountScreen.kt:151`.
- Replace magic strings in `Settings.syncPaused` + `SharedPreferences("sync")` `MISSES` with a small `SyncHealth` state (`Active(otpMisses)` | `Paused(Reason)`).
- Guard `Gmail.save` / `setDeleteAfterReading` with `Gmail.lock`, and hoist `rememberConnecting()` above the `Setup`/`Connected` branch in `GmailContent.kt`.

### 8. App Root Boot & Account Teardown
**Files:** `app/src/vitapmate/App.kt:49-151`, `app/src/vitapmate/SettingsScreen.kt:312-333`, `app/src/vitapmate/AccountScreen.kt:153-165`.
- Consolidate `signedIn`, `user`, `semesterRead`, `onboarding`, and `SemesterChoice.selected` into one `AppSession` enum (`Booting` | `BootFailed` | `SignedOut` | `LoadingAccount` | `PickingSemester` | `Onboarding` | `Ready`), and share one `teardownAccount()` helper between sign-out and account switch.

---

## Already Clean (No State Machine Needed)
- `rust/vtop-ffi/src/remote/route/plan.rs` (`RoutePlan`, `ServerFetch`) — pure, clock-injected, table-tested.
- `rust/vtop-proto/` page/login/OTP machines — pure I/O-free state machines.
- `rust/vtop-ffi/src/store/cookies.rs` (`Place::{Own, Shared, Gone}`).
- `core/src/vitapmate/data/BiometricFetch.kt` (`afterStall` / `Next`).
- `rust/vtop-ffi/src/remote/account_proof.rs` — linear async pipeline with RAII `Shut` drop guard.
- `core/src/vitapmate/data/WebSocket.kt` — clean reader/writer lifecycle.
- `app/src/vitapmate/PlayUpdate.kt`, `ClassMode.kt`, `LiveUpdates.kt`, `CalendarSync.kt`.
