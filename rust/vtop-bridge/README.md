# vtop-bridge (the engine)

The VTOP work behind the Cloudflare edge (`vtop-edge`), run on Railway from
`../Dockerfile.engine` (`RAILWAY_DOCKERFILE_PATH`); the edge reaches it at its
`ENGINE_URL`. It stores nothing: accounts,
keys, devices, mailboxes and pending requests live in the edge's Durable
Objects. The engine only:

- checks whose a VTOP session is (`/engine/session`);
- seals saved credentials for an account (`/engine/seal`);
- signs in with saved credentials, reading the OTP from the student's
  forwarded mail (`/engine/login`);
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
sign-in and OTP forwarding is set up. They are dropped when VTOP rejects
them, when the account's `vaultTtlSecs` runs out, or when the student
forgets them from the app.

## Configuration

Set as variables on the Railway service:

| Variable            | Default | Notes                                        |
| ------------------- | ------- | -------------------------------------------- |
| `VAULT_KEY`         | —       | required; base64 of 32 bytes                 |
| `EDGE_URL`          | —       | required; the edge's public URL              |
| `MAIL_RELAY_SECRET` | —       | required; the edge's `BRIDGE_SECRET`, 32+ characters |
| `OTP_WAIT_SECS`     | 45      | wait for the forwarded OTP mail on sign-in   |
| `PORT`              | 8080    | listens on `[::]`, IPv4 where that fails     |

Do not rotate `VAULT_KEY` casually: sealed credentials become unreadable,
are cleared, and the phone is asked again.

## Run and test

```sh
cd rust
cargo test -p vtop-bridge
VAULT_KEY="$(openssl rand -base64 32)" EDGE_URL=http://localhost:8787 \
  MAIL_RELAY_SECRET="$(openssl rand -hex 32)" cargo run -p vtop-bridge
```

Deployed with the edge: `npx wrangler deploy` in `rust/vtop-edge` builds
`rust/Dockerfile.engine`.
