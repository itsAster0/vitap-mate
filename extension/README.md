# VITAP Mate – VTOP Sign In (Chrome extension)

Signs Chrome in to VTOP with an access key from the VITAP Mate app. The
extension shows a pairing code; the app (Connected apps → Pair a browser)
makes a key for it, and the extension collects the key from the server
(`POST /v1/pair`, see `rust/vtop-edge/API.md`). A key pasted by hand works
too. With the key, the server hands back a VTOP session from its cache, the
phone, or a server-side sign-in; the extension installs the cookies and
opens VTOP. It never sees a password.

## Server

The server (vtop-edge) address comes from `.env`:

```
VITE_BRIDGE_URL=https://vmate.aster0.dev
```

The build writes its origin into the manifest's `host_permissions`. Students
can point the extension at another server under **Settings** in the popup;
Chrome asks for permission to that host (`optional_host_permissions`), and
**Use default** goes back to the built-in one.

## Develop

Bun comes from mise (`mise.toml` at the repo root).

```sh
bun install
bun run test     # vitest
bun run check    # svelte-check
bun run build    # dist/ — load it with chrome://extensions → Load unpacked
bun run build:firefox
```
