// `signed` functions: a phone without a JWT yet signs the call with its
// Ed25519 key (vm1 over `CALL /<functionName>` and the JSON `payload`). The
// device id comes from the key, so no row is needed before the first token.

import { v } from "convex/values";
import { fromBase64Url } from "./base64";
import { deviceIdFor } from "./crypto";
import { fail } from "./errors";
import { parseObject } from "./util";
import { verifyVm1 } from "./vm1";

export const signedArgs = {
  flow: v.string(),
  signed: v.object({ key: v.string(), time: v.union(v.string(), v.number()), sig: v.string() }),
  payload: v.string(),
};

export type SignedArgs = { flow: string; signed: { key: string; time: string | number; sig: string }; payload: string };

export type SignedCall = { deviceId: string; signKey: string; body: Record<string, unknown> };

/** Checks the signature of a call to `functionName` (`auth:token`) at `nowSecs`, then reads the
 * payload as a JSON object. Throws `bad_signature`, `stale_time` (with `serverTime`) or
 * `bad_request`, as the edge refused signed requests. */
export async function verifySigned(functionName: string, args: SignedArgs, nowSecs: number): Promise<SignedCall> {
  const keyText = args.signed.key.trim().replace(/=+$/, "");
  const signKey = fromBase64Url(keyText);
  if (signKey?.length !== 32) fail("bad_signature", "The request signature does not check out.");
  const result = await verifyVm1(
    {
      method: "CALL",
      pathWithQuery: `/${functionName}`,
      time: String(args.signed.time),
      sig: args.signed.sig,
      body: new TextEncoder().encode(args.payload),
    },
    signKey,
    nowSecs,
  );
  if (result === "stale_time") {
    fail("stale_time", "The phone's clock is off; correct it by serverTime.", { serverTime: nowSecs });
  }
  if (result !== "ok") fail("bad_signature", "The request signature does not check out.");
  const body = parseObject(args.payload);
  if (!body) fail("bad_request", "Invalid JSON body.");
  return { deviceId: await deviceIdFor(signKey), signKey: keyText, body };
}
