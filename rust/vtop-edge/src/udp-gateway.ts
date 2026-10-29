// UDP mode's gateway and its keys.
//
// - `GET /v1/udp`: where the gateway is and its current public key, so the
//   app follows a move or a new key without a release. Every phone keeps
//   the answer until the same `expiresAt` (the next Monday 00:00 IST), then
//   asks again; a gateway that cannot read its handshake makes it ask sooner.
// - `GET /internal/udp-keys`, from the engine (engine secret): the private
//   keys the gateway answers with, the current one first, sealed to the
//   gateway's identity key (`GATEWAY_IDENTITY_KEY`). The engine passes them
//   on to the gateway without being able to open them.
//
// `UdpKeys` (one object, named `keys`) makes a new random key pair each
// Monday 00:00 IST and keeps the one before for a week, for phones still
// holding it; older keys are deleted, so a handshake recorded then cannot be
// opened later even if a current key leaks.

import { DurableObject } from "cloudflare:workers";

import { utf8ToBytes } from "@noble/hashes/utils.js";

import { type Env, error, isEngine, now } from "./common";
import { fromBase64Url } from "./shared/base64";
import { seal } from "./shared/mail-seal";

/** What the box is bound to (vtop-shared's `UDP_KEYS_SEALED_FOR`). */
export const SEALED_FOR = "udp-keys";

const WEEK_SECS = 7 * 24 * 3600;
/** A Monday 00:00 IST (Sunday 18:30 UTC), which weeks are counted from. */
const MONDAY_IST = Date.UTC(2024, 0, 7, 18, 30) / 1000;

/** The first Monday 00:00 IST after `now` (unix seconds). */
export function nextMondayIst(now: number): number {
  return MONDAY_IST + (Math.floor((now - MONDAY_IST) / WEEK_SECS) + 1) * WEEK_SECS;
}

/** A gateway key pair (hex), current until `until`. */
export interface UdpKey {
  privateKey: string;
  publicKey: string;
  until: number;
}

/** `keys` at `now`: a new current key once the current one's week is over,
 * and the one before kept for a week; older ones dropped. */
export async function rotated(keys: UdpKey[], now: number, make: (until: number) => Promise<UdpKey>): Promise<UdpKey[]> {
  if (keys.length > 0 && keys[0].until > now) return keys;
  const kept = keys.filter((key) => key.until > now - WEEK_SECS).slice(0, 1);
  return [await make(nextMondayIst(now)), ...kept];
}

const hex = (bytes: Uint8Array) => [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");

/** A random X25519 key pair, as Noise uses for the gateway's static key. */
export async function newKey(until: number): Promise<UdpKey> {
  const pair = (await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"])) as CryptoKeyPair;
  const jwk = (await crypto.subtle.exportKey("jwk", pair.privateKey)) as JsonWebKey;
  const privateKey = fromBase64Url(jwk.d ?? "");
  const publicKey = fromBase64Url(jwk.x ?? "");
  if (privateKey?.length !== 32 || publicKey?.length !== 32) throw new Error("X25519 export");
  return { privateKey: hex(privateKey), publicKey: hex(publicKey), until };
}

export class UdpKeys extends DurableObject<Env> {
  /** The keys at `now`, rotated first when a week has turned. */
  async keys(at: number): Promise<UdpKey[]> {
    const stored = (await this.ctx.storage.get<UdpKey[]>("keys")) ?? [];
    const keys = await rotated(stored, at, newKey);
    if (keys !== stored) {
      await this.ctx.storage.put("keys", keys);
      console.log(`udp: a new gateway key, until ${new Date(keys[0].until * 1000).toISOString()}`);
    }
    return keys;
  }
}

function keysStub(env: Env) {
  return env.UDP_KEYS.getByName("keys");
}

export async function udpGateway(env: Env): Promise<Response> {
  const ports = env.UDP_PORTS.split(",")
    .map((p) => Number(p.trim()))
    .filter((p) => Number.isInteger(p) && p > 0 && p < 65536);
  if (!env.UDP_HOST || ports.length === 0) return error(503, "udp_unavailable", "UDP mode is not set up.");
  const [current] = await keysStub(env).keys(now());
  return Response.json(
    { host: env.UDP_HOST, ports, publicKey: current.publicKey, expiresAt: current.until },
    { headers: { "cache-control": "no-store" } },
  );
}

export async function udpKeys(req: Request, env: Env): Promise<Response> {
  if (!isEngine(req, env)) return error(401, "unauthorized", "The engine secret is required.");
  const keys = await keysStub(env).keys(now());
  const plain = utf8ToBytes(JSON.stringify({ keys: keys.map((key) => key.privateKey) }));
  const sealed = seal(env.GATEWAY_IDENTITY_KEY ?? "", SEALED_FOR, plain);
  if (sealed === null) return error(503, "udp_unavailable", "GATEWAY_IDENTITY_KEY is not set.");
  return Response.json({ sealed }, { headers: { "cache-control": "no-store" } });
}
