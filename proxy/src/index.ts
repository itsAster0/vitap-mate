// vmate-proxy: VTOP's /vtop/ paths, compressed, for phones on the Proxy route. It keeps no
// state and sees only what the phone would send VTOP itself. Convex hands phones its URL
// (`proxy:info`), so it can move without an app update.

import { proxyVtop } from "./proxy";

export default {
  fetch: (req: Request) => proxyVtop(req),
} satisfies ExportedHandler;
