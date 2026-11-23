// What the HTTP actions share: bearer tokens, the internal tokens, CORS, and
// a ConvexError turned back into the edge's `{ code, error }` reply.

import type { RouteSpec } from "convex/server";
import { type ActionCtx, httpAction } from "../_generated/server";
import { hashesEqual, sha256Hex } from "./crypto";
import { empty, error, errorData } from "./errors";
import { parseObject } from "./util";

export function bearer(req: Request): string {
  const value = req.headers.get("authorization") ?? "";
  return value.startsWith("Bearer ") ? value.slice("Bearer ".length).trim() : "";
}

/** The caller holds `token`. Compared as hashes, so the time taken says nothing about it. */
export async function holds(req: Request, token: string | undefined): Promise<boolean> {
  return !!token && hashesEqual(await sha256Hex(token), await sha256Hex(bearer(req)));
}

export const notInternal = () => error(401, "unauthorized", "The internal token is required.");

export async function jsonBody(req: Request): Promise<Record<string, unknown> | null> {
  return parseObject(await req.text());
}

/** HTTP status for an error code thrown by a function, as the edge answered it. */
const STATUS: Record<string, number> = {
  bad_request: 400,
  key_unknown: 401,
  not_linked: 403,
  request_not_found: 404,
  mailbox_not_found: 404,
  rate_limited: 429,
};

/** Runs a handler; a thrown ConvexError becomes its `{ code, error }` reply, anything else
 * `500 internal_error`, never Convex's own page. */
export function route(handler: (ctx: ActionCtx, req: Request) => Promise<Response>) {
  return httpAction(async (ctx, req) => {
    try {
      return await handler(ctx, req);
    } catch (e) {
      const data = errorData(e);
      if (data) return Response.json(data, { status: STATUS[data.code] ?? 400 });
      console.log(`http: ${req.method} ${new URL(req.url).pathname} threw: ${e instanceof Error ? e.name : "unknown"}`);
      return error(500, "internal_error", "Something went wrong on the server. Try again.");
    }
  });
}

/** CORS as the bridge sends it: every origin, or the listed ones in `ALLOWED_ORIGINS`. */
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

/** An account route with CORS, and its `OPTIONS` (`204`). */
export function corsRoutes(method: "GET" | "POST", where: { path: string } | { pathPrefix: string }, handler: (ctx: ActionCtx, req: Request) => Promise<Response>): RouteSpec[] {
  return [
    { ...where, method, handler: route(async (ctx, req) => withCors(await handler(ctx, req), req.headers.get("origin"))) },
    { ...where, method: "OPTIONS", handler: httpAction(async (_ctx, req) => withCors(empty(), req.headers.get("origin"))) },
  ] as RouteSpec[];
}
