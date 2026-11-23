// vm1 signatures, as vtop-shared's `device_sig` makes them. Over HTTP the
// message is built from the request; on the socket a `signed` function uses
// `METHOD = "CALL"`, `path = "/" + functionName` and `body = payload`, so the
// phone signs with the same code either way.

import { fromBase64Url } from "./base64";
import { sha256Hex } from "./crypto";

export const MAX_SKEW_SECS = 120;

export type Vm1Result = "ok" | "bad_signature" | "stale_time";

/** base64url with or without padding. */
export function base64Url(text: string): Uint8Array | null {
  return fromBase64Url(text.trim().replace(/=+$/, ""));
}

export async function signedMessage(method: string, pathWithQuery: string, time: string, body: Uint8Array): Promise<Uint8Array> {
  return new TextEncoder().encode(`vm1\n${method.toUpperCase()}\n${pathWithQuery}\n${time}\n${await sha256Hex(body)}`);
}

/** Whether `sig` is `signKey`'s vm1 signature at a time within ±120 s of `nowSecs`. */
export async function verifyVm1(
  req: { method: string; pathWithQuery: string; time: string; sig: string; body: Uint8Array },
  signKey: Uint8Array,
  nowSecs: number,
): Promise<Vm1Result> {
  const time = req.time.trim();
  const sig = base64Url(req.sig);
  if (!/^\d{1,12}$/.test(time) || sig?.length !== 64 || signKey.length !== 32) return "bad_signature";
  if (Math.abs(nowSecs - Number(time)) > MAX_SKEW_SECS) return "stale_time";
  try {
    const key = await crypto.subtle.importKey("raw", new Uint8Array(signKey), { name: "Ed25519" }, false, ["verify"]);
    const message = await signedMessage(req.method, req.pathWithQuery, time, req.body);
    return (await crypto.subtle.verify({ name: "Ed25519" }, key, new Uint8Array(sig), new Uint8Array(message))) ? "ok" : "bad_signature";
  } catch {
    return "bad_signature";
  }
}
