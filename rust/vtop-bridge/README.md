# vtop-bridge (the engine)

The VTOP work behind the backend (`backend/`), run on nest as `vmate-engine`
(with the Railway `engine-v2` service as a standby) from `../Dockerfile.engine`; the backend reaches it at its
`ENGINE_URL` (nest at http://100.126.222.47:8090 over NetBird). It stores nothing: accounts,
keys, devices, mailboxes and pending requests live in the backend's Convex
deployment. The engine only:

- checks whose a VTOP session is (`/engine/session`);
- seals saved credentials for an account (`/engine/seal`);
- signs in with saved credentials one step per call (`/engine/login`):
  whole with the OTP read from the student's Gmail, or stopping at VTOP's
  OTP and resuming when Convex sends the forwarded code;
- fetches and parses pages on a phone's own session for "Fetch on the
  server", one (`/engine/fetch`) or several in one round trip
  (`/engine/fetch-batch`), keeping nothing;
- signs a phone in for that mode with the password it sends each time
  (`/engine/vtop-login`, `/engine/vtop-login/otp`, `/engine/vtop-login/resend`);
- checks a mailbox owner for the edge (`/internal/mail-otp/owner`);
- serves the MCP tools (`/mcp`, from `vtop-mcp`).

Every route except `/healthz` and `/mcp` needs the secret shared with the
edge (`Authorization: Bearer …`). Passwords, cookies, OTPs and keys are
never logged.

## Who can read what

Saved credentials and cached sessions are sealed with AES-256-GCM under
`VAULT_KEY` (see `vault.rs`); the associated data names the account, so a
sealed value copied into another account does not open. The edge stores
only the sealed values, but `VAULT_KEY` is set on the engine's Railway
service, so whoever controls that service can open them. Sealing protects against a leak of stored data, not against the
operator.

Credentials are only sent by the phone when the student turns on server
sign-in and the server can read the OTP: OTP forwarding is set up, or
(without forwarding) Gmail is connected, in which case the phone sends its
Gmail refresh token and the OAuth client it was issued to (the app's own,
or the student's own key with its secret) along with them. They are sealed
together, so the engine needs no Google settings of its own. When the
phone's OTP auto-fill is Gmail (`otp: "gmail"`), the engine reads Gmail even
with a forwarding address; otherwise forwarding comes first and Gmail is only
used without one. When Google
refuses the token, only the Gmail access is dropped (`gmail_revoked`); the
password stays. Credentials are dropped when VTOP rejects them, when the
account's `vaultTtlSecs` runs out, or when the student forgets them from
the app.

## Configuration

Every variable is required, with no defaults in code: the deploy's env files
(`deploy/hawk/compose.yaml` and the files `deploy/hawk/up.sh` writes) hold each value.

| Variable            | Notes                                        |
| ------------------- | -------------------------------------------- |
| `VAULT_KEY`         | base64 of 32 bytes                           |
| `CONVEX_URL`        | Convex's HTTP actions                        |
| `INTERNAL_TOKEN`    | shared with Convex (bearer, both ways), 32+ characters |
| `GATEWAY_TOKEN`     | the UDP gateway's own bearer, 32+ characters; taken only on `/engine/fetch*` and `/engine/vtop-login*` |
| `OTP_WAIT_SECS`     | wait for the OTP mail (forwarded or Gmail) on sign-in, e.g. 45 |
| `PORT`              | listens on `[::]`, IPv4 where that fails, e.g. 8080 |

Do not rotate `VAULT_KEY` casually: sealed credentials become unreadable,
are cleared, and the phone is asked again.

## Run and test

```sh
cd rust
cargo test -p vtop-bridge
VAULT_KEY="$(openssl rand -base64 32)" CONVEX_URL=http://localhost:3211 \
  INTERNAL_TOKEN="$(openssl rand -hex 32)" GATEWAY_TOKEN="$(openssl rand -hex 32)" \
  OTP_WAIT_SECS=45 PORT=8080 cargo run -p vtop-bridge
```

Deployed by `bun run deploy` in `deploy/` (`alchemy.run.ts`), which builds
`rust/Dockerfile.engine` for nest and the Railway service `engine-v2`.
