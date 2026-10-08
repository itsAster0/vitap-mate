# VITAP Mate – VTOP Sign In (Chrome extension)

Signs Chrome in to VTOP with an access key from the VITAP Mate app
(More → Connected apps). The key goes to vtop-bridge, which hands back a VTOP
session from the cache, the phone, or a server-side sign-in; the extension
installs the cookies and opens VTOP. It never sees a password.

## Server

The bridge address comes from `.env`:

```
VITE_BRIDGE_URL=https://vtop-bridge.aster0.dev
```

The build writes its origin into the manifest's `host_permissions`. Students
can point the extension at another bridge under **Server** in the side panel;
Chrome asks for permission to that host (`optional_host_permissions`), and
**Use default** goes back to the built-in one.

## Develop

```sh
pnpm install
pnpm test     # vitest
pnpm check    # svelte-check
pnpm build    # dist/ — load it with chrome://extensions → Load unpacked
```
