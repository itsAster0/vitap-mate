# vtop-edge

The Cloudflare Worker and Durable Objects at `vmate.aster0.dev`, in
TypeScript, run with bun. The repo's `AGENTS.md` and `rust/ARCHITECTURE.md`
still apply; these are the edge's own rules.

## Commands

```sh
bun run test     # vitest
bun run check    # tsc
```

Run both after every change. There is no formatter: match the code around
you.

## Routes

- Every route the Worker answers is in `ROUTES` (`src/routes.ts`), with its
  method and auth kind (`signed`, `key`, `internal`, `none`). `index.ts`
  dispatches them; add a new route to both.
- Every route is documented in `API.md`, in a table headed `Route` or a
  `### METHOD /path` heading. `src/routes.test.ts` fails when `ROUTES`,
  `API.md` and the dispatch disagree.
- Errors go through `error()` in `src/common.ts` (`{ code, error }`, codes
  snake_case; `notFound`, `methodNotAllowed`, `badRequest`); never change a
  code or status a client reads. The table in `API.md` between
  `<!-- routes:start -->` and `<!-- routes:end -->` is made from `ROUTES`
  (with each route's `summary`) by `bun run docs`; the tests fail when stale.
- Formats shared with the engine live in `src/shared/` and mirror
  `vtop-shared`. Their test vectors and sample bodies are in
  `rust/vtop-shared/vectors/`, read by the tests on both sides; change them
  together.

## State

All server state lives in Durable Objects (`Device`, `Mailbox`, `Index`,
`Account`, `FcmToken`, `OAuth`), each in its own SQLite tables. The engine
and the UDP gateway keep none.

## Deploying

`bun run deploy` deploys the edge, the engine and the gateway to production.
Never run it without an explicit OK for that deploy.
