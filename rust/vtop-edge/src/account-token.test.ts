import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { accountObject } from "../test/account-harness";
import { AccountRecord, phoneId } from "./account-state";
import { type AccountClaims, TOKEN_EXPIRES_HEADER, TOKEN_HEADER, TOKEN_TTL_SECS, issueToken, verifyToken } from "./account-token";
import { deviceIdFor, signedMessage } from "./auth";
import type { Env } from "./common";
import type { DeviceInfo } from "./device";
import worker from "./index";
import { fromBase64Url, toBase64Url } from "./shared/base64";

const T = 1_800_000_000;
const SECRET = "test-secret";
const env = { ACCOUNT_TOKEN_SECRET: SECRET } as Env;
const device = (id: string): DeviceInfo => ({ id, signKey: "", sealKey: "", label: null, platform: "android", fcmToken: "" });

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T * 1000);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function code(response: Response): Promise<string> {
  return ((await response.json()) as { code: string }).code;
}

function claimsOf(token: string): AccountClaims {
  return JSON.parse(new TextDecoder().decode(fromBase64Url(token.split(".")[1])!)) as AccountClaims;
}

describe("account tokens", () => {
  it("are HS256 JWTs naming the student and the install, for 14 days", () => {
    const issued = issueToken(env, "22BCE0001", "d1", T)!;
    expect(issued.expiresAt).toBe((T + TOKEN_TTL_SECS) * 1000);
    const [jose] = issued.token.split(".");
    expect(JSON.parse(new TextDecoder().decode(fromBase64Url(jose)!))).toEqual({ alg: "HS256", typ: "JWT" });
    expect(claimsOf(issued.token)).toEqual({ sub: "22BCE0001", dev: "d1", iat: T, exp: T + 14 * 86_400 });
    expect(verifyToken(env, issued.token, "d1", T + 60)).toEqual(claimsOf(issued.token));
  });

  it("expire", async () => {
    const { token } = issueToken(env, "22BCE0001", "d1", T)!;
    const late = verifyToken(env, token, "d1", T + TOKEN_TTL_SECS) as Response;
    expect(late.status).toBe(401);
    expect(await code(late)).toBe("token_expired");
  });

  it("are refused tampered, malformed, from another install or under another secret", async () => {
    const { token } = issueToken(env, "22BCE0001", "d1", T)!;
    const [jose, , sig] = token.split(".");
    const forged = toBase64Url(new TextEncoder().encode(JSON.stringify({ sub: "22BCE0002", dev: "d1", iat: T, exp: T + 999 })));
    const none = toBase64Url(new TextEncoder().encode(JSON.stringify({ alg: "none", typ: "JWT" })));
    const flipped = (sig[0] === "A" ? "B" : "A") + sig.slice(1);
    for (const [bad, dev, e] of [
      [`${jose}.${forged}.${sig}`, "d1", env],
      [`${jose}.${token.split(".")[1]}.${flipped}`, "d1", env],
      [`${none}.${token.split(".")[1]}.`, "d1", env],
      ["nope", "d1", env],
      ["", "d1", env],
      [token, "d2", env],
      [token, "d1", { ACCOUNT_TOKEN_SECRET: "other" } as Env],
      [token, "d1", {} as Env],
    ] as const) {
      const refused = verifyToken(e, bad, dev, T) as Response;
      expect(refused.status, bad).toBe(401);
      expect(await code(refused)).toBe("token_invalid");
    }
    expect(issueToken({} as Env, "22BCE0001", "d1", T)).toBeNull();
  });
});

const checked = { status: "valid" as const, registrationNumber: "22BCE0001", ownerHash: "oh1", sealedSession: "s" };

/** An account with phone d1, linked at `linkedAt`, and an engine that
 * counts its calls. `index` is the Index objects' entries by name. */
function rig(linkedAt = T) {
  let engineCalls = 0;
  const index = new Map<string, string>();
  const INDEX = {
    getByName: (name: string) => ({
      get: async () => index.get(name) ?? null,
      put: async (value: string) => void index.set(name, value),
      remove: async (value: string) => void (index.get(name) === value && index.delete(name)),
    }),
  };
  const r = accountObject(
    () => {
      engineCalls += 1;
      return new Response(null, { status: 503 });
    },
    { ACCOUNT_TOKEN_SECRET: SECRET, INDEX },
  );
  const doc = new AccountRecord("22BCE0001", T);
  doc.addPhone({ deviceId: "d1", label: null, addedAt: linkedAt, seenAt: linkedAt });
  r.seed(doc);
  return { ...r, index, engineCalls: () => engineCalls };
}

/** The claims of the token `response` carries. */
function tokenOf(response: Response): AccountClaims {
  return claimsOf(response.headers.get(TOKEN_HEADER)!);
}

describe("the account object", () => {
  it("issues a token to a linked phone after a cookie check", async () => {
    const { account } = rig();
    const response = await account.app("account", { checked, mailboxId: null, body: {}, device: device("d1") });
    expect(response.status).toBe(200);
    expect(((await response.json()) as { thisPhone: boolean }).thisPhone).toBe(true);
    const token = response.headers.get(TOKEN_HEADER)!;
    expect(claimsOf(token)).toMatchObject({ sub: "22BCE0001", dev: "d1", iat: T });
    expect(response.headers.get(TOKEN_EXPIRES_HEADER)).toBe(String((T + TOKEN_TTL_SECS) * 1000));
    // On a 204 too.
    const semester = await account.app("semester", { checked, mailboxId: null, body: { semesterId: "AP2026" }, device: device("d1") });
    expect(semester.status).toBe(204);
    expect(semester.headers.has(TOKEN_HEADER)).toBe(true);
  });

  it("issues one to an unlinked install too, but none on a failure or a delete", async () => {
    const r = rig();
    const stranger = await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d9") });
    expect(stranger.status).toBe(200);
    expect(((await stranger.json()) as { thisPhone: boolean }).thisPhone).toBe(false);
    expect(tokenOf(stranger)).toMatchObject({ sub: "22BCE0001", dev: "d9", iat: T });
    // The mail-otp routes can find the student by owner hash, to revoke.
    expect(r.index.get("student:oh1")).toBe("22BCE0001");
    const failed = await r.account.app("revoke-key", { checked, mailboxId: null, body: { id: "x" }, device: device("d1") });
    expect(failed.status).toBe(404);
    expect(failed.headers.has(TOKEN_HEADER)).toBe(false);
    const deleted = await r.account.app("delete", { checked, mailboxId: null, body: {}, device: device("d1") });
    expect(deleted.status).toBe(204);
    expect(deleted.headers.has(TOKEN_HEADER)).toBe(false);
    expect(r.index.has("student:oh1")).toBe(false);
  });

  it("gives an account with no phones only token rows", async () => {
    const r = accountObject(() => new Response(null, { status: 503 }), {
      ACCOUNT_TOKEN_SECRET: SECRET,
      INDEX: { getByName: () => ({ get: async () => null, put: async () => {} }) },
    });
    const issued = await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d9") });
    expect(r.load()).toBeNull();
    expect(await r.account.tokenOwner("d9", tokenOf(issued).iat)).toBe("oh1");
  });

  it("keeps linked-only routes linked-only for an unlinked install with a token", async () => {
    const r = rig();
    const claims = tokenOf(await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d9") }));
    const call = (op: Parameters<typeof r.account.app>[0], body = {}) =>
      r.account.app(op, { checked: null, token: claims, mailboxId: null, body, device: device("d9") });
    for (const op of ["create-key", "move-keys", "settings", "semester", "remove-phone", "clear-sign-ins", "delete"] as const) {
      const refused = await call(op);
      expect(refused.status, op).toBe(403);
      expect(await code(refused)).toBe("not_linked");
    }
    // With no phone linked yet: 409.
    const doc = r.load()!;
    doc.phones = [];
    r.seed(doc);
    const none = await call("semester", { semesterId: "AP2026" });
    expect(none.status).toBe(409);
    expect(await code(none)).toBe("not_linked");
    // Identity holds all the same.
    expect(await (await call("account")).json()).toMatchObject({ registrationNumber: "22BCE0001", thisPhone: false });
    expect(r.engineCalls()).toBe(0);
  });

  it("lets a newer token kill the older one", async () => {
    const r = rig();
    const old = tokenOf(await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d1") }));
    vi.setSystemTime((T + 5) * 1000);
    const fresh = tokenOf(await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d1") }));
    const call = (token: AccountClaims) =>
      r.account.app("account", { checked: null, token, mailboxId: null, body: {}, device: device("d1") });
    expect(await code(await call(old))).toBe("token_invalid");
    expect((await call(fresh)).status).toBe(200);
  });

  it("takes a token instead of the session, with no engine call and no new token", async () => {
    const r = rig();
    const issued = await r.account.app("account", { checked, mailboxId: null, body: {}, device: device("d1") });
    const claims = claimsOf(issued.headers.get(TOKEN_HEADER)!);
    const response = await r.account.app("semester", { checked: null, token: claims, mailboxId: null, body: { semesterId: "AP2026" }, device: device("d1") });
    expect(response.status).toBe(204);
    expect(response.headers.has(TOKEN_HEADER)).toBe(false);
    expect(r.load()!.semesterId).toBe("AP2026");
    const view = await r.account.app("account", { checked: null, token: claims, mailboxId: null, body: {}, device: device("d1") });
    expect(await view.json()).toMatchObject({ registrationNumber: "22BCE0001", thisPhone: true });
    expect(r.engineCalls()).toBe(0);
  });

  it("revokes on remove-phone, keys/move, devices/delete and account delete", async () => {
    const r = rig();
    const doc = r.load()!;
    for (const d of ["d2", "d3", "d4"]) doc.addPhone({ deviceId: d, label: null, addedAt: T, seenAt: T });
    r.seed(doc);
    const tokens = new Map<string, AccountClaims>();
    for (const d of ["d1", "d2", "d3", "d4", "d9"]) {
      tokens.set(d, tokenOf(await r.account.app("account", { checked, mailboxId: null, body: {}, device: device(d) })));
    }
    const holds = async (d: string) => (await r.account.tokenOwner(d, tokens.get(d)!.iat)) !== null;
    const byToken = (op: Parameters<typeof r.account.app>[0], body: object) =>
      r.account.app(op, { checked: null, token: tokens.get("d1")!, mailboxId: null, body: body as Record<string, unknown>, device: device("d1") });
    expect((await byToken("remove-phone", { id: phoneId("d2") })).status).toBe(204);
    expect(await holds("d2")).toBe(false);
    expect((await byToken("move-keys", { from: phoneId("d3") })).status).toBe(200);
    expect(await holds("d3")).toBe(false);
    await r.account.forgetPhone("d4");
    expect(await holds("d4")).toBe(false);
    // devices/delete revokes an unlinked install's token too.
    await r.account.forgetPhone("d9");
    expect(await holds("d9")).toBe(false);
    expect(await holds("d1")).toBe(true);

    expect((await byToken("delete", {})).status).toBe(204);
    expect(await code(await byToken("account", {}))).toBe("token_invalid");
    expect(await holds("d1")).toBe(false);
  });
});

// Through the Worker, signed as the app signs.
const seed = Uint8Array.from({ length: 32 }, (_, i) => i + 1);
const signKey = ed25519.getPublicKey(seed);
const deviceId = deviceIdFor(signKey);

function signedPost(path: string, body: object): Request {
  const text = JSON.stringify(body);
  const time = String(T);
  const sig = toBase64Url(ed25519.sign(signedMessage("POST", path, time, new TextEncoder().encode(text)), seed));
  return new Request(`https://vmate.aster0.dev${path}`, {
    method: "POST",
    headers: { "x-vm-time": time, "x-vm-sig": sig, "x-vm-device": deviceId },
    body: text,
  });
}

function workerEnv() {
  const info: DeviceInfo = { ...device(deviceId), signKey: toBase64Url(signKey), sealKey: toBase64Url(x25519.keygen().publicKey) };
  const calls: Array<{ account: string; call: Record<string, unknown> }> = [];
  const engine: string[] = [];
  vi.stubGlobal("fetch", async (url: string) => {
    engine.push(new URL(url).pathname);
    return Response.json({ status: "valid", registrationNumber: "22BCE0001", ownerHash: "oh1" });
  });
  const all = {
    ACCOUNT_TOKEN_SECRET: SECRET,
    ENGINE_URL: "http://engine",
    INTERNAL_TOKEN: "s",
    DEVICE: { getByName: () => ({ info: async () => info }) },
    INDEX: { getByName: () => ({ get: async () => null }) },
    ACCOUNT: {
      getByName: (account: string) => ({
        app: async (_op: string, call: Record<string, unknown>) => {
          calls.push({ account, call });
          return Response.json({ ok: true });
        },
      }),
    },
  } as unknown as Env;
  return { env: all, calls, engine };
}

describe("account routes", () => {
  it("hand a valid token to its student's account without asking the engine", async () => {
    const { env: e, calls, engine } = workerEnv();
    const { token } = issueToken(e, "22BCE0001", deviceId, T - 60)!;
    const response = await worker.fetch(signedPost("/v1/account", { accountToken: token, session: { cookies: "A=1" } }), e);
    expect(response.status).toBe(200);
    expect(engine).toEqual([]);
    expect(calls).toHaveLength(1);
    expect(calls[0].account).toBe("22BCE0001");
    expect(calls[0].call).toMatchObject({ checked: null, token: { sub: "22BCE0001", dev: deviceId, iat: T - 60 } });
  });

  it("refuse expired, other-install and tampered tokens before the account", async () => {
    const { env: e, calls } = workerEnv();
    const old = issueToken(e, "22BCE0001", deviceId, T - TOKEN_TTL_SECS)!.token;
    const other = issueToken(e, "22BCE0001", "0".repeat(32), T)!.token;
    const tampered = issueToken({ ACCOUNT_TOKEN_SECRET: "x" } as Env, "22BCE0001", deviceId, T)!.token;
    expect(await code(await worker.fetch(signedPost("/v1/keys", { accountToken: old }), e))).toBe("token_expired");
    expect(await code(await worker.fetch(signedPost("/v1/keys", { accountToken: other }), e))).toBe("token_invalid");
    expect(await code(await worker.fetch(signedPost("/v1/keys", { accountToken: tampered }), e))).toBe("token_invalid");
    expect(calls).toEqual([]);
  });

  it("still take the session as before", async () => {
    const { env: e, calls, engine } = workerEnv();
    const response = await worker.fetch(signedPost("/v1/account", { session: { cookies: "A=1" } }), e);
    expect(response.status).toBe(200);
    expect(engine).toEqual(["/engine/session"]);
    expect(calls[0].call).toMatchObject({ checked: { registrationNumber: "22BCE0001", ownerHash: "oh1" }, mailboxId: null });
    expect(calls[0].call.token).toBeUndefined();
  });
});

describe("mail-otp tokens", () => {
  /** The Worker over a real `Account` for 22BCE0001 and a mailbox of this
   * install and `others`, owned by no one yet; `engine` lists the engine
   * calls. */
  function mailEnv(others: string[] = []) {
    const { env: base } = workerEnv();
    const index = new Map<string, string>();
    const INDEX = {
      getByName: (name: string) => ({
        get: async () => index.get(name) ?? null,
        put: async (value: string) => void index.set(name, value),
        remove: async (value: string) => void (index.get(name) === value && index.delete(name)),
      }),
    };
    const { account } = accountObject(undefined, { ACCOUNT_TOKEN_SECRET: SECRET, INDEX });
    const engine: string[] = [];
    vi.stubGlobal("fetch", async (url: string) => {
      engine.push(new URL(url).pathname);
      return Response.json({ ownerHash: "oh1", registrationNumber: "22BCE0001" });
    });
    const member = (d: string) => ({ deviceId: d, sealed: null, sealedAt: null, addedAt: T, seenAt: T });
    let doc: Record<string, unknown> | null = {
      id: "k3f9x2ab7q",
      ownerHash: null,
      members: [member(deviceId), ...others.map(member)],
      createdAt: T,
      updatedAt: T,
    };
    const all = {
      ...base,
      MAIL_DOMAIN: "vmate.aster0.dev",
      INDEX,
      MAILBOX: {
        getByName: () => ({
          doc: async () => doc && structuredClone(doc),
          save: async (next: Record<string, unknown>) => void (doc = structuredClone(next)),
          remove: async () => void (doc = null),
        }),
      },
      ACCOUNT: { getByName: (name: string) => (name === "22BCE0001" ? account : base.ACCOUNT.getByName(name)) },
    } as unknown as Env;
    const post = async (path: string, body: object) => worker.fetch(signedPost(path, body), all);
    return { env: all, engine, account, post, index };
  }

  /** Owns the mailbox by cookie, as a forwarding-only install would; the
   * token that came back. */
  async function owned(m: ReturnType<typeof mailEnv>): Promise<string> {
    const response = await m.post("/v1/mail-otp/owner", { id: "k3f9x2ab7q", session: { cookies: "A=1" } });
    expect(response.status).toBe(204);
    return response.headers.get(TOKEN_HEADER)!;
  }

  it("give a forwarding-only install a token that then needs no engine", async () => {
    const m = mailEnv();
    const token = await owned(m);
    expect(claimsOf(token)).toMatchObject({ sub: "22BCE0001", dev: deviceId });
    expect(m.engine).toEqual(["/internal/mail-otp/owner"]);

    const owner = await m.post("/v1/mail-otp/owner", { id: "k3f9x2ab7q", accountToken: token });
    expect(owner.status).toBe(204);
    expect(owner.headers.has(TOKEN_HEADER)).toBe(false);
    const reclaim = await m.post("/v1/mail-otp/reclaim", { accountToken: token });
    expect(reclaim.status).toBe(200);
    expect(await reclaim.json()).toMatchObject({ id: "k3f9x2ab7q" });
    expect(reclaim.headers.has(TOKEN_HEADER)).toBe(false);
    expect(m.engine).toEqual(["/internal/mail-otp/owner"]);

    // By cookie, reclaim issues one too.
    const byCookie = await m.post("/v1/mail-otp/reclaim", { session: { cookies: "A=1" } });
    expect(byCookie.status).toBe(200);
    expect(claimsOf(byCookie.headers.get(TOKEN_HEADER)!)).toMatchObject({ sub: "22BCE0001", dev: deviceId });
  });

  it("still refuse linked-only account routes to that install", async () => {
    const m = mailEnv();
    const token = await owned(m);
    const keys = await m.post("/v1/keys", { accountToken: token });
    expect(keys.status).toBe(409);
    expect(await code(keys)).toBe("not_linked");
    const view = await m.post("/v1/account", { accountToken: token });
    expect(await view.json()).toMatchObject({ registrationNumber: "22BCE0001", phoneLinked: false });
  });

  it("revoke the removed device's token on devices/remove", async () => {
    const m = mailEnv(["d2"]);
    await owned(m);
    await m.account.issueToken("22BCE0001", "d2", "oh1");
    expect(await m.account.tokenOwner("d2", T)).toBe("oh1");
    expect((await m.post("/v1/mail-otp/devices/remove", { id: "k3f9x2ab7q", removeDevice: "d2" })).status).toBe(204);
    expect(await m.account.tokenOwner("d2", T)).toBeNull();
    expect(await m.account.tokenOwner(deviceId, T)).toBe("oh1");
  });

  it("revoke this device's token on delete, and every member's with everywhere", async () => {
    const m = mailEnv(["d2", "d3"]);
    const token = await owned(m);
    for (const d of ["d2", "d3"]) await m.account.issueToken("22BCE0001", d, "oh1");
    expect((await m.post("/v1/mail-otp/delete", { id: "k3f9x2ab7q" })).status).toBe(204);
    expect(await code(await m.post("/v1/mail-otp/reclaim", { accountToken: token }))).toBe("token_invalid");
    expect(await m.account.tokenOwner("d2", T)).toBe("oh1");

    const n = mailEnv(["d2", "d3"]);
    await owned(n);
    for (const d of ["d2", "d3"]) await n.account.issueToken("22BCE0001", d, "oh1");
    expect((await n.post("/v1/mail-otp/delete", { id: "k3f9x2ab7q", everywhere: true })).status).toBe(204);
    for (const d of [deviceId, "d2", "d3"]) expect(await n.account.tokenOwner(d, T), d).toBeNull();
  });

  it("answer the mail route even when revoking fails", async () => {
    const m = mailEnv(["d2"]);
    await owned(m);
    (m.account as unknown as Record<string, unknown>).revokeTokens = async () => {
      throw new Error("down");
    };
    expect((await m.post("/v1/mail-otp/devices/remove", { id: "k3f9x2ab7q", removeDevice: "d2" })).status).toBe(204);
  });
});
