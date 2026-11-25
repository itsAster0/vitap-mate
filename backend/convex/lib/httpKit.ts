// What the HTTP actions share: bearer tokens, the internal tokens, CORS, and
// a ConvexError turned back into the edge's `{ code, error }` reply.

import type { RouteSpec } from "convex/server";
import { type ActionCtx, httpAction } from "../_generated/server";
import { hashesEqual, sha256Hex } from "./crypto";
import { empty, error, errorData } from "./errors";
import { env, logError, parseObject } from "./util";

/** Number of trusted reverse proxies in front of Convex (`PROXY_HOPS`, required, >= 1). */
export function proxyHops(): number {
  const raw = env("PROXY_HOPS");
  if (!/^[1-9][0-9]*$/.test(raw)) throw new Error("PROXY_HOPS must be a positive integer");
  const n = Number(raw);
  if (!Number.isSafeInteger(n) || n < 1) throw new Error("PROXY_HOPS must be a positive integer");
  return n;
}

/** Client address from `X-Forwarded-For`, taking the entry `hops` from the end (written by our
 * outermost proxy; earlier entries came from the caller and can say anything). Fewer entries:
 * the first there is. Null when the header is missing or empty. */
export function clientIp(req: Request, hops = proxyHops()): string | null {
  const raw = req.headers.get("x-forwarded-for");
  if (!raw) return null;
  const entries = raw
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
  if (entries.length === 0) return null;
  return entries[Math.max(0, entries.length - hops)] ?? null;
}

export function bearer(req: Request): string {
  const value = req.headers.get("authorization") ?? "";
  return value.startsWith("Bearer ") ? value.slice("Bearer ".length).trim() : "";
}

/** The caller holds `token`. Compared as hashes, so the time taken says nothing about it. */
export async function holds(req: Request, token: string | undefined): Promise<boolean> {
  return !!token && hashesEqual(await sha256Hex(token), await sha256Hex(bearer(req)));
}

/** Whether header `name` holds `token` (compared as hashes, like [holds]). */
export async function headerHolds(req: Request, name: string, token: string | undefined): Promise<boolean> {
  return !!token && hashesEqual(await sha256Hex(token), await sha256Hex(req.headers.get(name) ?? ""));
}

/** An address as a caller may state it: IPv4 or IPv6 characters only, at most 45 of them. */
export function addressOf(value: string | null): string | null {
  const address = value?.trim() ?? "";
  return /^[0-9A-Fa-f:.]{2,45}$/.test(address) ? address : null;
}

export const notInternal = () => error(401, "unauthorized", "The internal token is required.");

export async function jsonBody(req: Request): Promise<Record<string, unknown> | null> {
  return parseObject(await req.text());
}

/** HTTP status for an error code thrown by a function. */
const STATUS: Record<string, number> = {
  bad_request: 400,
  unauthenticated: 401,
  device_removed: 401,
  key_unknown: 401,
  not_linked: 403,
  demo_unavailable: 403,
  request_not_found: 404,
  mailbox_not_found: 404,
  no_saved_credentials: 409,
  rate_limited: 429,
};

/** Reads `req`'s body up to `limit` bytes, returning null if `content-length` or the streamed
 * bytes exceed it. */
export async function readBodyCapped(req: Request, limit: number): Promise<ArrayBuffer | null> {
  const declared = Number(req.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > limit) return null;
  if (!req.body) return new ArrayBuffer(0);
  const reader = req.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > limit) {
        await reader.cancel();
        return null;
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.byteLength;
  }
  return out.buffer;
}

async function handle(ctx: ActionCtx, req: Request, handler: (ctx: ActionCtx, req: Request) => Promise<Response>): Promise<Response> {
  try {
    return await handler(ctx, req);
  } catch (e) {
    const data = errorData(e);
    if (data) return Response.json(data, { status: STATUS[data.code] ?? 400 });
    // `/mcp/<key>` carries an access key in the path.
    const path = new URL(req.url).pathname.replace(/^\/mcp\/.*/, "/mcp/…");
    logError({}, `http: ${req.method} ${path} threw`, e);
    return error(500, "internal_error", "Something went wrong on the server. Try again.");
  }
}

/** Runs a handler; a thrown ConvexError becomes its `{ code, error }` reply, anything else
 * `500 internal_error`, never Convex's own page. */
export function route(handler: (ctx: ActionCtx, req: Request) => Promise<Response>) {
  return httpAction((ctx, req) => handle(ctx, req, handler));
}

/** CORS: every origin, or the listed ones in `ALLOWED_ORIGINS`. */
export function withCors(response: Response, origin: string | null): Response {
  const out = new Response(response.body, response);
  const allowed = (process.env.ALLOWED_ORIGINS ?? "")
    .split(",")
    .map((o) => o.trim())
    .filter(Boolean);
  if (allowed.length === 0) {
    out.headers.set("access-control-allow-origin", "*");
  } else if (origin !== null && allowed.includes(origin)) {
    out.headers.set("access-control-allow-origin", origin);
    out.headers.set("vary", "Origin");
  }
  out.headers.set("access-control-allow-methods", "GET, POST, DELETE, OPTIONS");
  out.headers.set("access-control-allow-headers", "Content-Type, Authorization");
  return out;
}

/** An account route with CORS applied even to error replies, and its `OPTIONS` (`204`). */
export function corsRoutes(method: "GET" | "POST", where: { path: string } | { pathPrefix: string }, handler: (ctx: ActionCtx, req: Request) => Promise<Response>): RouteSpec[] {
  return [
    { ...where, method, handler: httpAction(async (ctx, req) => withCors(await handle(ctx, req, handler), req.headers.get("origin"))) },
    { ...where, method: "OPTIONS", handler: httpAction(async (_ctx, req) => withCors(empty(), req.headers.get("origin"))) },
  ] as RouteSpec[];
}
