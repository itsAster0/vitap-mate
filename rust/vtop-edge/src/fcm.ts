// Firebase Cloud Messaging. The OAuth token is minted from the service
// account with WebCrypto (RS256) and cached for its hour in the `FcmToken`
// Durable Object, so pushes don't each pay for a signature.

import { DurableObject } from "cloudflare:workers";

import { type Env, now, variable } from "./common";
import { fromBase64, toBase64Url } from "./shared/base64";

/** An OAuth scope name, not an address. */
const SCOPE = "https://www.googleapis.com/auth/firebase.messaging";
/** Mint a new token this long before the old one expires. */
const REFRESH_MARGIN_SECS = 300;

export interface ServiceAccount {
  client_email: string;
  private_key: string;
  project_id: string;
  /** Where the assertion is exchanged; part of every service account file. */
  token_uri: string;
}

const utf8 = new TextEncoder();

/** `header.claims` of the OAuth assertion, base64url. */
export function unsignedJwt(account: ServiceAccount, now: number): string {
  const header = { alg: "RS256", typ: "JWT" };
  const claims = {
    iss: account.client_email,
    scope: SCOPE,
    aud: account.token_uri,
    iat: now,
    exp: now + 3600,
  };
  return `${toBase64Url(utf8.encode(JSON.stringify(header)))}.${toBase64Url(utf8.encode(JSON.stringify(claims)))}`;
}

/** The DER bytes inside a PEM `PRIVATE KEY` block. */
export function pemToDer(pem: string): Uint8Array | null {
  const body = pem
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => !line.startsWith("-----"))
    .join("");
  return fromBase64(body);
}

/** The FCM message: data values are strings only. */
export function message(token: string, data: Record<string, unknown>) {
  const strings = Object.fromEntries(
    Object.entries(data).map(([key, value]) => [key, typeof value === "string" ? value : JSON.stringify(value)]),
  );
  return { message: { token, data: strings, android: { priority: "HIGH", ttl: "120s" } } };
}

async function signRs256(der: Uint8Array, data: string): Promise<string> {
  const algorithm = { name: "RSASSA-PKCS1-v1_5", hash: { name: "SHA-256" } };
  const key = await crypto.subtle.importKey("pkcs8", der, algorithm, false, ["sign"]);
  const signature = await crypto.subtle.sign(algorithm, key, utf8.encode(data));
  return toBase64Url(new Uint8Array(signature));
}

export function serviceAccount(env: Env): ServiceAccount {
  try {
    return JSON.parse(env.FIREBASE_CREDENTIALS_JSON) as ServiceAccount;
  } catch (e) {
    throw new Error(`service account: ${e}`);
  }
}

async function mint(account: ServiceAccount, now: number): Promise<{ token: string; expires: number }> {
  const der = pemToDer(account.private_key);
  if (!der) throw new Error("service account key is not PEM");
  const unsigned = unsignedJwt(account, now);
  const jwt = `${unsigned}.${await signRs256(der, unsigned)}`;
  const response = await fetch(account.token_uri, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: `grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=${jwt}`,
  });
  if (response.status !== 200) throw new Error(`token exchange failed: ${response.status}`);
  const token = await response.json<{ access_token: string; expires_in: number }>();
  return { token: token.access_token, expires: now + token.expires_in };
}

/** Caches the FCM OAuth token. One instance, named `token`. */
export class FcmToken extends DurableObject<Env> {
  private cached: { token: string; expires: number } | null = null;

  async token(): Promise<string> {
    const at = now();
    if (!this.cached || this.cached.expires <= at + REFRESH_MARGIN_SECS) {
      this.cached = await mint(serviceAccount(this.env), at);
    }
    return this.cached.token;
  }
}

/** `token_invalid`: the token no longer reaches an install. */
export type SendOutcome = "sent" | "token_invalid" | "failed";

/** Sends a high-priority data message. */
export async function send(env: Env, token: string, data: Record<string, unknown>): Promise<SendOutcome> {
  const account = serviceAccount(env);
  let access: string;
  try {
    access = await env.FCM_TOKEN.getByName("token").token();
  } catch (e) {
    console.log(`fcm: no access token: ${e}`);
    return "failed";
  }
  const url = `${variable(env, "FCM_URL")}/v1/projects/${account.project_id}/messages:send`;
  const response = await fetch(url, {
    method: "POST",
    headers: { authorization: `Bearer ${access}`, "content-type": "application/json" },
    body: JSON.stringify(message(token, data)),
  });
  if (response.status === 200) return "sent";
  if (response.status === 404) return "token_invalid";
  if (response.status === 400 && (await response.text()).includes("UNREGISTERED")) return "token_invalid";
  return "failed";
}
