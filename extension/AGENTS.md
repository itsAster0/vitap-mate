# extension/AGENTS.md

The vitapmate browser extension for Chrome and Firefox (Svelte 5, Vite,
Tailwind, bun from the root `mise.toml`). It pairs with the app for an access
key and signs the browser in to VTOP with a session from the server
(`backend/API.md`). It never sees a password. The root `AGENTS.md` rules
apply.

```
src/main.ts, App.svelte, app.css   popup UI
src/background.ts                  service worker (an event page on Firefox)
src/content.ts                     content script on VTOP
src/bridge.ts, cookies.ts, state.ts  server calls, cookie install, stored state (*.test.ts next to each)
public/manifest.json, icons/       MV3 manifest; vite.config.ts writes host_permissions
```

```sh
bun install
bun run check && bun run test && bun run build   # or `mise run check-ext`; dist/
bun run build:firefox                            # dist-firefox/
```

`.env` is tracked and holds only `VITE_BRIDGE_URL`, the server URL. Keep
secrets out of it and out of the bundle.
