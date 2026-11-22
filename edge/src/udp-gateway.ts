// UDP mode's gateway: where it is and its public keys.
//
// - `POST /internal/udp-keys`, from the gateway (`INTERNAL_TOKEN`): its
//   host, ports and current public keys. The gateway makes and keeps its
//   own keys; the edge only passes the public halves on.
// - `GET /v1/udp` (signed, for phones): the latest of those, so the app
//   follows a move or a new key without a release.
//
// The latest report is kept in the `Index` object named `udp`.

import { base64Url } from "./auth";
import { type Env, badRequest, empty, error, isInternal, jsonBody, notInternal } from "./common";

export interface UdpKey {
  id: string;
  /** X25519, base64url without padding. */
  publicKey: string;
  /** Unix seconds the key is used from and until. */
  from: number;
  until: number;
}

export interface UdpGateway {
  host: string;
  ports: number[];
  keys: UdpKey[];
}

const isTime = (n: unknown): n is number => Number.isSafeInteger(n) && (n as number) >= 0;

/** The gateway's report, if well formed; public keys are kept unpadded. */
export function parseGateway(raw: Record<string, unknown> | null): UdpGateway | null {
  const { host, ports, keys } = raw ?? {};
  if (typeof host !== "string" || !host.trim() || host.length > 255) return null;
  if (!Array.isArray(ports) || ports.length === 0 || ports.length > 16) return null;
  if (!ports.every((p) => Number.isInteger(p) && p > 0 && p < 65536)) return null;
  if (!Array.isArray(keys) || keys.length === 0 || keys.length > 8) return null;
  const parsed: UdpKey[] = [];
  for (const key of keys as Array<Record<string, unknown> | null>) {
    const { id, publicKey, from, until } = key ?? {};
    if (typeof id !== "string" || !id || id.length > 64 || typeof publicKey !== "string") return null;
    if (base64Url(publicKey)?.length !== 32 || !isTime(from) || !isTime(until)) return null;
    parsed.push({ id, publicKey: publicKey.trim().replace(/=+$/, ""), from, until });
  }
  return { host: host.trim(), ports: ports as number[], keys: parsed };
}

function gatewayStub(env: Env) {
  return env.INDEX.getByName("udp");
}

export async function udpGateway(env: Env): Promise<Response> {
  const stored = await gatewayStub(env).get();
  if (stored === null) return error(503, "udp_unavailable", "UDP mode is not set up.");
  return new Response(stored, { headers: { "content-type": "application/json", "cache-control": "no-store" } });
}

export async function udpKeys(req: Request, env: Env): Promise<Response> {
  if (!isInternal(req, env)) return notInternal();
  const gateway = parseGateway(await jsonBody(req));
  if (!gateway) return badRequest("Expected { host, ports, keys: [{ id, publicKey, from, until }] }.");
  await gatewayStub(env).put(JSON.stringify(gateway));
  console.log(`udp: the gateway reported ${gateway.keys.length} key(s)`);
  return empty();
}
