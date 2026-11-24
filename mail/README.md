# vmate-mail

The Cloudflare Email Worker for backend v3. Mail to the forwarding addresses
(`otp+<id>@vmate.aster0.dev`) comes here. The Worker keeps no state: it reads the
address id, the VTOP OTP or Gmail's forwarding confirmation (the same parsing as
the v2 edge's) and POSTs it to Convex. The contract is in
`docs/plans/2026-11-23-backend-v3-api.md`, under "Mail Worker → Convex".

```
POST {CONVEX_URL}/internal/mail
Authorization: Bearer <MAIL_TOKEN>
{ "addressId", "receivedAt", "otp", "gmailConfirm", "from", "subject" }
```

`receivedAt` is in milliseconds. It is the mail's own `Date:`, capped at the moment the
mail arrived, so a retry carries the same time. `gmailConfirm` is
`{ code, link, requester }` or null.

| Convex answers | The Worker |
|---|---|
| `200` | is done |
| `404 mailbox_not_found` | forwards the mail to `FALLBACK_FORWARD`, or rejects it when that is empty |
| anything else, or no answer | puts the body on the `vmate-mail-retry` Queue |

The queue consumer tries again after 10, 20, 40, 80 and then every 120 seconds,
and drops the mail 10 minutes after it was first queued, since an OTP is
useless by then. Mail that is neither an OTP nor a confirmation is dropped. Mail
to any other address goes to `FALLBACK_FORWARD`.

## Settings

| Name | Kind | What |
|---|---|---|
| `CONVEX_URL` | var (`wrangler.toml`) | Convex's HTTP actions, `https://vmate.aster0.dev` |
| `FALLBACK_FORWARD` | var | where other mail goes; empty rejects it |
| `MAIL_TOKEN` | secret | the same value as `MAIL_TOKEN` in the Convex env |
| `MAIL_QUEUE` | Queue binding | `vmate-mail-retry` |

`.env.example` lists them. For `wrangler dev`, put them in `.dev.vars`.

## Commands

```sh
bun install
bun run test
bun run check
```

Deploy only when asked (see the top of `wrangler.toml`): create the queue once,
set the secret, `bunx wrangler deploy`, then point the Email Routing rule
`otp@vmate.aster0.dev` at `vmate-mail` instead of `vmate-edge`.
