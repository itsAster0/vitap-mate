# backend

The v3 server state: one self-hosted Convex deployment. It replaces the
edge's Durable Objects. The HTTP contract is [`API.md`](API.md), and the
migration design is `docs/plans/2026-11-23-backend-v3.md`.

## Layout

```
convex/schema.ts        every table; regno only on accounts, the rest link by accountId
convex/auth.config.ts   customJwt provider (PUBLIC_URL + JWT_PUBLIC_JWK)
convex/convex.config.ts Convex app config (@convex-dev/rate-limiter)
convex/auth.ts          TLSNotary tickets (taken and judged by vtop-verifier over /internal/proof-*), auth:token (24 h JWT + non-expiring rotating refresh chain), signOut
convex/account.ts       account:view, account:signIns and the account's settings (cacheSession off by default), phones, sign-ins, delete
convex/app.ts           app:status, app:sessionClear, app:login and app:loginResult (live)
convex/logins.ts        server sign-ins as scheduled engine steps: the OTP mail resumes them
convex/screen.ts        screen:view, the Server screen's one live read (kept for older apps)
convex/proxy.ts         proxy:info, the Proxy route's Worker (PROXY_URL)
convex/pair.ts          pair:info and pairing a browser or AI app
convex/devices.ts       the install itself
convex/sessions.ts      key holders' session requests, sessions:pending, sessions:answer, sessions:scrubRequest (30 s cookie scrub when storage off), sessions:evictMcpCache
convex/keys.ts          access keys for the extension and MCP: named when made, renamed, revoked (revoking evicts MCP RAM cache)
convex/extension.ts     the extension's live queries (extension:key, request, pairing)
convex/guides.ts        public queries serving guides/*.json
convex/identity.ts      internal caller check for actions
convex/mail.ts          forwarding addresses (register, join), the sealed inbox, mail in
convex/oauth.ts         MCP OAuth (register, authorize, token)
convex/udp.ts           udp:keys and /internal/udp-keys gateway reports
convex/vtop.ts          vtop:fetch, fetchBatch and login* on the engine
convex/http.ts          HTTP actions: key routes, pairing, OAuth, /mcp, /v1/app/login, /guides/*, /internal/*
convex/push.ts          FCM (off without FIREBASE_CREDENTIALS_JSON)
convex/sweeps.ts        periodic cleanup backstops for expired rows, run by crons.ts
convex/crons.ts         cron schedules (5 min and 15 min sweeps)
convex/migrate.ts       the one-time backfill from the edge's Durable Objects (admin only)
convex/lib/             pure rules (accountState.ts), crypto, mailSeal, vm1, rateLimits, helpers
guides/                 help JSON and WebP images (bundled into convex/lib/guideFiles.ts by dev/gen-guides.ts)
test/kit.ts             convex-test setup, fake phones, a stub engine
dev/                    gen-guides.ts, guideFiles.ts, demo.ts (anonymiser), smoke.ts
scripts/backfill.ts     reads the edge's /internal/export, imports with migrate:importBatch
```

## Commands

```sh
bun install
bun run check        # tsc
bun run test         # vitest + convex-test
bunx convex deploy   # to CONVEX_SELF_HOSTED_URL in .env.local
bun run smoke        # against https://vmate.aster0.dev (SMOKE_URL to change)
```

`mise run check-backend` runs the check and the tests (unit tests stub their
env in `test/kit.ts`). The deployment's env is listed in `.env.example`.
`.env.local` holds the deploy target and admin key and is gitignored.

## Backfill from the edge

`scripts/backfill.ts` reads every Durable Object of the old edge through its
`POST /internal/export` (on vmate-old.aster0.dev; its code is in the
`flutter`-era history, removed from `main`) and imports it into the deployment in
`.env.local` with the internal mutation `migrate:importBatch`, called with the
admin key. Rows keep their old ids (device id, regno, key hash, address id,
OAuth client id). It prints only counts.

```sh
# EDGE_URL, INTERNAL_TOKEN; CF_API_TOKEN and CF_ACCOUNT_ID to list every object
bun scripts/backfill.ts --dry-run            # counts per kind, imports nothing
bun scripts/backfill.ts --save exports.json  # import; keep the exports (0600) to replay
bun scripts/backfill.ts --from exports.json  # replay saved or synthetic exports
```

Without the Cloudflare API it reads only the names in `--ids ids.json`
(`{ "account": [...], "device": [...], "mailbox": [...] }`), the OAuth object
and what those accounts name. Running it again is safe: missing rows are
inserted, a row the import made is brought up to a newer edge row (the delta
pass after `FREEZE`), and nothing Convex wrote since is undone. Not moved:
session requests, OAuth requests and codes, OTP waiters and boxes, device event
logs (all short-lived), and mailboxes that no account claims. A phone whose
device object is missing stays unlinked until its next `auth:token`.

## Logs

Log lines carry `[flow=… regno=…]` (`log` in `convex/lib/util.ts`). The regno
comes from the JWT or the linked account. Calls to the engine send the same
values as `x-vtop-flow` and `x-vtop-regno`. Never log tokens, cookies, OTPs or
keys.
