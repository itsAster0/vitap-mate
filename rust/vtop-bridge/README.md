# vtop-bridge

Hands VTOP sessions to the browser extension and the MCP server without
either of them ever seeing a password. A student's phone (VITAP Mate) is the
source of truth: the bridge asks it for cookies over FCM. If the student
turns on offline sign-in, the phone also sends its username, password and
Gmail token, and the bridge can then sign in on its own while the phone is
away.

Accounts are keyed by the VTOP-verified registration number (usernames can
change). Every cookie the bridge accepts is checked against VTOP first, and
the registration number VTOP reports must match the account.

The service is stateless. Everything it keeps lives in Firestore:

| Collection        | What                                                        |
| ----------------- | ----------------------------------------------------------- |
| `accounts`        | FCM token, sealed vault (AES-256-GCM), cached session       |
| `access_keys`     | access keys, stored as SHA-256 hashes                       |
| `bridge_requests` | pending phone requests; TTL on `expiresAt`                  |

Passwords, cookies, tokens and keys are never logged.

## How a session is served

1. A cached session is checked with VTOP (about 0.3 s) and returned if VTOP
   still accepts it as this student's. One VTOP rejects is dropped; if VTOP
   cannot be reached the cache is returned unchecked.
2. Otherwise a pending request is created and filled in the background:
   - saved credentials and *always use saved credentials* (the default) →
     the server signs in itself (Gmail OTP auto-fetch if needed), and asks
     the phone only if that fails;
   - saved credentials without it → the phone is asked first; if it stays
     silent for the chosen wait (10/20/45 s, default 20) or cannot be
     reached, the server signs in itself;
   - no credentials → the phone is asked over FCM; a dead FCM token is
     cleared and the caller gets `phone_unreachable`.

   Credentials VTOP rejects, or a revoked Gmail token, wipe the vault.
3. Callers poll `GET /v1/session/requests/{id}`. A phone answer that arrives
   after the request expired is still cached.

Per-account settings (`POST /v1/account/settings`, app secret required):
`alwaysUseVault`, `phoneWaitSecs`, and `vaultTtlSecs` (86400, 172800, or
null to keep saved credentials until they stop working). With an expiry,
the timer restarts each time the phone resends the credentials, and the
bridge asks for them on every phone request.

Callers that find a cached session rejected call `/v1/session/expire` and
ask again once.

## API

Bearer API (`Authorization: Bearer vtm_…`, 60/min per account):

| Route                              | Purpose                            |
| ---------------------------------- | ---------------------------------- |
| `GET /v1/whoami`                   | registration number for the key    |
| `POST /v1/session`                 | ready cookies, or a pending id     |
| `GET /v1/session/requests/{id}`    | poll a pending request             |
| `POST /v1/session/expire`          | drop the cached session            |

Account API, used by the app. The proof of ownership is a live VTOP session
in the body (10/min per IP):

`POST /v1/link`, `/v1/account`, `/v1/keys`, `/v1/keys/revoke`,
`/v1/account/fcm-token`, `/v1/account/forget-credentials`,
`/v1/account/delete`, `/v1/account/settings`.
Linking returns an app secret; key, phone, settings and delete calls need
it.

Phone side: `POST /cookie/callback` (the app's answer). The legacy
`/cookie` endpoint for old extension builds still asks the phone and records
nothing.

## Configuration

| Variable                    | Default | Notes                                        |
| --------------------------- | ------- | -------------------------------------------- |
| `PUBLIC_BASE_URL`           | Railway | callback URL base sent to phones; defaults to `https://$RAILWAY_PUBLIC_DOMAIN` |
| `FIREBASE_CREDENTIALS_JSON` | —       | required; service account (FCM + Firestore)  |
| `VAULT_KEY`                 | —       | required; base64 of 32 bytes                 |
| `REQUEST_TIMEOUT_SECS`      | 120     | 30–600                                       |
| `SESSION_CACHE_SECS`        | 604800  | upper bound on a cached session (checked anyway) |
| `GMAIL_WAIT_SECS`           | 45      | wait for the OTP mail on server login        |
| `ALLOWED_ORIGINS`           | —       | comma-separated CORS origins                 |
| `PUBLIC_MCP_URL`            | Railway | shown to the app as the MCP URL              |
| `PORT`                      | 8080    | listens on `[::]`                            |

Do not rotate `VAULT_KEY` casually: sealed credentials become unreadable,
are cleared, and the phone is asked again.

## Run and test

```sh
cd rust
cargo test -p vtop-bridge
PUBLIC_BASE_URL=... FIREBASE_CREDENTIALS_JSON="$(cat sa.json)" \
  VAULT_KEY="$(openssl rand -base64 32)" cargo run -p vtop-bridge
```

Railway builds it with Railpack from `rust/railpack.vtop-bridge.json`
(service `vtop-bridge` in `.railway/railway.ts`).
