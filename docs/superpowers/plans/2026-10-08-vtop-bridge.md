# vtop-bridge and vtop-mcp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two new Rust services: `vtop-bridge` (Go-compatible FCM cookie bridge plus credential vault and session broker) and `vtop-mcp` (MCP server that gets sessions from the bridge and calls `vtop-core`), plus the small app and extension changes, deployed to Railway with serverless on.

**Architecture:** Both crates join the `rust/` workspace beside `vtop-core` and `vtop-server`. The bridge keeps all state in Firestore over REST (so app sleeping loses nothing), seals secrets with AES-256-GCM, and is the only component that ever sees a password or Gmail token. The MCP service is stateless: bearer key → username → `POST /v1/session` on the bridge → `VtopClient::with_session` → `vtop-core` fetcher.

**Tech Stack:** Rust 2021, axum 0.8, tokio, reqwest (workspace), serde, tracing, `gcp_auth` 0.12, `aes-gcm` 0.11, `uuid` 1, `time` 0.3, `async-trait`, `rmcp` 3.x, `wiremock` 0.6 (tests); Flutter/Dart for the app; Svelte/Vite for the extension.

**Spec:** `docs/superpowers/specs/2026-10-08-vtop-bridge-design.md`

## Global Constraints

- Bridge HTTP contract for `/cookie`, `/cookie/status/:requestId`, `DELETE /cookie/status/:requestId`, `/cookie/callback`, `/healthz` matches `../fmc-token-go/server.go` exactly: paths, JSON field names, status codes, error codes, `Retry-After`, CORS headers, 64 KiB body limit, 10 `POST /cookie` per minute per client IP.
- API errors are `{"code": "...", "error": "..."}`.
- Credentials are stored only when the callback carries `credentials.gmail`. The session cache is written regardless.
- Session cache lifetime `SESSION_CACHE_SECS` default `1800`; cookie request lifetime `REQUEST_TIMEOUT_SECS` default `120`, valid 30–600; `PHONE_WAIT_SECS` default `60`; `GMAIL_WAIT_SECS` default `45`; poll interval returned to the extension `1500` ms.
- Vault wipe happens only on `VtopError::InvalidCredentials` or Google `invalid_grant`. Every other failure keeps the vault.
- Firestore collections: `bridge_requests`, `accounts`, `fcm_index`. Account ids are the upper-cased username. FCM index ids are lowercase SHA-256 hex of the FCM token.
- Sealed values are `base64(STANDARD)` of `nonce(12) || ciphertext`, associated data = `"<collection>/<document id>"`.
- Never log passwords, Gmail tokens, cookies, CSRF tokens, FCM tokens, response tokens or vault contents. Types holding them do not derive `Debug`.
- `vtop-mcp` never receives a password or Gmail token; it depends on `vtop-core` but not on the bridge crate.
- Benchmark data and raw VTOP page dumps are never committed.
- No deploy, Railway variable change or domain move without the user's explicit OK at that moment.

## Review Focus

1. Callback arrives after the request expired, or twice → 410 / 409, and the account is **not** updated by the stale or second callback.
2. Extension calls `/cookie` with an FCM token that has no `fcm_index` entry (first use, or the token rotated) → falls through to the normal FCM flow, no 404.
3. VTOP down or CAPTCHA failing during a vault login → error returned, vault still present afterwards.
4. Two MCP tool calls for the same user at once with an empty cache → exactly one VTOP login.
5. Railway sleep: the first request after waking succeeds with no in-memory state (no startup task required before serving `/cookie` or `/v1/session`).

Owning tasks: 1 → Tasks 5 and 8, 2 → Task 8, 3 → Task 7, 4 → Task 7, 5 → Task 8.

## File Structure

```
rust/Cargo.toml                        modify: workspace members
rust/railpack.vtop-bridge.json         create
rust/railpack.vtop-mcp.json            create
rust/vtop-bridge/
  Cargo.toml
  README.md
  src/main.rs        env → Config → AppState → serve, graceful shutdown
  src/lib.rs         AppState, router(), shared middleware
  src/config.rs      Config::from_env, validation
  src/error.rs       ApiError (code, message, status) → JSON
  src/vault.rs       Sealer: seal/open
  src/google.rs      TokenSource trait, ServiceAccountTokens (gcp_auth)
  src/store.rs       Store trait, docs, MemoryStore
  src/firestore.rs   FirestoreStore (REST, value encoding)
  src/fcm.rs         Messenger trait, FcmMessenger (HTTP v1)
  src/cookies.rs     CookieEditorCookie, header ⇄ cookies, validate
  src/requests.rs    /cookie* handlers, rate limiter, CORS
  src/accounts.rs    Accounts: sealed account read/write, record_callback
  src/gmail.rs       GmailRefresher trait, GoogleOAuthRefresher
  src/login.rs       VtopLogin trait, CoreLogin (vtop-core)
  src/broker.rs      Broker::session / expire, per-user lock, phone wait
  src/session_api.rs /v1/session, /v1/session/expire
  tests/cookie_api.rs   ported Go tests
  tests/session_api.rs
rust/vtop-mcp/
  Cargo.toml
  README.md
  src/main.rs
  src/config.rs      Config::from_env, parse MCP_KEYS
  src/bridge.rs      BridgeClient (SessionSource trait)
  src/tools.rs       rmcp tool router, with_retry
  src/lib.rs         router(): bearer auth layer + /mcp + /health
  tests/tools.rs
.railway/railway.ts                    modify: add both services
lib/core/utils/fcm_cookie_bridge_service.dart   modify
test/fcm_cookie_bridge_service_test.dart        modify
extension/src/background.ts, extension/public/manifest.json  modify (Task 12)
```

---

### Task 1: Bridge crate skeleton, config, errors, health

**Files:**
- Modify: `rust/Cargo.toml` (`members = ["vtop-core", "vtop-server", "vtop-bridge", "vtop-mcp"]` — add `vtop-mcp` in Task 9)
- Create: `rust/vtop-bridge/Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/error.rs`

**Interfaces:**
- Produces:
  - `pub struct Config { public_base_url: String, credentials_json: String, vault_key: [u8; 32], service_key: String, request_timeout: Duration, session_cache: Duration, phone_wait: Duration, gmail_wait: Duration, allowed_origins: Vec<String>, listen: SocketAddr, poll_after: Duration }`
  - `impl Config { pub fn from_env() -> Result<Config, String>; pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Config, String> }`
  - `pub struct ApiError { pub status: StatusCode, pub code: &'static str, pub message: String }` implementing `IntoResponse` as `{"code","error"}`; constructor `ApiError::new(status, code, message)`.
  - `pub fn router(state: Arc<AppState>) -> Router` in `lib.rs`; `AppState` grows in later tasks.

Dependencies: `vtop-core` (path), axum 0.8, tokio (workspace, `rt-multi-thread, macros, net, signal, time, sync`), reqwest/serde/serde_json (workspace), tracing, tracing-subscriber, async-trait, uuid (`v4`), time (`formatting, parsing, serde`), base64, sha2, hex, subtle, rand, aes-gcm, gcp_auth; dev: tower (`util`), http-body-util, wiremock.

- [ ] **Step 1: Write failing tests in `config.rs`**

```rust
fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> { /* map lookup */ }
const KEY: &str = /* base64 of 32 zero bytes */ "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
fn base() -> Vec<(&'static str, &'static str)> { vec![
  ("PUBLIC_BASE_URL","https://bridge.example"), ("FIREBASE_CREDENTIALS_JSON","{}"),
  ("VAULT_KEY",KEY), ("BRIDGE_SERVICE_KEY","0123456789abcdef0123456789abcdef")] }

#[test] fn defaults_apply() {
  let c = Config::from_vars(vars(&base())).unwrap();
  assert_eq!(c.request_timeout, Duration::from_secs(120));
  assert_eq!(c.session_cache, Duration::from_secs(1800));
  assert_eq!(c.phone_wait, Duration::from_secs(60));
  assert_eq!(c.gmail_wait, Duration::from_secs(45));
  assert_eq!(c.poll_after, Duration::from_millis(1500));
  assert_eq!(c.listen.port(), 8080);
  assert!(c.allowed_origins.is_empty());
}
#[test] fn public_base_url_must_be_https_origin() { /* "http://x", "https://x/?a=1", "" → Err */ }
#[test] fn request_timeout_bounds() { /* "29" Err, "601" Err, "30" Ok */ }
#[test] fn vault_key_must_be_32_bytes() { /* 16-byte base64 → Err */ }
#[test] fn service_key_min_32_chars() { /* 31 chars → Err */ }
#[test] fn credentials_json_must_parse() { /* "{" → Err */ }
#[test] fn port_env_sets_listen() { /* PORT=9000 → listen.port()==9000 */ }
```

- [ ] **Step 2:** `cd rust && cargo test -p vtop-bridge config` → FAIL (not defined).
- [ ] **Step 3:** Implement `Config::from_vars` / `from_env`, `ApiError`, `lib.rs` with `GET /healthz` returning `{"status":"ok","service":"vtop-bridge"}` and `Cache-Control: no-store` (store health check added in Task 3), `main.rs` with tracing-subscriber (`RUST_LOG`, default `info`), bind, `with_graceful_shutdown` on SIGTERM/SIGINT.
- [ ] **Step 4:** `cargo test -p vtop-bridge` → PASS; `cargo clippy -p vtop-bridge -- -D warnings` clean.
- [ ] **Step 5:** Commit `feat(vtop-bridge): crate skeleton, config and health`.

### Task 2: Vault sealing

**Files:** Create `rust/vtop-bridge/src/vault.rs`

**Interfaces:**
- Produces: `pub struct Sealer` (no `Debug`); `Sealer::new(key: [u8; 32]) -> Sealer`; `seal(&self, aad: &str, plaintext: &[u8]) -> String`; `open(&self, aad: &str, sealed: &str) -> Result<Vec<u8>, VaultError>`; `seal_json<T: Serialize>(&self, aad, &T) -> String`; `open_json<T: DeserializeOwned>(&self, aad, &str) -> Result<T, VaultError>`. `pub enum VaultError { Corrupt }`.

- [ ] **Step 1: Failing tests**

```rust
#[test] fn round_trip() { let s = Sealer::new([7;32]); let c = s.seal("accounts/A", b"pw"); assert_eq!(s.open("accounts/A", &c).unwrap(), b"pw"); }
#[test] fn nonce_differs_each_seal() { assert_ne!(s.seal("a", b"x"), s.seal("a", b"x")); }
#[test] fn wrong_aad_rejected() { assert!(s.open("accounts/B", &s.seal("accounts/A", b"x")).is_err()); }
#[test] fn wrong_key_rejected() { assert!(Sealer::new([8;32]).open("a", &s.seal("a", b"x")).is_err()); }
#[test] fn garbage_rejected() { assert!(s.open("a", "not base64!").is_err()); assert!(s.open("a", "AAAA").is_err()); }
```

- [ ] **Step 2:** `cargo test -p vtop-bridge vault` → FAIL.
- [ ] **Step 3:** Implement with `aes_gcm::Aes256Gcm`, random 12-byte nonce from the OS RNG, `Payload { msg, aad }`.
- [ ] **Step 4:** `cargo test -p vtop-bridge vault` → PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): AES-GCM vault sealing`.

### Task 3: Store trait, in-memory store, Firestore store

**Files:** Create `src/store.rs`, `src/google.rs`, `src/firestore.rs`; modify `src/lib.rs` (`AppState.store: Arc<dyn Store>`, `/healthz` calls `store.health()` with a 2 s timeout → 503 `{"status":"unavailable","service":"vtop-bridge"}`).

**Interfaces:**
- Produces (`store.rs`):

```rust
pub struct RequestDoc { pub id: String, pub response_token: Option<String>, pub status: RequestStatus,
  pub cookies_json: Option<String>, pub error: Option<String>, pub fcm_hash: Option<String>,
  pub want_credentials: bool, pub created_at: u64, pub expires_at: u64 }
pub enum RequestStatus { Pending, Success, Error }
pub struct AccountDoc { pub username: String, pub vault: Option<String>, pub session: Option<String>,
  pub session_expires_at: Option<u64>, pub fcm_token: Option<String>, pub updated_at: u64 }
pub enum CompleteError { NotFound, BadToken, Expired, AlreadyHandled, Store(StoreError) }
pub struct StoreError(pub String);

#[async_trait] pub trait Store: Send + Sync {
  async fn create_request(&self, doc: &RequestDoc) -> Result<(), StoreError>;
  async fn get_request(&self, id: &str) -> Result<Option<RequestDoc>, StoreError>;
  /// Atomic: checks token (constant time), expiry, Pending; sets status/cookies_json/error,
  /// clears response_token. Returns the updated doc.
  async fn complete_request(&self, id: &str, token: &str, status: RequestStatus,
      cookies_json: Option<String>, error: Option<String>, now: u64) -> Result<RequestDoc, CompleteError>;
  async fn delete_request(&self, id: &str) -> Result<(), StoreError>; // missing = Ok
  async fn get_account(&self, username: &str) -> Result<Option<AccountDoc>, StoreError>;
  async fn put_account(&self, doc: &AccountDoc) -> Result<(), StoreError>; // full overwrite
  async fn username_for_fcm(&self, fcm_hash: &str) -> Result<Option<String>, StoreError>;
  async fn put_fcm_index(&self, fcm_hash: &str, username: &str) -> Result<(), StoreError>;
  async fn health(&self) -> Result<(), StoreError>;
}
pub struct MemoryStore { /* Mutex<HashMap>s */ }  // pub, used by tests in later tasks
```

- Produces (`google.rs`): `#[async_trait] pub trait TokenSource: Send + Sync { async fn token(&self) -> Result<String, StoreError>; fn project_id(&self) -> &str; }`, `ServiceAccountTokens::from_json(&str) -> Result<Self, String>` using `gcp_auth::CustomServiceAccount::from_json`, scope `https://www.googleapis.com/auth/cloud-platform`, project id from the JSON's `project_id`.
- Produces (`firestore.rs`): `FirestoreStore::new(http: reqwest::Client, tokens: Arc<dyn TokenSource>, base_url: String)`; `base_url` defaults to `https://firestore.googleapis.com/v1` (tests pass the wiremock URI). Documents at `projects/{p}/databases/(default)/documents/{collection}/{id}`.

Firestore field types: strings → `stringValue`, bools → `booleanValue`, `created_at`/`expires_at`/`session_expires_at`/`updated_at` → `timestampValue` (RFC 3339, so the TTL policy on `expiresAt` works). Field names are camelCase as in the spec.

`complete_request` on Firestore: `GET` the doc, check in code, then `PATCH` with `currentDocument.updateTime=<the GET's updateTime>` and an `updateMask`; a `FAILED_PRECONDITION` (HTTP 400/409 with that status) maps to `AlreadyHandled`. `create_request` uses `currentDocument.exists=false`.

- [ ] **Step 1: Failing tests** — one shared suite run against `MemoryStore` (in `store.rs`) and the encode/decode functions (in `firestore.rs`):

```rust
#[tokio::test] async fn complete_rejects_bad_token() // CompleteError::BadToken, doc still Pending
#[tokio::test] async fn complete_rejects_expired()   // now >= expires_at → Expired
#[tokio::test] async fn complete_twice_is_already_handled()
#[tokio::test] async fn complete_clears_response_token() // returned doc.response_token == None
#[tokio::test] async fn delete_missing_is_ok()
#[tokio::test] async fn account_round_trip_and_fcm_index()
#[test] fn request_doc_encodes_and_decodes() // to_fields → from_fields equals input; expiresAt is timestampValue
#[test] fn account_doc_encodes_absent_fields_as_missing()
#[tokio::test] async fn firestore_complete_precondition_failure_is_already_handled() // wiremock: GET 200, PATCH 400 FAILED_PRECONDITION
#[tokio::test] async fn firestore_get_404_is_none()
```

- [ ] **Step 2:** `cargo test -p vtop-bridge store firestore` → FAIL.
- [ ] **Step 3:** Implement `MemoryStore`, `ServiceAccountTokens`, `FirestoreStore` (REST via reqwest, bearer from `TokenSource`, 5 s per-call timeout).
- [ ] **Step 4:** Tests PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): Firestore-backed store`.

### Task 4: FCM messenger

**Files:** Create `src/fcm.rs`

**Interfaces:**
- Produces: `#[async_trait] pub trait Messenger: Send + Sync { async fn send(&self, fcm_token: &str, data: &BTreeMap<String, String>) -> Result<(), FcmError>; }`; `pub enum FcmError { TokenInvalid, Misconfigured, Unavailable }`; `FcmMessenger::new(http, tokens: Arc<dyn TokenSource>, base_url)` (default `https://fcm.googleapis.com/v1`); `pub struct FakeMessenger` (records sends, configurable result) for tests.

Request: `POST {base}/projects/{p}/messages:send` with `{"message":{"token","data",{"android":{"priority":"HIGH","ttl":"120s"}}}}`.
Error mapping from `error.details[].errorCode` / `error.status`: `UNREGISTERED`, `INVALID_ARGUMENT`, `SENDER_ID_MISMATCH` → `TokenInvalid`; `THIRD_PARTY_AUTH_ERROR`, HTTP 401/403 → `Misconfigured`; anything else → `Unavailable`.

- [ ] **Step 1: Failing wiremock tests:** `sends_high_priority_data_message` (asserts body JSON), `unregistered_maps_to_token_invalid`, `sender_mismatch_maps_to_token_invalid`, `forbidden_maps_to_misconfigured`, `server_error_maps_to_unavailable`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): FCM HTTP v1 messenger`.

### Task 5: Cookie-request API (port of the Go service)

**Files:** Create `src/cookies.rs`, `src/requests.rs`, `tests/cookie_api.rs`; modify `src/lib.rs` (routes, CORS, body limit, access log, `AppState { config, store, messenger, sealer, … }`).

**Interfaces:**
- Consumes: `Store`, `MemoryStore`, `Messenger`, `FakeMessenger`, `Config`, `ApiError`.
- Produces:
  - `cookies.rs`: `#[derive(Serialize, Deserialize, Clone)] pub struct CookieEditorCookie` with the Go JSON fields (`domain, hostOnly, httpOnly, name, path, sameSite, secure, session, storeId?, value, expirationDate?, id?`); `pub fn validate(&[CookieEditorCookie]) -> Result<(), &'static str>` (non-empty; name, value, domain present); `pub fn to_header(&[CookieEditorCookie]) -> String` (`name=value; …`); `pub fn from_header(&str) -> Vec<CookieEditorCookie>` matching the app's `cookieEditorCookiesFromHeader` (domain `vtop.vitap.ac.in`, hostOnly true, httpOnly false, path `/`, sameSite `unspecified`, secure true, session true, storeId `"0"`, id = 1-based index of the part).
  - `requests.rs`: `pub struct PhoneRequests { store: Arc<dyn Store>, messenger: Arc<dyn Messenger>, public_base_url: String, request_timeout: Duration }` with `pub async fn start(&self, fcm_token: &str, want_credentials: bool) -> Result<RequestDoc, ApiError>` (creates the doc, sends FCM, deletes the doc and maps `FcmError` exactly as Go on failure: 410 `TOKEN_INVALID`, 503 `FCM_MISCONFIGURED`, 502 `FCM_UNAVAILABLE`). FCM data keys: `type="vtop_cookie_request"`, `requestId`, `responseToken`, `callbackUrl="{PUBLIC_BASE_URL}/cookie/callback"`, `wantCredentials`.
  - Handlers `post_cookie`, `get_status`, `delete_status`, `post_callback`. Callback parsing accepts and ignores (until Task 8) `username`, `fcmToken`, `credentials`.
  - `pub fn fcm_hash(token: &str) -> String` (SHA-256 lowercase hex).

Rate limit: fixed window 10/min keyed by the first `X-Forwarded-For` address (fall back to peer address); 429 `RATE_LIMITED` "Too many login requests. Try again shortly." with `Retry-After: 60`. CORS exactly as `addCorsHeaders` in `main.go`; `OPTIONS` on any path → 204.

- [ ] **Step 1: Port `../fmc-token-go/server_test.go`** into `tests/cookie_api.rs` using `router()` + `tower::ServiceExt::oneshot`, `MemoryStore`, `FakeMessenger`. Keep every Go case (status codes, codes, bodies). Add:

```rust
#[tokio::test] async fn late_callback_after_expiry_is_gone_and_doc_deleted()      // Review Focus 1
#[tokio::test] async fn second_callback_conflicts_and_keeps_first_cookies()       // Review Focus 1
#[tokio::test] async fn fcm_data_carries_want_credentials()
#[test] fn from_header_matches_app_cookie_editor_shape()
#[test] fn header_round_trip() // to_header(from_header("A=1; B=2")) == "A=1; B=2"
```

- [ ] **Step 2:** `cargo test -p vtop-bridge --test cookie_api` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): Go-compatible cookie request API`.

### Task 6: Gmail token refresh and vtop-core login adapter

**Files:** Create `src/gmail.rs`, `src/login.rs`

**Interfaces:**
- Produces (`gmail.rs`):

```rust
#[derive(Serialize, Deserialize, Clone)] pub struct GmailGrant { pub refresh_token: String, pub client_id: String,
  #[serde(default)] pub client_secret: Option<String>, #[serde(default)] pub delete_after_reading: bool }
pub struct FreshToken { pub access_token: String, pub expires_at: u64 }
pub enum RefreshError { InvalidGrant, Unavailable(String) }
#[async_trait] pub trait GmailRefresher: Send + Sync { async fn refresh(&self, grant: &GmailGrant) -> Result<FreshToken, RefreshError>; }
pub struct GoogleOAuthRefresher { /* http, token_url default https://oauth2.googleapis.com/token */ }
```

Form body: `grant_type=refresh_token`, `refresh_token`, `client_id`, plus `client_secret` only when present. HTTP 400 with `error=invalid_grant` → `InvalidGrant`; everything else non-200 → `Unavailable`.

- Produces (`login.rs`):

```rust
pub enum LoginError { InvalidCredentials, Failed(String) }
#[async_trait] pub trait VtopLogin: Send + Sync {
  async fn login(&self, username: &str, password: &str, gmail: GmailAccess, wait: Duration) -> Result<SessionState, LoginError>; }
pub struct CoreLogin { pub config: vtop_core::VtopConfig }
```

`CoreLogin` builds `VtopClient::builder().config(..).with_credentials(username, password)`, calls `login_with_gmail_otp(Some(&gmail), wait)`, returns `client.session_state()` on `Ok`. `VtopError::InvalidCredentials` → `LoginError::InvalidCredentials`; `OTPRequired` (Gmail had no usable email) → `Failed("otp_not_found: <gmail code>")`; other errors → `Failed(error.to_string())`.

- [ ] **Step 1: Failing wiremock tests:** `refresh_sends_secret_only_when_present` (two cases, assert form body), `invalid_grant_maps`, `server_error_is_unavailable`, `success_computes_expires_at` (`expires_in: 3599` → `now+3599`).
- [ ] **Step 2:** FAIL. **Step 3:** Implement both files (`CoreLogin` has no unit test; it is covered by the live check in Task 13). **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): Gmail refresh and vtop-core login adapter`.

### Task 7: Accounts and broker

**Files:** Create `src/accounts.rs`, `src/broker.rs`

**Interfaces:**
- Consumes: `Store`, `Sealer`, `GmailRefresher`, `VtopLogin`, `PhoneRequests::start` (Task 5), `Config`.
- Produces (`accounts.rs`):

```rust
#[derive(Serialize, Deserialize)] pub struct Vault { pub password: String, pub gmail: GmailGrant }
pub struct Accounts { store: Arc<dyn Store>, sealer: Arc<Sealer>, session_ttl: Duration }
impl Accounts {
  pub async fn cached_session(&self, username: &str, now: u64) -> Result<Option<SessionState>, StoreError>;
  pub async fn vault(&self, username: &str) -> Result<Option<Vault>, StoreError>;
  pub async fn fcm_token(&self, username: &str) -> Result<Option<String>, StoreError>;
  pub async fn save_session(&self, username: &str, session: &SessionState, now: u64) -> Result<(), StoreError>;
  pub async fn clear_session(&self, username: &str) -> Result<(), StoreError>;
  pub async fn clear_vault(&self, username: &str) -> Result<(), StoreError>;
  /// From a phone callback: saves the session, the sealed FCM token, the fcm_index entry,
  /// and the vault only when `vault` is Some.
  pub async fn record_callback(&self, username: &str, fcm_token: &str, session: &SessionState,
      vault: Option<Vault>, now: u64) -> Result<(), StoreError>;
}
```

Usernames are upper-cased on every entry point. A sealed value that fails to open is treated as absent and cleared (key rotation).

- Produces (`broker.rs`):

```rust
pub enum BrokerError { AccountUnknown, PhoneTimeout, PhoneError(String), Login(String), Store(StoreError), Fcm(ApiError) }
pub struct Broker { accounts: Arc<Accounts>, store: Arc<dyn Store>, phone: Arc<PhoneRequests>, refresher: Arc<dyn GmailRefresher>, login: Arc<dyn VtopLogin>, phone_wait: Duration, gmail_wait: Duration, locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> }
impl Broker {
  /// Steps 1–3 of the spec.
  pub async fn session(&self, username: &str) -> Result<SessionState, BrokerError>;
  /// Steps 1–2 only (used by POST /cookie); Ok(None) means "ask the phone".
  pub async fn session_without_phone(&self, username: &str) -> Result<Option<SessionState>, BrokerError>;
  pub async fn expire(&self, username: &str) -> Result<(), BrokerError>;
}
```

Per-username lock is held across steps 1–3. The phone wait polls `store.get_request` every 1 s until `Success` (then reads `cached_session`), `Error` (→ `PhoneError(error)`), or `phone_wait` elapses (→ `PhoneTimeout`); the request doc is deleted in every case. `GmailAccess` is built as `{ access_token, delete_after_reading, expires_at: Some(..) }`.

- [ ] **Step 1: Failing tests** (`MemoryStore`, `FakeMessenger`, fake `GmailRefresher` and `VtopLogin` with call counters; a helper that completes the pending request through `store.complete_request` + `accounts.record_callback` from a spawned task to simulate the phone):

```rust
#[tokio::test] async fn cache_hit_skips_login_and_phone()
#[tokio::test] async fn expired_cache_logs_in_with_vault_and_caches()       // session_expires_at = now + 1800
#[tokio::test] async fn invalid_credentials_wipes_vault_then_asks_phone()   // FCM data wantCredentials == "1"
#[tokio::test] async fn invalid_grant_wipes_vault_then_asks_phone()
#[tokio::test] async fn transient_login_error_keeps_vault()                 // Review Focus 3: LoginError::Failed → Err(Login), vault still Some
#[tokio::test] async fn refresh_unavailable_keeps_vault()
#[tokio::test] async fn no_vault_asks_phone_with_want_credentials()
#[tokio::test] async fn phone_timeout_returns_phone_timeout_and_deletes_request() // phone_wait = 50 ms
#[tokio::test] async fn unknown_account_is_account_unknown()
#[tokio::test] async fn concurrent_sessions_log_in_once()                   // Review Focus 4: 2 joined calls, login counter == 1
#[tokio::test] async fn expire_clears_only_session()
#[tokio::test] async fn record_callback_without_vault_keeps_existing_vault()
#[tokio::test] async fn undecryptable_vault_is_cleared_and_treated_absent()
```

`wantCredentials` is `"1"` exactly when no vault exists at send time.

- [ ] **Step 2:** `cargo test -p vtop-bridge broker accounts` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(vtop-bridge): sealed accounts and session broker`.

### Task 8: Wire the broker into the HTTP API

**Files:** Create `src/session_api.rs`, `tests/session_api.rs`; modify `src/requests.rs`, `src/lib.rs`, `src/main.rs` (build real `FirestoreStore`, `FcmMessenger`, `GoogleOAuthRefresher`, `CoreLogin`).

**Interfaces:**
- Consumes: `Broker`, `Accounts`, `fcm_hash`, `cookies::{from_header, to_header}`.
- Produces: `POST /v1/session {"username"}` → `200 {"session": SessionState}`; `POST /v1/session/expire {"username"}` → 204. Header `X-Service-Key` compared in constant time with `BRIDGE_SERVICE_KEY`. Error mapping: missing/wrong key 401 `unauthorized`; `AccountUnknown` 404 `account_unknown`; `PhoneTimeout` 504 `phone_timeout`; `PhoneError`/`Login` 502 `login_failed` (message = error text); `Store` 503 `STORE_UNAVAILABLE`.

Changes in `requests.rs`:
- `post_cookie`: `fcm_index` lookup → if a username is found, `broker.session_without_phone`; on `Some(session)` create the request doc already `Success` with `cookies_json = from_header(&session.cookies)`, return 202 with the same body shape (the extension's next poll gets the cookies). On `None`, lookup miss, or broker error, fall through to `PhoneRequests::start` with `want_credentials = vault is None`.
- `post_callback`: after `complete_request` succeeds with `Success` and the body has `username` + `fcmToken`, call `accounts.record_callback` with `SessionState { cookies: to_header(&cookies), ..Default }` and a `Vault` only when `credentials.gmail` is present. A store failure here is logged and does not fail the callback (the extension still gets its cookies).

- [ ] **Step 1: Failing tests**

```rust
// tests/cookie_api.rs
#[tokio::test] async fn cookie_with_unindexed_fcm_token_uses_phone()         // Review Focus 2
#[tokio::test] async fn cookie_served_from_cache_without_fcm()               // FakeMessenger sends == 0, status poll returns cookies
#[tokio::test] async fn cookie_with_vault_and_transient_login_error_asks_phone_without_credentials() // wantCredentials == "0"
#[tokio::test] async fn callback_with_credentials_and_gmail_stores_vault()
#[tokio::test] async fn callback_with_credentials_without_gmail_stores_no_vault()
#[tokio::test] async fn callback_after_expiry_does_not_touch_account()        // Review Focus 1
// tests/session_api.rs
#[tokio::test] async fn session_requires_service_key()                       // none and wrong → 401
#[tokio::test] async fn session_returns_cached_state()
#[tokio::test] async fn session_unknown_account_404()
#[tokio::test] async fn session_phone_timeout_504()
#[tokio::test] async fn expire_then_session_logs_in_again()
#[tokio::test] async fn fresh_state_serves_without_warmup()                   // Review Focus 5: new AppState over a pre-filled MemoryStore answers /v1/session immediately
```

- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** `cargo test -p vtop-bridge` all PASS; clippy clean.
- [ ] **Step 5:** Commit `feat(vtop-bridge): broker-backed cookie and session endpoints`.

### Task 9: vtop-mcp service

**Files:** Create `rust/vtop-mcp/{Cargo.toml, src/main.rs, src/lib.rs, src/config.rs, src/bridge.rs, src/tools.rs, tests/tools.rs}`; modify `rust/Cargo.toml` members.

**Interfaces:**
- Consumes: bridge `POST /v1/session`, `POST /v1/session/expire` (Task 8); `vtop_core::{VtopClient, SessionState, VtopError, types::*, inputs::*}`.
- Produces:
  - `Config { bridge_url: String, service_key: String, keys: HashMap<String, String> /* key → USERNAME */, listen: SocketAddr }`, `Config::from_vars`. `MCP_KEYS` entries are `key=username`; keys under 32 chars, duplicates, or an empty list → Err.
  - `#[async_trait] pub trait SessionSource: Send + Sync { async fn session(&self, username: &str) -> Result<SessionState, String>; async fn expire(&self, username: &str) -> Result<(), String>; }`; `BridgeClient` implements it over reqwest with a 90 s timeout (covers `PHONE_WAIT_SECS` + login).
  - `pub async fn with_retry<T, F, Fut>(source: &dyn SessionSource, username: &str, call: F) -> Result<T, String> where F: Fn(VtopClient) -> Fut, Fut: Future<Output = VtopResult<T>>` — gets a session, builds `VtopClient::builder().with_session(&s)`, runs `call`; on `SessionExpired` calls `expire` and repeats once.
  - Tools (rmcp tool router; each returns the `vtop-core` type as JSON text): `get_semesters()`, `get_attendance(semester_id)`, `get_full_attendance(semester_id, course_id, course_type)`, `get_timetable(semester_id)`, `get_marks(semester_id)`, `get_exam_schedule(semester_id)`, `get_grades(semester_id)`, `get_grade_details(semester_id, course_id)`, `get_grade_history()`, `get_biometric(date /* DD/MM/YYYY */)`. Inputs are validated with the same `vtop_core::inputs` constructors `vtop-server/src/routes.rs` uses; a validation error is a tool error, not a transport error.
  - `router(state) -> Router`: `GET /health`; `/mcp` served by rmcp's streamable HTTP service in **stateless** mode, behind a layer that maps `Authorization: Bearer <key>` to the username (constant-time compare) and stores it in request extensions for the tool handler; missing/unknown key → 401 before rmcp runs.

Check the `rmcp` 3.x docs for the current tool macro (`#[tool_router]` / `#[tool]`) and the stateless option on its streamable-HTTP server config before writing `tools.rs`; the names above are the contract, the macro syntax follows the crate.

- [ ] **Step 1: Failing tests** (`tests/tools.rs`, fake `SessionSource`; for `with_retry`, a closure returning `Err(SessionExpired)` first):

```rust
#[test] fn mcp_keys_parse_and_upper_case_usernames()
#[test] fn short_or_duplicate_keys_rejected()
#[tokio::test] async fn retry_expires_and_retries_once()          // expire called 1×, session called 2×
#[tokio::test] async fn retry_gives_up_after_second_expiry()
#[tokio::test] async fn mcp_without_bearer_is_401()
#[tokio::test] async fn mcp_tools_list_names_all_ten_tools()      // JSON-RPC initialize + tools/list over oneshot
#[tokio::test] async fn bad_semester_id_is_tool_error()
```

- [ ] **Step 2:** `cargo test -p vtop-mcp` → FAIL. **Step 3:** Implement. **Step 4:** PASS; clippy clean.
- [ ] **Step 5:** Commit `feat(vtop-mcp): stateless MCP server over the bridge`.

### Task 10: App sends username, FCM token and credentials

**Files:** Modify `lib/core/utils/fcm_cookie_bridge_service.dart`, `test/fcm_cookie_bridge_service_test.dart`

**Interfaces:**
- `postCookieCallbackWithRetry` gains optional `String? username`, `String? fcmToken`, `Map<String, dynamic>? credentials`; each is added to the body only when non-null.
- New `@visibleForTesting Map<String, dynamic>? bridgeCredentialsPayload({required bool wantCredentials, required String password, required EmailOtpOAuthSession? gmail})` → `null` unless `wantCredentials && gmail != null && gmail.hasGmailScope && gmail.refreshToken.isNotEmpty`; otherwise `{'password', 'gmail': {'refresh_token', 'client_id', 'client_secret'?, 'delete_after_reading'}}`. For `sharedBuiltIn`, `client_id` is `googleOauthClientId`; for `personalByok`, `oauthClientId` / `oauthClientSecret`. `delete_after_reading` comes from the app's existing "Delete After Reading" preference.
- `handleVtopCookieBridgeMessage` reads `data['wantCredentials'] == '1'`, the user's username/password from `vtopUserProvider`, the session from `GoogleEmailOtpAuthService.loadSession()`, the token from `FirebaseMessaging.instance.getToken()`.

- [ ] **Step 1: Failing tests:** `payload is null when not requested`, `payload is null without a Gmail session`, `payload is null without gmail.modify scope`, `byok payload carries client secret`, `shared payload uses built-in client id and no secret`, `callback body includes username and fcmToken` (fake http client asserts JSON).
- [ ] **Step 2:** `flutter test test/fcm_cookie_bridge_service_test.dart` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** PASS; `flutter analyze` clean.
- [ ] **Step 5:** Commit `feat(app): send credentials to the cookie bridge when it asks`.

### Task 11: Railway config and READMEs

**Files:** Create `rust/railpack.vtop-bridge.json`, `rust/railpack.vtop-mcp.json`, `rust/vtop-bridge/README.md`, `rust/vtop-mcp/README.md`; modify `.railway/railway.ts`.

- Railpack files copy `rust/railpack.json` with `--package vtop-bridge` / `vtop-mcp` and matching `startCommand`.
- `railway.ts`: add `vtop-bridge` and `vtop-mcp` services like `vtop-server` (root `/rust`, Railpack, `sleepApplication: true`, healthcheck `/healthz` / `/health`, watch patterns on their own crate plus `vtop-core`, workspace files and their railpack file), each with variable `RAILPACK_CONFIG_FILE` pointing at its railpack file. Update the `partial` comment: this repo now manages three services; `vtop_fmc` stays untouched until retired.
- READMEs: purpose, endpoints, configuration table (from the spec), local run, Claude Code connection snippet for `vtop-mcp`:
  `claude mcp add --transport http vtop https://<mcp-domain>/mcp --header "Authorization: Bearer <key>"`.

- [ ] **Step 1:** Confirm in Railway docs (`mcp__railway__search-docs`) that `RAILPACK_CONFIG_FILE` selects the config file and whether private-network requests wake a sleeping service; record the answer in the bridge README and set `BRIDGE_URL` guidance accordingly (private `http://vtop-bridge.railway.internal:<PORT>` if it wakes, public domain otherwise).
- [ ] **Step 2:** `railway config plan` (no apply) shows only the two added services and no change to `vtop-server`.
- [ ] **Step 3:** `cd rust && cargo build --locked --profile server -p vtop-bridge -p vtop-mcp` succeeds.
- [ ] **Step 4:** Commit `chore: Railway config and docs for vtop-bridge and vtop-mcp`.

### Task 12: Extension endpoint switch

**Files:** Modify `extension/src/background.ts:9`, `extension/public/manifest.json:23`

- [ ] **Step 1:** Set `API_BASE_URL` to the bridge's Railway domain (known after Task 13's deploy) and add it to `host_permissions` beside `https://be-va.kryxen.dev/*`.
- [ ] **Step 2:** `cd extension && pnpm test && pnpm build` pass.
- [ ] **Step 3:** Commit `chore(extension): point at vtop-bridge for testing`. After the domain swap in Task 13, revert `API_BASE_URL` to `https://be-va.kryxen.dev` and drop the temporary host permission in a second commit.

### Task 13: Live verification and rollout (each deploy step needs the user's OK)

- [ ] **Step 1: Shared-client refresh proof.** With the emulator's Gmail OTP session, call `GoogleOAuthRefresher::refresh` from a throwaway example (not committed) using the built-in client id and no secret. Expected: an access token. If Google demands a secret, stop and report; the spec's shared-client path needs a decision.
- [ ] **Step 2: Local end to end.** Run the bridge locally against the real Firestore project with `PUBLIC_BASE_URL` set to a tunnel URL; run the app build with the new callback fields; from the extension, log in once (phone path, `wantCredentials=1`), confirm `accounts/<USERNAME>` has `vault`, `session`, `fcmToken` (sealed) and nothing readable. Turn the phone's network off, wait for the cache to expire (temporarily `SESSION_CACHE_SECS=60`), log in again from the extension: served by a vault login with the Gmail OTP.
- [ ] **Step 3: Local MCP.** Run `vtop-mcp` against the local bridge, `claude mcp add` it, call `get_attendance` for the current semester; then expire the session through `/v1/session/expire` and call again.
- [ ] **Step 4: Ask the user**, then create the Firestore TTL policy on `bridge_requests.expiresAt`, set Railway variables (`PUBLIC_BASE_URL`, `FIREBASE_CREDENTIALS_JSON`, fresh `VAULT_KEY` from `openssl rand -base64 32`, `BRIDGE_SERVICE_KEY`, `MCP_KEYS`, `BRIDGE_URL`), `railway config apply`, deploy both services, generate Railway domains.
- [ ] **Step 5:** Repeat Steps 2–3 against the deployed services, including one request after the services have slept.
- [ ] **Step 6: Ask the user**, then move `be-va.kryxen.dev` to `vtop-bridge` (the app reads the callback URL from each FCM message, so it needs no rebuild beyond Task 10), finish Task 12's revert, and leave the Go service running but unused until the user retires it.
