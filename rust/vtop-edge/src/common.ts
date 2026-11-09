// What the routes and the Durable Objects share: the environment, error
// replies, calls to the engine and to app installs.

import { Inflate, strFromU8 } from "fflate";
import { EDGE_VERSION } from "./version";
import type { Account } from "./account";
import type { Device } from "./device";
import type { FcmToken } from "./fcm";
import type { Index, Mailbox } from "./store";
import { hash, hashesEqual } from "./shared/tokens";

export interface Env {
  DEVICE: DurableObjectNamespace<Device>;
  MAILBOX: DurableObjectNamespace<Mailbox>;
  INDEX: DurableObjectNamespace<Index>;
  FCM_TOKEN: DurableObjectNamespace<FcmToken>;
  ACCOUNT: DurableObjectNamespace<Account>;
  MAIL_DOMAIN: string;
  ENGINE_URL: string;
  FCM_URL: string;
  PUBLIC_URL: string;
  ALLOWED_ORIGINS: string;
  FALLBACK_FORWARD: string;
  FIREBASE_CREDENTIALS_JSON: string;
  /** Shared with the engine and the UDP gateway: sent on every call to the
   * engine and required on `/internal/*`. */
  INTERNAL_TOKEN: string;
}

/** Unix seconds. */
export function now(): number {
  return Math.floor(Date.now() / 1000);
}

/** `{"code", "error"}`, as the bridge answers. */
export function error(status: number, code: string, message: string): Response {
  return Response.json({ code, error: message }, { status });
}

/** `GET /healthz`: the service, its version and the Cloudflare location
 * (`request.cf.colo`) that answered. */
export function healthBody(cf: { colo?: unknown } | undefined): {
  service: "vtop-edge";
  status: "ok";
  version: string;
  colo: string | null;
} {
  const colo = cf?.colo;
  return { service: "vtop-edge", status: "ok", version: EDGE_VERSION, colo: typeof colo === "string" ? colo : null };
}

/** The install named in a call is not registered here (the edge lost it,
 * or it was registered with another server); the app registers again. */
export function unknownDevice(): Response {
  return error(404, "unknown_device", "This install is not registered.");
}

export function badRequest(message: string): Response {
  return error(400, "bad_request", message);
}

export function empty(status = 204): Response {
  return new Response(null, { status });
}

/** A request's JSON body as an object; null when it is not one. */
export async function jsonBody(req: Request): Promise<Record<string, unknown> | null> {
  return parseJson(new Uint8Array(await req.arrayBuffer()), req.headers);
}

/** A body read already, as `jsonBody` parses it (`x-vtop-encoding:
 * deflate-raw` is inflated first). */
export function parseJson(bytes: Uint8Array, headers: Headers): Record<string, unknown> | null {
  try {
    const raw = headers.get("x-vtop-encoding") === "deflate-raw" ? inflateCapped(bytes, MAX_INFLATED) : bytes;
    const body: unknown = JSON.parse(strFromU8(raw));
    return body !== null && typeof body === "object" && !Array.isArray(body)
      ? (body as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

/** A string field of a parsed body. */
export function str(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

/** A required `[vars]` entry; every address the edge uses is configured. */
export function variable(env: Env, name: "MAIL_DOMAIN" | "ENGINE_URL" | "FCM_URL" | "PUBLIC_URL"): string {
  const value = (env[name] ?? "").trim();
  if (!value) throw new Error(`${name} is not configured`);
  return value.replace(/\/+$/, "");
}

/** Where MCP clients connect: this Worker's `/mcp`. */
export function mcpUrl(env: Env): string {
  return `${variable(env, "PUBLIC_URL").replace(/\/+$/, "")}/mcp`;
}

export function bearer(req: Request): string {
  const value = req.headers.get("authorization") ?? "";
  return value.startsWith("Bearer ") ? value.slice("Bearer ".length).trim() : "";
}

/** The caller (the engine or the UDP gateway) holds `INTERNAL_TOKEN`. */
export function isInternal(req: Request, env: Env): boolean {
  const token = env.INTERNAL_TOKEN ?? "";
  return token !== "" && hashesEqual(hash(token), hash(bearer(req)));
}

export function notInternal(): Response {
  return error(401, "unauthorized", "The internal token is required.");
}

// ---- the engine ---------------------------------------------------------------

/** Sends a request to the engine at `ENGINE_URL`. */
export function engineSend(env: Env, path: string, init: RequestInit): Promise<Response> {
  return fetch(`${variable(env, "ENGINE_URL")}${path}`, init);
}

/** Posts to the engine with `INTERNAL_TOKEN`. */
export function enginePost(env: Env, path: string, body: unknown): Promise<Response> {
  return engineSend(env, path, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${env.INTERNAL_TOKEN}` },
    body: JSON.stringify(body),
  });
}

/** Posts to the engine and reads a JSON answer; throws with the reason. */
export async function engineJson<T>(env: Env, path: string, body: unknown): Promise<T> {
  const response = await enginePost(env, path, body);
  if (response.status !== 200) throw new Error(`engine answered ${response.status}`);
  return response.json<T>();
}

// ---- app installs ---------------------------------------------------------------

/** Device ids are 32 lowercase hex characters. */
export function isDeviceId(id: string): boolean {
  return /^[0-9a-f]{32}$/.test(id);
}

export function deviceStub(env: Env, id: string) {
  return env.DEVICE.getByName(id);
}

/** The forwarding-address index entry of a student (`owner:<hash>`). */
export function ownerStub(env: Env, ownerHash: string) {
  return env.INDEX.getByName(`owner:${ownerHash}`);
}

/** The most a compressed request body may expand to. */
const MAX_INFLATED = 1 << 20;

/** Inflates deflate-raw `data`, throwing past `limit` bytes or on a bad stream. */
function inflateCapped(data: Uint8Array, limit: number): Uint8Array {
  const chunks: Uint8Array[] = [];
  let size = 0;
  let ended = false;
  const inflate = new Inflate((chunk, final) => {
    size += chunk.length;
    if (size > limit) throw new Error("Body too large.");
    chunks.push(chunk);
    ended ||= final;
  });
  inflate.push(data, true);
  if (!ended) throw new Error("Truncated body.");
  const out = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.length;
  }
  return out;
}
