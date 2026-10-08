# Multi-user Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn vtop-bridge into a multi-user service where every client (extension, vtop-mcp) uses one personal access key and one session API, accounts are verified registration numbers, and users manage everything from the app.

**Architecture:** vtop-core gains `verify_session`. The bridge keys accounts by verified registration number, stores hashed access keys, and serves a bearer-key session API whose slow path (vault login, then phone over FCM) runs in the background on a pollable request. Account management endpoints take a live VTOP session as proof. vtop-mcp forwards the caller's key. The extension and the app switch to the new API.

**Tech Stack:** as the first plan (Rust axum/reqwest/rmcp, Firestore REST, Flutter + forui, Svelte extension).

**Spec:** `docs/superpowers/specs/2026-10-08-multi-user-design.md`

## Global Constraints

- Account id = upper-cased registration number from `verify_session`; never a client-claimed username.
- Access key format `vtm_` + 64 hex chars; stored only as SHA-256 hex; `id` = first 8 hex of the hash; max 10 keys per account.
- Bearer session API: 60 requests/min per account. Account management API: 10 requests/min per client IP.
- Session cache 30 min; a session request lives `REQUEST_TIMEOUT_SECS` (120 s); vtop-mcp waits at most 75 s on a pending request.
- Vault stored only with consent and a Gmail refresh token; vault wiped only on invalid credentials, `invalid_grant`, or a vault login whose registration number differs from the account.
- Never log keys, cookies, passwords, tokens; types holding them have no `Debug`.
- No silent "am I linked?" call from the app: it sends a session to the bridge only after the user linked on this install.
- Legacy FCM-token `/cookie` endpoints keep their Go contract (always ask the phone, record nothing) until removed after the extension update.
- No deploy, variable change or domain move without the user's OK at that moment.

## Review Focus

1. A key or session for account A used against B's request id, key id or account → 404/403, nothing changed. (Tasks 4, 5, 6)
2. A phone callback whose cookies verify to a different registration number than the request's account → rejected, request stays pending, account untouched. (Task 6)
3. VTOP unreachable while verifying a callback → 503 so the app retries; request not completed. (Task 6)
4. A revoked key keeps working in vtop-mcp only until its 5-minute cache expires; after that 401. (Task 7)
5. FCM `UNREGISTERED` during the phone path → token cleared, request error `phone_unreachable`, keys unaffected. (Task 5)

## File Structure

```
rust/vtop-core/src/client/auth.rs        + verify_session
rust/vtop-core/src/client/tests.rs       + tests
rust/vtop-bridge/src/verify.rs           SessionVerifier, CoreVerifier (new)
rust/vtop-bridge/src/keys.rs             key generation, hashing, Keys service (new)
rust/vtop-bridge/src/store.rs            + KeyDoc, key/account queries; RequestDoc.account
rust/vtop-bridge/src/firestore.rs        + access_keys, runQuery, deletes
rust/vtop-bridge/src/accounts.rs         id = regno; Vault.username
rust/vtop-bridge/src/broker.rs           start/fulfil (background), regno check, phone_unreachable
rust/vtop-bridge/src/session_api.rs      bearer API (rewritten)
rust/vtop-bridge/src/account_api.rs      session-proof API (new)
rust/vtop-bridge/src/requests.rs         legacy /cookie simplified; callback verifies
rust/vtop-bridge/src/config.rs           - BRIDGE_SERVICE_KEY, - PHONE_WAIT_SECS, + PUBLIC_MCP_URL
rust/vtop-bridge/src/fakes.rs            + FakeVerifier
rust/vtop-mcp/src/{config,bridge,lib,tools}.rs  key pass-through, whoami cache, polling
extension/src/{background.ts,bridge.ts,App.svelte,bridge.test.ts}
lib/core/utils/vtop_bridge_account_service.dart (new), lib/features/settings/... (new page)
lib/core/utils/fcm_cookie_bridge_service.dart, test/...
```

---

### Task 1: `VtopClient::verify_session`

**Files:** `rust/vtop-core/src/client/auth.rs`, `rust/vtop-core/src/client/tests.rs`

**Interfaces:** Produces `pub async fn verify_session(&self) -> VtopResult<String>` — calls `validate_authenticated_session`; `Ok(true)` → `registration_number()`; `Ok(false)` → `Err(SessionExpired)`.

- [ ] Step 1: tests in `tests.rs` using its mock server: `verify_session_returns_registration_number` (content page with CSRF + regno), `verify_session_on_login_redirect_is_expired`.
- [ ] Step 2: `cd rust && cargo test -p vtop-core verify_session` → FAIL.
- [ ] Step 3: implement. Step 4: PASS. Step 5: commit `feat(vtop-core): verify_session`.

### Task 2: Verifier, keys and store additions

**Files:** create `verify.rs`, `keys.rs`; modify `store.rs`, `firestore.rs`, `fakes.rs`, `lib.rs`.

**Interfaces:**
- `verify.rs`: `pub enum VerifyError { Expired, Unavailable(String) }`; `#[async_trait] pub trait SessionVerifier { async fn registration_number(&self, session: &SessionState) -> Result<String, VerifyError>; }`; `CoreVerifier { config: VtopConfig }` (builds `with_session`, calls `verify_session`, maps `SessionExpired`/`AuthenticationFailed` → Expired, others → Unavailable). `fakes::FakeVerifier` maps cookie header `"…REG=<regno>…"` → regno, `"expired"` → Expired, `"down"` → Unavailable.
- `store.rs`: `pub struct KeyDoc { hash, registration_number, label, created_at, last_used_at }`; `RequestDoc` gains `account: Option<String>`; trait gains `put_key`, `get_key(hash)`, `list_keys(regno)`, `delete_key(hash)`, `delete_account(regno)` (account doc + its keys + its requests). `ACCESS_KEYS = "access_keys"`.
- `firestore.rs`: key encode/decode; `list_keys` and request lookup via `POST …/documents:runQuery` with a `fieldFilter` on `registrationNumber`; `delete_account` lists and deletes.
- `keys.rs`: `pub fn new_key() -> String`, `pub fn hash(key) -> String`, `pub fn key_id(hash) -> &str`; `pub struct Keys { store }` with `create(regno, label, now) -> Result<(String /*id*/, String /*key*/), KeyError>` (`KeyError::Limit` at 10, `Store`), `resolve(key, now) -> Result<Option<String>, StoreError>` (updates `last_used_at` when older than 60 s), `list(regno)`, `revoke(regno, id) -> Result<bool, _>` (false when the id is not this account's).

- [ ] Step 1 tests: `new_key_format`, `hash_is_not_key`, `create_respects_limit`, `resolve_unknown_none`, `resolve_touches_last_used_once_per_minute`, `revoke_other_accounts_key_is_false` (Review Focus 1), `delete_account_removes_keys_and_requests`; firestore wiremock: `list_keys_runs_query`, `key_round_trip`.
- [ ] Steps 2–5: FAIL → implement → PASS → commit `feat(vtop-bridge): verifier, access keys, store queries`.

### Task 3: Accounts keyed by registration number

**Files:** `accounts.rs`, `broker.rs` (call sites only), existing tests.

**Interfaces:** `Vault { username, password, gmail }`; every `Accounts` method takes a registration number; `record_callback` → `record_phone(regno, fcm_token: Option<&str>, session, vault: Option<Vault>, now)`; `set_fcm_token(regno, token)`, `clear_fcm_token(regno)`; `fcm_index` writes removed.

- [ ] Steps: adapt tests (regno ids, vault username) → FAIL → implement → PASS → commit `refactor(vtop-bridge): accounts keyed by registration number`.

### Task 4: Broker: background fulfilment

**Files:** `broker.rs`.

**Interfaces:**
- `pub enum Started { Ready(SessionState), Pending(RequestDoc) }`
- `Broker::start(regno) -> Result<Started, BrokerError>`: cache hit → Ready; else create a pending request (`account = regno`) and spawn `fulfil`.
- `fulfil(regno, doc)`: vault login (username from vault; resulting `registration_number` must equal `regno`, else wipe vault) → complete the request with the session and cache it; on no vault / wiped / transient failure → send FCM to the account's token (`wantCredentials` per consent: `"1"` only when no vault) ; no token → complete with error `phone_unreachable`; FCM `TokenInvalid` → clear token, error `phone_unreachable`.
- `Broker::poll(regno, id) -> Result<RequestView, BrokerError>` where `RequestView { Pending, Ready(SessionState), Error(String), Expired }`; a request of another account → `NotFound` (Review Focus 1).
- `Broker::expire(regno)`.
- Remove `session`, `session_without_phone`, `wait_for_phone`, `PHONE_WAIT_SECS`.

- [ ] Step 1 tests: `cache_hit_is_ready`, `vault_login_completes_request_and_caches`, `vault_login_with_other_regno_wipes_vault_and_asks_phone`, `invalid_credentials_wipes_and_asks_phone_with_want_credentials`, `transient_login_error_keeps_vault_and_asks_phone_without_credentials`, `no_fcm_token_is_phone_unreachable`, `unregistered_token_is_cleared_and_phone_unreachable` (Review Focus 5), `poll_other_accounts_request_is_not_found`, `poll_expired_request`, `concurrent_starts_log_in_once`.
- [ ] Steps 2–5 → commit `feat(vtop-bridge): background session fulfilment`.

### Task 5: Bearer session API

**Files:** rewrite `session_api.rs`; `lib.rs`; `tests/session_api.rs`.

**Interfaces:** layer `require_key` (Bearer → `Keys::resolve` → `Account(regno)` extension; 401 `key_unknown`; per-account 60/min → 429). Routes: `GET /v1/whoami`, `POST /v1/session`, `GET /v1/session/requests/{id}`, `POST /v1/session/expire`. Ready/poll responses carry `session` and `cookies` (`cookies::from_header`). Remove `X-Service-Key`, `BRIDGE_SERVICE_KEY`.

- [ ] Tests: `no_or_unknown_key_401`, `whoami`, `ready_from_cache`, `pending_then_ready_after_callback`, `poll_other_accounts_request_404`, `phone_unreachable_error_reported`, `expire_then_pending`, `rate_limited_per_account`.
- [ ] → commit `feat(vtop-bridge): bearer-key session API`.

### Task 6: Account API and verified callback

**Files:** create `account_api.rs`, modify `requests.rs`, `config.rs` (+ `PUBLIC_MCP_URL`, default `https://vtop-mcp-production.up.railway.app/mcp`), tests `tests/account_api.rs`, `tests/cookie_api.rs`.

**Interfaces:** session-proof extractor (`{"session": {"cookies"}}` → `CoreVerifier` → regno; Expired → 401 `session_invalid`; Unavailable → 503 `vtop_unreachable`); IP limiter 10/min. Endpoints exactly as the spec table (`/v1/link`, `/v1/keys`, `/v1/account`, `/v1/keys/revoke`, `/v1/account/fcm-token`, `/v1/account/forget-credentials`, `/v1/account/delete`). `/v1/keys` response adds `mcpUrl`.
Callback: for a request with `account`, verify cookies first: mismatch → 403 `account_mismatch` (request stays pending, Review Focus 2); Unavailable → 503 (Review Focus 3); match → complete, cache session, store vault when `credentials.gmail` present (username from `body.username`). Legacy requests (`account == None`): complete only, record nothing. Legacy `POST /cookie` always asks the phone.

- [ ] Tests: `link_creates_account_and_caches_session`, `link_with_gmail_credentials_stores_vault`, `key_create_list_revoke`, `key_limit_409`, `revoke_other_accounts_key_404`, `delete_removes_everything`, `forget_credentials`, `fcm_token_replaced`, `expired_session_401`, `vtop_down_503`, `callback_mismatch_403_keeps_pending`, `callback_vtop_down_503`, `callback_match_completes_and_caches`, `legacy_cookie_always_asks_phone`.
- [ ] → commit `feat(vtop-bridge): account API and verified callbacks`.

### Task 7: vtop-mcp key pass-through

**Files:** `vtop-mcp/src/{config,bridge,lib,tools}.rs`, tests.

**Interfaces:** `Config { bridge_url, listen }`; `SessionSource::{whoami(key), session(key), expire(key)}`; `BridgeClient::session` posts `/v1/session`, polls `/v1/session/requests/{id}` every `pollAfterMs` up to 75 s; error text from `{"status":"error","error"}`. Auth layer: `whoami` cached 5 min by key hash; 401 otherwise; `Caller(key)`.

- [ ] Tests: `unknown_key_401`, `whoami_cached`, `revoked_key_401_after_cache_expiry` (inject clock/ttl; Review Focus 4), `pending_session_polled_until_ready`, `pending_error_surfaces`, existing retry tests adapted.
- [ ] → commit `feat(vtop-mcp): forward the caller's access key`.

### Task 8: Extension uses the session API

**Files:** `extension/src/bridge.ts`, `background.ts`, `App.svelte`, `bridge.test.ts`.

**Interfaces:** `bridge.ts` gains `isAccessKey(s)` (`/^vtm_[0-9a-f]{64}$/`) and `interpretSessionResponse(status, json)` → `{kind: "ready", cookies} | {kind: "pending", requestId, pollAfterMs} | {kind: "error", message}`; `friendlyLoginError` gains `key_unknown`, `phone_unreachable`, `phone_timeout`. `background.ts` login: `POST /v1/session` with Bearer, poll `/v1/session/requests/{id}`. After installing cookies, if VTOP bounces to its login page, call `POST /v1/session/expire` and log in once more (one retry). Popup field label "Access key", helper text "Create one in VITAP Mate → Connected apps"; non-key values show that hint.

- [ ] Tests (vitest) for the helpers → FAIL → implement (OpenCode allowed for `background.ts`/`App.svelte`) → `pnpm test && pnpm build` → commit `feat(extension): log in with an access key`.

### Task 9: App: bridge account client, linked flag, callback consent

**Files:** create `lib/core/utils/vtop_bridge_account_service.dart`; modify `fcm_cookie_bridge_service.dart`; tests.

**Interfaces:** `VtopBridgeAccountService(http.Client, baseUrl)` with `link`, `createKey`, `account`, `revokeKey`, `updateFcmToken`, `forgetCredentials`, `deleteAccount` (each takes the cookie header); `bridgeLinkedProvider` (prefs bool) and `bridgeServerSignInProvider` (consent bool, prefs); FCM `onTokenRefresh` → `updateFcmToken` only when linked; callback sends `credentials` only when consent is on.

- [ ] Tests: request bodies and error mapping with `MockClient`; consent gating of the callback payload; token refresh only when linked → commit `feat(app): bridge account client and consent`.

### Task 10: App: "Connected apps" page

- [ ] Step 1: propose 2–3 layouts with ASCII previews (AskUserQuestion), wait for the pick (memory: propose UI options before building).
- [ ] Step 2: build with forui per the pick; route from Settings; hot reload on the emulator; screenshot each state (not linked, linked with keys, key just created, phone unreachable).
- [ ] Step 3: `flutter analyze` and widget smoke test → commit `feat(app): connected apps page`.

### Task 11: Deploy and live verification (each step needs the user's OK)

- [ ] Remove `BRIDGE_SERVICE_KEY`/`MCP_KEYS` variables, update `.railway/railway.ts` env lists, `railway up` both services.
- [ ] Live: link from the emulator, create a key, extension login with it, Claude Code tool call with the cache empty and no saved credentials (phone answers), revoke, 401 after cache expiry.
- [ ] Update READMEs.
