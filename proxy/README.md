# vmate-proxy

The Cloudflare Worker behind the app's Proxy route. A phone on that route signs in to
VTOP and fetches it itself, but through this Worker: the requests are the same, and
VTOP's uncompressed pages reach the phone compressed. Only `/vtop/` paths are forwarded,
to `https://vtop.vitap.ac.in` and nowhere else. It keeps no state and logs no request.

Phones learn its URL from Convex (`proxy:info`, the `PROXY_URL` env), as they learn the
UDP gateway's from `udp:keys`, so it can move without an app update.

```sh
bun install
bun run check && bun run test
bunx wrangler deploy      # vmate-proxy on vmate-proxy.aster0.dev
```

The phone swaps VTOP's origin for this Worker's on each request (vtop-ffi `route.rs`),
and this Worker swaps it back on the way in and out: `Location`, `Origin` and `Referer`
name VTOP to VTOP and the Worker to the phone, and cookies lose their `Domain`. Its own
errors are JSON 502-504, which the phone reads as the route failing (then Direct for
5 minutes); VTOP's errors come through as they are.
