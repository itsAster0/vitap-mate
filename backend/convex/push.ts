// Firebase Cloud Messaging wake-up pushes: high-priority data messages whose
// values are strings. The OAuth token is minted from the service account with
// Web Crypto (RS256) and cached while the isolate lives. Without
// FIREBASE_CREDENTIALS_JSON nothing is sent: an open app still sees its
// requests and mail through live queries.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import { internalAction, internalMutation, internalQuery } from "./_generated/server";
import { toBase64Url } from "./lib/base64";
import { deviceFcmToken, deviceRow } from "./lib/identity";
import { logError, logWarn, now } from "./lib/util";

/** An OAuth scope name, not an address. */
const SCOPE = "https://www.googleapis.com/auth/firebase.messaging";
/** Google's FCM v1 API origin; tests override with `FCM_URL` (unset or empty uses this). */
const GOOGLE_FCM_URL = "https://fcm.googleapis.com";
/** Mint a new token this long before the old one expires. */
const REFRESH_MARGIN_SECS = 300;
/** Timeout for Google OAuth token exchange and FCM message sends. */
export const FCM_TIMEOUT_MS = 10_000;
/** Retry delays for a transient failure: total 20 s, well inside the 120 s session request TTL. */
export const RETRY_DELAYS_MS = [5_000, 15_000] as const;

export type ServiceAccount = { client_email: string; private_key: string; project_id: string; token_uri: string };

const utf8 = new TextEncoder();

/** The FCM message: data values are strings only. */
export function message(token: string, data: Record<string, unknown>) {
  const strings = Object.fromEntries(
    Object.entries(data).map(([key, value]) => [key, typeof value === "string" ? value : JSON.stringify(value)]),
  );
  return { message: { token, data: strings, android: { priority: "HIGH", ttl: "120s" } } };
}

let cached: { email: string; token: string; expires: number } | null = null;

async function accessToken(account: ServiceAccount): Promise<string> {
  const at = now();
  if (cached && cached.email === account.client_email && cached.expires > at + REFRESH_MARGIN_SECS) return cached.token;
  const part = (o: object) => toBase64Url(utf8.encode(JSON.stringify(o)));
  const unsigned = `${part({ alg: "RS256", typ: "JWT" })}.${part({ iss: account.client_email, scope: SCOPE, aud: account.token_uri, iat: at, exp: at + 3600 })}`;
  const body = account.private_key
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => !l.startsWith("-----"))
    .join("");
  const der = Uint8Array.from(atob(body), (c) => c.charCodeAt(0));
  const algorithm = { name: "RSASSA-PKCS1-v1_5", hash: { name: "SHA-256" } };
  const key = await crypto.subtle.importKey("pkcs8", der, algorithm, false, ["sign"]);
  const sig = new Uint8Array(await crypto.subtle.sign(algorithm, key, utf8.encode(unsigned)));
  const response = await fetch(account.token_uri, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: `grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=${unsigned}.${toBase64Url(sig)}`,
    signal: AbortSignal.timeout(FCM_TIMEOUT_MS),
  });
  if (response.status !== 200) throw new Error(`token exchange failed: ${response.status}`);
  const token = (await response.json()) as { access_token: string; expires_in: number };
  cached = { email: account.client_email, token: token.access_token, expires: at + token.expires_in };
  return token.access_token;
}

export type SendOutcome = "sent" | "token_invalid" | "failed" | "off";

export async function sendFcm(token: string, data: Record<string, unknown>): Promise<SendOutcome> {
  const json = process.env.FIREBASE_CREDENTIALS_JSON;
  if (!json) return "off";
  let account: ServiceAccount;
  try {
    account = JSON.parse(json) as ServiceAccount;
  } catch {
    logError({}, "fcm: FIREBASE_CREDENTIALS_JSON is not JSON");
    return "off";
  }
  let access: string;
  try {
    access = await accessToken(account);
  } catch (e) {
    logError({}, "fcm: no access token", e);
    return "failed";
  }
  const base = (process.env.FCM_URL?.trim() || GOOGLE_FCM_URL).replace(/\/+$/, "");
  const response = await fetch(`${base}/v1/projects/${account.project_id}/messages:send`, {
    method: "POST",
    headers: { authorization: `Bearer ${access}`, "content-type": "application/json" },
    body: JSON.stringify(message(token, data)),
    signal: AbortSignal.timeout(FCM_TIMEOUT_MS),
  });
  if (response.status === 200) return "sent";
  if (response.status === 401) {
    // Drop the cached OAuth token so the next try mints a fresh one instead of failing until expiry.
    cached = null;
    logWarn({}, "fcm: send answered 401; cleared cached access token");
    return "failed";
  }
  if (response.status === 404) return "token_invalid";
  if (response.status === 400 && (await response.text()).includes("UNREGISTERED")) return "token_invalid";
  logWarn({}, `fcm: send answered ${response.status}`);
  return "failed";
}

export const send = internalAction({
  args: { deviceId: v.string(), data: v.any(), attempt: v.number() },
  handler: async (ctx, { deviceId, data, attempt }) => {
    const token = await ctx.runQuery(internal.push.tokenOf, { deviceId });
    if (!token) return;
    let outcome: SendOutcome;
    try {
      outcome = await sendFcm(token, data as Record<string, unknown>);
    } catch (e) {
      logWarn({}, `fcm: send error: ${e instanceof Error ? e.name : "unknown"}`);
      outcome = "failed";
    }
    if (outcome === "token_invalid") {
      console.log("fcm: token is gone, cleared");
      await ctx.runMutation(internal.push.clearToken, { deviceId, token });
    } else if (outcome === "failed" && attempt < RETRY_DELAYS_MS.length) {
      await ctx.scheduler.runAfter(RETRY_DELAYS_MS[attempt], internal.push.send, { deviceId, data, attempt: attempt + 1 });
    } else if (outcome === "failed") {
      logError({}, `fcm: send failed after ${attempt + 1} tries`);
    }
  },
});

export const tokenOf = internalQuery({
  args: { deviceId: v.string() },
  handler: async (ctx, { deviceId }) => deviceFcmToken(await deviceRow(ctx, deviceId)),
});

/** Google says the token is dead: stop sending to it until the app sends a new one (unless it
 * already has). */
export const clearToken = internalMutation({
  args: { deviceId: v.string(), token: v.string() },
  handler: async (ctx, { deviceId, token }) => {
    const device = await deviceRow(ctx, deviceId);
    if (device && deviceFcmToken(device) === token) await ctx.db.patch(device._id, { fcmToken: undefined });
  },
});
