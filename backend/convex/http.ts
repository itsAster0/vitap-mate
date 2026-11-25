// HTTP actions on the same paths as the old edge (API.md), so the
// extension, MCP clients, the engine, the gateway and the mail Worker don't
// change: key routes, pairing, OAuth, `/mcp`, `/internal/*` and `/healthz`.
// Phones use functions over the socket instead (see the contract,
// docs/plans/2026-11-23-backend-v3-api.md).

import { httpRouter } from "convex/server";
import { rateLimits } from "./lib/rateLimits";
import { internal } from "./_generated/api";
import type { ActionCtx } from "./_generated/server";
import { KEY_PREFIX, type KeyVia, ready } from "./lib/accountState";
import { fromBase64 } from "./lib/base64";
import { randomHex, sha256Hex } from "./lib/crypto";
import { GUIDE_FILES } from "./lib/guideFiles";
import { badRequest, empty, error } from "./lib/errors";
import { engineJson, engineSend } from "./lib/engine";
import { bearer, clientIp, corsRoutes, holds, jsonBody, notInternal, readBodyCapped, route } from "./lib/httpKit";
import { signAccessJwt } from "./lib/jwt";
import { seal } from "./lib/mailSeal";
import { page, pairingPage } from "./lib/oauthPage";
import { ALPHABET, POLL_AFTER_MS, pairCode } from "./lib/pairing";
import { accountId, env, logError, logWarn, now, str } from "./lib/util";
import { actionCaller } from "./lib/identity";
import { hostOf, redirectMatches, redirectUriOk, s256 } from "./oauth";
import { begin, cachedCookies } from "./sessions";
import { type LoginAnswer, loginFor } from "./app";
import { parseGateway } from "./udp";

export const VERSION = "3.2.0";

const http = httpRouter();
const get = (path: string, handler: (ctx: ActionCtx, req: Request) => Promise<Response>) => http.route({ path, method: "GET", handler: route(handler) });
const post = (path: string, handler: (ctx: ActionCtx, req: Request) => Promise<Response>) => http.route({ path, method: "POST", handler: route(handler) });

const MAX_MCP_BODY_BYTES = 64 * 1024;
const MCP_TIMEOUT_MS = 32_000;

function decodeSegment(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    return badRequest("Malformed URL encoding.");
  }
}

/** Checks per-IP limit first so one throttled caller cannot burn the global allowance. */
async function rateLimited(
  ctx: ActionCtx,
  req: Request,
  ipLimit: "oauthRegisterIp" | "oauthAuthorizeIp" | "oauthPollIp" | "oauthTokenIp" | "pairIp",
  globalLimit: "oauthRegister" | "oauthAuthorize" | "oauthPoll" | "oauthToken" | "pairGlobal",
): Promise<boolean> {
  if (!(await rateLimits.limit(ctx, ipLimit, { key: clientIp(req) ?? "unknown" })).ok) return true;
  return !(await rateLimits.limit(ctx, globalLimit)).ok;
}

get("/healthz", async () => Response.json({ service: "vmate-convex", version: VERSION }));

// ---- key holders (extension, MCP) -----------------------------------------------------------

const keyUnknown = () => error(401, "key_unknown", "Unknown or revoked access key.");

/** The SHA-256 of the access key on the request, or null when it has none. */
async function keyHashOf(req: Request): Promise<string | null> {
  const presented = bearer(req);
  return presented.startsWith(KEY_PREFIX) ? sha256Hex(presented) : null;
}

/** Which app a key call comes from: vtop-mcp says so in `x-vtop-client`; anything else is the
 * browser extension. */
function keyVia(req: Request): KeyVia {
  return req.headers.get("x-vtop-client")?.trim().toLowerCase() === "mcp" ? "mcp" : "extension";
}

for (const spec of [
  ...corsRoutes("GET", { path: "/v1/whoami" }, async (ctx, req) => {
    const keyHash = await keyHashOf(req);
    const view = keyHash && (await ctx.runQuery(internal.sessions.whoami, { keyHash }));
    return view ? Response.json(view) : keyUnknown();
  }),
  ...corsRoutes("POST", { path: "/v1/session" }, async (ctx, req) => {
    const keyHash = await keyHashOf(req);
    const started = keyHash && (await ctx.runMutation(internal.sessions.start, { keyHash, via: keyVia(req) }));
    if (!started) return keyUnknown();
    if (started.limited) return error(429, "rate_limited", "Too many requests for this account.", { headers: { "retry-after": "60" } });
    const cached = await cachedCookies(ctx, started.accountId, started.regno, started.cached);
    console.log(`[regno=${started.regno}] account: a key asked for a session (${cached ? "cached" : "new request"})`);
    if (cached) return Response.json(ready(cached.cookies, started.source));
    const opened = await ctx.runMutation(internal.sessions.open, { accountId: started.accountId, keyHash, requestId: crypto.randomUUID() });
    if (opened.opened) await begin(ctx, opened.requestId, opened.opened);
    const state = await ctx.runQuery(internal.sessions.state, { requestId: opened.requestId });
    return Response.json(state?.startedView ?? opened.joined, { status: 202, headers: { "retry-after": "2" } });
  }),
  ...corsRoutes("GET", { pathPrefix: "/v1/session/requests/" }, async (ctx, req) => {
    const keyHash = await keyHashOf(req);
    const accountId = keyHash && (await ctx.runQuery(internal.sessions.keyHolder, { keyHash }));
    if (!accountId) return keyUnknown();
    const requestId = decodeSegment(new URL(req.url).pathname.slice("/v1/session/requests/".length)).trim();
    const { status, body } = await ctx.runQuery(internal.sessions.poll, { accountId, requestId });
    return Response.json(body, { status });
  }),
  ...corsRoutes("POST", { path: "/v1/session/expire" }, async (ctx, req) => {
    const keyHash = await keyHashOf(req);
    const accountId = keyHash && (await ctx.runQuery(internal.sessions.keyHolder, { keyHash }));
    if (!accountId) return keyUnknown();
    await ctx.runMutation(internal.sessions.clearSession, { accountId });
    return empty();
  }),
  // A key JWT for the extension's socket (extension.ts). `no-store`: it is a credential.
  ...corsRoutes("POST", { path: "/v1/key/token" }, async (ctx, req) => {
    const keyHash = await keyHashOf(req);
    if (!keyHash) return keyUnknown();
    const ok = await ctx.runMutation(internal.extension.tokenFor, { keyHash });
    if (!ok) return keyUnknown();
    if (ok.limited) return error(429, "rate_limited", "Too many token requests for this account.", { headers: { "retry-after": "60" } });
    const { jwt, expiresAt } = await signAccessJwt({ sub: keyHash, kind: "key" }, now());
    return Response.json({ jwt, expiresAt }, { headers: { "cache-control": "no-store", pragma: "no-cache" } });
  }),
  // The extension polls for the key the app made with its code.
  ...corsRoutes("POST", { path: "/v1/pair" }, async (ctx, req) => {
    if (await rateLimited(ctx, req, "pairIp", "pairGlobal")) {
      return error(429, "rate_limited", "Too many pairing requests; slow down.", { headers: { "retry-after": "10" } });
    }
    const code = pairCode(str((await jsonBody(req))?.code));
    if (code === null) return error(400, "bad_request", "code must be a pairing code.");
    const key = await ctx.runMutation(internal.pair.take, { codeHash: await sha256Hex(code) });
    if (key === null) return Response.json({ status: "pending", pollAfterMs: POLL_AFTER_MS }, { status: 202 });
    console.log("pair: handed a key to the extension");
    return Response.json({ status: "ready", key });
  }),
]) {
  http.route(spec);
}

// ---- OAuth for MCP clients ------------------------------------------------------------------

const issuer = () => env("PUBLIC_URL");
const mcpUrl = () => `${issuer()}/mcp`;
const challenge = () => `Bearer resource_metadata="${issuer()}/.well-known/oauth-protected-resource/mcp"`;

/** `/register` and `/token` answer RFC 6749's `{ error, error_description }`. */
function oauthError(code: string, description: string, status = 400): Response {
  return Response.json({ error: code, error_description: description }, { status, headers: { "cache-control": "no-store" } });
}

function cors(response: Response): Response {
  const out = new Response(response.body, response);
  out.headers.set("access-control-allow-origin", "*");
  out.headers.set("access-control-allow-methods", "GET, POST, OPTIONS");
  out.headers.set("access-control-allow-headers", "Content-Type, Authorization, MCP-Protocol-Version");
  out.headers.set("access-control-expose-headers", "WWW-Authenticate");
  return out;
}

const resourceMetadata = async () =>
  cors(Response.json({ resource: mcpUrl(), authorization_servers: [issuer()], bearer_methods_supported: ["header"], resource_name: "vitapmate" }));

const serverMetadata = async () => {
  const base = issuer();
  return cors(
    Response.json({
      issuer: base,
      authorization_endpoint: `${base}/authorize`,
      token_endpoint: `${base}/token`,
      registration_endpoint: `${base}/register`,
      response_types_supported: ["code"],
      grant_types_supported: ["authorization_code"],
      code_challenge_methods_supported: ["S256"],
      token_endpoint_auth_methods_supported: ["none"],
      authorization_response_iss_parameter_supported: true,
    }),
  );
};

for (const path of ["/.well-known/oauth-protected-resource", "/.well-known/oauth-protected-resource/mcp"]) get(path, resourceMetadata);
for (const path of ["/.well-known/oauth-authorization-server", "/.well-known/oauth-authorization-server/mcp"]) get(path, serverMetadata);
for (const path of [
  "/.well-known/oauth-protected-resource",
  "/.well-known/oauth-protected-resource/mcp",
  "/.well-known/oauth-authorization-server",
  "/.well-known/oauth-authorization-server/mcp",
  "/register",
  "/token",
]) {
  http.route({ path, method: "OPTIONS", handler: route(async () => cors(empty())) });
}

/** The resource a client may ask a token for (RFC 8707): `/mcp`, or this host. */
const resourceOk = (resource: string) => {
  const trim = (u: string) => u.replace(/\/+$/, "");
  return [mcpUrl(), issuer()].map(trim).includes(trim(resource));
};

post("/register", async (ctx, req) => {
  const body = await jsonBody(req);
  if (!body) return cors(oauthError("invalid_client_metadata", "The body must be a JSON object."));
  const uris = body.redirect_uris;
  if (!Array.isArray(uris) || uris.length === 0 || uris.length > 20 || !uris.every((u) => typeof u === "string" && redirectUriOk(u))) {
    return cors(oauthError("invalid_redirect_uri", "redirect_uris must be 1 to 20 https, loopback http or app-scheme URIs without a fragment."));
  }
  const grants = body.grant_types;
  if (grants !== undefined && (!Array.isArray(grants) || !grants.includes("authorization_code"))) {
    return cors(oauthError("invalid_client_metadata", "Only the authorization_code grant is supported."));
  }
  let name = [...(str(body.client_name)?.trim().replace(/\s+/g, " ") || "MCP app")].slice(0, 40).join("");
  // The app reads a key named "Browser" as the extension's.
  if (name.toLowerCase() === "browser") name = "MCP app";
  if (await rateLimited(ctx, req, "oauthRegisterIp", "oauthRegister")) {
    return cors(error(429, "rate_limited", "Too many apps registered just now; try again later.", { headers: { "retry-after": "600" } }));
  }
  const at = now();
  const clientId = `vmc_${randomHex(16)}`;
  await ctx.runMutation(internal.oauth.register, { clientId, name, redirectUris: uris as string[], at });
  console.log("oauth: registered a client");
  return cors(
    Response.json(
      {
        client_id: clientId,
        client_id_issued_at: at,
        client_name: name,
        redirect_uris: uris,
        grant_types: ["authorization_code"],
        response_types: ["code"],
        token_endpoint_auth_method: "none",
      },
      { status: 201, headers: { "cache-control": "no-store" } },
    ),
  );
});

get("/authorize", async (ctx, req) => {
  if (await rateLimited(ctx, req, "oauthAuthorizeIp", "oauthAuthorize")) {
    const limited = page("Too many requests", "<p>Too many sign-in attempts right now. Try again in a few minutes.</p>", 429);
    limited.headers.set("retry-after", "60");
    return limited;
  }
  const params = new URL(req.url).searchParams;
  const clientId = params.get("client_id") ?? "";
  const redirectUri = params.get("redirect_uri") ?? "";
  const client = clientId ? await ctx.runQuery(internal.oauth.client, { clientId }) : null;
  // Without a known client and one of its redirect URIs, nothing is sent anywhere (RFC 6749
  // §4.1.2.1).
  if (client === null) return page("Unknown app", "<p>This app is not registered here. Start connecting again from the app.</p>", 400);
  if (!redirectMatches(client.redirectUris, redirectUri)) {
    return page("Wrong return address", "<p>This app asked to come back to an address it did not register.</p>", 400);
  }
  const state = params.get("state");
  const back = (code: string, description: string) => {
    const url = new URL(redirectUri);
    url.searchParams.set("error", code);
    url.searchParams.set("error_description", description);
    if (state !== null) url.searchParams.set("state", state);
    url.searchParams.set("iss", issuer());
    return new Response(null, { status: 302, headers: { location: url.href } });
  };
  if (params.get("response_type") !== "code") return back("unsupported_response_type", "Only response_type=code is supported.");
  const codeChallenge = params.get("code_challenge") ?? "";
  if (params.get("code_challenge_method") !== "S256" || !/^[A-Za-z0-9_-]{43}$/.test(codeChallenge)) {
    return back("invalid_request", "PKCE with code_challenge_method=S256 is required.");
  }
  if (state !== null && state.length > 4096) return back("invalid_request", "state is too long.");
  const resource = params.get("resource");
  if (resource !== null && !resourceOk(resource)) return back("invalid_target", "Unknown resource.");
  const code = Array.from(crypto.getRandomValues(new Uint8Array(10)), (b) => ALPHABET[b & 31]).join("");
  const poll = randomHex(32);
  await ctx.runMutation(internal.oauth.open, {
    clientId: client.id,
    redirectUri,
    challenge: codeChallenge,
    state,
    codeHash: await sha256Hex(code),
    pollHash: await sha256Hex(poll),
    at: now(),
  });
  console.log("oauth: opened an authorize request");
  return pairingPage(client, hostOf(redirectUri), { pairCode: code, poll });
});

post("/authorize/poll", async (ctx, req) => {
  if (await rateLimited(ctx, req, "oauthPollIp", "oauthPoll")) {
    return error(429, "rate_limited", "Too many poll requests; slow down.", { headers: { "retry-after": "10", "cache-control": "no-store" } });
  }
  const id = str((await jsonBody(req))?.poll) ?? "";
  if (!/^[0-9a-f]{64}$/.test(id)) return error(400, "bad_request", "poll must be a poll id.", { fields: { status: "expired" } });
  const grant = randomHex(32);
  const polled = await ctx.runMutation(internal.oauth.poll, { pollHash: await sha256Hex(id), grantHash: await sha256Hex(grant), at: now() });
  const headers = { "cache-control": "no-store" };
  if (polled.status === "pending") return Response.json({ status: "pending", pollAfterMs: POLL_AFTER_MS }, { status: 202, headers });
  if (polled.status === "expired") return error(410, "request_expired", "The sign-in request expired.", { fields: { status: "expired" }, headers });
  console.log("oauth: an authorize request was approved");
  const url = new URL(polled.redirectUri);
  url.searchParams.set("code", grant);
  if (polled.state !== null) url.searchParams.set("state", polled.state);
  url.searchParams.set("iss", issuer());
  return Response.json({ status: "ready", redirect: url.href }, { headers });
});

post("/token", async (ctx, req) => {
  if (await rateLimited(ctx, req, "oauthTokenIp", "oauthToken")) {
    return cors(error(429, "rate_limited", "Too many token requests; try again later.", { headers: { "retry-after": "60", "cache-control": "no-store" } }));
  }
  const form = new URLSearchParams(await req.text());
  if (form.get("grant_type") !== "authorization_code") return cors(oauthError("unsupported_grant_type", "Only authorization_code is supported."));
  const code = form.get("code") ?? "";
  const clientId = form.get("client_id") ?? "";
  const redirectUri = form.get("redirect_uri") ?? "";
  const verifier = form.get("code_verifier") ?? "";
  if (!code || !clientId || !redirectUri) return cors(oauthError("invalid_request", "code, client_id and redirect_uri are required."));
  if (!/^[A-Za-z0-9._~-]{43,128}$/.test(verifier)) return cors(oauthError("invalid_request", "code_verifier is missing or malformed."));
  const resource = form.get("resource");
  if (resource !== null && !resourceOk(resource)) return cors(oauthError("invalid_target", "Unknown resource."));
  const redeemed = await ctx.runMutation(internal.oauth.redeem, {
    codeHash: await sha256Hex(code),
    clientId,
    redirectUri,
    verifierS256: await s256(verifier),
    at: now(),
  });
  if ("error" in redeemed) return cors(oauthError(redeemed.error, redeemed.description));
  console.log("oauth: issued a token");
  return cors(Response.json({ access_token: redeemed.key, token_type: "Bearer" }, { headers: { "cache-control": "no-store", pragma: "no-cache" } }));
});

// ---- MCP --------------------------------------------------------------------------------------

/** `/mcp` and `/mcp/<key>`: the key is checked here, then the request goes to the engine as it
 * came, with the caller's address for its rate limits. A client with no key is told where to
 * get one (MCP authorization), and so are the engine's own 401s. */
async function mcp(ctx: ActionCtx, req: Request): Promise<Response> {
  const url = new URL(req.url);
  const pathKey = url.pathname.startsWith("/mcp/") ? url.pathname.slice("/mcp/".length).split("/")[0].trim() : "";
  const unauthorized = (code: string, message: string) =>
    error(401, code, message, { headers: { "www-authenticate": challenge() } });
  if (req.method !== "OPTIONS") {
    const presented = pathKey || bearer(req);
    if (!presented) return unauthorized("unauthorized", "Connect with OAuth or an access key from the app.");
    const accountId = presented.startsWith(KEY_PREFIX) ? await ctx.runQuery(internal.sessions.keyHolder, { keyHash: await sha256Hex(presented) }) : null;
    if (!accountId) return unauthorized("key_unknown", "Unknown or revoked access key.");
  }
  const headers = new Headers();
  for (const [name, value] of req.headers) {
    if (!["host", "x-forwarded-for", "x-real-ip", "content-length"].includes(name) && !name.startsWith("cf-")) headers.append(name, value);
  }
  const ip = clientIp(req);
  if (ip) headers.set("x-forwarded-for", ip);
  const init: RequestInit = { method: req.method, headers, signal: AbortSignal.timeout(MCP_TIMEOUT_MS) };
  if (!["GET", "HEAD", "OPTIONS"].includes(req.method)) {
    const body = await readBodyCapped(req, MAX_MCP_BODY_BYTES);
    if (body === null) return error(413, "too_large", "The request is too large.");
    if (body.byteLength > 0) init.body = body;
  }
  let response: Response;
  try {
    response = await engineSend(`${url.pathname}${url.search}`, init);
  } catch (e) {
    if (e instanceof Error && (e.name === "TimeoutError" || e.name === "AbortError")) {
      logWarn({}, "mcp: engine timed out");
      return error(504, "vtop_timeout", "The MCP tool took too long to answer.");
    }
    logError({}, "mcp: engine unreachable", e);
    return error(503, "engine_unavailable", "The server can't reach the MCP tools right now.");
  }
  if (response.status !== 401 || response.headers.has("www-authenticate")) return response;
  const out = new Response(response.body, response);
  out.headers.set("www-authenticate", challenge());
  return out;
}

for (const method of ["GET", "POST", "DELETE", "OPTIONS", "PUT", "PATCH"] as const) {
  http.route({ path: "/mcp", method, handler: route(mcp) });
  http.route({ pathPrefix: "/mcp/", method, handler: route(mcp) });
}

// ---- the engine, the gateway and the mail Worker ------------------------------------------------

// ---- the UDP gateway's sign-in for a phone (`app:login` over HTTP, with the phone's JWT) ----------

const LOGIN_ERROR_STATUS: Record<string, number> = {
  invalid_credentials: 401,
  request_not_found: 404,
  vtop_timeout: 504,
};

/** A sign-in answer for the UDP gateway: pending is `202`, followed at its `requestId`. */
function loginReply(answer: LoginAnswer): Response {
  if (answer.status === "error") return error(LOGIN_ERROR_STATUS[answer.code] ?? 503, answer.code, answer.error);
  return Response.json(answer, { status: answer.status === "ok" ? 200 : 202 });
}

post("/v1/app/login", async (ctx, req) => loginReply(await loginFor(ctx, str((await jsonBody(req))?.flow) ?? "udp")));

http.route({
  pathPrefix: "/v1/app/login/",
  method: "GET",
  handler: route(async (ctx, req) => {
    const me = await actionCaller(ctx);
    if (!me.account) return error(403, "demo_unavailable", "The demo account can't do this.");
    const requestId = decodeSegment(new URL(req.url).pathname.slice("/v1/app/login/".length)).trim();
    return loginReply(await ctx.runQuery(internal.app.loginRequest, { accountId: me.account._id, requestId }));
  }),
});

/** The internal token, or the route's own. */
function internalRoute(path: string, token: () => Array<string | undefined>, handler: (ctx: ActionCtx, req: Request) => Promise<Response>) {
  post(path, async (ctx, req) => {
    for (const t of token()) if (await holds(req, t)) return handler(ctx, req);
    return notInternal();
  });
}

internalRoute("/internal/udp-keys", () => [process.env.GATEWAY_TOKEN], async (ctx, req) => {
  const gateway = parseGateway(await jsonBody(req));
  if (!gateway) return error(400, "bad_request", "Expected { host, ports, keys: [{ id, publicKey, from, until }] }.");
  await ctx.runMutation(internal.udp.report, gateway);
  console.log(`udp: the gateway reported ${gateway.keys.length} key(s)`);
  return empty();
});

// The verifier's own token opens these two routes and nothing else.
const verifierTokens = () => [process.env.VERIFIER_TOKEN];

internalRoute("/internal/proof-ticket", verifierTokens, async (ctx, req) => {
  const ticket = str((await jsonBody(req))?.ticket) ?? "";
  const taken = await ctx.runMutation(internal.auth.takeTicket, { ticket });
  if (taken && "busy" in taken) return error(429, "too_many_proofs", "The server is proving many accounts right now; try again in a minute.");
  return taken ? Response.json(taken) : error(404, "challenge_invalid", "Unknown, used or expired ticket.");
});

/** A proved regno's mailbox owner hash, from the engine: it is keyed by `VAULT_KEY`, which
 * seals the vaults and stays with the engine. Undefined when the engine can't say: the account
 * keeps the hash it has, and a new one gets it at its next proof. */
async function ownerHashOf(regno: string): Promise<string | undefined> {
  try {
    const { ownerHash } = await engineJson<{ ownerHash?: unknown }>("/engine/owner-hash", { registrationNumber: regno }, { regno });
    return str(ownerHash);
  } catch (e) {
    logError({ regno }, "auth: no owner hash for a proof", e);
    return undefined;
  }
}

internalRoute("/internal/proof-done", verifierTokens, async (ctx, req) => {
  const body = await jsonBody(req);
  const ticket = str(body?.ticket);
  const regno = str(body?.regno);
  const code = str(body?.code);
  if (!ticket || (!code && !regno)) return error(400, "bad_request", "Expected { ticket, regno } or { ticket, code }.");
  const ownerHash = regno && !code ? await ownerHashOf(accountId(regno)) : undefined;
  const done = await ctx.runMutation(internal.auth.proofDone, { ticket, regno, ownerHash, code });
  return done ? empty() : error(404, "challenge_invalid", "Unknown, used or expired ticket.");
});

/** A mail from the mail Worker (`mail/`): sealed here to each install on the address, then stored
 * and pushed; an OTP also goes to the student's server sign-ins waiting for it. */
internalRoute("/internal/mail", () => [process.env.MAIL_TOKEN], async (ctx, req) => {
  const body = await jsonBody(req);
  const addressId = str(body?.addressId) ?? "";
  const receivedAt = body?.receivedAt;
  if (!body || typeof receivedAt !== "number" || !Number.isFinite(receivedAt)) return error(400, "bad_request", "Invalid JSON body.");
  const at = now();
  const sentAt = Math.max(0, Math.min(Math.floor(receivedAt / 1000), at));
  const otp = str(body.otp);
  const confirm = body.gmailConfirm as Record<string, unknown> | null | undefined;
  let payload: Record<string, unknown>;
  if (otp && /^[0-9]{6}$/.test(otp)) {
    payload = { kind: "otp", code: otp, sentAt };
  } else if (confirm && typeof confirm === "object") {
    const field = (k: string) => (typeof confirm[k] === "string" ? (confirm[k] as string) : null);
    if (field("code") === null && field("link") === null) return error(400, "bad_request", "gmailConfirm needs a code or a link.");
    payload = { kind: "gmail_confirm", code: field("code"), link: field("link"), requester: field("requester"), sentAt };
  } else {
    return error(400, "bad_request", "Expected an otp or a gmailConfirm.");
  }
  let recipients = await ctx.runQuery(internal.mail.recipients, { addressId });
  if (!recipients) {
    // An old address waiting for its student's first proof: taken and dropped, as no install can
    // open it yet. A bounce could make Gmail turn the forwarding off.
    if (await ctx.runMutation(internal.mail.noteUnclaimed, { addressId, kind: payload.kind as "otp" | "gmail_confirm", sentAt })) {
      return Response.json({ ok: true, delivered: 0 });
    }
    // Claimed in between the two reads: deliver it.
    recipients = await ctx.runQuery(internal.mail.recipients, { addressId });
    if (!recipients) return error(404, "mailbox_not_found", "No such forwarding address.");
  }
  const json = JSON.stringify(payload);
  const plaintext = new TextEncoder().encode(json);
  const boxes = [];
  for (const { deviceId, sealKey } of recipients.members) {
    const sealed = await seal(sealKey, addressId, plaintext);
    if (sealed !== null) boxes.push({ deviceId, sealed });
  }
  const kind = payload.kind as "otp" | "gmail_confirm";
  const code = kind === "otp" ? { otp: payload.code as string } : {};
  const stored = await ctx.runMutation(internal.mail.deliver, { addressId, kind, sentAt, plaintext: kind === "gmail_confirm" ? json : "", boxes, ...code });
  if (!stored) return error(404, "mailbox_not_found", "No such forwarding address.");
  console.log(`[regno=${recipients.regno}] mail-otp: relayed a ${kind} to ${boxes.length} device(s)`);
  return Response.json({ ok: true });
});

// The help content (guides/, bundled by dev/gen-guides.ts), as the v2 edge's Static Assets
// served it: phones keep a copy and revalidate with the ETag.
http.route({
  pathPrefix: "/guides/",
  method: "GET",
  handler: route(async (_ctx, req) => {
    const file = GUIDE_FILES[decodeSegment(new URL(req.url).pathname.slice("/guides/".length))];
    if (!file) return error(404, "not_found", "No such route.");
    const headers = { "cache-control": "public, max-age=3600", etag: file.etag };
    if (req.headers.get("if-none-match") === file.etag) return new Response(null, { status: 304, headers });
    return new Response(new Uint8Array(fromBase64(file.body)!), { headers: { ...headers, "content-type": file.type } });
  }),
});

// Every other path: the edge's `not_found`, not Convex's page.
for (const method of ["GET", "POST", "DELETE", "PUT", "PATCH"] as const) {
  http.route({ pathPrefix: "/", method, handler: route(async () => error(404, "not_found", "No such route.")) });
}

export default http;
