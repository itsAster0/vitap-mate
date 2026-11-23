// Every route the Worker answers (index.ts dispatches them), with who may
// call it and a one-line summary. API.md documents each one, and its route
// table is made from this list (`bun run docs`); routes.test.ts holds the
// three to each other and to the dispatch. `{name}` is a path parameter,
// `/*` a prefix. `/guides/*` is Static Assets and never reaches the Worker.
// No imports with Worker APIs here: `dev/docs.ts` runs it under bun.

/** As in API.md's Auth table. */
export type RouteAuth = "signed" | "key" | "internal" | "none";

export type Route = { method: "GET" | "POST" | "ANY"; path: string; auth: RouteAuth; summary: string };

const get = (path: string, auth: RouteAuth, summary: string): Route => ({ method: "GET", path, auth, summary });
const post = (path: string, auth: RouteAuth, summary: string): Route => ({ method: "POST", path, auth, summary });
const any = (path: string, auth: RouteAuth, summary: string): Route => ({ method: "ANY", path, auth, summary });

export const ROUTES: Route[] = [
  get("/healthz", "none", "Service, version and Cloudflare location."),
  any("/vtop/*", "none", "Compression proxy to VTOP for phones."),
  // Installs. `register` is signed by the key it registers.
  post("/v1/devices/register", "signed", "Registers an install's keys; returns its device id."),
  post("/v1/devices/update", "signed", "Changes the install's FCM token, label or seal key."),
  post("/v1/devices/info", "signed", "The install as the edge keeps it."),
  post("/v1/devices/delete", "signed", "Deletes the install, optionally leaving its address and account."),
  get("/v1/events", "signed", "The install's event log: catch-up, WebSocket or server-sent events."),
  get("/v1/udp", "signed", "The UDP gateway's host, ports and keys."),
  post("/v1/app/status", "signed", "A linked phone's view of its account."),
  post("/v1/app/session/clear", "signed", "Drops the account's cached VTOP session."),
  post("/v1/app/login", "signed", "Has the server sign in with the saved credentials."),
  // Forwarding addresses (index.ts).
  post("/v1/mail-otp/register", "signed", "Makes a forwarding address for this install."),
  post("/v1/mail-otp/delete", "signed", "Leaves the address, or deletes it."),
  post("/v1/mail-otp/leave", "signed", "Leaves the address; an owned one stays for the student."),
  post("/v1/mail-otp/inbox", "signed", "The latest sealed OTP and Gmail confirmation."),
  post("/v1/mail-otp/owner", "signed", "Ties the address to the VTOP student."),
  post("/v1/mail-otp/reclaim", "signed", "Joins this install to the student's address."),
  post("/v1/mail-otp/devices", "signed", "The installs on the address."),
  post("/v1/mail-otp/devices/remove", "signed", "Takes another install off the address."),
  // VTOP on the engine (vtop-phone.ts PHONE_ROUTES).
  post("/v1/vtop/fetch", "signed", "Fetches a page on the engine."),
  post("/v1/vtop/fetch-batch", "signed", "Fetches several pages of a semester in one call."),
  post("/v1/vtop/login", "signed", "Signs in to VTOP on the engine."),
  post("/v1/vtop/login/otp", "signed", "Sends the sign-in OTP."),
  post("/v1/vtop/login/resend", "signed", "Asks VTOP to send the OTP again."),
  // Accounts (account.ts).
  get("/v1/whoami", "key", "The access key's account and settings."),
  post("/v1/session", "key", "A VTOP session: cached, or a pending request."),
  get("/v1/session/requests/{id}", "key", "Polls a pending session request."),
  post("/v1/session/expire", "key", "Drops the cached session."),
  post("/v1/pair", "none", "The extension polls for the key made for its code."),
  post("/v1/pair/info", "none", "Whether a pairing code is a browser's or an MCP client's."),
  // Phones, by session or account token (account.ts APP_OPS).
  post("/v1/link", "signed", "Links this phone to the student's account."),
  post("/v1/keys", "signed", "Makes an access key."),
  post("/v1/keys/revoke", "signed", "Revokes an access key."),
  post("/v1/keys/outings", "signed", "Lets a key apply for outings, or not."),
  post("/v1/keys/move", "signed", "Moves keys from unlinked phones to this one."),
  post("/v1/account", "signed", "The account view."),
  post("/v1/account/forget-credentials", "signed", "Forgets the saved password."),
  post("/v1/account/delete", "signed", "Deletes the account."),
  post("/v1/account/settings", "signed", "Changes the account settings."),
  post("/v1/account/semester", "signed", "Sets the semester agents use."),
  post("/v1/account/remove-phone", "signed", "Unlinks a phone."),
  post("/v1/account/clear-sign-ins", "signed", "Drops the sign-in history."),
  post("/v1/session/answer", "signed", "A phone's answer to a session request."),
  post("/v1/account/credentials-rejected", "signed", "VTOP rejected the password on the phone."),
  // Account proofs (account-proof.ts).
  post("/v1/account/prove/start", "signed", "A challenge for an account proof."),
  get("/v1/account/prove", "signed", "The account proof's WebSocket, relayed to the engine; ends with the account token."),
  // OAuth for MCP clients (oauth.ts).
  get("/.well-known/oauth-protected-resource", "none", "RFC 9728 resource metadata."),
  get("/.well-known/oauth-protected-resource/mcp", "none", "RFC 9728 resource metadata."),
  get("/.well-known/oauth-authorization-server", "none", "RFC 8414 server metadata."),
  get("/.well-known/oauth-authorization-server/mcp", "none", "RFC 8414 server metadata."),
  get("/authorize", "none", "The pairing page for an MCP client."),
  post("/register", "none", "Registers an MCP client (RFC 7591)."),
  post("/token", "none", "Swaps a code and PKCE verifier for an access key."),
  post("/authorize/poll", "none", "The pairing page polls for approval."),
  // Passed on to the engine, which checks the key.
  any("/mcp", "key", "MCP, passed on to the engine."),
  any("/mcp/{key}", "key", "MCP with the key in the path, passed on to the engine."),
  // The engine and the UDP gateway.
  post("/internal/mail-otp/await", "internal", "The engine waits for a student's next OTP."),
  post("/internal/udp-keys", "internal", "The UDP gateway reports its keys (GATEWAY_TOKEN)."),
  post("/internal/accounts/login-done", "internal", "The engine reports a server sign-in."),
];

/** The Markdown table of `routes`, as API.md's generated block holds it. */
export function routeTable(routes: Route[] = ROUTES): string {
  const cell = (text: string) => text.replace(/\|/g, "\\|");
  const rows = routes.map((r) => `| ${r.method} | \`${r.path}\` | ${r.auth} | ${cell(r.summary)} |`);
  return ["| Method | Path | Auth | Summary |", "|---|---|---|---|", ...rows].join("\n");
}

export const ROUTES_START = "<!-- routes:start -->";
export const ROUTES_END = "<!-- routes:end -->";

/** `markdown` with the block between the route markers made anew. */
export function withRouteTable(markdown: string, routes: Route[] = ROUTES): string {
  const start = markdown.indexOf(ROUTES_START);
  const end = markdown.indexOf(ROUTES_END);
  if (start < 0 || end < start) throw new Error(`API.md needs ${ROUTES_START} … ${ROUTES_END}`);
  const note = "<!-- Made from src/routes.ts by `bun run docs`; do not edit. -->";
  return `${markdown.slice(0, start)}${ROUTES_START}\n${note}\n\n${routeTable(routes)}\n\n${markdown.slice(end)}`;
}
