# Multi-user vtop-bridge with one access path for every client

Date: 2026-10-08
Status: draft, awaiting review
Builds on: `2026-10-08-vtop-bridge-design.md`

## Goal

Any VITAP Mate user can opt in from the app and connect any number of
clients (the Chrome extension, Claude Code through vtop-mcp, later others).
Every client uses the **same credential and the same API**, and no user can
read or overwrite another user's account.

## Decisions

| Topic | Decision |
| --- | --- |
| Identity | An account is its **registration number, verified with VTOP**, never a username a client claims. |
| Credential | One kind: a personal **access key** (`vtm_…`), created in the app, with a label. A key works for every client, so one key can serve both the extension and Claude Code; separate keys only make revoking one client possible. The extension stops using the FCM token as its identity. |
| API | One session API on the bridge, `Authorization: Bearer <access key>`, used by the extension directly and by vtop-mcp on the user's behalf. |
| Getting a session | Cache (30 min) → saved credentials (only if consented) → ask the phone over FCM. For most users the phone is the normal path once the cache expires. |
| vtop-mcp | A thin MCP front end: forwards the caller's access key; holds no keys or shared secret of its own. `MCP_KEYS` and `BRIDGE_SERVICE_KEY` are removed. |
| Storage of keys | SHA-256 hashes in Firestore; a key is shown once. At most 10 keys per account. |
| Credentials | Stored only with explicit consent and a Gmail refresh token (unchanged rule). |
| User control | From the app: list and revoke keys, forget saved credentials, delete all my data. |
| Legacy | The FCM-token `/cookie` endpoints stay until the updated extension ships, then are removed. The single test account keyed by username is dropped; the owner re-links. |

## Verifying who a session belongs to

`vtop-core` gains `VtopClient::verify_session() -> VtopResult<String>`: loads
`/vtop/content` with the session's cookies (the existing
`validate_authenticated_session`) and returns the registration number, or
`SessionExpired`.

The bridge uses it through a trait so tests need no VTOP:

```rust
#[async_trait] pub trait SessionVerifier: Send + Sync {
    async fn registration_number(&self, session: &SessionState) -> Result<String, VerifyError>;
}
pub enum VerifyError { Expired, Unavailable(String) }
```

Rules:

- Every session stored for an account is verified first; its registration
  number must be the account id.
- A vault login's session must carry the account's registration number; a
  mismatch is treated like invalid credentials (vault wiped).
- A phone callback is accepted only if its cookies verify to the account that
  asked.
- The registration number never changes; the VTOP username can. Only the
  vault holds the username. After a username change the vault login fails as
  invalid credentials, the vault is wiped, the phone path runs, and the
  phone's next consented callback stores credentials with the new username.
  Keys and the account are unaffected.

## Where state lives

| Service | State |
| --- | --- |
| `vtop-server` | None. Callers send their session with every request (unchanged). |
| `vtop-mcp` | None that matters: MCP runs stateless, and the only memory is a 5-minute cache of key → account answers, rebuilt after a sleep. |
| `vtop-bridge` | Stateless in memory; all state is in Firestore (accounts, sealed vault and session, hashed keys, pending requests). It is the only service that stores anything, and the only one that ever sees passwords or Gmail tokens. |

## Data model

| Collection | Id | Fields |
| --- | --- | --- |
| `accounts` | registration number | sealed `vault` (`username`, `password`, `gmail`), sealed `session` + `sessionExpiresAt`, sealed `fcmToken`, `updatedAt` |
| `access_keys` | SHA-256 hex of the key | `registrationNumber`, `label`, `createdAt`, `lastUsedAt` |
| `session_requests` | UUID | as `bridge_requests` today, plus `registrationNumber`; TTL on `expiresAt` |

`fcm_index` is no longer needed once the legacy endpoints are removed.

## Bridge API

### The session API (every client)

`Authorization: Bearer <access key>`. Unknown or revoked key → `401
key_unknown`. 60 requests per minute per account.

| Endpoint | Response |
| --- | --- |
| `GET /v1/whoami` | `{registrationNumber, label}` |
| `POST /v1/session` | `200 {status: "ready", session, cookies}` when the cache or a vault login answers within ~3 s; otherwise `202 {status: "pending", requestId, pollAfterMs, expiresAt}` |
| `GET /v1/session/requests/{id}` | `202 {status: "pending"}`, `200 {status: "ready", session, cookies}`, `200 {status: "error", error}`, `410 request_expired`; another account's id → `404` |
| `POST /v1/session/expire` | `204`; drops the cached session (VTOP said it expired) |

`session` is `SessionState` (for vtop-core clients); `cookies` is the
Cookie-Editor list (for the extension). A pending request runs the slow path
(vault login, then the phone) in the background on the same request.

### Account management (the app)

All take a live VTOP `session` (`{"cookies": "..."}`) as proof; the bridge
verifies it and acts on that registration number only. 10 requests per
minute per client IP.

| Endpoint | Body besides `session` | Response |
| --- | --- | --- |
| `POST /v1/link` | `fcmToken`, optional `credentials` `{username, password, gmail}` | `204`; creates or updates the account, caches the session, stores the FCM token, stores the vault when `credentials.gmail` is present |
| `POST /v1/keys` | `label` | `201 {id, key}` (key shown once); `409 key_limit` |
| `POST /v1/account` | – | `{registrationNumber, savedCredentials, phoneLinked, keys: [{id, label, createdAt, lastUsedAt}]}` |
| `POST /v1/keys/revoke` | `id` | `204` |
| `POST /v1/account/fcm-token` | `fcmToken` | `204` |
| `POST /v1/account/forget-credentials` | – | `204` |
| `POST /v1/account/delete` | – | `204`; account, keys and pending requests removed |

`id` is the first 8 hex characters of the key's hash. Errors: `401
session_invalid`, `503 vtop_unreachable`, `429 rate_limited`.

### Phone callback

`POST /cookie/callback` keeps its shape. The bridge verifies the cookies and
completes the request only if they belong to the request's account. With
consent, `credentials` refresh the vault.

## The phone path

With no cached session and no saved credentials, the bridge sends the
account's phone an FCM request and waits up to `PHONE_WAIT_SECS` (60 s):

- The app answers in the background: signs in with its stored credentials
  (OTP from Gmail if the app's Gmail OTP setup is on) and posts the cookies.
- The session is cached for 30 minutes, so a burst of calls from any client
  wakes the phone once.
- Phone offline or app killed: the request ends in `error: phone_timeout`.
- Keys belong to the account, not the phone, so a changed FCM token never
  invalidates them.
- Token refresh while installed: the app keeps a local "linked" flag and,
  while it is set, calls `/v1/account/fcm-token` on `onTokenRefresh` and on
  start when the token differs from the last one sent.
- Reinstall, cleared data or a new phone: the flag is gone and the bridge
  still holds the old token. Cache and vault logins keep working. The next FCM
  send gets `UNREGISTERED`; the bridge clears the token and the request ends
  in `error: phone_unreachable` with "Open VITAP Mate → Connected apps →
  Reconnect this phone". That button sends a live session and the new token
  (`/v1/account/fcm-token`) and sets the flag again. The app never sends a
  session to the bridge without the user having linked (no silent "am I
  linked?" check at start).
- One phone per account: the most recently registered token wins.

## Clients

### Extension

The popup's "token" field becomes "Access key". Login: `POST /v1/session`,
then poll `/v1/session/requests/{id}` when pending, then install `cookies`.
Storage key stays `token`; an old FCM token in it gets a clear "paste an
access key from the app" message.

### vtop-mcp

The bearer key from Claude Code is the user's access key. The auth layer
calls `/v1/whoami` (cached 5 minutes in memory) and answers 401 for unknown
keys. Tools call `/v1/session` with the same key, wait on a pending request
up to 75 s, and call `/v1/session/expire` and retry once when VTOP says the
session expired.

### App

A settings page "Connected apps" (forui). Before building, layout options are
proposed with previews and the user picks one. Content:

- What it does and exactly what is stored.
- Switch: "Let the server sign in for me when my phone is offline" (needs
  Gmail OTP setup; off by default).
- "Create key": label → key shown once, with copy buttons for the key
  (paste into the extension) and the ready-made `claude mcp add …` command;
  one key may serve both.
- "Reconnect this phone" (shown when not linked on this install, or after
  `phone_unreachable`).
- Key list with revoke; "Forget saved credentials"; "Delete my data".

Linking happens automatically on the first key creation.

## Testing

- Bridge, against `MemoryStore`, a fake `SessionVerifier` and existing fakes:
  every endpoint; account isolation (A's key or session can never read,
  poll, revoke or delete B's data; a callback with A's cookies cannot complete
  B's request); vault mismatch wipes the vault; key limit; keys stored only as
  hashes; delete removes keys and requests; phone path for a key-authenticated
  request; FCM token replacement; `UNREGISTERED` clears the token and yields
  `phone_unreachable`; keys still work after the token is replaced; one key
  used by both the session API and whoami.
- vtop-core: `verify_session` against the existing mock-server tests.
- vtop-mcp: whoami cache, unknown key 401, pending request waited out,
  expired-session retry.
- Extension: login flow with a mocked bridge (ready, pending → ready, error,
  401).
- App: payload and switch logic unit tests; the page verified on the emulator
  with screenshots.
- Live: link from the emulator, create two keys, log in from the extension
  with one and call a tool from Claude Code with the other (cache empty, no
  saved credentials → phone answers), revoke one, confirm 401.

## Out of scope

- OAuth for claude.ai connectors.
- Admin endpoints.
- Write tools (hostel outing etc.).

## App secret (added after the final review)

A live VTOP session is not proof of ownership on its own: every key holder
can get one from `/v1/session`. `/v1/link` therefore returns an app secret
(`vta_` + 64 hex, stored as SHA-256 in `accounts.appSecretHash`) that the
app keeps in secure storage. Creating and revoking keys, changing the FCM
token and deleting the account need it (403 `app_secret_invalid`).
`/v1/account` without it reports `thisPhone: false` and lists no keys.
Forgetting credentials needs no secret, since it only removes access.

Linking again with the secret keeps keys and saved credentials. Linking
without it (a reinstall, or someone with a stolen session) revokes every key
and drops saved credentials unless new ones come with the link, so a leaked
key cannot be used to mint keys that outlive its revocation. The app asks
before a link that would revoke keys.

## Sign-in settings (added 2026-10-08)

- Every cached session is checked with VTOP before it is handed out. One VTOP
  rejects, or that belongs to another student, is dropped; if VTOP cannot be
  reached the cache is handed out unchecked. Because of this the cache lives
  until VTOP rejects it (`SESSION_CACHE_SECS` default 7 days, an upper bound).
- Per-account settings, changed from the linked app (app secret required)
  through `POST /v1/account/settings`, shown under Offline sign-in:
  - `alwaysUseVault` (default true): with saved credentials, the server signs
    in straight away and asks the phone only if that fails. When false, the
    phone is asked first and the server signs in after `phoneWaitSecs`
    (10/20/45, default 20) of silence, or at once if the phone cannot be
    reached.
  - `vaultTtlSecs` (1 day, 2 days, or null = until VTOP or Google rejects
    them): counted from the last time the phone sent the credentials; while
    set, every phone request asks for them again.
  - Clear saved credentials = forget-credentials (turns offline sign-in off).
