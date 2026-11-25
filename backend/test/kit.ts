/// <reference types="vite/client" />
// Fixtures for the function and route tests: a fresh convex-test backend at a
// fixed time, installs that sign as the app does, test secrets, a fake engine
// behind `fetch`, and a helper that proves an account the way the engine
// reports a proof.

import rateLimiter from "@convex-dev/rate-limiter/test";
import { convexTest } from "convex-test";
import { vi } from "vitest";
import { api, internal } from "../convex/_generated/api";
import schema from "../convex/schema";

export const modules = import.meta.glob(["../convex/**/*.ts", "!../convex/**/*.test.ts"]);

export const T0 = 1_800_000_000;
export const REGNO = "22BCE0001";
export const TOKENS = { INTERNAL_TOKEN: "internal-test", MAIL_TOKEN: "mail-test", GATEWAY_TOKEN: "gateway-test", VERIFIER_TOKEN: "verifier-test", ACCOUNT_TOKEN_SECRET: "account-secret-test" };

export type T = ReturnType<typeof convexTest>;

const b64url = (b: Uint8Array) => btoa(String.fromCharCode(...b)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const hex = (b: ArrayBuffer) => Array.from(new Uint8Array(b), (x) => x.toString(16).padStart(2, "0")).join("");

let jwk: string | null = null;

/** A backend at `T0` with the test environment. */
export async function setup(): Promise<T> {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(T0 * 1000);
  if (jwk === null) {
    const pair = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign"])) as CryptoKeyPair;
    jwk = JSON.stringify({ ...(await crypto.subtle.exportKey("jwk", pair.privateKey)), kid: "test" });
  }
  for (const [k, v] of Object.entries({
    ...TOKENS,
    JWT_PRIVATE_JWK: jwk,
    ENGINE_URL: "https://engine.test",
    PUBLIC_URL: "https://vmate.test",
    MAIL_DOMAIN: "vmate.test",
    PROXY_HOPS: "1",
    FIREBASE_CREDENTIALS_JSON: "",
  })) {
    vi.stubEnv(k, v);
  }
  const t = convexTest(schema, modules);
  rateLimiter.register(t);
  return t;
}

/** Moves the clock by `secs`. */
export function advance(secs: number): void {
  vi.setSystemTime(Date.now() + secs * 1000);
}

const PKCS8 = { Ed25519: "302e020100300506032b657004220420", X25519: "302e020100300506032b656e04220420" };
const fromHex = (h: string) => Uint8Array.from(h.match(/../g)!.map((b) => Number.parseInt(b, 16)));

async function keyFromSeed(name: "Ed25519" | "X25519", seed: Uint8Array) {
  const usages: KeyUsage[] = name === "Ed25519" ? ["sign"] : ["deriveBits"];
  const key = await crypto.subtle.importKey("pkcs8", new Uint8Array([...fromHex(PKCS8[name]), ...seed]), { name }, true, usages);
  const { d, x } = await crypto.subtle.exportKey("jwk", key);
  return { key, secret: d!, public: x! };
}

export type Phone = Awaited<ReturnType<typeof phone>>;

/** The `n`th test install: its keys, device id, and calls signed as the app signs them. */
export async function phone(n: number) {
  const seed = (salt: number) => Uint8Array.from({ length: 32 }, (_, i) => (i * 7 + n * 31 + salt) % 256);
  const sign = await keyFromSeed("Ed25519", seed(1));
  const seal = await keyFromSeed("X25519", seed(2));
  const deviceId = hex(await crypto.subtle.digest("SHA-256", Uint8Array.from(atob(sign.public.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0)))).slice(0, 32);
  const signed = async (fn: string, body: object, time = Math.floor(Date.now() / 1000)) => {
    const payload = JSON.stringify(body);
    const msg = `vm1\nCALL\n/${fn}\n${time}\n${hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(payload)))}`;
    const sig = b64url(new Uint8Array(await crypto.subtle.sign({ name: "Ed25519" }, sign.key, new TextEncoder().encode(msg))));
    return { flow: "test", signed: { key: sign.public, time: String(time), sig }, payload };
  };
  return { deviceId, signKey: sign.public, sealKey: seal.public, sealSecret: seal.secret, signed, device: { sealKey: seal.public, platform: "android", label: `Phone ${n}` } };
}

export type Engine = (path: string, body: unknown, headers: Headers) => Response | Promise<Response> | undefined;

/** Answers calls to the engine with `answer`; anything else it gives no answer for is a 404. */
export function engine(answer: Engine) {
  const calls: Array<{ path: string; body: unknown; headers: Headers }> = [];
  vi.stubGlobal("fetch", async (input: string | URL | Request, init?: RequestInit) => {
    const url = new URL(typeof input === "string" || input instanceof URL ? input : input.url);
    const headers = new Headers(init?.headers);
    let body: unknown = null;
    if (typeof init?.body === "string") {
      try {
        body = JSON.parse(init.body);
      } catch {
        body = init.body;
      }
    }
    calls.push({ path: url.pathname, body, headers });
    return (await answer(url.pathname, body, headers)) ?? Response.json({ code: "not_found", error: "no" }, { status: 404 });
  });
  return calls;
}

/** Proves `regno` for install `p` as the engine would report it, then trades the ticket for
 * tokens. Returns the tokens and a client with the JWT's claims. */
export async function prove(t: T, p: Phone, regno = REGNO, ownerHash = `owner-${regno}`) {
  const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
  await t.mutation(internal.auth.takeTicket, { ticket });
  await t.mutation(internal.auth.proofDone, { ticket, regno, ownerHash });
  const tokens = await t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device }));
  const as = t.withIdentity({ subject: p.deviceId, regno: tokens.regno, acct: tokens.account });
  return { ...tokens, as };
}

/** The `code` of a ConvexError the promise rejects with, or "none". */
export async function codeOf(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
    return "none";
  } catch (e) {
    const data = (e as { data?: unknown }).data;
    if (data && typeof data === "object" && "code" in data) return String((data as { code: unknown }).code);
    if (typeof data === "string") {
      try {
        return String((JSON.parse(data) as { code: unknown }).code);
      } catch {
        return data;
      }
    }
    throw e;
  }
}
