# vtop-edge API

The Worker at `https://vmate.aster0.dev`. JSON in and out unless noted.
Errors are `{ "code": "...", "error": "human text" }` with the HTTP status;
`code` is snake_case and stable, `error` is for people, and a few add fields
(`stale_time` adds `serverTime`). Every route answers `404 not_found` for no
such route, `405 method_not_allowed` for a known path with another method,
`400 bad_request` for a body that is not the JSON it wants, and `500
internal_error` when the edge fails. Exceptions: `/mcp` passes the upstream
answer on, and OAuth's `/register` and `/token` use RFC 6749's `{ error,
error_description }`. Times are unix seconds unless a field says RFC 3339.

## Auth

| Kind | How |
|---|---|
| **signed** | A phone: `x-vm-device`, `x-vm-time`, `x-vm-sig` (below). |
| **key** | An access key (extension, MCP): `Authorization: Bearer vtm_…`. |
| **internal** | The engine: `Authorization: Bearer <INTERNAL_TOKEN>`. `/internal/udp-keys`, from the UDP gateway, takes `GATEWAY_TOKEN` (or `INTERNAL_TOKEN`). |
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
  `rust/vtop-shared/vectors/device_sig.json`).

Failures, before the route runs:

| Status | `code` | When |
|---|---|---|
| 401 | `bad_signature` | headers missing or malformed, or the signature does not verify |
| 401 | `stale_time` | `x-vm-time` off by more than 120 s; the body adds `serverTime` |
| 404 | `unknown_device` | no install with that id: register again |

A body may be sent deflate-raw compressed with `x-vtop-encoding:
deflate-raw` (up to 1 MiB inflated), on every JSON route.

## Routes

Every route, with its auth kind; the sections below give the details.

<!-- routes:start -->
<!-- Made from src/routes.ts by `bun run docs`; do not edit. -->

| Method | Path | Auth | Summary |
|---|---|---|---|
| GET | `/healthz` | none | Service, version and Cloudflare location. |
| POST | `/v1/devices/register` | signed | Registers an install's keys; returns its device id. |
| POST | `/v1/devices/update` | signed | Changes the install's FCM token, label or seal key. |
| POST | `/v1/devices/info` | signed | The install as the edge keeps it. |
| POST | `/v1/devices/delete` | signed | Deletes the install, optionally leaving its address and account. |
| GET | `/v1/events` | signed | The install's event log: catch-up, WebSocket or server-sent events. |
| GET | `/v1/udp` | signed | The UDP gateway's host, ports and keys. |
| POST | `/v1/app/status` | signed | A linked phone's view of its account. |
| POST | `/v1/app/session/clear` | signed | Drops the account's cached VTOP session. |
| POST | `/v1/app/login` | signed | Has the server sign in with the saved credentials. |
| POST | `/v1/mail-otp/register` | signed | Makes a forwarding address for this install. |
| POST | `/v1/mail-otp/delete` | signed | Leaves the address, or deletes it. |
| POST | `/v1/mail-otp/leave` | signed | Leaves the address; an owned one stays for the student. |
| POST | `/v1/mail-otp/inbox` | signed | The latest sealed OTP and Gmail confirmation. |
| POST | `/v1/mail-otp/owner` | signed | Ties the address to the VTOP student. |
| POST | `/v1/mail-otp/reclaim` | signed | Joins this install to the student's address. |
| POST | `/v1/mail-otp/devices` | signed | The installs on the address. |
| POST | `/v1/mail-otp/devices/remove` | signed | Takes another install off the address. |
| POST | `/v1/vtop/fetch` | signed | Fetches a page on the engine. |
| POST | `/v1/vtop/fetch-batch` | signed | Fetches several pages of a semester in one call. |
| POST | `/v1/vtop/login` | signed | Signs in to VTOP on the engine. |
| POST | `/v1/vtop/login/otp` | signed | Sends the sign-in OTP. |
| POST | `/v1/vtop/login/resend` | signed | Asks VTOP to send the OTP again. |
| GET | `/v1/whoami` | key | The access key's account and settings. |
| POST | `/v1/session` | key | A VTOP session: cached, or a pending request. |
| GET | `/v1/session/requests/{id}` | key | Polls a pending session request. |
| POST | `/v1/session/expire` | key | Drops the cached session. |
| POST | `/v1/pair` | none | The extension polls for the key made for its code. |
| POST | `/v1/key/token` | key | A short JWT for the extension's socket (`extension:*` queries). |
| POST | `/v1/pair/info` | none | Whether a pairing code is a browser's or an MCP client's. |
| POST | `/v1/link` | signed | Links this phone to the student's account. |
| POST | `/v1/keys` | signed | Makes an access key. |
| POST | `/v1/keys/revoke` | signed | Revokes an access key. |
| POST | `/v1/keys/outings` | signed | Lets a key apply for outings, or not. |
| POST | `/v1/keys/move` | signed | Moves keys from unlinked phones to this one. |
| POST | `/v1/account` | signed | The account view. |
| POST | `/v1/account/forget-credentials` | signed | Forgets the saved password. |
| POST | `/v1/account/delete` | signed | Deletes the account. |
| POST | `/v1/account/settings` | signed | Changes the account settings. |
| POST | `/v1/account/semester` | signed | Sets the semester agents use. |
| POST | `/v1/account/remove-phone` | signed | Unlinks a phone. |
| POST | `/v1/account/clear-sign-ins` | signed | Drops the sign-in history. |
| POST | `/v1/session/answer` | signed | A phone's answer to a session request. |
| POST | `/v1/account/credentials-rejected` | signed | VTOP rejected the password on the phone. |
| POST | `/v1/account/prove/start` | signed | A challenge for an account proof. |
| GET | `/v1/account/prove` | signed | The account proof's WebSocket, relayed to the engine; ends with the account token. |
| GET | `/.well-known/oauth-protected-resource` | none | RFC 9728 resource metadata. |
| GET | `/.well-known/oauth-protected-resource/mcp` | none | RFC 9728 resource metadata. |
| GET | `/.well-known/oauth-authorization-server` | none | RFC 8414 server metadata. |
| GET | `/.well-known/oauth-authorization-server/mcp` | none | RFC 8414 server metadata. |
| GET | `/authorize` | none | The pairing page for an MCP client. |
| POST | `/register` | none | Registers an MCP client (RFC 7591). |
| POST | `/token` | none | Swaps a code and PKCE verifier for an access key. |
| POST | `/authorize/poll` | none | The pairing page polls for approval. |
| ANY | `/mcp` | key | MCP, passed on to the engine. |
| ANY | `/mcp/{key}` | key | MCP with the key in the path, passed on to the engine. |
| POST | `/internal/mail-otp/await` | internal | The engine waits for a student's next OTP. |
| POST | `/internal/udp-keys` | internal | The UDP gateway reports its keys (GATEWAY_TOKEN). |
| POST | `/internal/accounts/login-done` | internal | The engine reports a server sign-in. |
| POST | `/internal/export` | internal | One Durable Object's stored state, for the move to v3. |

<!-- routes:end -->

## Health

| Route | Auth | Response |
|---|---|---|
| `GET /healthz` | none | `{ service: "vtop-edge", status: "ok", version, colo }` |

## Guides (`/guides/*`) — none

Help content the app fetches when a screen opens and keeps on the phone,
served by Workers Static Assets from `guides/` (`assets` in
`alchemy.run.ts`). A request for a file never runs the Worker: no signature,
no Durable Objects. A path with no file falls through to the Worker's `404
not_found`.

Responses carry the asset `ETag` and `Cache-Control: public, max-age=3600`
(`guides/_headers`). Clients keep the last copy and revalidate with
`If-None-Match` (`304` when unchanged), and show the kept copy when offline.

Every JSON file has `"schema": 1`. The number goes up only on a breaking
change (a field removed or its meaning changed); new optional fields and new
block types keep it, and clients skip blocks they don't know. An app that
gets a higher schema asks the student to update it (the newer copy replaces
the kept one), so raise it only together with an app release.

To change the content, edit the JSON (and images) in `guides/` and run `bun
run deploy`; phones pick it up on their next fetch, with no app release.
`src/guides.test.ts` checks both files.

### `GET /guides/forwarding.json`

The mail forwarding guide (Login → OTP auto-fill).

```json
{ "schema": 1, "title": "Set up forwarding", "subtitle": "About 2 minutes in Gmail, once",
  "blocks": [
    { "type": "callout", "icon": "mail-check", "tone": "lab", "text": "…" },
    { "type": "step", "title": "…", "body": "… Put {sender} in From …",
      "callout": { "icon": "refresh-cw", "tone": "warning", "text": "…" },
      "image": "forwarding/4-noreload.webp", "caption": "…" } ] }
```

- Blocks in display order; steps are numbered by the app in order.
- `step`: `title` and `body`; `callout` (shown under the body), `image` and
  `caption` are optional. `{sender}` in a body is replaced by the app with
  VTOP's OTP sender.
- `image` is relative to `/guides/` (`/guides/forwarding/1-add.webp`), WebP.
- `icon`: `mail-check`, `refresh-cw`, `triangle-alert`, `info`. `tone`:
  `lab`, `warning`, `success`, `danger`, `accent` (the app's theme tones).

### `GET /guides/updates.json`

More → About: what the app does and what's new. One file for every app
version: each app shows only what it has.

```json
{ "schema": 1,
  "features": [ { "title": "Classes",
                  "items": ["…", { "text": "…", "since": 3000300 }] } ],
  "releases": [ { "version": "3.0.0", "code": 3000000, "summary": "…",
                  "sections": [ { "title": "New", "items": ["…"] } ] } ] }
```

- `releases`: newest first, `code` the release's `versionCode`. An app lists
  the releases up to its own code; the newest of those is its "What's new",
  with its sections as tabs. Newer releases stay hidden.
- `features`: sections in display order. An item is a string, or `{ text,
  since }` for a feature added in the release with `versionCode` `since`;
  apps older than that leave it out. Remove a feature by deleting its item.

### `GET /guides/demo.json`

The demo account's pages (`rust/vtop-ffi/src/demo.rs`): signing in with its
fixed username and password downloads this instead of asking VTOP, and every
read page comes from it.

```json
{ "schema": 1,
  "pages": { "semesters": { … }, "attendance@AP2026275": { … },
             "full_attendance@AP2026275?course_id=…&course_type=…": { … } } }
```

- Each key is the page's name in `vtop-bridge/src/pages.rs`, `@semester` for
  a semester's page, then its arguments as `?key=value&…`; each value is the
  page's `data` as `/v1/vtop/fetch` answers it. A page with other arguments
  stands in for one not recorded (another day's biometrics).
- Recorded from a real account (a debuggable app with `demo_record.json` in
  its data folder), then `bun dev/demo.ts <recording>` in `backend/` swaps
  in a made-up student and fails if a personal value is left.

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
| `POST /internal/udp-keys` | internal (`GATEWAY_TOKEN`) | the same shape, from the gateway | `204`; `400` if malformed |

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
| `/v1/mail-otp/inbox` | `{ id }` | `{ sealed, receivedAt, confirmation, lastOtpAt }`; `sealed: null` without a box (boxes live 5 min); `confirmation` is Gmail's latest forwarding confirmation sealed to this device (kept for 24 hours or until an OTP arrives), else `null`; `lastOtpAt` is when the latest OTP was sent, or `null` |
| `/v1/mail-otp/owner` | `{ id, session: { cookies } }` or `{ id, accountToken }` | `204`; ties the address to the VTOP student; `409 address_exists` when the student has another address, `409 owner_mismatch` when this one is another student's, `401 session_invalid`, `503 vtop_unreachable`; token errors as in [Account tokens](#account-tokens) |
| `/v1/mail-otp/reclaim` | `{ session: { cookies } }` or `{ accountToken }` | `200 { id, address }`: this device joins the student's address |
| `/v1/mail-otp/devices` | `{ id }` | `{ devices: [{ deviceId, label, addedAt, seenAt, thisDevice }] }` |
| `/v1/mail-otp/devices/remove` | `{ id, removeDevice }` | `204`; `404 unknown_device`; revokes the removed device's account token |
| `/v1/mail-otp/delete` | `{ id, everywhere? }` | `204`; leaves the address, or deletes it when last or `everywhere: true`; revokes the account token of this device (with `everywhere`, of every device on the address) |
| `/v1/mail-otp/leave` | `{ id }` | `204`; this device leaves the address and its account token is revoked. An owned address is kept with no device left, so the student's Gmail filter keeps working and their next sign-in joins it with `reclaim`; an unowned one is deleted with its last device. The app calls it on sign-out and when another student signs in |

`owner` and `reclaim` take an account token in place of the session, and
after a session check issue one to this device like the account routes do
([Account tokens](#account-tokens)), whether or not it is a linked phone.

Up to 5 devices per address. Mail is sealed per device to its current
`sealKey` and delivered as a `mail_otp` event and to `/inbox`.

## Accounts — phones

All `POST`, signed. Routes that send `session` need the phone's live VTOP
session (`{ cookies: "<Cookie header>" }`); the engine says whose it is.
Instead of `session` they take `accountToken` (below), which needs no
engine or VTOP call; with both, the token is used. New apps never send
`session` here: they get the token from an [account proof](#account-proof)
and send only that. **Deprecated:** `session` on these routes and on
`/v1/mail-otp/owner` and `/reclaim` stays for older installs only and will
be removed in a later release (about two weeks after the proof ships). Errors common to those:
`400` without `session.cookies` (or a token), `401 session_invalid`, `503
vtop_unreachable`, `403 not_linked` (this device is not one of the
account's phones), `409 not_linked` (no phone linked yet).

### Account tokens

A token says who the student is, for any device that showed their VTOP
session, linked or not. Every success that checked the session with the
engine (the routes below and `/v1/mail-otp/owner` and `/reclaim`) carries a
new one for the calling device in two headers (`/v1/account/delete` none):

- `x-vm-account-token`: a compact JWT, HS256 with `ACCOUNT_TOKEN_SECRET`.
  Header `{ alg: "HS256", typ: "JWT" }`; claims `sub` (registration number),
  `dev` (device id of the signed request), `iat`, `exp = iat + 14 days`
  (unix seconds).
- `x-vm-account-token-expires-at`: `exp` in unix milliseconds.

Status and body stay as they were. A call made with a token never gets
another: after 14 days the phone sends the session once more. The
student's `Account` object keeps one row per device, the `iat` of its live
token, and issuing replaces it. The edge accepts a token when its signature
holds, it is unexpired, `dev` is the device that signed the request, and
the device's row has that `iat`; so a newer token kills the older one.
Revoking deletes the row: removing a phone, `/v1/keys/move` unlinking it,
`/v1/devices/delete`, `/v1/mail-otp/devices/remove` and `/v1/mail-otp/delete`
(as in the table above), and `/v1/account/delete` (every device). The
account also keeps the student's owner hash, for the mail-otp routes, and
the index entry `student:<owner hash>` names the account, so those routes
can revoke. A student who never linked a phone thus has an `Account` with
only these rows.

A token proves only who the student is: routes that need a linked phone
answer `403 not_linked` or `409 not_linked` to an unlinked device with a
valid token, as they do with its session. Errors:

| Status | `code` | When |
|---|---|---|
| 401 | `token_invalid` | bad signature or shape, another device's, or revoked (as above); the phone drops it and sends the session |
| 401 | `token_expired` | past `exp` |

### Account proof

How a new app gets its token without its VTOP cookie leaving the phone: it
proves its registration number with TLSNotary in proxy mode (`rust/vtop-tlsn`,
`rust/ARCHITECTURE.md` "Account proof"), and the edge issues the token exactly
as after a session check (same claims, the device's `iat` row replaced, the
owner hash and `student:<owner hash>` kept).

| Route | Request | Response |
|---|---|---|
| `/v1/account/prove/start` | `{}` | `{ challenge, expiresAt }`: 32 lower-case hex digits (128 random bits), good for 2 minutes and one try; a new one replaces the last. `expiresAt` in unix ms |

### `GET /v1/account/prove`

Signed (empty body, the query in `pathWithQuery`), a WebSocket:
`?challenge=<the challenge>`. Before the upgrade: `426 upgrade_required`
without `Upgrade: websocket`, `401 challenge_invalid` when it is not this
device's live challenge (it is used up either way), `503 proof_busy` when
the engine runs its most proofs at once, `503 engine_unavailable`.

Then binary messages carry the tlsn session both ways, relayed to the
engine's `GET /engine/proof?device=<this device>&challenge=<it>` (below),
which builds the statement from those two before it reads a byte: the
request must be `GET /vtop/content?n=<binding>` with
`binding = hex(sha256("vitapmate-tlsn-v1:" + challenge + ":" + deviceId))[..32]`,
so a proof made for another device or challenge fails. The engine's verdict
goes to the edge only. The phone's last message is one text frame, then the
socket closes:

- `{ accountToken, expiresAt, registrationNumber }`: the token (as in the
  headers above; `expiresAt` in unix ms);
- or `{ code, error }`: `proof_failed` (no proof: signed out, a refused
  page, a broken proof; `error` says which step), `token_unavailable`
  (no `ACCOUNT_TOKEN_SECRET`), `internal_error`.

A text message from the phone, or either side closing first, closes both
without a token.

| Route | Request | Response |
|---|---|---|
| `/v1/link` | `{ session, label?, semesterId?, credentials? }` | `{ savedCredentials, forwarding }`; adds this device (up to 5 phones) |
| `/v1/account` | `{ session }` | the account view: `registrationNumber, createdAt, settings, semesterId, linked, savedCredentials, vaultSavedAt, vaultGmail, credentialsRejectedAt, forwarding, phoneLinked, phoneCount, thisPhone, phones[], keys[]` (lists only for a linked phone); each key `{ id, label, createdAt, lastUsedAt, phoneId, outings, uses: [{ at, via: "extension"\|"mcp" }] }`, newest use first, at most 10 |
| `/v1/keys` | `{ session, label?, outings?, pairCode? }` | `201 { id, key, mcpUrl }` (the key is shown once); `409 key_limit`; with `pairCode` the extension that showed the code also gets the key (below) |
| `/v1/keys/revoke` | `{ session, id }` | `204`; `404 key_not_found` |
| `/v1/keys/outings` | `{ session, id, outings: bool }` | `204` |
| `/v1/keys/move` | `{ session, from?: phoneId }` | `{ moved }`; this phone now answers for the keys of phone `from` (which is unlinked) and of phones no longer linked; without `from` only the latter; `404 phone_not_found` |
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
{ refreshToken, clientId, clientSecret?, deleteAfterReading? }, otp?:
"forwarding" | "gmail" }`; it is sealed by the engine and kept only when the
server can read the code that way (`otp` is the phone's OTP auto-fill; without
it, forwarding or Gmail). With `otp: "gmail"` the engine reads Gmail even when a
forwarding address exists. The phone sends it again when its OTP auto-fill
changes.

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
| `POST /v1/key/token` | key | `{ jwt, expiresAt }` (no-store): a 24 h ES256 JWT `{ sub: keyHash, kind: "key" }` for the Convex socket, never a phone's; not a use of the key; `429` past 10 at once or 30 a minute |
| `POST /v1/pair` | none | `{ code }` → `200 { status: "ready", key }` once the app made a key with this `pairCode`, else `202 { status: "pending", pollAfterMs }`; `400` for a malformed code |
| `POST /v1/pair/info` | none | `{ code }` → `{ kind: "mcp", name, host }` when the code is an MCP client's authorize page (below) still waiting, else `{ kind: "browser" }`; `400` for a malformed code. The app asks before it approves. |

**Pairing.** The extension makes a 10-character Crockford base32 code
(`XXXXX-XXXXX`; case, dashes and spaces ignored, `O` read as `0`, `I`/`L`
as `1`), shows it, and polls `POST /v1/pair`. The app sends it as
`pairCode` on `/v1/keys`; the edge holds the new key under the code's
SHA-256 for 5 minutes and hands it out once.

**Live.** Builds from 3.0.0 open the Convex socket with a key JWT and watch
`extension:key` (the account, null once revoked), `extension:request`
(a request's stage, the server's sign-in step and `expiresAt`, never cookies),
and `extension:pairing({ codeHash })` (`{ ready }` once the app approved; no sign-in).
The newest build comes from GitHub, never the server. Cookies and keys still come once over
`GET /v1/session/requests/{id}` and `POST /v1/pair`.

When the code is an MCP client's (see OAuth below), `/v1/keys` names the
key after the client (`label` is ignored), hands it to that sign-in
instead, and answers with `app: { name, host }` (else `app: null`); `410
pair_expired` when the sign-in ran out meanwhile.

## VTOP for phones

| Route | Auth | Notes |
|---|---|---|
| `POST /v1/vtop/fetch` | signed | body passed to the engine's `/engine/fetch` as is |
| `POST /v1/vtop/fetch-batch` | signed | body passed to the engine's `/engine/fetch-batch` as is (below) |
| `POST /v1/vtop/login` | signed | → `/engine/vtop-login` |
| `POST /v1/vtop/login/otp` | signed | → `/engine/vtop-login/otp` |
| `POST /v1/vtop/login/resend` | signed | → `/engine/vtop-login/resend` |

Bodies up to 64 KiB (`413 too_large`) on `/v1/vtop/*`. Answers are JSON
(Brotli to the phone via Cloudflare) or `application/vnd.vmate.page+br`
passed byte for byte. `504 vtop_timeout` after 32 s, `503 engine_unavailable`.

**Batches.** `/v1/vtop/fetch-batch` fetches several pages of one semester on
one session in one round trip:

```json
{ "session": { "cookies": "…", "csrf_token": "…", "registration_number": "…" },
  "semester": "AP2026275", "dictionary": "<page dictionary id>",
  "pages": [ { "name": "attendance" }, { "name": "biometric", "args": { "date": "28/10/2026" } } ] }
```

- `pages`: 1–16, names and `args` as in `/v1/vtop/fetch`; `semester` applies
  to every page. The engine runs 4 at a time on one VTOP client, all within
  25 s.
- `200 { "data": [answer, …] }`, one answer per page in order: `{ "data": … }`
  (the page, as `/v1/vtop/fetch` answers it) or that page's own
  `{ "code", "error" }` (`unknown_page`, `bad_arguments`, `vtop_unavailable`,
  `vtop_timeout`, `parse_error`, …), so one bad page does not fail the rest.
- `401 session_expired` for the whole batch when VTOP ended the session (any
  page saying so); `400 bad_arguments` for a malformed body or over 16 pages.
- With the phone's `dictionary`, the `data` array comes packed
  (`application/vnd.vmate.page+br`) with the dictionary for the name
  `/v1/vtop/fetch-batch`.
- A server without batches answers `404` (`not_found` from an older edge, a
  bare `404` from an older engine); the app then fetches page by page.
- Over UDP the same batch is a request named `/v1/vtop/fetch-batch` whose
  `args` are the pages by two-digit index (`"00"`, `"01"`, …), each its name or,
  with arguments, its JSON (`vtop_shared::wire_page::batch_args`); the gateway
  sends it to `/engine/fetch-batch`.

## MCP

| Route | Auth | Notes |
|---|---|---|
| `/mcp`, `/mcp/<key>` | key | any method, passed to the engine |

Passed to `ENGINE_URL` unchanged (the client's own
`Authorization: Bearer vtm_…` or path key `/mcp/vtm_…` included), with
`x-forwarded-for` set to the client's address. On `/mcp` or `/mcp/` without
an `Authorization` header (except `OPTIONS`) the edge answers `401` itself
with `WWW-Authenticate: Bearer
resource_metadata="<PUBLIC_URL>/.well-known/oauth-protected-resource/mcp"`;
the engine's own `401`s get the same header if missing.

## OAuth for MCP clients

MCP authorization (2025-06-18): OAuth 2.1, authorization code with PKCE
`S256` only, public clients, dynamic registration. The token is an
ordinary `vtm_…` access key, so revoking, last uses, the outings switch and
the 10-key cap apply as for any key. It does not expire; there are no
refresh tokens. All routes but `/authorize*` send
`access-control-allow-origin: *` and answer `OPTIONS` with `204`. Errors
are `{ error, error_description }` (RFC 6749), `cache-control: no-store`.

| Route | Auth | Request | Response |
|---|---|---|---|
| `GET /.well-known/oauth-protected-resource`, `…/mcp` | none | — | RFC 9728: `{ resource: "<PUBLIC_URL>/mcp", authorization_servers: ["<PUBLIC_URL>"], bearer_methods_supported: ["header"], resource_name }` |
| `GET /.well-known/oauth-authorization-server`, `…/mcp` | none | — | RFC 8414: `issuer`, `authorization_endpoint`, `token_endpoint`, `registration_endpoint`, `response_types_supported: ["code"]`, `grant_types_supported: ["authorization_code"]`, `code_challenge_methods_supported: ["S256"]`, `token_endpoint_auth_methods_supported: ["none"]`, `authorization_response_iss_parameter_supported: true` |
| `POST /register` | none | RFC 7591 JSON: `redirect_uris` (1–20), `client_name?`, `grant_types?` (must include `authorization_code`) | `201 { client_id, client_id_issued_at, client_name, redirect_uris, grant_types, response_types, token_endpoint_auth_method: "none" }`; `400 invalid_redirect_uri`, `400 invalid_client_metadata` |
| `GET /authorize` | none | `response_type=code`, `client_id`, `redirect_uri`, `code_challenge`, `code_challenge_method=S256`, `state?`, `resource?`, `scope?` (ignored) | the pairing page (HTML); `400` page for an unknown client or unregistered `redirect_uri` (nothing is sent to it); other errors go back to `redirect_uri` with `error`, `error_description`, `state`, `iss` |
| `POST /authorize/poll` | none | `{ poll }` (from the page) | `202 { status: "pending", pollAfterMs }`, `200 { status: "ready", redirect }`, `410 { status: "expired", code: "request_expired", error }` (`400` with `bad_request` for a malformed `poll`) |
| `POST /token` | none | form: `grant_type=authorization_code`, `code`, `client_id`, `redirect_uri`, `code_verifier`, `resource?` | `{ access_token: "vtm_…", token_type: "Bearer" }`; `400 invalid_grant` (unknown, used or expired code, other client, other `redirect_uri`, verifier mismatch), `unsupported_grant_type`, `invalid_request`, `invalid_target` |

Rules:

- Redirect URIs: `https`, `http` on `localhost`/`127.0.0.1`/`[::1]`, or an
  app's own scheme (not `javascript`, `data`, `file` and the like); no
  fragment; at most 512 characters. `/authorize` and `/token` want one of
  the registered URIs exactly, except that a loopback URI may change port
  (RFC 8252).
- `resource`, when sent, must be `<PUBLIC_URL>/mcp` or `<PUBLIC_URL>`
  (`invalid_target`).
- Names are cut to 40 characters (`MCP app` when missing). At most 2000
  clients are kept; registering past that drops the one unused longest.

**Approval.** The page shows the client's name, its redirect host, a
10-character pairing code (`XXXXX-XXXXX`) and its QR, and polls
`/authorize/poll` every 2 s. In the app, Connected apps → Pair a browser
scans or takes the code, asks `/v1/pair/info` to show the client's name,
then makes the key with `pairCode` on `/v1/keys` (outings off). The next
poll turns the request into a one-time code and the page goes to
`redirect_uri?code=…&state=…&iss=<PUBLIC_URL>`. The request lives 10
minutes, the code 60 seconds and is gone after one `/token` try. Clients,
requests and codes live in the `OAuth` Durable Object (one, named
`oauth`); pairing codes, poll ids and codes are stored as SHA-256 only.

## Internal (engine and gateway)

All need `Authorization: Bearer <INTERNAL_TOKEN>`, else `401 unauthorized`;
`/internal/udp-keys` takes the gateway's own `GATEWAY_TOKEN` too, which opens
nothing else, so a compromised gateway cannot wait for OTPs or reach accounts.

| Route | Caller | Request | Response |
|---|---|---|---|
| `POST /internal/accounts/login-done` | engine | `{ account, requestId, outcome }` (`LoginDone` in `src/shared/engine-api.ts`) | `204` |
| `POST /internal/mail-otp/await` | engine | `{ ownerHash, publicKey, since, waitSecs }` | `{ sealed }` or `{ sealed: null }`; `404 mailbox_not_found` |
| `POST /internal/udp-keys` | gateway | see UDP | `204` |
| `POST /internal/export` | backfill | `{ kind, id }` or `{ kind, objectId }` | `{ kind, id \| objectId, found, tables, kv, omitted }` |

`/internal/export` is for the one-time move to backend v3
(`backend/scripts/backfill.ts`). It reads one Durable Object and changes
nothing, not even a missing table. `kind` is `device`, `account`, `mailbox`,
`oauth` or `fcm`; `id` is the object's name (a device id, a registration
number, a mailbox id, `oauth`) and `objectId` its 64-hex id, as the Cloudflare
API lists a namespace's objects. `tables` has each table's rows as stored
(sealed values as they are), `kv` a mailbox's Gmail confirmation and latest
OTP time, and `omitted` the row counts of the tables left out: short-lived ones,
some holding live secrets (an account's session `requests` and `pending`,
OAuth `requests` and `grants`, a mailbox's engine `waiters`), and a device's
`events`. `fcm` is always `found: false`: that object caches Google's token in
memory only. The `Index` objects keep no list of their names, and everything
they hold is in the accounts' and mailboxes' own tables, so they aren't
exported.

## Edge → engine

Every call carries `Authorization: Bearer <INTERNAL_TOKEN>`:
`POST /engine/session`, `/engine/seal`, `/engine/login` (answers `202`, then
reports to `/internal/accounts/login-done`), `GET /engine/proof` (the
[account proof](#get-v1accountprove)'s WebSocket: `?device=&challenge=`, 32
lower-case hex each, `400` otherwise, `503 proof_busy` past 4 at once; binary
messages both ways, then the engine's text frame `{ status: "ok",
registrationNumber, ownerHash }` or `{ status: "failed", reason }`),
`/internal/mail-otp/owner`
(`{ cookies }` → `{ ownerHash, registrationNumber }`, `401` if VTOP
refuses; an engine without `registrationNumber` means no token), and the
`/engine/fetch`, `/engine/fetch-batch`, `/engine/vtop-login*` routes above. `/mcp` keeps the
client's header instead.

## Configuration

Vars: `MAIL_DOMAIN`, `ENGINE_URL`, `FCM_URL`, `PUBLIC_URL`,
`ALLOWED_ORIGINS`, `FALLBACK_FORWARD`. Secrets: `FIREBASE_CREDENTIALS_JSON`,
`INTERNAL_TOKEN`, `GATEWAY_TOKEN` (the UDP gateway's, for `/internal/udp-keys`
only), `ACCOUNT_TOKEN_SECRET` (signs account tokens; unset, none
are issued and every token is `token_invalid`; changing it voids them all).
`FREEZE` (any value, for the cutover to backend v3): every route that writes
the Durable Objects answers `503 frozen` with `retry-after: 60`. Still open:
`/healthz`, `/internal/*`, mail coming in, GET routes but `GET
/v1/account/prove`, and the POST routes that only read (`/v1/devices/info`,
`/v1/app/status`, `/v1/mail-otp/inbox`, `/v1/mail-otp/devices`,
`/v1/pair/info`, `/v1/vtop/*`).
