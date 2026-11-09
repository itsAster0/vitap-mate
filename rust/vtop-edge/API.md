# vtop-edge API

The Worker at `https://vmate.aster0.dev`. JSON in and out unless noted.
Errors are `{ "code": "...", "error": "human text" }` with the HTTP status.
Times are unix seconds unless a field says RFC 3339.

## Auth

| Kind | How |
|---|---|
| **signed** | A phone: `x-vm-device`, `x-vm-time`, `x-vm-sig` (below). |
| **key** | An access key (extension, MCP): `Authorization: Bearer vtm_…`. |
| **internal** | The engine or the UDP gateway: `Authorization: Bearer <INTERNAL_TOKEN>`. |
| **none** | Public. |

### Signed requests (vm1)

Each install keeps an Ed25519 seed (32 bytes) and an X25519 key pair. Both
public keys are base64url, 32 bytes. Every phone request but `register`
carries:

- `x-vm-device: <deviceId>` (32 lowercase hex)
- `x-vm-time: <unix seconds>`, accepted within ±120 s of the edge's clock
- `x-vm-sig: base64url(Ed25519(seed, msg))`, 64 bytes, where

```
msg = "vm1\n" + METHOD + "\n" + pathWithQuery + "\n" + time + "\n" + hex(sha256(body))
```

- `METHOD` upper case; `pathWithQuery` is the URL's path plus `?query`
  exactly as sent (`/v1/events?after=3`); `time` is the `x-vm-time` string.
- `hex` is lowercase. The body is the bytes on the wire (after any
  `x-vtop-encoding: deflate-raw` compression). An empty body hashes to
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- The edge sends base64url without padding and accepts it with or without.
- Same scheme as `vtop_shared::device_sig` (test vector in
  `src/auth.test.ts`).

Failures, before the route runs:

| Status | `code` | When |
|---|---|---|
| 401 | `bad_signature` | headers missing or malformed, or the signature does not verify |
| 401 | `stale_time` | `x-vm-time` off by more than 120 s; the body adds `serverTime` |
| 404 | `unknown_device` | no install with that id: register again |

A body may be sent deflate-raw compressed with `x-vtop-encoding:
deflate-raw` (up to 1 MiB inflated), on every JSON route.

## Health

| Route | Auth | Response |
|---|---|---|
| `GET /healthz` | none | `{ service: "vtop-edge", status: "ok", version, colo }` |

## Devices

### `POST /v1/devices/register` — signed by the key it registers

`x-vm-time` and `x-vm-sig` as above, made with the new seed; `x-vm-device`
is not needed.

```json
{ "signKey": "<Ed25519 public, b64url>", "sealKey": "<X25519 public, b64url>",
  "label": "Pixel 8", "platform": "android", "fcmToken": "…optional…" }
```

→ `201 { "deviceId" }` the first time, `200 { "deviceId" }` after. The id is
derived from `signKey` (first 16 bytes of its SHA-256, hex), so registering
the same key again gives the same id and updates `sealKey`, `label`,
`platform` and, when sent, `fcmToken`. `400 bad_request` for a bad key,
label or platform (`[a-z0-9_-]{1,20}`).

### `POST /v1/devices/update` — signed

`{ "fcmToken"?, "label"?, "sealKey"? }`, at least one → `204`.

### `POST /v1/devices/info` — signed

`{}` → `{ deviceId, label, platform, pushToken: bool, createdAt, events,
latest }`: `events` is how many are in the log (kept 24 h), `latest` the
last sequence number. Keys and the FCM token are never sent back.

### `POST /v1/devices/delete` — signed

`{ "registrationNumber"?, "mailbox"? }` → `204`. Deletes the install's
object: its keys, FCM token and event log; open sockets close. With
`mailbox` (a mailbox id) it first leaves that address, which is deleted
when it was the last device on it; with `registrationNumber` it is taken
off that account's phones (its access keys stay, but get no phone to ask).
The same key can register again later and gets the same id, afresh.

### `GET /v1/events?after=<seq>` — signed

The install's event log (kept 24 h). Three forms:

- plain: `200 { latest, events: [Event] }`, at most 100 after `after`;
  ask again from the last `seq`.
- `Upgrade: websocket`: frames `{ type: "hello", latest }` then
  `{ type: "event", event }`; send `ping`, get `pong`.
- `Accept: text/event-stream`: the same frames as server-sent events.

`Event = { seq, at, kind, ... }`:

| `kind` | Fields |
|---|---|
| `mail_otp` | `mailbox`, `sealed` (box sealed to the install's `sealKey`, see `mail_seal`) |
| `vtop_cookie_request` | `requestId`, `callbackUrl`, `wantCredentials` (`"1"`/`"0"`) |
| `credentials_rejected` | — |

Each event also goes out by FCM as a data message with the same fields
(values as strings, objects JSON-encoded).

## UDP

| Route | Auth | Request | Response |
|---|---|---|---|
| `GET /v1/udp` | signed | — | `{ host, ports: [int], keys: [{ id, publicKey, from, until }] }`; `503 udp_unavailable` before the gateway reports |
| `POST /internal/udp-keys` | internal | the same shape, from the gateway | `204`; `400` if malformed |

`publicKey` is X25519, base64url without padding. The edge keeps only the
latest report; the gateway should post at start, after each rotation and
every few hours. 1–16 ports, 1–8 keys.

## Mail OTP (forwarding addresses `otp+<id>@vmate.aster0.dev`)

All `POST`, signed; a phone is its `deviceId`. `id` is the mailbox id (10
characters). `404 mailbox_not_found` when the mailbox does not exist or the
calling device is not on it.

| Route | Request | Response |
|---|---|---|
| `/v1/mail-otp/register` | `{}` | `201 { id, address }` with this device as the only member |
| `/v1/mail-otp/inbox` | `{ id }` | `{ sealed, receivedAt }` or `{ sealed: null }` (boxes live 5 min) |
| `/v1/mail-otp/owner` | `{ id, session: { cookies } }` | `204`; ties the address to the VTOP student; `409 address_exists`, `401 session_invalid`, `503 vtop_unreachable` |
| `/v1/mail-otp/reclaim` | `{ session: { cookies } }` | `200 { id, address }`: this device joins the student's address |
| `/v1/mail-otp/devices` | `{ id }` | `{ devices: [{ deviceId, label, addedAt, seenAt, thisDevice }] }` |
| `/v1/mail-otp/devices/remove` | `{ id, removeDevice }` | `204`; `404 unknown_device` |
| `/v1/mail-otp/delete` | `{ id, everywhere? }` | `204`; leaves the address, or deletes it when last or `everywhere: true` |

Up to 5 devices per address. Mail is sealed per device to its current
`sealKey` and delivered as a `mail_otp` event and to `/inbox`.

## Accounts — phones

All `POST`, signed. Routes that send `session` need the phone's live VTOP
session (`{ cookies: "<Cookie header>" }`); the engine says whose it is.
Errors common to those: `400` without `session.cookies`, `401
session_invalid`, `503 vtop_unreachable`, `403 not_linked` (this device is
not one of the account's phones), `409 not_linked` (no phone linked yet).

| Route | Request | Response |
|---|---|---|
| `/v1/link` | `{ session, label?, semesterId?, credentials? }` | `{ savedCredentials, forwarding }`; adds this device (up to 5 phones) |
| `/v1/account` | `{ session }` | the account view: `registrationNumber, createdAt, settings, semesterId, linked, savedCredentials, vaultSavedAt, vaultGmail, credentialsRejectedAt, forwarding, phoneLinked, phoneCount, thisPhone, phones[], keys[]` (lists only for a linked phone); each key `{ id, label, createdAt, lastUsedAt, phoneId, outings, uses: [{ at, via: "extension"\|"mcp" }] }`, newest use first, at most 10 |
| `/v1/keys` | `{ session, label?, outings?, pairCode? }` | `201 { id, key, mcpUrl }` (the key is shown once); `409 key_limit`; with `pairCode` the extension that showed the code also gets the key (below) |
| `/v1/keys/revoke` | `{ session, id }` | `204`; `404 key_not_found` |
| `/v1/keys/outings` | `{ session, id, outings: bool }` | `204` |
| `/v1/account/settings` | `{ session, settings: { alwaysUseVault, phoneWaitSecs: 10\|20\|45, vaultTtlSecs: 86400\|172800\|null } }` | `204` |
| `/v1/account/semester` | `{ session, semesterId }` | `204` |
| `/v1/account/remove-phone` | `{ session, id }` (phone id from `/v1/account`) | `204`; `404 phone_not_found` |
| `/v1/account/forget-credentials` | `{ session }` | `204` |
| `/v1/account/clear-sign-ins` | `{ session }` | `204`; drops the sign-in history (`signIns` of `/v1/app/status`) |
| `/v1/account/delete` | `{ session }` | `204` |
| `/v1/account/credentials-rejected` | `{ registrationNumber }` | `204`; VTOP rejected the password on the phone |
| `/v1/app/status` | `{ registrationNumber }` | `{ sessionSavedAt, sessionSource, vaultSavedAt, vaultExpiresAt, vaultGmail, credentialsRejectedAt, forwarding, signIns[], phones, keys[] }` |
| `/v1/app/session/clear` | `{ registrationNumber }` | `{ ok: true }` |
| `/v1/app/login` | `{ registrationNumber }` | `{ status: "ok", session: { cookies, csrf_token, registration_number } }`; `409 no_saved_credentials`, `401 invalid_credentials`, `503 server_login_failed`, `504 vtop_timeout` (waits up to 90 s) |
| `/v1/session/answer?account=<regno>` | `{ requestId, cookies: [{ name, value, domain }] , credentials?: { username, password, gmail? } }` or `{ requestId, error }` | `{ ok: true, late? }`; `403 not_asked` unless this device was asked, `403 account_mismatch`, `401 session_invalid`, `404 request_not_found` |

`credentials` (on `/v1/link` and answers) is `{ username, password, gmail?:
{ refreshToken, clientId, clientSecret?, deleteAfterReading? } }`; it is
sealed by the engine and kept only with forwarding or Gmail set up.

The answer route is the `callbackUrl` of a `vtop_cookie_request` event; sign
it with that exact path and query.

## Accounts — access keys (extension, MCP)

CORS: `ALLOWED_ORIGINS` (empty: `*`), methods `GET, POST, DELETE, OPTIONS`,
headers `Content-Type`; `OPTIONS` answers `204`. CORS headers are set on all
account routes. `401 key_unknown` for a bad key. `x-vtop-client: mcp` marks
an MCP use.

| Route | Auth | Response |
|---|---|---|
| `GET /v1/whoami` | key | `{ registrationNumber, keyLabel, keyCreatedAt, keyLastUsedAt, keyOutings, semesterId, settings, phoneLinked, savedCredentials, vaultSavedAt, credentialsRejectedAt }` |
| `POST /v1/session` | key | `200 { status: "ready", session: { cookies }, cookies: [Cookie-Editor], source }` from the cache, or `202 { status: "pending", stage, serverDueAt, requestId, pollAfterMs, expiresAt }`; `429 rate_limited` |
| `GET /v1/session/requests/{id}` | key | `202` pending, `200` ready or `{ status: "error", code, error }`, `410 request_expired` |
| `POST /v1/session/expire` | key | `204`; drops the cached session |
| `POST /v1/pair` | none | `{ code }` → `200 { status: "ready", key }` once the app made a key with this `pairCode`, else `202 { status: "pending", pollAfterMs }`; `400` for a malformed code |

**Pairing.** The extension makes a 10-character Crockford base32 code
(`XXXXX-XXXXX`; case, dashes and spaces ignored, `O` read as `0`, `I`/`L`
as `1`), shows it, and polls `POST /v1/pair`. The app sends it as
`pairCode` on `/v1/keys`; the edge holds the new key under the code's
SHA-256 for 5 minutes and hands it out once.

## VTOP for phones

| Route | Auth | Notes |
|---|---|---|
| `/vtop/*` | none | compression proxy to `https://vtop.vitap.ac.in/vtop/*`, any method; cookies and redirects pass through |
| `POST /v1/vtop/fetch` | signed | body passed to the engine's `/engine/fetch` as is |
| `POST /v1/vtop/login` | signed | → `/engine/vtop-login` |
| `POST /v1/vtop/login/otp` | signed | → `/engine/vtop-login/otp` |
| `POST /v1/vtop/login/resend` | signed | → `/engine/vtop-login/resend` |

Bodies up to 64 KiB (`413 too_large`). Answers are JSON (Brotli to the
phone via Cloudflare) or `application/vnd.vmate.page+br` passed byte for
byte. `504 vtop_timeout` after 32 s, `503 engine_unavailable`.

## MCP

`/mcp`, `/mcp/*`: passed to `ENGINE_URL` unchanged (the client's own
`Authorization: Bearer vtm_…` included), with `x-forwarded-for` set to the
client's address.

## Internal (engine and gateway)

All need `Authorization: Bearer <INTERNAL_TOKEN>`, else `401 unauthorized`.

| Route | Caller | Request | Response |
|---|---|---|---|
| `POST /internal/accounts/login-done` | engine | `{ account, requestId, outcome }` (`LoginDone` in `src/shared/engine-api.ts`) | `204` |
| `POST /internal/mail-otp/await` | engine | `{ ownerHash, publicKey, since, waitSecs }` | `{ sealed }` or `{ sealed: null }`; `404 mailbox_not_found` |
| `POST /internal/udp-keys` | gateway | see UDP | `204` |

## Edge → engine

Every call carries `Authorization: Bearer <INTERNAL_TOKEN>`:
`POST /engine/session`, `/engine/seal`, `/engine/login` (answers `202`, then
reports to `/internal/accounts/login-done`), `/internal/mail-otp/owner`
(`{ cookies }` → `{ ownerHash }`, `401` if VTOP refuses), and the
`/engine/fetch`, `/engine/vtop-login*` routes above. `/mcp` keeps the
client's header instead.

## Configuration

Vars: `MAIL_DOMAIN`, `ENGINE_URL`, `FCM_URL`, `PUBLIC_URL`,
`ALLOWED_ORIGINS`, `FALLBACK_FORWARD`. Secrets: `FIREBASE_CREDENTIALS_JSON`,
`INTERNAL_TOKEN`.
