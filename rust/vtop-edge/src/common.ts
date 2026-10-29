// What the routes and the Durable Objects share: the environment, error
// replies, calls to the engine and to app installs.

import { Inflate, strFromU8 } from "fflate";
import { EDGE_VERSION } from "./version";
import type { Account } from "./account";
import type { Device } from "./device";
import type { FcmToken } from "./fcm";
import type { Index, Mailbox } from "./store";
import type { UdpKeys } from "./udp-gateway";
import { DEVICE_SECRET_PREFIX, hash, hashesEqual, randomToken } from "./shared/tokens";

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
  UDP_KEYS: DurableObjectNamespace<UdpKeys>;
  /** UDP mode's gateway (`GET /v1/udp`): host and comma-separated ports. */
  UDP_HOST: string;
  UDP_PORTS: string;
  FIREBASE_CREDENTIALS_JSON: string;
  BRIDGE_SECRET: string;
  /** The UDP gateway's identity public key (base64url): its keys are sealed
   * to it. */
  GATEWAY_IDENTITY_KEY: string;
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
  try {
    const body: unknown =
      req.headers.get("x-vtop-encoding") === "deflate-raw"
        ? JSON.parse(strFromU8(inflateCapped(new Uint8Array(await req.arrayBuffer()), MAX_INFLATED)))
        : await req.json();
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

/** The caller holds the secret shared with the engine. */
export function isEngine(req: Request, env: Env): boolean {
  const secret = env.BRIDGE_SECRET ?? "";
  return secret !== "" && hashesEqual(hash(secret), hash(bearer(req)));
}

// ---- the engine ---------------------------------------------------------------

/** Sends a request to the engine at `ENGINE_URL`. */
export function engineSend(env: Env, path: string, init: RequestInit): Promise<Response> {
  return fetch(`${variable(env, "ENGINE_URL")}${path}`, init);
}

/** Posts to the engine with the shared secret. */
export function enginePost(env: Env, path: string, body: unknown): Promise<Response> {
  return engineSend(env, path, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${env.BRIDGE_SECRET}` },
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

/** A new install for this FCM token: its id and secret. */
export async function createDevice(env: Env, fcmToken: string): Promise<{ id: string; secret: string }> {
  const id = randomToken("").slice(0, 32);
  const secret = randomToken(DEVICE_SECRET_PREFIX);
  if (!(await deviceStub(env, id).init(id, hash(secret), fcmToken))) {
    throw new Error("device create failed: the id is taken");
  }
  return { id, secret };
}

export async function deviceMatches(env: Env, id: string, secret: string): Promise<boolean> {
  return isDeviceId(id) && (await deviceStub(env, id).auth(hash(secret)));
}

/** False when the install no longer exists. */
export function setDeviceFcm(env: Env, id: string, token: string): Promise<boolean> {
  return deviceStub(env, id).setFcm(token);
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
