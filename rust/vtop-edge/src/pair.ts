// Pairing the browser extension with the app, so no one copies a key.
//
// The extension makes a pairing code and shows it. The app sends it with
// `POST /v1/keys` (`pairCode`); the edge then holds the new key under the
// code's hash for `PAIR_TTL_SECS`. The extension polls `POST /v1/pair
// { code }` and gets the key once; the entry is gone after that or when
// the time is up.

import { type Env, badRequest, str } from "./common";
import { hash } from "./shared/tokens";

export const PAIR_TTL_SECS = 300;
const POLL_AFTER_MS = 2000;

/** Crockford base32, the alphabet the extension draws codes from. */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CODE_LENGTH = 10;

/** A code as typed or scanned, without dashes or spaces and with the
 * letters that look like digits read as digits; null when it is not one. */
export function pairCode(input: string | undefined): string | null {
  const code = (input ?? "")
    .toUpperCase()
    .replace(/[\s-]/g, "")
    .replace(/O/g, "0")
    .replace(/[IL]/g, "1");
  if (code.length !== CODE_LENGTH || [...code].some((c) => !ALPHABET.includes(c))) return null;
  return code;
}

export function pairStub(env: Env, code: string) {
  return env.INDEX.getByName(`pair:${hash(code)}`);
}

/** `POST /v1/pair { code }`, from the extension: the key once the app has
 * made it (`200 { status: "ready", key }`), else `202` pending. */
export async function claim(body: Record<string, unknown> | null, env: Env): Promise<Response> {
  const code = pairCode(str(body?.code));
  if (code === null) return badRequest("code must be a pairing code.");
  const key = await pairStub(env, code).take();
  if (key === null) return Response.json({ status: "pending", pollAfterMs: POLL_AFTER_MS }, { status: 202 });
  console.log("pair: handed a key to the extension");
  return Response.json({ status: "ready", key });
}
