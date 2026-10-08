# vtop-bridge and vtop-mcp: session broker, credential vault and MCP server

Date: 2026-10-08
Status: draft, awaiting review

## Goal

Replace the Go FCM cookie bridge (`../fmc-token-go`) with Rust services in
this repository that:

1. keep the bridge contract the Chrome extension and the app already use;
2. can produce a VTOP session while the phone is offline, by logging in with
   saved credentials and reading the OTP from Gmail;
3. expose VTOP data to MCP clients (Claude Code) through `vtop-core`.

Only the bridge ever sees a password or a Gmail token. The extension and the
MCP server receive session cookies and nothing else.

## Services (Railway project `vitap-mate`, serverless on for all three)

| Service | Crate | Role | Sees passwords |
| --- | --- | --- | --- |
| `vtop-server` | `rust/vtop-server` (exists, unchanged) | Stateless VTOP data API for the app | Only for the app's own `/v1/auth/*` calls |
| `vtop-bridge` | `rust/vtop-bridge` (new) | FCM cookie bridge for the extension, credential vault, session broker for the MCP service | Yes, the only store |
| `vtop-mcp` | `rust/vtop-mcp` (new) | MCP server; gets a session from the bridge, calls `vtop-core` | No |

```
extension ──/cookie──▶ vtop-bridge ◀──/v1/session── vtop-mcp ◀── Claude Code
                         │   ▲                         │
                    FCM  ▼   │ /cookie/callback        ▼
                        phone app                 vtop-core ──▶ VTOP
```

## Decisions

| Topic | Decision |
| --- | --- |
| Users | Single user now. Data is keyed per account so other users can be added later without a migration. |
| Framework | axum, reqwest, tracing; same conventions as `vtop-server`. |
| State | Firestore (REST API) in the bridge. The MCP service keeps no state. |
| Google auth | Service account from `FIREBASE_CREDENTIALS_JSON` (bridge only), scope `cloud-platform`, for Firestore and FCM HTTP v1. |
| Credentials at rest | AES-256-GCM, key from `VAULT_KEY` (bridge only). |
| Bridge ↔ MCP | `POST /v1/session` on the bridge, authenticated with `BRIDGE_SERVICE_KEY` (shared secret). URL from `BRIDGE_URL`, so the private network or the public domain can be used. |
| MCP transport | Streamable HTTP at `/mcp` via `rmcp`, stateless mode. |
| MCP auth | `Authorization: Bearer <key>`; `MCP_KEYS=key=username[,key=username]` in the MCP service. |
| MCP scope | Read-only tools first. Write actions (hostel outing etc.) are a later change. |
| Firestore collections | New names (`bridge_requests`, `accounts`, `fcm_index`) so the Go service can run alongside. |
| Rollout | New services on Railway domains, tested, then `be-va.kryxen.dev` moves to `vtop-bridge` and the Go service is retired. Every deploy is confirmed first. |

## When credentials are saved

Every VTOP login needs an emailed OTP. A password without Gmail access cannot
log in while the phone is offline, so:

- Credentials are saved **only** when the phone sends a Gmail refresh token
  with them. Without one, nothing about the login is stored.
- The session (cookies) is cached for 30 minutes regardless, so repeated calls
  do not wake the phone. A session stays valid without a new OTP.

## Gmail access

The phone sends the refresh token and the OAuth client that issued it:

| App setup | Sent | Bridge refresh |
| --- | --- | --- |
| Shared built-in (AppAuth, `GOOGLE_OAUTH_CLIENT_ID`) | `refresh_token`, `client_id` | `POST https://oauth2.googleapis.com/token`, no secret |
| Personal BYOK | `refresh_token`, `client_id`, `client_secret` | Same, with the secret |

Plus `delete_after_reading`. The bridge mints a one-hour access token only
when it has to log in and passes it to
`VtopClient::login_with_gmail_otp(Some(&GmailAccess), wait)`. The phone keeps
using the same refresh token.

Refresh without a secret for the shared client is unverified; the live test
must prove it before it is relied on.

A refresh token from a consent screen in Google's "Testing" mode expires after
seven days. Publishing the consent screen removes the limit.

## Data model (Firestore, bridge only)

| Collection | Document id | Fields |
| --- | --- | --- |
| `bridge_requests` | UUID | `status`, `responseToken`, `cookies` (JSON string), `error`, `fcmHash`, `wantCredentials`, `createdAt`, `expiresAt` (TTL). |
| `accounts` | VTOP username, upper-cased | `vault` (sealed), `session` (sealed `SessionState`), `sessionExpiresAt`, `fcmToken` (sealed), `updatedAt`. |
| `fcm_index` | SHA-256 hex of the FCM token | `username`. |

Sealed fields are `base64(nonce || ciphertext)` with the document id as
associated data, so a blob cannot be moved to another account.

The vault holds `password`, `gmail.refresh_token`, `gmail.client_id`,
optional `gmail.client_secret`, `gmail.delete_after_reading`.

## Getting a session (bridge)

`Broker::session(username)`:

1. **Cached session** with `sessionExpiresAt` in the future: return it.
2. **Saved credentials**: refresh the Gmail access token, log in with
   `vtop-core`, answer the OTP from Gmail, cache the new session for 30
   minutes, return it.
   - `VtopError::InvalidCredentials`, or Google answers `invalid_grant`:
     delete the vault, continue at step 3.
   - Anything else (network, VTOP 5xx, CAPTCHA, timeout, OTP email not found):
     return the error and keep the vault.
3. **Ask the phone**: send an FCM request with `wantCredentials=1` when no
   vault exists, wait up to `PHONE_WAIT_SECS` for the callback, return the
   session it stored.

`Broker::expire(username)` deletes the cached session (the MCP service calls
it when VTOP reports the session expired, then asks again once).

Concurrent calls for one username share one login (per-username async lock).
Railway runs one instance, so there is no cross-instance lock.

## Bridge HTTP API

Unchanged from the Go service (paths, JSON fields, status codes, CORS,
10 requests per minute per client IP on `POST /cookie`):

- `POST /cookie` `{fcmToken}` (also `fmcToken`)
- `GET /cookie/status/:requestId`
- `DELETE /cookie/status/:requestId`
- `POST /cookie/callback`
- `GET /healthz`

Changes:

- `POST /cookie` looks up the username through `fcm_index` and tries steps
  1–2 first. If they produce a session, the request is created already
  `success` with cookie-editor cookies, so the extension's polling is
  unchanged. Otherwise the FCM request is sent as today.
- FCM data gains `wantCredentials` (`"1"` or `"0"`).
- The callback body gains optional fields:

  ```json
  { "requestId": "...", "responseToken": "...", "cookies": [...],
    "username": "22BCE0000", "fcmToken": "...",
    "credentials": { "password": "...",
      "gmail": { "refresh_token": "...", "client_id": "...",
                 "client_secret": "...", "delete_after_reading": true } } }
  ```

  With `username`, the callback also caches the session for that account and
  updates `fcmToken` and `fcm_index`. `credentials` is stored only when
  `gmail` is present. The response token proves the callback came from the
  phone that received the FCM message.

New, for the MCP service (header `X-Service-Key: <BRIDGE_SERVICE_KEY>`):

| Endpoint | Body | Response |
| --- | --- | --- |
| `POST /v1/session` | `{"username"}` | `200 {"session": SessionState}` |
| `POST /v1/session/expire` | `{"username"}` | `204` |

Errors: `401 unauthorized`, `404 account_unknown` (the phone never called
back for this username, so there is no FCM token to ask), `504 phone_timeout`,
`502 login_failed` with the VTOP error text.

## MCP service

`/mcp`, streamable HTTP, stateless (no in-memory MCP sessions, so sleeping is
harmless). The bearer key maps to a username. Each tool:

1. gets `SessionState` from the bridge;
2. builds `VtopClient::builder().with_session(&state)` and calls `vtop-core`;
3. on `VtopError::SessionExpired`, calls `/v1/session/expire`, gets a new
   session and retries once.

Results are the `vtop-core` types serialised as JSON (same shapes as
`vtop-server`).

Tools: `get_semesters`, `get_attendance`, `get_full_attendance`,
`get_timetable`, `get_marks`, `get_exam_schedule`, `get_grades`,
`get_grade_details`, `get_grade_history`, `get_biometric`.

## App change

`lib/core/utils/fcm_cookie_bridge_service.dart`: always add `username` and
`fcmToken` to the callback. When the FCM data has `wantCredentials=1` and
the Gmail OAuth session is ready (`GoogleEmailOtpAuthService.isReady()`),
add `credentials`. Without a Gmail session, send no password.

## Extension change

`extension/src/background.ts` and `extension/public/manifest.json`: point
`API_BASE_URL` at the bridge's Railway domain for testing, back to
`be-va.kryxen.dev` after the swap. No behaviour change.

## Configuration

`vtop-bridge`:

| Variable | Required | Purpose |
| --- | --- | --- |
| `PUBLIC_BASE_URL` | yes | HTTPS origin for the FCM callback URL |
| `FIREBASE_CREDENTIALS_JSON` | yes | Service account JSON |
| `VAULT_KEY` | yes | 32 bytes, base64 |
| `BRIDGE_SERVICE_KEY` | yes | Shared secret for `/v1/session`, at least 32 characters |
| `REQUEST_TIMEOUT_SECS` | no, `120` | Cookie request lifetime (30–600) |
| `SESSION_CACHE_SECS` | no, `1800` | Cached session lifetime |
| `PHONE_WAIT_SECS` | no, `60` | How long `/v1/session` waits for the phone |
| `GMAIL_WAIT_SECS` | no, `45` | How long a login waits for the OTP email |
| `ALLOWED_ORIGINS` | no, `*` | CORS |
| `PORT` | Railway | Listen port |

`vtop-mcp`:

| Variable | Required | Purpose |
| --- | --- | --- |
| `BRIDGE_URL` | yes | Bridge origin |
| `BRIDGE_SERVICE_KEY` | yes | Same value as the bridge |
| `MCP_KEYS` | yes | `key=username`, comma-separated |
| `PORT` | Railway | Listen port |

## Errors and logging

Bridge API errors keep the Go shape `{"code", "error"}`. Passwords, tokens,
cookies, FCM tokens and vault contents are never logged; logs carry method,
route, status and latency.

## Testing

- Port `fmc-token-go/server_test.go` cases against in-memory `Store` and fake
  `Messenger`.
- Unit tests for every branch of the broker (cache hit, cache expired, vault
  login, invalid credentials wipe, `invalid_grant` wipe, transient error keeps
  vault, phone fallback, phone timeout).
- Vault seal/open round trip; wrong key and wrong document id rejected.
- MCP: key → username mapping, session-expired retry, against a fake bridge.
- Live check before any deploy: Gmail refresh with the shared client, then a
  full offline login, using the emulator's session and Gmail setup.

## Out of scope

- MCP write actions.
- OAuth for claude.ai connectors.
- Multi-user onboarding (consent screen, per-user key issuance, delete my data).
- Retiring the Go service (after the swap is confirmed working).
