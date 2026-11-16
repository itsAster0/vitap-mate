// Account tokens: whose a phone is, for 14 days after a real check of its
// VTOP cookie, so account routes need no VTOP round trip. A compact JWT,
// HS256 with `ACCOUNT_TOKEN_SECRET`: `sub` the registration number, `dev`
// the install it was issued to, `iat` and `exp` (unix seconds). It goes out
// in response headers and comes back as `accountToken` in the body. Only the
// install holding `dev`'s signing key can use it; the student's `Account`
// object keeps each install's live `iat` and turns down any other, or one
// revoked (account.ts). Any install that showed the cookie gets one, linked
// or not: it says who the student is, not that the phone is linked. A token
// never buys another.

import { hmac } from "@noble/hashes/hmac.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { utf8ToBytes } from "@noble/hashes/utils.js";

import { type Env, error } from "./common";
import { fromBase64Url, toBase64Url } from "./shared/base64";
import { hashesEqual } from "./shared/tokens";

export const TOKEN_HEADER = "x-vm-account-token";
/** When the token expires, in unix milliseconds. */
export const TOKEN_EXPIRES_HEADER = "x-vm-account-token-expires-at";
export const TOKEN_TTL_SECS = 14 * 86_400;

export interface AccountClaims {
  sub: string;
  dev: string;
  iat: number;
  exp: number;
}

const JOSE = toBase64Url(utf8ToBytes(JSON.stringify({ alg: "HS256", typ: "JWT" })));

function mac(secret: string, input: string): string {
  return toBase64Url(hmac(sha256, utf8ToBytes(secret), utf8ToBytes(input)));
}

/** A token for `sub` on install `dev`, issued at `at`; null without a secret. */
export function issueToken(env: Env, sub: string, dev: string, at: number): { token: string; expiresAt: number } | null {
  if (!env.ACCOUNT_TOKEN_SECRET) return null;
  const claims: AccountClaims = { sub, dev, iat: at, exp: at + TOKEN_TTL_SECS };
  const input = `${JOSE}.${toBase64Url(utf8ToBytes(JSON.stringify(claims)))}`;
  return { token: `${input}.${mac(env.ACCOUNT_TOKEN_SECRET, input)}`, expiresAt: claims.exp * 1000 };
}

export function tokenInvalid(): Response {
  return error(401, "token_invalid", "The account token does not hold; send the VTOP session instead.");
}

/** The claims of `token` when it is ours, unexpired at `at` and issued to
 * install `dev`; else the refusal. */
export function verifyToken(env: Env, token: string, dev: string, at: number): AccountClaims | Response {
  const parts = token.split(".");
  const secret = env.ACCOUNT_TOKEN_SECRET;
  if (!secret || parts.length !== 3 || parts[0] !== JOSE || !hashesEqual(mac(secret, `${parts[0]}.${parts[1]}`), parts[2])) {
    console.log("account token: bad signature or shape");
    return tokenInvalid();
  }
  let claims: Partial<AccountClaims>;
  try {
    claims = JSON.parse(new TextDecoder().decode(fromBase64Url(parts[1]) ?? new Uint8Array())) as Partial<AccountClaims>;
  } catch {
    return tokenInvalid();
  }
  const { sub, iat, exp } = claims;
  if (typeof sub !== "string" || !Number.isInteger(iat) || !Number.isInteger(exp)) return tokenInvalid();
  if (claims.dev !== dev) {
    console.log("account token: another install's");
    return tokenInvalid();
  }
  if (at >= exp!) return error(401, "token_expired", "The account token has expired; send the VTOP session once more.");
  return { sub, dev, iat: iat!, exp: exp! };
}

/** `response` carrying `issued` in its headers. */
export function withToken(response: Response, issued: { token: string; expiresAt: number } | null): Response {
  if (issued === null) return response;
  const out = new Response(response.body, response);
  out.headers.set(TOKEN_HEADER, issued.token);
  out.headers.set(TOKEN_EXPIRES_HEADER, String(issued.expiresAt));
  return out;
}
