# Broader Architecture & Structural Audit (Beyond State Machines)

Structural, concurrency, type-safety, persistence, and backend improvements across the codebase that go beyond the state-machine refactors tracked in [`2026-11-24-state-machines.md`](2026-11-24-state-machines.md).

---

## High Priority

### 1. `Core::forget(&[LAST_STUDENT])` wipes the current student's cached `pages/` directory
- **Files:** `rust/vtop-ffi/src/remote/backend.rs:1168, 1182, 1220`, `rust/vtop-ffi/src/store.rs:21-44`
- **Problem:** `Core::forget(&self, files: &[&str])` unconditionally runs `fs::remove_dir_all(self.at(PAGES))` (`store.rs:37`) and `decoded::forget(&self.dir)` (`store.rs:43`) while holding `forwarding::FILE_LOCK`. When Student B signs in over Student A offline and later comes online, `sign_out_last_student` removes `server_last_student.json` via `self.forget(&[LAST_STUDENT])`, silently wiping all of Student B's cached pages on disk and in memory.
- **Recommended change:** Split `store.rs` into `remove_file(&self, file: &str)` (removes one file + `decoded::forget(&path)`) and `forget_student_data(&self, files: &[&str])` (explicitly clears `pages/` + student files). Use `remove_file` in `sign_out_last_student`, `forget_account_token`, and single-file dev helpers.

### 2. Uncancellable OS-thread-per-sleep in `vtop_core::sync::sleep`
- **Files:** `rust/vtop-core/src/sync.rs:152-184`; callers: `rust/vtop-ffi/src/remote/backend.rs:645`, `rust/vtop-ffi/src/remote/account_proof.rs:206, 373`, `rust/vtop-ffi/src/remote/udp.rs:100`
- **Problem:** Every `sleep(duration)` spawns a new OS thread (`"vtop-sleep"`) calling `std::thread::sleep(duration)` with no cancellation on `Drop`. Every Convex RPC (`within(15s|35s, ...)`) completes in milliseconds and drops the timeout future, stranding an OS thread for 15–35 s; account proofs strand 150 s threads; UDP pacing spawns a thread per wait.
- **Recommended change:** Replace per-call thread spawning with a single lazy `Condvar` + `BinaryHeap<(Reverse<Instant>, u64, Waker)>` timer wheel thread, with a `Drop` guard on the `Sleep` future that removes its entry by timer ID.

### 3. Systemic cross-thread Compose Snapshot mutations in global Kotlin singletons
- **Files:**
  - `core/src/vitapmate/data/Activity.kt:26, 29, 37, 40-42` + `core/src/vitapmate/data/Freshness.kt:82, 121`
  - `core/src/vitapmate/data/Settings.kt:17-29, 194-200` + `app/src/vitapmate/PushService.kt:69` + `app/src/vitapmate/BackgroundSync.kt:181, 198, 208, 218-225` + `core/src/vitapmate/data/VtopSync.kt:132, 162` + `core/src/vitapmate/data/Gmail.kt:190, 192`
  - `core/src/vitapmate/data/Gmail.kt:93, 106, 189, 205, 280, 287, 428` + `core/src/vitapmate/data/AppCore.kt:68`
  - `core/src/vitapmate/data/AccountState.kt:14` + `core/src/vitapmate/data/PageState.kt:105` + `core/src/vitapmate/data/VtopSync.kt:88`
  - `core/src/vitapmate/data/SemesterChoice.kt:19-23, 30` + `core/src/vitapmate/data/VtopSync.kt:113`
  - `core/src/vitapmate/data/Docs.kt:130-131, 337, 400, 438, 446, 482, 494`
- **Problem:** Compose `mutableStateOf` / `mutableStateListOf` / `mutableStateMapOf` are mutated directly from `Dispatchers.IO`, `"core-init"`, WorkManager, and FCM binder threads without thread confinement. `Activity.running` / `recent` are mutated concurrently across IO coroutines with zero locking, risking `IndexOutOfBoundsException` and Snapshot apply conflicts. `problemOf()` (`PageState.kt:105`) also mutates `AccountState.rejected` as a hidden side effect of formatting an error (including from `Dispatchers.IO` via `BackgroundProblems.report`).
- **Recommended change:** Back cross-thread singleton state with `MutableStateFlow` + atomic `.update { ... }` on immutable collections/records, exposing `collectAsState()` adapters at the UI boundary. Remove the `AccountState.rejected = true` side effect from `problemOf()`.

### 4. Non-atomic / racy file persistence and unsynchronized Keystore key creation
- **Files:** `rust/vtop-ffi/src/store.rs:95-103`, `rust/vtop-ffi/src/remote/consent.rs:108-119, 204, 237`, `rust/vtop-ffi/src/dev.rs:341`, `core/src/vitapmate/data/Secrets.kt:31-37, 59-68`, `core/src/vitapmate/data/Docs.kt:498-503`, `core/src/vitapmate/observability/ActivityHistory.kt:36-52`
- **Problem:**
  - `Core::write_bytes` (`store.rs:97-98`) writes to a fixed `path.with_extension("tmp")` without per-file locking or `sync_all()`; concurrent writers to the same file race on the same `.tmp` path.
  - `consent.rs:115` and `dev.rs:341` do direct non-atomic `fs::write` (and `consent.rs` performs disk I/O while holding `Shared::consent`).
  - `Secrets.kt:34-36`, `Docs.kt:500-502`, and `ActivityHistory.kt:49-51` use fixed `.tmp` filenames and ignore `File.renameTo()`'s `Boolean` return value.
  - `Secrets.key()` (`Secrets.kt:59-68`) is unsynchronized: concurrent first-time `seal`/`open` calls can race on `generateKey()`, silently overwriting a key already used to seal data.
  - `ActivityHistory.load()` (`line 36`) is not `@Synchronized` while `save()` and `trim()` are.
- **Recommended change:** Use unique sibling temp files (`<name>.<pid>.<atomic_seq>.tmp`) + `sync_all()` + `rename` (cleaning up on error) in one atomic-write helper in Rust (`store.rs`) and Kotlin (`AtomicFile` or checked unique-tmp rename). Synchronize `Secrets.key()` and `ActivityHistory.load()`, and move `consent.rs` disk writes outside its mutex.

### 5. Kotlin `Log` events bypass Rust's secret-field redaction
- **Files:** `rust/vtop-ffi/src/obs.rs:73-86` vs `157-200`
- **Problem:** Rust `tracing` events pass string fields through `Fields::insert_text` → `secret_name()` redaction, whereas `log_event()` (`obs.rs:84`) inserts Kotlin's `fields` map directly into `extra` without calling `secret_name()`.
- **Recommended change:** Apply `secret_name(&key)` redaction inside `log_event()` before `emit()`.

### 6. Unsynchronized cold-start initialization & repeated background `Settings.load`
- **Files:** `core/src/vitapmate/data/AppCore.kt:36, 38, 44-46, 120-121`, `core/src/vitapmate/data/Settings.kt:194-200`, `app/src/vitapmate/PushService.kt:69-70`, `app/src/vitapmate/BackgroundSync.kt:181-182, 198-199`
- **Problem:** `AppCore.started` is a plain non-`@Volatile` `Boolean` checked from Main, FCM binder, and WorkManager threads; concurrent cold starts can spawn duplicate `"core-init"` threads and construct two `Core` instances. `Settings.load(app)` has no once-guard and runs again on every FCM message and WorkManager job from background threads, clobbering live Compose state while the UI is active. `AppCore.proving` is similarly unsynchronized.
- **Recommended change:** Guard `AppCore.start` and `Settings.load` with `AtomicBoolean.compareAndSet(false, true)` (or initialize once in a custom `Application.onCreate()`), and guard `proving` atomically.

---

## Medium Priority

### 7. Process-global `static` locks/state in `vtop-ffi` and repeated disk reads on `device.json`
- **Files:** `rust/vtop-ffi/src/device.rs:19, 107-122, 144-145`, `rust/vtop-ffi/src/remote/forwarding.rs:43, 45`, `rust/vtop-ffi/src/remote/udp.rs:35`, `rust/vtop-ffi/src/pages/changes/store.rs:27`, `rust/vtop-ffi/src/demo.rs:171`, `rust/vtop-ffi/src/obs.rs:31-32, 58-61`
- **Problem:** Five process-global `static Mutex<()>` locks (`device::LOCK`, `forwarding::FILE_LOCK`, `forwarding::LEAVE_LOCK`, `changes::store::LOCK`, `demo::record_demo::LOCK`) and `udp::JWT_SENT: AtomicU64` couple independent `Core` instances. `device.rs` does double lock + double disk read in `keep_refresh`/`set_clock_offset`/`set_push_token`, and `server_now()` reads and parses `device.json` from disk under `LOCK` on every timestamp check.
- **Recommended change:** Move per-core locks, `udp_jwt_sent`, and an in-memory `Mutex<Option<Device>>` cache onto `Shared`.

### 8. Stringly-typed error matching and keys across internal Rust/Kotlin boundaries
- **Files:**
  - `rust/vtop-ffi/src/remote/udp.rs:254, 264` + `rust/vtop-ffi/src/remote/route.rs:503` + `rust/vtop-ffi/src/remote/route/batch.rs:185-193, 202`
  - `rust/vtop-ffi/src/remote/consent.rs:137-139`, `rust/vtop-ffi/src/remote/account_proof.rs:214`
  - `core/src/vitapmate/data/Live.kt:60-64`, `core/src/vitapmate/data/Gmail.kt:228, 262, 279, 347`
  - `core/src/vitapmate/data/Settings.kt:176-181, 242` + `rust/vtop-ffi/src/pages/changes.rs:233-245`
- **Problem:** `udp_fetch` formats human strings into `Result<Value, String>` which callers parse back with `.ends_with(" session_expired")` and `.contains("answered 404")`. Similar brittle string prefix/suffix checks appear in `batch.rs`, `account_proof.rs`, `consent.rs`, `Live.kt`, `Gmail.kt`, and `Settings.notifies(kind: String)`.
- **Recommended change:** Replace with typed enums: `UdpFetchError`, `ServerError::NotFound`, `ConsentDecision` (`fn is_allowed(self) -> bool`), `GmailErrorCode`, and `ChangeKind` keys for `Settings.notifies`.

### 9. Cancel-safety memory leak in `Core::fetch_pages` prefetch map
- **Files:** `rust/vtop-ffi/src/remote/route/batch.rs:78-99`
- **Problem:** `fetch_pages` inserts parsed page JSON `Value`s into `Shared::routes().prefetched` and only removes them after `page_alone` completes. If the coroutine is cancelled mid-flight (sweep timeout / screen exit), entries remain in memory indefinitely.
- **Recommended change:** Wrap the inserted keys in a small RAII `Drop` guard (`PrefetchGuard`) so cancellation always cleans them up.

### 10. Stale re-insertion race in `store/decoded.rs`
- **Files:** `rust/vtop-ffi/src/store/decoded.rs:70-115`
- **Problem:** `decoded::read` reads `Stamp::of(path)`, drops `CACHE`, parses the file, then re-inserts under the old `stamp`. A concurrent write calling `decoded::forget(&path)` during the parse can be overwritten by the stale read.
- **Recommended change:** Bump a monotonic `generation: u64` on `Cache` in `forget()`, and skip caching if `generation` changed while unlocked.

### 11. Head-of-line blocking in Convex `otpInbox` cleanup cron
- **Files:** `backend/convex/sweeps.ts:43-49`, `backend/convex/schema.ts:244-255`
- **Problem:** `otpInbox` mixes 5-minute `"otp"` rows and 24-hour `"gmail_confirm"` rows under one `by_receivedAt` index. `sweeps:quarter` takes the oldest 500 rows and skips young `"gmail_confirm"` rows; ≥ 500 confirmations in 24 h starve deletion of all expired `"otp"` rows.
- **Recommended change:** Add an `expiresAt` field + `by_expiresAt` index (matching `proofTickets`/`pairings`/`oauthCodes`), or index `["kind", "receivedAt"]` and sweep each kind separately.

### 12. Polling loops replaceable with event-driven primitives
- **Files:**
  - `app/src/vitapmate/PushService.kt:136` (`while (!hasServerIdentity(f)) delay(5_000)`)
  - `core/src/vitapmate/data/Docs.kt:305-323` (`DownloadManager.query` every 500 ms)
  - `core/src/vitapmate/data/Gmail.kt:340-352` (`untilReachable` blind 2 s retry for 3 min)
  - `core/src/vitapmate/observability/Inspector.kt:39-45` (500 ms `LogTail` file polling)
- **Recommended change:** Emit an identity-change signal (`MutableStateFlow`) for `Push.follow`; listen for `DownloadManager.ACTION_DOWNLOAD_COMPLETE` via `BroadcastReceiver` + `CompletableDeferred`; await `Foreground.shownFlow` / `NetworkCallback` in `Gmail.untilReachable`; use `FileObserver` or an in-process `SharedFlow` tick for `Inspector`.

---

## Low Priority

### 13. Synchronous Main-thread disk and Keystore I/O
- **Files:** `core/src/vitapmate/data/Gmail.kt:213-215`, `core/src/vitapmate/data/Docs.kt:169`
- **Problem:** `Gmail.setDeleteAfterReading` runs Keystore AES-GCM encryption and file write/rename synchronously on the UI thread; `Docs.init` reads `index.json` synchronously during `MainActivity.onCreate`.
- **Recommended change:** Dispatch `setDeleteAfterReading` onto `Dispatchers.IO` under `Gmail.lock`, and load `Docs`'s index asynchronously on `Docs.scope`.

### 14. Schema & framing hygiene across `backend/convex/`, `rust/vtop-shared/`, and local JSON stores
- **String-packed error codes in `sessionRequests`** (`backend/convex/sessions.ts:38-40, 504, 511, 518`, `backend/convex/lib/accountState.ts:32, 90-93, 487`): store `errorCode` and `error` as separate fields instead of packing `"code: message"` and splitting on `:`.
- **Duplicated cascade-delete list & LRU eviction** (`backend/convex/lib/accounts.ts:163` vs `backend/convex/account.ts:186`; `backend/convex/lib/mailboxes.ts:234-243` vs `accountState.ts:200`): share one `ACCOUNT_OWNED_TABLES` constant and reuse `leastRecent`.
- **`WirePageResponse::encode` framing & `batch_args` ordering** (`rust/vtop-shared/src/wire_page.rs:39-49, 192-202`): don't set the `session` flag bit (`flags |= 2`) if `long(&mut v, session)` returns `None` (`> 65535` bytes), and guard batch length `< 100` (or encode index numerically) so `BTreeMap` key ordering can't mis-order.
- **Unversioned local JSON files**: only `demo.json` (`schema: u32`) and `changes.json` (`format: u8`) carry a version; `account.json`, `device.json`, `forwarding.json`, `consent.json`, `route.json`, and `docs/index.json` lack schema versions and silently fall back to `None` on corruption.
