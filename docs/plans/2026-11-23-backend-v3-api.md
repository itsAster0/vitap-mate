# Backend v3 API contract

What the Convex backend (`backend/`), the app (`rust/vtop-ffi` and `core/`), the
mail Worker (`mail/`) and the engine agree on. The plan is
`docs/plans/2026-11-23-backend-v3.md`. When this contract and the plan disagree,
this contract wins. Change it only on purpose, and tell the other parts.

Test deployment: `https://vmate3.aster0.dev` (self-hosted Convex on hawk). Caddy
sends `/api/*` and `/version` to Convex's API and everything else to its HTTP
actions. The admin key is in infra `secrets/vmate-convex.env.enc`. Put it in
`backend/.env.local`, which is gitignored, and never print it.

## Rule one: port, don't redesign

Each edge route in `edge/API.md` becomes one Convex function or HTTP action, with
the same request body as args and the same JSON as the return value. Errors
become `ConvexError({ code, error, ...extra })`, with the same `code` strings as
today. So the app swaps its transport and keeps its request and response types.
Binary `application/vnd.vmate.page+br` answers come back as `ArrayBuffer`. Every
function the phone calls also takes `flow: string`, which is logged on every line.

## Auth kinds

| Kind | How |
|---|---|
| `public` | No auth. |
| `signed` | Args carry `signed: { key, time, sig }`, plus `payload: string` (JSON) in place of the body. `key` is the Ed25519 public key (base64url). `sig` is vm1 over `METHOD = "CALL"`, `path = "/" + functionName` (for example `/auth:token`) and `body = payload`. The device id is `hex(sha256(key))[:32]`, so no lookup is needed. Time window ±120 s. Same scheme as `vtop_shared::device_sig`. |
| `jwt` | The socket's `setAuth` JWT. `ctx.auth.getUserIdentity()` gives `subject` (device id), `regno`, `acct`, `demo?`. Every `jwt` function also checks that the device row still exists and is linked to `acct`, so a removed device fails at once with `ConvexError({ code: "device_removed" })`. |
| `key` | HTTP actions only: `Authorization: Bearer vtm_…`, or `/mcp/<key>`, as today. |
| `internal` | HTTP actions only: `Authorization: Bearer <INTERNAL_TOKEN>`. The mail route takes `MAIL_TOKEN` instead, and the gateway's route takes `GATEWAY_TOKEN`. |

`regno` always comes from the JWT, never from args.

Errors are `ConvexError({ code, error })`, with today's `code` strings. New ones:
`unauthenticated` (no JWT), `device_removed`, `demo_unavailable`, `proof_pending`
(the ticket's proof hasn't finished) and `refresh_invalid`.

## Data model

`regno` is stored once, on `accounts` (unique index). Every row a student owns
links to it with `accountId` and a `by_account` index: `devices` (optional:
set by `auth:token`, cleared when the phone is removed), `accessKeys`,
`refreshTokens`, `signIns`, `mailboxes`, `mailboxDevices`, `otpInbox`,
`sessionRequests`, `pairings`, `oauthRequests` (once approved), `oauthCodes` and
`v2Tokens`. A proved `proofTickets` row keeps its `regno` until `auth:token`
uses it up. There are no embedded lists: an account's phones are its linked
`devices`, its keys are `accessKeys` rows (with their label, phone, outings and
latest uses), and its sign-ins are `signIns` rows (the latest 10). Logs carry
`regno` from the JWT or the linked account, not from a column.

## Tokens

- **Access JWT:** ES256, `iss: "https://vmate3.aster0.dev"` (from env
  `JWT_ISSUER`), `aud: "vmate"`, 15 minutes. Claims: `sub` (device id), `regno`,
  `acct`, optional `demo: true`. Convex verifies it through `auth.config.ts` as a
  `customJwt` provider. The JWKS comes from the env `JWT_PUBLIC_JWK` (a data
  URI); the private key is `JWT_PRIVATE_JWK`.
- **Refresh token:** `vrt_` + 32 random bytes (base64url), stored as a sha256
  hash with its device and a chain id. A new token replaces the old one on every
  use. Reusing a replaced token deletes the whole chain and fails with
  `refresh_reused`. Tokens expire after 90 days unused.

## Identity: TLSNotary only

There's one way to prove who a student is: the TLSNotary proof. The account is
never established from cookies. Access keys for the extension and MCP are made by
a phone that already holds a JWT, so they follow from the same proof.

1. The phone signs in to VTOP: Direct, with the OTP typed by hand, or over the
   Server route with `vtop:login*`.
2. `auth:proveStart` (signed) returns `{ ticket }`.
3. The phone opens `wss://<host>/proof?ticket=<ticket>`, which Caddy sends
   straight to the engine. The engine POSTs `/internal/proof-ticket { ticket }`
   (internal) to Convex, which uses the ticket up and returns `{ device }`, or
   `404 challenge_invalid`. The engine then runs today's proof (binding =
   `hex(SHA-256("vitapmate-tlsn-v1:" + ticket + ":" + device))[..32]`, `vtop-tlsn`
   unchanged) and POSTs the verdict to `/internal/proof-done { ticket, regno, ownerHash }`
   or `{ ticket, code }`. The phone's last message on the socket is
   `{ ok: true, registrationNumber }` or `{ code, error }`.
4. The phone calls `auth:token { ticket, device }` and gets its JWT and refresh
   token. From here on it calls `auth:token { refreshToken }`.

## Functions the phone calls (over the socket)

Module and function names are Convex paths (`auth:token`).

### Before an account (`public` / `signed`)

| Function | Kind | Auth | Args → returns |
|---|---|---|---|
| `guides:updates`, `guides:demo`, `guides:forwarding` | query | public | `{}` → the JSON today's `/guides/*.json` serves |
| `udp:keys` | query | public | `{}` → today's `/v1/udp` body |
| `auth:proveStart` | mutation | signed | payload `{}` → `{ ticket, expiresAt }`. The ticket is the TLSNotary challenge: 2 minutes, one at a time per device, kept in `proofTickets` with the device id. |
| `vtop:login`, `vtop:loginOtp`, `vtop:loginResend` | action | signed | today's `/v1/vtop/login*` bodies and answers (server-route sign-in). It proves nothing: the phone proves afterwards with the session it got back, like everyone else. |
| `auth:token` | action (it signs the ES256 JWT) | signed | payload is exactly one of: `{ ticket, device }` (a ticket the engine marked proved), `{ refreshToken }`, `{ v2AccountToken, device }` (a v2 account token, accepted only during the migration), or `{ demo: true, device }`, where `device = { sealKey, platform, label?, fcmToken? }`. Returns `{ jwt, jwtExpiresAt, refreshToken, regno, account }`. A proved ticket creates or joins the account by `regno` and creates or updates the device row. A `v2AccountToken` is checked with `ACCOUNT_TOKEN_SECRET`, as `edge/src/account-token.ts` does, and must also match a live row in `v2Tokens` (device id + `iat`, imported by the backfill from the edge's `tokens`; no secrets): a token the edge revoked or replaced fails `token_invalid`, and a trade-in marks its row used, so a replay fails `token_invalid` too. Every grant sets `devices.accountId`; a refresh also marks the phone seen. A sixth phone unlinks the one seen least recently. |

### With a JWT

| Today | Function | Kind |
|---|---|---|
| `POST /v1/devices/update`, `/info`, `/delete` | `devices:update`, `devices:info`, `devices:remove` `{ mailbox? }` | mutation / query / mutation. `info` has no `events` or `latest`. |
| `GET /v1/events?after=` | gone | the app watches live queries; FCM only wakes a closed app (below) |
| (new) | `sessions:pending` | query (live): `[{ requestId, expiresAt, wantCredentials }]`, the session requests waiting on this phone (the handover prompt) |
| `POST /v1/app/status`, `/session/clear`, `/login` | `app:status` (query), `app:sessionClear`, `app:login` (action) | |
| `POST /v1/mail-otp/register`, `delete`, `leave`, `owner`, `reclaim`, `devices`, `devices/remove` | `mail:register`, `mail:remove`, `mail:leave`, `mail:devices` (query), `mail:removeDevice` | owner and reclaim go away: a mailbox belongs to the account from the start, and `register` from another phone joins it |
| `POST /v1/mail-otp/inbox` | `mail:inbox` | query (live): the newest sealed OTP for this device, and the Gmail confirmation |
| `POST /v1/vtop/fetch`, `fetch-batch` | `vtop:fetch`, `vtop:fetchBatch` | action, bodies as today |
| `POST /v1/link` | gone | `auth:token` with a ticket links |
| `/v1/link`'s `credentials` | `account:saveCredentials` `{ label?, credentials }` | action: the engine seals them (`/engine/seal`) and the account keeps the vault, only when the server can read the OTP (Gmail access sent along, or a forwarding address). `label` renames this phone. Returns `{ savedCredentials, forwarding }`, as `/v1/link` did. |
| `POST /v1/keys`, `/keys/revoke`, `/keys/outings`, `/keys/move` | `keys:create` (action: the key needs real randomness), `keys:revoke`, `keys:setOutings`, `keys:move` | mutation |
| `POST /v1/account` | `account:view` | query (live) |
| `POST /v1/account/forget-credentials`, `delete`, `settings`, `semester`, `remove-phone`, `clear-sign-ins` | `account:forgetCredentials`, `account:remove`, `account:setSettings`, `account:setSemester`, `account:removeDevice`, `account:clearSignIns` | mutation. `account:remove` deletes every row the account owns. |
| `POST /v1/account/credentials-rejected` | gone | no caller |
| `POST /v1/session/answer` | `sessions:answer` | action (the engine checks the cookies first) |
| (new) | `auth:signOut` `{ everywhere? }` | mutation: deletes this device's refresh chain, or every chain on the account |

### Demo

`auth:token { demo: true }` issues a JWT with `regno: "DEMO2026"` and
`demo: true`. Mutations that change server state, and `vtop:*` engine calls,
refuse it with `demo_unavailable`. Queries answer from the `guides:demo` data.
The demo may still refresh, `auth:signOut` and use `devices:*`; it has no account
row, and `sessions:pending` answers `[]`.

## HTTP actions (same paths as today, so the extension, MCP and the gateway don't change)

| Path | Auth | Notes |
|---|---|---|
| `GET /healthz` | public | `{ service: "vmate-convex", version }` |
| `GET /guides/*` | public | what the edge's Static Assets served from `edge/guides` (JSON and images), with `Cache-Control: public, max-age=3600` and an ETag (`If-None-Match` → 304). Bundled at deploy by `backend/dev/gen-guides.ts`; a test fails while the bundle and `edge/guides` differ. |
| `GET /v1/whoami`, `POST /v1/session`, `GET /v1/session/requests/{id}`, `POST /v1/session/expire` | key | as today |
| `POST /v1/pair`, `/v1/pair/info` | public | as today |
| `/.well-known/oauth-*`, `/authorize`, `/register`, `/token`, `/authorize/poll` | public | as today |
| `/mcp`, `/mcp/{key}` | key | checks the key, then forwards to `ENGINE_URL` as today |
| `POST /internal/udp-keys` | `GATEWAY_TOKEN` | as today |
| `POST /internal/accounts/login-done` | internal | the engine's report on a server sign-in, as today |
| `POST /internal/mail-otp/await` | internal | as today. It long-polls the inbox for at most `waitSecs` (60 at most). It goes away once login runs in steps. |
| `POST /internal/mail` | `MAIL_TOKEN` | from the mail Worker, below |
| `POST /internal/proof-ticket` | internal | the engine uses up a ticket: `{ ticket }` → `{ device }`, or `404 challenge_invalid` |
| `POST /internal/proof-done` | internal | the engine's verdict: `{ ticket, regno, ownerHash }` or `{ ticket, code }`. Marks the ticket proved or failed. |

Convex calls the engine with today's edge → engine API
(`edge/src/shared/engine-api.ts`, `ENGINE_URL`, `INTERNAL_TOKEN`). The engine
changes in two places: its reports go to the Convex URL instead of the edge, and
it serves the public `/proof?ticket=` WebSocket itself, as above. The edge used
to relay that socket, which Convex can't do. `/internal/mail-otp/owner` (the
cookie check) is no longer called.

## Mail Worker → Convex

The Email Worker (`mail/`) keeps today's parsing (`edge/src/mail.ts`): the
address id from `otp+<id>@`, the OTP, and the Gmail forwarding confirmation. It
POSTs:

```json
{ "addressId": "…", "receivedAt": 1795440000000, "otp": "123456", "gmailConfirm": null, "from": "…", "subject": "…" }
```

`gmailConfirm` is `{ code, link, requester }` (a code or a link at least) or `null`.

- `200 { ok: true }`: stored.
- `404 mailbox_not_found`: the Worker sends the mail to `FALLBACK_FORWARD` as
  today.
- Anything else: the Worker puts the mail on a Cloudflare Queue and retries with
  backoff for up to 10 minutes.

Convex seals the OTP to every device on the mailbox (X25519 `sealKey`, the same
`mail_seal` format as today, `rust/vtop-shared`), writes `otpInbox`, and sends
FCM from an action, with the same data payload as today's `edge/src/fcm.ts`
minus `seq`: `{ kind, at, … }` with `kind` one of `mail_otp` (`mailbox`,
`sealed`), `vtop_cookie_request` (`requestId`, `wantCredentials`) or
`credentials_rejected`. Without `FIREBASE_CREDENTIALS_JSON` nothing is sent.

## Env (backend)

`JWT_ISSUER`, `JWT_PRIVATE_JWK`, `JWT_PUBLIC_JWK`, `INTERNAL_TOKEN`,
`MAIL_TOKEN`, `GATEWAY_TOKEN`, `ACCOUNT_TOKEN_SECRET`, `ENGINE_URL`,
`FIREBASE_CREDENTIALS_JSON`, `PUBLIC_URL`, `ALLOWED_ORIGINS`, and optionally
`MAIL_DOMAIN` (default `vmate.aster0.dev`) and `FCM_URL` (tests). List them with
placeholders in `backend/.env.example`.
