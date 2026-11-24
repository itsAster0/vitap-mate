// Tokens the backend signs or checks with Web Crypto.
//
// - The access JWT: ES256 with `JWT_PRIVATE_JWK` (a P-256 JWK as JSON). Convex
//   checks it through auth.config.ts against `JWT_PUBLIC_JWK`. ECDSA needs
//   randomness, so only actions sign.
// - Legacy v2 account token: HS256 with `ACCOUNT_TOKEN_SECRET`, traded once
//   for a refresh token while older installs move over.

import { fromBase64Url, toBase64Url } from "./base64";
import { fail } from "./errors";
import { hashesEqual } from "./crypto";
import { env } from "./util";

export const AUDIENCE = "vmate";
export const ACCESS_TTL_SECS = 24 * 3600;

const utf8 = new TextEncoder();
const part = (value: object) => toBase64Url(utf8.encode(JSON.stringify(value)));

export type AccessClaims = { sub: string; regno: string; acct: string; demo?: true };
/** An access key's JWT for the extension's socket: `sub` is the key's SHA-256. It carries no
 * `regno` or `acct`, so nothing that wants a phone can take it. */
export type KeyClaims = { sub: string; kind: "key" };

/** A compact JWT for `claims`, valid for 24 hours from `nowSecs`. */
export async function signAccessJwt(claims: AccessClaims | KeyClaims, nowSecs: number): Promise<{ jwt: string; expiresAt: number }> {
  const raw = process.env.JWT_PRIVATE_JWK;
  if (!raw) throw new Error("JWT_PRIVATE_JWK is not configured");
  const jwk = JSON.parse(raw) as JsonWebKey & { kid?: string };
  const { kid, ...keyData } = jwk;
  const key = await crypto.subtle.importKey("jwk", keyData, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
  const exp = nowSecs + ACCESS_TTL_SECS;
  const header = part({ alg: "ES256", typ: "JWT", ...(kid ? { kid } : {}) });
  const payload = part({ iss: env("PUBLIC_URL"), aud: AUDIENCE, iat: nowSecs, exp, ...claims });
  // Web Crypto's ECDSA signature is the raw r‖s that JWS wants.
  const sig = new Uint8Array(await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, key, utf8.encode(`${header}.${payload}`)));
  return { jwt: `${header}.${payload}.${toBase64Url(sig)}`, expiresAt: exp * 1000 };
}

export type V2Claims = { sub: string; dev: string; iat: number; exp: number };

const V2_JOSE = toBase64Url(utf8.encode(JSON.stringify({ alg: "HS256", typ: "JWT" })));

async function mac(secret: string, input: string): Promise<string> {
  const key = await crypto.subtle.importKey("raw", utf8.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return toBase64Url(new Uint8Array(await crypto.subtle.sign("HMAC", key, utf8.encode(input))));
}

/** A v2 token, as the edge issued them (for tests and the backfill's checks). */
export async function issueV2Token(secret: string, claims: V2Claims): Promise<string> {
  const input = `${V2_JOSE}.${part(claims)}`;
  return `${input}.${await mac(secret, input)}`;
}

/** The claims of a v2 account token issued to install `dev` and unexpired at
 * `at`; else throws `token_invalid` or `token_expired`, as the edge answered. */
export async function verifyV2Token(token: string, dev: string, at: number): Promise<V2Claims> {
  const invalid = () => fail("token_invalid", "The account token does not hold; prove the account instead.");
  const parts = token.split(".");
  const secret = process.env.ACCOUNT_TOKEN_SECRET;
  if (!secret || parts.length !== 3 || parts[0] !== V2_JOSE) return invalid();
  if (!hashesEqual(await mac(secret, `${parts[0]}.${parts[1]}`), parts[2])) return invalid();
  let claims: Partial<V2Claims>;
  try {
    claims = JSON.parse(new TextDecoder().decode(fromBase64Url(parts[1]) ?? new Uint8Array())) as Partial<V2Claims>;
  } catch {
    return invalid();
  }
  const { sub, iat, exp } = claims;
  if (typeof sub !== "string" || !Number.isInteger(iat) || !Number.isInteger(exp)) return invalid();
  if (claims.dev !== dev) return invalid();
  if (at >= (exp as number)) fail("token_expired", "The account token has expired; prove the account instead.");
  return { sub, dev, iat: iat as number, exp: exp as number };
}
