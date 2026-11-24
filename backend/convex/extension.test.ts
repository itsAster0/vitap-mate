// The extension's socket: key JWTs and its live queries.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, codeOf, engine, phone, prove, setup, type T } from "../test/kit";
import { api, internal } from "./_generated/api";
import { keyId } from "./lib/accountState";
import { sha256Hex } from "./lib/crypto";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

const post = (t: T, path: string, body: unknown, headers: Record<string, string> = {}) =>
  t.fetch(path, { method: "POST", headers: { "content-type": "application/json", ...headers }, body: JSON.stringify(body) });

/** A proved phone with a key, and a client holding that key's JWT claims. */
async function withKey(t: T) {
  const p = await phone(1);
  const proved = await prove(t, p);
  const made = await proved.as.action(api.keys.create, { flow: "f", label: "Laptop" });
  const keyHash = await sha256Hex(made.key);
  return { ...proved, key: made.key, keyHash, ext: t.withIdentity({ subject: keyHash, kind: "key" }) };
}

describe("key JWTs", () => {
  it("are signed for a known key only, without counting as a use", async () => {
    const t = await setup();
    const { key } = await withKey(t);
    const res = await t.fetch("/v1/key/token", { method: "POST", headers: { authorization: `Bearer ${key}` } });
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe("no-store");
    const { jwt, expiresAt } = (await res.json()) as { jwt: string; expiresAt: number };
    const claims = JSON.parse(atob(jwt.split(".")[1].replace(/-/g, "+").replace(/_/g, "/")));
    expect(claims).toMatchObject({ sub: await sha256Hex(key), kind: "key", aud: "vmate" });
    expect(claims.regno).toBeUndefined();
    expect(expiresAt).toBeGreaterThan(Date.now());
    const rows = await t.run((ctx) => ctx.db.query("accessKeys").collect());
    expect(rows[0].lastUsedAt).toBeNull();
    expect((await t.fetch("/v1/key/token", { method: "POST", headers: { authorization: "Bearer vtm_nope" } })).status).toBe(401);
    expect((await t.fetch("/v1/key/token", { method: "OPTIONS" })).status).toBe(204);
  });

  it("are rate limited per account", async () => {
    const t = await setup();
    const { key } = await withKey(t);
    const ask = () => t.fetch("/v1/key/token", { method: "POST", headers: { authorization: `Bearer ${key}` } });
    for (let i = 0; i < 10; i++) expect((await ask()).status).toBe(200);
    expect((await ask()).status).toBe(429);
  });

  it("never act as a phone, and a phone's JWT reads nothing here", async () => {
    const t = await setup();
    const { ext, as } = await withKey(t);
    expect(await codeOf(ext.query(api.app.status, { flow: "f" }))).toBe("unauthenticated");
    expect(await codeOf(ext.action(api.keys.create, { flow: "f" }))).toBe("unauthenticated");
    expect(await as.query(api.extension.key, {})).toBeNull();
    expect(await as.query(api.extension.request, { requestId: "x" })).toEqual({ status: "key_unknown" });
  });
});

describe("live queries", () => {
  it("show the key's account and its own phone, and null once revoked", async () => {
    const t = await setup();
    const { ext, as, keyHash } = await withKey(t);
    expect(await ext.query(api.extension.key, {})).toMatchObject({
      registrationNumber: REGNO,
      keyLabel: "Laptop",
      phoneLinked: true,
      phone: { notifications: expect.any(Boolean) },
      cachedSession: false,
    });
    await as.mutation(api.keys.revoke, { flow: "f", id: keyId(keyHash) });
    expect(await ext.query(api.extension.key, {})).toBeNull();
    expect(await t.withIdentity({ subject: keyHash, kind: "key" }).query(api.extension.request, { requestId: "x" })).toEqual({ status: "key_unknown" });
  });

  it("follow a session request without its cookies, for its own account only", async () => {
    const t = await setup();
    const { ext, key, as } = await withKey(t);
    engine((path, body) => {
      if (path === "/engine/session") {
        const cookies = (body as { cookies?: string }).cookies ?? "JSESSIONID=abc";
        return Response.json({ status: "valid", registrationNumber: REGNO, cookies, sealedSession: "sealed", ownerHash: "o" });
      }
    });
    const started = (await (await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${key}` } })).json()) as { requestId: string };
    const pending = await ext.query(api.extension.request, { requestId: started.requestId });
    expect(pending).toMatchObject({ status: "pending", stage: "phone", login: null, expiresAt: expect.any(Number) });
    await as.action(api.sessions.answer, {
      flow: "f",
      requestId: started.requestId,
      cookies: [{ name: "JSESSIONID", value: "abc", domain: "vtop.vitap.ac.in" }],
    });
    const ready = await ext.query(api.extension.request, { requestId: started.requestId });
    expect(ready).toEqual({ status: "ready", from: "phone", expiresAt: expect.any(Number) });
    expect(JSON.stringify(ready)).not.toContain("abc");
    // Another student's key sees nothing of it.
    const other = await prove(t, await phone(2), "23XYZ0002");
    const otherKey = await other.as.action(api.keys.create, { flow: "f" });
    const theirs = t.withIdentity({ subject: await sha256Hex(otherKey.key), kind: "key" });
    expect(await theirs.query(api.extension.request, { requestId: started.requestId })).toEqual({ status: "not_found" });
  });

  it("show the server's sign-in steps", async () => {
    const t = await setup();
    const { ext, key, account } = await withKey(t);
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      await ctx.db.patch(id, { sealedVault: "vault", vaultSavedAt: Math.floor(Date.now() / 1000) });
    });
    engine(() => undefined);
    const started = (await (await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${key}` } })).json()) as { requestId: string };
    expect(await ext.query(api.extension.request, { requestId: started.requestId })).toMatchObject({ status: "pending", stage: "server", login: "signing_in" });
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("logins").collect())[0];
      await ctx.db.patch(row._id, { sealedState: "half", busy: false });
    });
    expect(await ext.query(api.extension.request, { requestId: started.requestId })).toMatchObject({ login: "waiting_otp" });
  });

  it("say when the app approved a pairing code, never with its key", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const codeHash = await sha256Hex("ABCDEFGHJK");
    expect(await t.query(api.extension.pairing, { codeHash })).toEqual({ ready: false });
    expect(await t.query(api.extension.pairing, { codeHash: "nope" })).toEqual({ ready: false });
    await as.action(api.keys.create, { flow: "f", pairCode: "ABCDE-FGHJK" });
    const approved = await t.query(api.extension.pairing, { codeHash });
    expect(approved).toEqual({ ready: true });
    const res = (await (await post(t, "/v1/pair", { code: "ABCDE-FGHJK" })).json()) as { status: string };
    expect(res.status).toBe("ready");
    expect(await t.query(api.extension.pairing, { codeHash })).toEqual({ ready: false });
  });
});
