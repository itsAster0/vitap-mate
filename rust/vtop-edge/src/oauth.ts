// OAuth for MCP clients (MCP authorization, 2025-06-18): Claude and other
// clients connect to `/mcp` without anyone pasting a key.
//
// - Discovery: `/mcp` answers 401 with `WWW-Authenticate: Bearer
//   resource_metadata=…` (RFC 9728), which names this Worker as the
//   authorization server (RFC 8414 metadata).
// - `POST /register`: dynamic client registration (RFC 7591), public
//   clients only. Clients are kept in the `OAuth` object, at most
//   `MAX_CLIENTS`; the one unused longest makes room.
// - `GET /authorize`: OAuth 2.1 with PKCE (S256 only). No password: the
//   page shows a pairing code and its QR. The student scans or types it in
//   the app (Connected apps → Pair a browser), which makes an access key
//   with `pairCode` as for the extension (pair.ts); the key is named after
//   the client and cannot apply for outings until the student allows it.
//   The page polls `POST /authorize/poll` and, once the key is made, goes
//   to the client's `redirect_uri` with a one-time code.
// - `POST /token`: the code and its PKCE verifier for the key itself, as a
//   bearer token that does not expire. Revoking the key in the app ends it,
//   so there are no refresh tokens.
//
// Codes, verifiers and keys are kept only as their SHA-256 where they need
// not be read back, and never logged.

import { DurableObject } from "cloudflare:workers";
import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, randomBytes, utf8ToBytes } from "@noble/hashes/utils.js";

import { type Env, mcpUrl, now, str, variable } from "./common";
import { pairCode } from "./pair";
import { qrSvg } from "./qr";
import { toBase64Url } from "./shared/base64";
import { hash, hashesEqual } from "./shared/tokens";
import * as sql from "./sql";

/** How long the authorize page waits for the app. */
export const REQUEST_TTL_SECS = 600;
/** How long a code from `/authorize` may be swapped at `/token`. */
export const GRANT_TTL_SECS = 60;
export const MAX_CLIENTS = 2000;
const MAX_REDIRECT_URIS = 5;
const MAX_URI_CHARS = 512;
const MAX_NAME_CHARS = 40;
const MAX_STATE_CHARS = 512;
const POLL_AFTER_MS = 2000;
/** Crockford base32, as pair.ts reads codes. */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

const SCHEMA = `
CREATE TABLE clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    redirect_uris TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    used_at INTEGER NOT NULL
);
CREATE INDEX clients_used ON clients (used_at);
CREATE TABLE requests (
    code_hash TEXT PRIMARY KEY,
    poll_hash TEXT NOT NULL UNIQUE,
    client_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    challenge TEXT NOT NULL,
    state TEXT,
    expires_at INTEGER NOT NULL,
    access_key TEXT
);
CREATE TABLE grants (
    code_hash TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    challenge TEXT NOT NULL,
    access_key TEXT NOT NULL,
    expires_at INTEGER NOT NULL
)`;

export type Client = { id: string; name: string; redirectUris: string[] };

/** What the app is shown before it approves a code. */
export type PendingApp = { name: string; host: string };

export type Opened = { pairCode: string; poll: string; expiresAt: number };

export type Polled = { status: "pending" } | { status: "ready"; redirect: string } | { status: "expired" };

export type Redeemed = { key: string } | { error: string; description: string };

function hostOf(uri: string): string {
  try {
    const url = new URL(uri);
    return url.host || `${url.protocol}//`;
  } catch {
    return uri;
  }
}

function isLoopback(url: URL): boolean {
  return url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
}

/** A redirect URI a client may register: https, http on a loopback
 * address (RFC 8252), or an app's own scheme; never with a fragment. */
export function redirectUriOk(uri: string): boolean {
  if (uri.length > MAX_URI_CHARS) return false;
  let url: URL;
  try {
    url = new URL(uri);
  } catch {
    return false;
  }
  if (url.hash !== "" || uri.includes("#")) return false;
  if (url.protocol === "https:") return url.hostname !== "";
  if (url.protocol === "http:") return isLoopback(url);
  // Private-use schemes (`cursor://…`, `com.example.app:/cb`); not ones a
  // browser runs or reads files with.
  const scheme = url.protocol.slice(0, -1);
  return /^[a-z][a-z0-9+.-]*$/.test(scheme) && !["javascript", "data", "file", "blob", "vbscript", "about", "ws", "wss", "ftp"].includes(scheme);
}

/** Whether `asked` is one of the registered URIs: exactly, except that a
 * loopback URI may use any port (RFC 8252 §7.3). */
export function redirectMatches(registered: string[], asked: string): boolean {
  if (registered.includes(asked)) return true;
  let url: URL;
  try {
    url = new URL(asked);
  } catch {
    return false;
  }
  if (!isLoopback(url)) return false;
  url.port = "";
  return registered.some((uri) => {
    try {
      const known = new URL(uri);
      if (!isLoopback(known)) return false;
      known.port = "";
      return known.href === url.href;
    } catch {
      return false;
    }
  });
}

/** base64url(SHA-256(verifier)), as PKCE S256 wants it. */
export function s256(verifier: string): string {
  return toBase64Url(sha256(utf8ToBytes(verifier)));
}

function newPairCode(): string {
  return [...randomBytes(10)].map((b) => ALPHABET[b & 31]).join("");
}

export function oauthStub(env: Env) {
  return env.OAUTH.getByName("oauth");
}

/** Clients, the authorize requests waiting for the app, and the codes they
 * end in: one object, as there are few. */
export class OAuth extends DurableObject<Env> {
  private ready = false;

  private get db(): SqlStorage {
    const db = this.ctx.storage.sql;
    if (!this.ready) {
      sql.prepare(db, SCHEMA);
      this.ready = true;
    }
    return db;
  }

  /** Stores a client and returns its id, dropping the one unused longest
   * when full. */
  async register(name: string, redirectUris: string[], at: number): Promise<string> {
    const count = sql.int(sql.first(this.db, "SELECT COUNT(*) AS n FROM clients") ?? {}, "n") ?? 0;
    if (count >= MAX_CLIENTS) {
      this.db.exec(
        "DELETE FROM clients WHERE id IN (SELECT id FROM clients ORDER BY used_at LIMIT ?)",
        count - MAX_CLIENTS + 1,
      );
    }
    const id = `vmc_${bytesToHex(randomBytes(16))}`;
    this.db.exec(
      "INSERT INTO clients (id, name, redirect_uris, created_at, used_at) VALUES (?, ?, ?, ?, ?)",
      id,
      name,
      JSON.stringify(redirectUris),
      at,
      at,
    );
    return id;
  }

  async client(id: string): Promise<Client | null> {
    const row = sql.first(this.db, "SELECT * FROM clients WHERE id = ?", id);
    const name = row && sql.text(row, "name");
    const uris = row && sql.text(row, "redirect_uris");
    if (!row || name === null || uris === null) return null;
    return { id, name, redirectUris: JSON.parse(uris) as string[] };
  }

  /** A request from `/authorize` (checked already) that waits for the app. */
  async open(clientId: string, redirectUri: string, challenge: string, state: string | null, at: number): Promise<Opened> {
    this.sweep(at);
    this.db.exec("UPDATE clients SET used_at = ? WHERE id = ?", at, clientId);
    const code = newPairCode();
    const poll = bytesToHex(randomBytes(32));
    const expiresAt = at + REQUEST_TTL_SECS;
    this.db.exec(
      "INSERT INTO requests (code_hash, poll_hash, client_id, redirect_uri, challenge, state, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
      hash(code),
      hash(poll),
      clientId,
      redirectUri,
      challenge,
      state,
      expiresAt,
    );
    await this.wakeAt(expiresAt);
    return { pairCode: code, poll, expiresAt };
  }

  /** The client behind a pairing code still waiting for the app; null when
   * the code is not one of these (it may be the extension's). */
  async pending(code: string, at: number): Promise<PendingApp | null> {
    const row = sql.first(
      this.db,
      "SELECT c.name, r.redirect_uri FROM requests r JOIN clients c ON c.id = r.client_id WHERE r.code_hash = ? AND r.expires_at > ? AND r.access_key IS NULL",
      hash(code),
      at,
    );
    const name = row && sql.text(row, "name");
    const uri = row && sql.text(row, "redirect_uri");
    return name === null || uri === null ? null : { name, host: hostOf(uri) };
  }

  /** Hands the app's new key to the waiting request; false when there is
   * none (expired, or approved already). */
  async approve(code: string, key: string, at: number): Promise<boolean> {
    const found = this.db.exec(
      "UPDATE requests SET access_key = ? WHERE code_hash = ? AND expires_at > ? AND access_key IS NULL RETURNING code_hash",
      key,
      hash(code),
      at,
    );
    return found.toArray().length > 0;
  }

  /** The authorize page's poll: once approved, the request becomes a code
   * for the client and the page is told where to go. */
  async poll(poll: string, issuer: string, at: number): Promise<Polled> {
    const row = sql.first(this.db, "SELECT * FROM requests WHERE poll_hash = ?", hash(poll));
    if (!row || (sql.int(row, "expires_at") ?? 0) <= at) return { status: "expired" };
    const key = sql.text(row, "access_key");
    if (key === null) return { status: "pending" };
    const redirectUri = sql.text(row, "redirect_uri")!;
    const state = sql.text(row, "state");
    const code = bytesToHex(randomBytes(32));
    this.db.exec("DELETE FROM requests WHERE poll_hash = ?", hash(poll));
    this.db.exec(
      "INSERT INTO grants (code_hash, client_id, redirect_uri, challenge, access_key, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
      hash(code),
      sql.text(row, "client_id"),
      redirectUri,
      sql.text(row, "challenge"),
      key,
      at + GRANT_TTL_SECS,
    );
    await this.wakeAt(at + GRANT_TTL_SECS);
    const url = new URL(redirectUri);
    url.searchParams.set("code", code);
    if (state !== null) url.searchParams.set("state", state);
    url.searchParams.set("iss", issuer);
    return { status: "ready", redirect: url.href };
  }

  /** Swaps a code for its key, once: the code is gone after any try. */
  async redeem(code: string, clientId: string, redirectUri: string, verifier: string, at: number): Promise<Redeemed> {
    const codeHash = hash(code);
    const row = sql.first(this.db, "SELECT * FROM grants WHERE code_hash = ?", codeHash);
    this.db.exec("DELETE FROM grants WHERE code_hash = ?", codeHash);
    const bad = { error: "invalid_grant", description: "The code is unknown, used or expired." };
    if (!row || (sql.int(row, "expires_at") ?? 0) <= at) return bad;
    if (sql.text(row, "client_id") !== clientId) return bad;
    if (sql.text(row, "redirect_uri") !== redirectUri) {
      return { error: "invalid_grant", description: "redirect_uri does not match the authorization request." };
    }
    if (!hashesEqual(s256(verifier), sql.text(row, "challenge") ?? "")) {
      return { error: "invalid_grant", description: "code_verifier does not match the code_challenge." };
    }
    return { key: sql.text(row, "access_key")! };
  }

  private sweep(at: number): void {
    this.db.exec("DELETE FROM requests WHERE expires_at <= ?", at);
    this.db.exec("DELETE FROM grants WHERE expires_at <= ?", at);
  }

  private async wakeAt(at: number): Promise<void> {
    const due = (at + 1) * 1000;
    const set = await this.ctx.storage.getAlarm();
    if (set === null || set > due) await this.ctx.storage.setAlarm(due);
  }

  override async alarm(): Promise<void> {
    const at = now();
    this.sweep(at);
    const next = sql.int(
      sql.first(
        this.db,
        "SELECT MIN(expires_at) AS at FROM (SELECT expires_at FROM requests UNION ALL SELECT expires_at FROM grants)",
      ) ?? {},
      "at",
    );
    if (next !== null) await this.ctx.storage.setAlarm((next + 1) * 1000);
  }
}

// ---- routes -------------------------------------------------------------------

function issuer(env: Env): string {
  return variable(env, "PUBLIC_URL");
}

function resourceMetadataUrl(env: Env): string {
  return `${issuer(env)}/.well-known/oauth-protected-resource/mcp`;
}

/** The `WWW-Authenticate` value `/mcp` answers 401 with. */
export function challenge(env: Env): string {
  return `Bearer resource_metadata="${resourceMetadataUrl(env)}"`;
}

/** `/mcp` with no bearer token: where to get one. */
export function unauthorized(env: Env): Response {
  return Response.json(
    { code: "unauthorized", error: "Connect with OAuth or an access key from the app." },
    { status: 401, headers: { "www-authenticate": challenge(env) } },
  );
}

function oauthError(error: string, description: string, status = 400): Response {
  return Response.json({ error, error_description: description }, { status, headers: { "cache-control": "no-store" } });
}

function cors(response: Response): Response {
  const out = new Response(response.body, response);
  out.headers.set("access-control-allow-origin", "*");
  out.headers.set("access-control-allow-methods", "GET, POST, OPTIONS");
  out.headers.set("access-control-allow-headers", "Content-Type, Authorization, MCP-Protocol-Version");
  out.headers.set("access-control-expose-headers", "WWW-Authenticate");
  return out;
}

export function protectedResource(env: Env): Response {
  return Response.json({
    resource: mcpUrl(env),
    authorization_servers: [issuer(env)],
    bearer_methods_supported: ["header"],
    resource_name: "vitapmate",
  });
}

export function authorizationServer(env: Env): Response {
  const base = issuer(env);
  return Response.json({
    issuer: base,
    authorization_endpoint: `${base}/authorize`,
    token_endpoint: `${base}/token`,
    registration_endpoint: `${base}/register`,
    response_types_supported: ["code"],
    grant_types_supported: ["authorization_code"],
    code_challenge_methods_supported: ["S256"],
    token_endpoint_auth_methods_supported: ["none"],
    authorization_response_iss_parameter_supported: true,
  });
}

/** `POST /register` (RFC 7591), for public clients. */
export async function register(req: Request, env: Env): Promise<Response> {
  let body: Record<string, unknown>;
  try {
    const parsed: unknown = await req.json();
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error();
    body = parsed as Record<string, unknown>;
  } catch {
    return oauthError("invalid_client_metadata", "The body must be a JSON object.");
  }
  const uris = body.redirect_uris;
  if (
    !Array.isArray(uris) ||
    uris.length === 0 ||
    uris.length > MAX_REDIRECT_URIS ||
    !uris.every((u) => typeof u === "string" && redirectUriOk(u))
  ) {
    return oauthError(
      "invalid_redirect_uri",
      `redirect_uris must be 1 to ${MAX_REDIRECT_URIS} https, loopback http or app-scheme URIs without a fragment.`,
    );
  }
  const grants = body.grant_types;
  if (grants !== undefined && (!Array.isArray(grants) || !grants.includes("authorization_code"))) {
    return oauthError("invalid_client_metadata", "Only the authorization_code grant is supported.");
  }
  let name = [...(str(body.client_name)?.trim().replace(/\s+/g, " ") || "MCP app")].slice(0, MAX_NAME_CHARS).join("");
  // The app reads a key named "Browser" as the extension's.
  if (name.toLowerCase() === "browser") name = "MCP app";
  const at = now();
  const id = await oauthStub(env).register(name, uris as string[], at);
  console.log("oauth: registered a client");
  return Response.json(
    {
      client_id: id,
      client_id_issued_at: at,
      client_name: name,
      redirect_uris: uris,
      grant_types: ["authorization_code"],
      response_types: ["code"],
      // Public clients: whatever was asked for, there is no secret.
      token_endpoint_auth_method: "none",
    },
    { status: 201, headers: { "cache-control": "no-store" } },
  );
}

/** The resource a client may ask a token for (RFC 8707): this `/mcp`, or
 * the Worker itself. */
function resourceOk(env: Env, resource: string): boolean {
  const trim = (u: string) => u.replace(/\/+$/, "");
  return [mcpUrl(env), issuer(env)].map(trim).includes(trim(resource));
}

/** `GET /authorize`: checks the request, then shows the pairing page. */
export async function authorize(req: Request, env: Env): Promise<Response> {
  const params = new URL(req.url).searchParams;
  const clientId = params.get("client_id") ?? "";
  const redirectUri = params.get("redirect_uri") ?? "";
  const client = clientId ? await oauthStub(env).client(clientId) : null;
  // Without a known client and one of its redirect URIs, nothing is sent
  // anywhere (RFC 6749 §4.1.2.1).
  if (client === null) return page("Unknown app", "<p>This app is not registered here. Start connecting again from the app.</p>", 400);
  if (!redirectMatches(client.redirectUris, redirectUri)) {
    return page("Wrong return address", "<p>This app asked to come back to an address it did not register.</p>", 400);
  }
  const state = params.get("state");
  const back = (error: string, description: string) => {
    const url = new URL(redirectUri);
    url.searchParams.set("error", error);
    url.searchParams.set("error_description", description);
    if (state !== null) url.searchParams.set("state", state);
    url.searchParams.set("iss", issuer(env));
    return new Response(null, { status: 302, headers: { location: url.href } });
  };
  if (params.get("response_type") !== "code") return back("unsupported_response_type", "Only response_type=code is supported.");
  const codeChallenge = params.get("code_challenge") ?? "";
  if (params.get("code_challenge_method") !== "S256" || !/^[A-Za-z0-9_-]{43}$/.test(codeChallenge)) {
    return back("invalid_request", "PKCE with code_challenge_method=S256 is required.");
  }
  if (state !== null && state.length > MAX_STATE_CHARS) return back("invalid_request", "state is too long.");
  const resource = params.get("resource");
  if (resource !== null && !resourceOk(env, resource)) return back("invalid_target", "Unknown resource.");
  const opened = await oauthStub(env).open(client.id, redirectUri, codeChallenge, state, now());
  console.log("oauth: opened an authorize request");
  return pairingPage(client, hostOf(redirectUri), opened);
}

/** `POST /authorize/poll { poll }`, from the pairing page. */
export async function poll(req: Request, env: Env): Promise<Response> {
  let id = "";
  try {
    id = str(((await req.json()) as Record<string, unknown>)?.poll) ?? "";
  } catch {
    // Read as unknown below.
  }
  if (!/^[0-9a-f]{64}$/.test(id)) return Response.json({ status: "expired" }, { status: 400 });
  const polled = await oauthStub(env).poll(id, issuer(env), now());
  if (polled.status === "ready") console.log("oauth: an authorize request was approved");
  const headers = { "cache-control": "no-store" };
  if (polled.status === "pending") return Response.json({ ...polled, pollAfterMs: POLL_AFTER_MS }, { status: 202, headers });
  return Response.json(polled, { status: polled.status === "ready" ? 200 : 410, headers });
}

/** `POST /token`: authorization_code with PKCE, form-encoded. */
export async function token(req: Request, env: Env): Promise<Response> {
  let form: URLSearchParams;
  try {
    form = new URLSearchParams(await req.text());
  } catch {
    return oauthError("invalid_request", "The body must be form-encoded.");
  }
  const grantType = form.get("grant_type");
  if (grantType !== "authorization_code") {
    return oauthError("unsupported_grant_type", "Only authorization_code is supported.");
  }
  const code = form.get("code") ?? "";
  const clientId = form.get("client_id") ?? "";
  const redirectUri = form.get("redirect_uri") ?? "";
  const verifier = form.get("code_verifier") ?? "";
  if (!code || !clientId || !redirectUri) return oauthError("invalid_request", "code, client_id and redirect_uri are required.");
  if (!/^[A-Za-z0-9._~-]{43,128}$/.test(verifier)) return oauthError("invalid_request", "code_verifier is missing or malformed.");
  const resource = form.get("resource");
  if (resource !== null && !resourceOk(env, resource)) return oauthError("invalid_target", "Unknown resource.");
  const redeemed = await oauthStub(env).redeem(code, clientId, redirectUri, verifier, now());
  if ("error" in redeemed) return oauthError(redeemed.error, redeemed.description);
  console.log("oauth: issued a token");
  return Response.json(
    { access_token: redeemed.key, token_type: "Bearer" },
    { headers: { "cache-control": "no-store", pragma: "no-cache" } },
  );
}

/** `POST /v1/pair/info { code }`, from the app before it approves: the MCP
 * client behind an authorize page's code, or `{ kind: "browser" }`. */
export async function pairInfo(body: Record<string, unknown> | null, env: Env): Promise<Response> {
  const code = pairCode(str(body?.code));
  if (code === null) return Response.json({ code: "bad_request", error: "code must be a pairing code." }, { status: 400 });
  const app = await oauthStub(env).pending(code, now());
  return Response.json(app ? { kind: "mcp", ...app } : { kind: "browser" });
}

/** The OAuth routes, by `METHOD path`; null when the path is not one. */
export async function handle(req: Request, env: Env, path: string): Promise<Response | null> {
  const { method } = req;
  const open = [
    "/.well-known/oauth-protected-resource",
    "/.well-known/oauth-protected-resource/mcp",
    "/.well-known/oauth-authorization-server",
    "/.well-known/oauth-authorization-server/mcp",
    "/register",
    "/token",
  ];
  if (open.includes(path)) {
    if (method === "OPTIONS") return cors(new Response(null, { status: 204 }));
    if (path.startsWith("/.well-known/oauth-protected-resource")) return cors(method === "GET" ? protectedResource(env) : notAllowed());
    if (path.startsWith("/.well-known/")) return cors(method === "GET" ? authorizationServer(env) : notAllowed());
    if (method !== "POST") return cors(notAllowed());
    return cors(path === "/register" ? await register(req, env) : await token(req, env));
  }
  if (path === "/authorize") return method === "GET" ? authorize(req, env) : notAllowed();
  if (path === "/authorize/poll") return method === "POST" ? poll(req, env) : notAllowed();
  return null;
}

function notAllowed(): Response {
  return Response.json({ code: "method_not_allowed", error: "Method not allowed." }, { status: 405 });
}

// ---- the page -----------------------------------------------------------------

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

const STYLE = `
:root{color-scheme:dark;--bg:#0f172a;--card:#1e293b;--line:rgba(99,102,241,.4);--tint:rgba(99,102,241,.1);--text:#e2e8f0;--muted:#94a3b8;--code:#e0e7ff}
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--text);font:15px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;padding:16px}
main{width:100%;max-width:380px;background:var(--card);border-radius:16px;padding:24px;text-align:center}
h1{font-size:19px;margin:0 0 4px}p{margin:8px 0;color:var(--muted);font-size:13px}
.pair{border:1px solid var(--line);background:var(--tint);border-radius:12px;padding:12px;margin:16px 0}
.pair svg{width:144px;height:144px;border-radius:6px;display:block;margin:4px auto}
.code{font:600 20px ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:.15em;color:var(--code);margin:6px 0 0}
ol{text-align:left;color:var(--muted);font-size:13px;padding-left:20px;margin:8px 0}b{color:var(--text)}`;

function page(title: string, body: string, status = 200, script = ""): Response {
  const nonce = bytesToHex(randomBytes(16));
  const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="referrer" content="no-referrer"><title>${escapeHtml(title)} · vitapmate</title><style nonce="${nonce}">${STYLE}</style></head><body><main><h1>${escapeHtml(title)}</h1>${body}</main>${script ? `<script nonce="${nonce}">${script}</script>` : ""}</body></html>`;
  return new Response(html, {
    status,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "content-security-policy": `default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'; img-src data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`,
      "x-frame-options": "DENY",
      "referrer-policy": "no-referrer",
    },
  });
}

function pairingPage(client: Client, host: string, opened: Opened): Response {
  const shown = `${opened.pairCode.slice(0, 5)}-${opened.pairCode.slice(5)}`;
  const body = `<p><b>${escapeHtml(client.name)}</b> (${escapeHtml(host)}) wants to use your VTOP data through vitapmate.</p>
<div class="pair">${qrSvg(shown) ?? ""}<p class="code" aria-label="Pairing code">${shown}</p></div>
<ol><li>Open the vitapmate app</li><li>Connected apps → <b>Pair a browser</b></li><li>Scan the QR code or enter the code shown here</li></ol>
<p id="status">Waiting for the app… The code works for ${Math.round(REQUEST_TTL_SECS / 60)} minutes.</p>`;
  // The poll id is 64 hex characters, safe to put in the script as is.
  const script = `(function(){var s=document.getElementById("status");function tick(){fetch("/authorize/poll",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({poll:"${opened.poll}"})}).then(function(r){return r.json()}).then(function(j){if(j.status==="ready"){s.textContent="Approved. Going back to the app…";location.replace(j.redirect)}else if(j.status==="pending"){setTimeout(tick,j.pollAfterMs||2000)}else{s.textContent="The code has expired. Start connecting again from the app."}}).catch(function(){setTimeout(tick,4000)})}setTimeout(tick,2000)})();`;
  return page(`Connect ${client.name}`, body, 200, script);
}
