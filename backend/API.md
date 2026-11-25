# vitapmate backend v3 API

Self-hosted Convex at `https://vmate.aster0.dev` (on hawk, behind hawk's Caddy).
Phones use Convex functions over the WebSocket (`/api/*`); this document covers
the HTTP actions (`backend/convex/http.ts`) used by the browser extension, MCP
clients, the UDP gateway, the mail Worker and the TLSNotary verifier, plus the
Convex → engine contract.

JSON in and out unless noted. Errors are `{ "code": "...", "error": "human text" }`
with the HTTP status (`code` is `snake_case` and stable; `error` is for people).
Unknown paths answer `404 not_found`, invalid JSON `400 bad_request`, unhandled
failures `500 internal_error`. Exceptions: `/mcp` forwards upstream engine
responses as-is, and OAuth's `/register` and `/token` return RFC 6749
`{ error, error_description }`. Times are unix seconds unless noted as ms or
RFC 3339.

## Auth

| Kind | How |
|---|---|
| **key** | Access key (extension, MCP): `Authorization: Bearer vtm_…` (or `/mcp/<key>`). |
| **phone JWT** | Phone ES256 access JWT from `auth:token`: `Authorization: Bearer <jwt>`. |
| **internal** | Per-caller bearer token: `GATEWAY_TOKEN` (`/internal/udp-keys`), `VERIFIER_TOKEN` (`/internal/proof-ticket`, `/internal/proof-done`), `MAIL_TOKEN` (`/internal/mail`). |
| **none** | Public. |

Client IP for per-IP rate limits and `/mcp` forwarding is taken from
`X-Forwarded-For`, `PROXY_HOPS` entries from the end (matching
`rust/vtop-verifier`); `cf-connecting-ip` is ignored.

## HTTP routes

| Method | Path | Auth | Summary |
|---|---|---|---|
| `GET` | `/healthz` | none | `{ service: "vmate-convex", version }`. |
| `GET` | `/v1/whoami` | key | Access key's account, phone link and cache status. |
| `POST` | `/v1/session` | key | VTOP session from cache (`200`), or a pending request (`202`). |
| `GET` | `/v1/session/requests/{id}` | key | Polls a pending session request (`no-store`). |
| `POST` | `/v1/session/expire` | key | Drops the account's cached VTOP session (`204`). |
| `POST` | `/v1/key/token` | key | Short ES256 key JWT for the extension's Convex socket (`no-store`). |
| `POST` | `/v1/pair` | none | Extension polls for the key approved for its pairing code (`no-store`). |
| `GET` | `/.well-known/oauth-protected-resource`, `…/mcp` | none | RFC 9728 protected resource metadata. |
| `GET` | `/.well-known/oauth-authorization-server`, `…/mcp` | none | RFC 8414 authorization server metadata. |
| `POST` | `/register` | none | Dynamic MCP client registration (RFC 7591). |
| `GET` | `/authorize` | none | HTML pairing page for an MCP client. |
| `POST` | `/authorize/poll` | none | Pairing page polls for app approval (`no-store`). |
| `POST` | `/token` | none | Swaps an authorization code + PKCE verifier for an access key (`no-store`). |
| `GET`/`POST`/`DELETE`/`OPTIONS`/`PUT`/`PATCH` | `/mcp`, `/mcp/{key}` | key | MCP JSON-RPC, forwarded to the engine. |
| `POST` | `/v1/app/login` | phone JWT | Server sign-in with saved credentials for the UDP gateway (`no-store`). |
| `GET` | `/v1/app/login/{requestId}` | phone JWT | Polls a pending `/v1/app/login` (`no-store`). |
| `POST` | `/internal/udp-keys` | `GATEWAY_TOKEN` | UDP gateway reports its host, ports and weekly public keys (`204`). |
| `POST` | `/internal/proof-ticket` | `VERIFIER_TOKEN` | Verifier consumes an open TLSNotary proof ticket. |
| `POST` | `/internal/proof-done` | `VERIFIER_TOKEN` | Verifier reports a proof verdict (`204`). |
| `POST` | `/internal/mail` | `MAIL_TOKEN` | Mail Worker delivers a forwarded OTP or Gmail confirmation. |
| `GET` | `/guides/{path}` | none | Bundled help JSON and WebP images, with `ETag` and `304`. |

Every other `GET`, `POST`, `DELETE`, `PUT` or `PATCH` answers `404 not_found`.

## Access keys & pairing (extension, MCP)

Routes under `/v1/whoami`, `/v1/session*`, `/v1/key/token` and `/v1/pair` apply
CORS (`ALLOWED_ORIGINS`, empty allows `*`; methods `GET, POST, DELETE, OPTIONS`;
headers `Content-Type, Authorization`) and answer `OPTIONS` with `204`. Invalid
keys return `401 key_unknown`. `x-vtop-client: mcp` tags MCP key usage.

- **`GET /v1/whoami`** → `{ registrationNumber, keyLabel, keyCreatedAt, keyLastUsedAt, keyOutings, semesterId, settings, phoneLinked, phone: { label, notifications } | null, savedCredentials, vaultSavedAt, credentialsRejectedAt, cachedSession }`.
- **`POST /v1/session`** (`cache-control: no-store`) →
  - `200 { status: "ready", session: { cookies }, cookies: [Cookie-Editor], source: { from, cached, madeAt } }` when the cached session still validates on VTOP;
  - `202 { status: "pending", stage: "starting" | "phone" | "server", serverDueAt, requestId, pollAfterMs, expiresAt }` with `retry-after: 2` when a new or in-flight request is opened/joined;
  - `429 rate_limited` (`retry-after: 60`) past 60 calls/min per account.
- **`GET /v1/session/requests/{id}`** (`cache-control: no-store`) →
  - `202 { status: "pending", stage, serverDueAt }`,
  - `200 { status: "ready", session, cookies, source }`,
  - `200 { status: "error", code, error }`,
  - `404 request_not_found`, or `410 request_expired`.
- **`POST /v1/session/expire`** → `204`.
- **`POST /v1/key/token`** (`cache-control: no-store`) → `200 { jwt, expiresAt }` (24 h ES256 JWT with `{ sub: keyHash, kind: "key" }` for the extension's Convex socket; does not record a key use; `429 rate_limited` with `retry-after: 60` past the per-account bucket).
- **`POST /v1/pair`** (`cache-control: no-store`, per-IP + global rate limit) → `{ code }` (10-char Crockford base32, dashes/spaces/case ignored) → `202 { status: "pending", pollAfterMs }` or `200 { status: "ready", key }` (handed out once); `400 bad_request`, `429 rate_limited`.

**Extension live queries.** With the key JWT from `/v1/key/token`, the extension
subscribes over Convex to `extension:key` (live account view, `null` when
revoked), `extension:request({ requestId })` (live progress and server sign-in
step, never cookies), and `extension:pairing({ codeHash })` (`{ ready }`, no
auth). Cookies and paired keys still transfer once over HTTPS.

## OAuth 2.1 for MCP clients

Public PKCE (`S256` only) authorization code flow with dynamic client
registration. Issued bearer tokens are normal `vtm_…` access keys (capped at 10
per account, outings forced off on OAuth approval). Metadata, `/register` and
`/token` send `access-control-allow-origin: *` and answer `OPTIONS` with `204`.
`/register`, `/authorize`, `/authorize/poll` and `/token` are rate-limited per
IP and globally.

- **`GET /.well-known/oauth-protected-resource`**, **`…/mcp`** — RFC 9728 metadata pointing at `<PUBLIC_URL>/mcp`.
- **`GET /.well-known/oauth-authorization-server`**, **`…/mcp`** — RFC 8414 metadata (`authorization_endpoint`, `token_endpoint`, `registration_endpoint`, `S256`, `none`).
- **`POST /register`** (`cache-control: no-store`) — RFC 7591 JSON (`redirect_uris`: 1–20 `https`, loopback `http`, or app-scheme URIs without fragments; optional `client_name`, `grant_types`). Answers `201`. Caps stored clients at 2000 (LRU eviction).
- **`GET /authorize`** — `response_type=code`, `client_id`, `redirect_uri`, `code_challenge`, `code_challenge_method=S256`, optional `state` (≤ 4096 chars), optional `resource` (`<PUBLIC_URL>` or `<PUBLIC_URL>/mcp`). Returns the HTML pairing page with a 10-char code and QR (10 min TTL).
- **`POST /authorize/poll`** (`cache-control: no-store`) — `{ poll }` (64-hex) → `202 { status: "pending", pollAfterMs }`, `200 { status: "ready", redirect }`, or `410 request_expired`.
- **`POST /token`** (`cache-control: no-store`) — form-encoded `grant_type=authorization_code`, `code`, `client_id`, `redirect_uri`, `code_verifier`, optional `resource` → `200 { access_token: "vtm_…", token_type: "Bearer" }`. Codes live 60 s and are single-use.

## MCP (`/mcp`, `/mcp/{key}`)

Any method on `/mcp` or `/mcp/{key}`. Non-`OPTIONS` calls must present a valid
`vtm_…` key in `Authorization: Bearer …` or the path; otherwise Convex answers
`401` with `WWW-Authenticate: Bearer resource_metadata="<PUBLIC_URL>/.well-known/oauth-protected-resource/mcp"`
and CORS headers. Valid requests (body ≤ 64 KiB, else `413 too_large` with CORS)
are forwarded to `ENGINE_URL` with `x-forwarded-for` set from `PROXY_HOPS`
(timeout 32 s → `504 vtop_timeout`; unreachable → `503 engine_unavailable`, both
with CORS). Engine `401` responses get `WWW-Authenticate` added if missing.

## UDP gateway sign-in (`/v1/app/login`)

Authenticated with a real phone's ES256 access JWT (`Authorization: Bearer <jwt>`;
demo returns `403 demo_unavailable`). Responses carry `cache-control: no-store`.

- **`POST /v1/app/login`** — `{ flow? }` → `200 { status: "ok", session: { cookies, csrf_token, registration_number } }` or `202 { status: "pending", requestId }` (while waiting for forwarded OTP mail). Errors: `409 no_saved_credentials`, `401 invalid_credentials`, `504 vtop_timeout`, `503 server_login_failed`.
- **`GET /v1/app/login/{requestId}`** — polls the pending sign-in (`200`, `202`, `404 request_not_found`, `401 invalid_credentials`, `504 vtop_timeout`, `503 server_login_failed`).

## Internal routes

Each internal route accepts only its own bearer token (`401 unauthorized`
otherwise):

| Route | Token | Request | Response |
|---|---|---|---|
| `POST /internal/udp-keys` | `GATEWAY_TOKEN` | `{ host, ports: [int], keys: [{ id, publicKey, from, until }] }` (1–16 ports, 1–8 unpadded X25519 keys) | `204`; `400 bad_request` |
| `POST /internal/proof-ticket` | `VERIFIER_TOKEN` | `{ ticket }` (32 lower-case hex) | `200 { device }`; `404 challenge_invalid`; `429 too_many_proofs` (ticket stays open) |
| `POST /internal/proof-done` | `VERIFIER_TOKEN` | `{ ticket, regno }` or `{ ticket, code }` | `204`; `404 challenge_invalid`. On `regno`, Convex asks the engine's `/engine/owner-hash` for the mailbox owner hash |
| `POST /internal/mail` | `MAIL_TOKEN` | `{ addressId, receivedAt, otp?, gmailConfirm?: { code?, link?, requester? } }` (`receivedAt` in ms) | `200 { ok: true }` (or `{ ok: true, delivered: 0 }` for unclaimed); `400 bad_request`; `404 mailbox_not_found` |

## Guides (`/guides/{path}`)

Serves the files bundled from `backend/guides/` by `bun dev/gen-guides.ts`
(`forwarding.json`, `updates.json`, `demo.json`, and WebP images under
`forwarding/`). Carries `ETag` and `cache-control: public, max-age=3600`;
honors `If-None-Match` with `304`. JSON files carry `"schema": 1` and are also
readable as Convex queries (`guides:updates`, `guides:forwarding`, `guides:demo`).

## Convex functions (phones & extension)

Phones talk to Convex over `/api/*` using signed `vm1` calls (`auth:proveStart`,
`auth:token`, `vtop:login*`) or their ES256 access JWT (`account:*`, `app:*`,
`auth:signOut`, `devices:*`, `keys:*`, `mail:*`, `pair:info`, `screen:view`,
`sessions:*`, `vtop:fetch*`), plus public queries (`guides:*`, `proxy:info`,
`udp:keys`). TLSNotary account proofs connect to `wss://vmate.aster0.dev/proof?ticket=`
(routed by hawk's Caddy directly to `rust/vtop-verifier`, falling back to the
standby verifier on nest).

## Convex → engine (`rust/vtop-bridge`)

Every Convex → engine call carries `Authorization: Bearer <INTERNAL_TOKEN>` plus
`x-vtop-flow` and `x-vtop-regno` trace headers (except `/mcp`, which preserves
the client's key). The UDP gateway calls `/engine/fetch*` and `/engine/vtop-login*`
with `GATEWAY_TOKEN`.

| Route | Purpose |
|---|---|
| `POST /engine/session` | Validate cookies or a sealed session (`{ cookies }` or `{ sealedSession, account }`) → `SessionChecked`. |
| `POST /engine/seal` | Seal saved credentials (`SealVault`) under `VAULT_KEY` → `{ sealedVault }`. |
| `POST /engine/login` | One step of server sign-in: `{ account, sealedVault, forwarding }` or `{ account, sealedState, otp }` → tagged `LoginStepped` (`ok`, `otp_needed`, `invalid_credentials`, `vault_unusable`, `forwarding_required`, `gmail_revoked`, `failed` with optional `code` + `reason`). |
| `POST /engine/owner-hash` | `{ registrationNumber }` → `{ ownerHash }` (keyed by `VAULT_KEY`). |
| `POST /engine/fetch`, `/engine/fetch-batch` | Fetch one or 1–16 pages on a phone's session (JSON or `application/vnd.vmate.page+br`). |
| `POST /engine/vtop-login`, `…/otp`, `…/resend` | Server-route sign-in for a phone. |
| `POST /internal/mail-otp/owner` | Legacy session owner check (`{ cookies }` → `{ ownerHash, registrationNumber }`). |
| `ANY /mcp`, `/mcp/{key}` | MCP tools (`vtop-mcp`). |

## Configuration

Convex deployment env (listed in `backend/.env.example`, set via `bunx convex env set`):

- **Required:** `JWT_PRIVATE_JWK`, `JWT_PUBLIC_JWK`, `INTERNAL_TOKEN`,
  `MAIL_TOKEN`, `GATEWAY_TOKEN`, `VERIFIER_TOKEN`, `ACCOUNT_TOKEN_SECRET`,
  `ENGINE_URL`, `PUBLIC_URL`, `MAIL_DOMAIN`, `PROXY_HOPS` (`1` behind hawk's
  Caddy).
- **Optional (unset or empty turns off / uses default):** `ALLOWED_ORIGINS`
  (empty allows `*`), `PROXY_URL` (unset disables the Proxy route),
  `FIREBASE_CREDENTIALS_JSON` (unset disables FCM pushes), `FCM_URL` (test
  override; unset or empty uses `https://fcm.googleapis.com`).
