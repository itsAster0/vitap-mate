// Signed phone requests (vm1), as vtop-shared's `device_sig` signs them.
// Each install registers an Ed25519 key; every request then carries
// `x-vm-device` (its id), `x-vm-time` (unix seconds, within ±120 s) and
// `x-vm-sig`, the base64url signature over
// `"vm1\n" + METHOD + "\n" + path-with-query + "\n" + time + "\n" + hex(sha256(body))`.

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, utf8ToBytes } from "@noble/hashes/utils.js";

import { type Env, deviceStub, error, isDeviceId, now, parseJson, unknownDevice } from "./common";
import type { DeviceInfo } from "./device";
import { fromBase64Url } from "./shared/base64";

export const DEVICE_HEADER = "x-vm-device";
export const TIME_HEADER = "x-vm-time";
export const SIG_HEADER = "x-vm-sig";
export const MAX_SKEW_SECS = 120;

/** A request whose signature checked out: its install and its body. */
export interface Signed {
  device: DeviceInfo;
  body: Uint8Array;
  /** `body` as JSON, or null when it is not a JSON object. */
  json: Record<string, unknown> | null;
}

/** base64url with or without padding. */
export function base64Url(text: string): Uint8Array | null {
  return fromBase64Url(text.trim().replace(/=+$/, ""));
}

/** The device id of a signing key: the first 16 bytes of its SHA-256, hex. */
export function deviceIdFor(signKey: Uint8Array): string {
  return bytesToHex(sha256(signKey)).slice(0, 32);
}

export function signedMessage(method: string, pathWithQuery: string, time: string, body: Uint8Array): Uint8Array {
  return utf8ToBytes(`vm1\n${method.toUpperCase()}\n${pathWithQuery}\n${time}\n${bytesToHex(sha256(body))}`);
}

async function verify(signKey: Uint8Array, sig: Uint8Array, message: Uint8Array): Promise<boolean> {
  try {
    const key = await crypto.subtle.importKey("raw", signKey, { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, key, sig, message);
  } catch {
    return false;
  }
}

function badSignature(): Response {
  return error(401, "bad_signature", "The request signature does not check out.");
}

/** Null when `req` (with `body`) is signed by `signKey` at an acceptable
 * time; else the refusal. */
export async function checkSignature(req: Request, body: Uint8Array, signKey: Uint8Array): Promise<Response | null> {
  const time = req.headers.get(TIME_HEADER)?.trim() ?? "";
  const sig = base64Url(req.headers.get(SIG_HEADER) ?? "");
  if (!/^\d{1,12}$/.test(time) || sig?.length !== 64) return badSignature();
  const serverTime = now();
  if (Math.abs(serverTime - Number(time)) > MAX_SKEW_SECS) {
    return Response.json(
      { code: "stale_time", error: "The phone's clock is off; correct it by serverTime.", serverTime },
      { status: 401 },
    );
  }
  const url = new URL(req.url);
  const message = signedMessage(req.method, `${url.pathname}${url.search}`, time, body);
  return (await verify(signKey, sig, message)) ? null : badSignature();
}

/** Reads and checks a signed request from a registered install. */
export async function signed(req: Request, env: Env): Promise<Signed | Response> {
  const body = new Uint8Array(await req.arrayBuffer());
  const id = req.headers.get(DEVICE_HEADER)?.trim() ?? "";
  if (!isDeviceId(id)) return req.headers.has(DEVICE_HEADER) ? unknownDevice() : badSignature();
  const device = await deviceStub(env, id).info();
  if (!device) return unknownDevice();
  const signKey = base64Url(device.signKey);
  if (!signKey) return badSignature();
  const refused = await checkSignature(req, body, signKey);
  return refused ?? { device, body, json: parseJson(body, req.headers) };
}
