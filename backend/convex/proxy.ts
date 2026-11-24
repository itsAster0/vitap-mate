// The Proxy route's Worker (`proxy/`): where phones send VTOP's requests on that route. Its URL
// is `PROXY_URL` here, read by phones with `proxy:info`, so the Worker can move without a
// release.

import { query } from "./_generated/server";
import { fail } from "./lib/errors";

/** `PROXY_URL` if it is an https origin (no path), else null. */
export function proxyOrigin(raw: string | undefined): string | null {
  try {
    const url = new URL(raw ?? "");
    return url.protocol === "https:" && url.pathname === "/" && !url.search ? url.origin : null;
  } catch {
    return null;
  }
}

/** `{ url }`: the proxy's origin; `proxy_unavailable` when none is set. Public, like `udp:keys`. */
export const info = query({
  args: {},
  handler: async (): Promise<{ url: string }> => {
    const url = proxyOrigin(process.env.PROXY_URL);
    return url ? { url } : fail("proxy_unavailable", "The proxy is not set up.");
  },
});
