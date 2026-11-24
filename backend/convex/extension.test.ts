// The extension's socket: key JWTs and its live queries.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, codeOf, engine, phone, prove, setup, type T } from "../test/kit";
import { api } from "./_generated/api";
import { REQUEST_TIMEOUT_SECS, keyId } from "./lib/accountState";
import { sha256Hex } from "./lib/crypto";
import { PAIR_TTL_SECS } from "./lib/pairing";
import { REQUEST_TTL_SECS, s256 } from "./oauth";

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

  it("expires pairings, OAuth requests and session requests server-side so live queries re-run", async () => {
    const t = await setup();
    const { as, key } = await withKey(t);
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    // 1. Extension pairing row deleted at PAIR_TTL_SECS.
    const codeHash = await sha256Hex("ABCDE12345");
    await as.action(api.keys.create, { flow: "f", pairCode: "ABCDE-12345" });
    expect(await t.query(api.extension.pairing, { codeHash })).toEqual({ ready: true });
    // 2. OAuth authorize request marked expired at REQUEST_TTL_SECS while keys:create still sees pair_expired.
    const reg = await post(t, "/register", { redirect_uris: ["https://claude.ai/cb"], client_name: "Claude" });
    const { client_id } = (await reg.json()) as { client_id: string };
    const html = await (
      await t.fetch(
        `/authorize?${new URLSearchParams({
          response_type: "code",
          client_id,
          redirect_uri: "https://claude.ai/cb",
          code_challenge: await s256("v".repeat(43)),
          code_challenge_method: "S256",
        })}`,
      )
    ).text();
    const oauthCode = /aria-label="Pairing code">([0-9A-Z-]{11})</.exec(html)![1];
    expect(await as.query(api.pair.info, { code: oauthCode })).toEqual({ kind: "mcp", name: "Claude", host: "claude.ai" });
    // 3. Session request marked expired at REQUEST_TIMEOUT_SECS.
    const started = (await (await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${key}` } })).json()) as { requestId: string };
    expect(await as.query(api.sessions.pending, {})).toHaveLength(1);

    vi.advanceTimersByTime(REQUEST_TIMEOUT_SECS * 1000);
    await t.finishInProgressScheduledFunctions();
    expect((await t.run((ctx) => ctx.db.query("sessionRequests").first()))?.expired).toBe(true);
    expect(await as.query(api.sessions.pending, {})).toEqual([]);
    expect((await t.fetch(`/v1/session/requests/${started.requestId}`, { headers: { authorization: `Bearer ${key}` } })).status).toBe(410);

    vi.advanceTimersByTime((PAIR_TTL_SECS - REQUEST_TIMEOUT_SECS) * 1000);
    await t.finishInProgressScheduledFunctions();
    expect(await t.run((ctx) => ctx.db.query("pairings").collect())).toEqual([]);
    expect(await t.query(api.extension.pairing, { codeHash })).toEqual({ ready: false });

    vi.advanceTimersByTime((REQUEST_TTL_SECS - PAIR_TTL_SECS) * 1000);
    await t.finishInProgressScheduledFunctions();
    expect((await t.run((ctx) => ctx.db.query("oauthRequests").first()))?.expired).toBe(true);
    expect(await as.query(api.pair.info, { code: oauthCode })).toEqual({ kind: "browser" });
    expect(await codeOf(as.action(api.keys.create, { flow: "f", pairCode: oauthCode }))).toBe("pair_expired");
  });

  it("returns unchanged error shapes for structured and legacy sessionRequests rows", async () => {
    const t = await setup();
    const { ext, key, as, account } = await withKey(t);
    engine((path) => (path === "/engine/session" ? Response.json({ status: "expired" }) : undefined));
    const started = (await (await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${key}` } })).json()) as { requestId: string };
    expect(
      await codeOf(
        as.action(api.sessions.answer, {
          flow: "f",
          requestId: started.requestId,
          cookies: [{ name: "JSESSIONID", value: "expired", domain: "vtop.vitap.ac.in" }],
        }),
      ),
    ).toBe("session_invalid");
    const stored = await t.run(async (ctx) =>
      (await ctx.db.query("sessionRequests").collect()).find((r) => r.requestId === started.requestId),
    );
    expect(stored?.errorCode).toBe("phone_session_invalid");
    expect(stored?.error).toBe("VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.");
    const extView = await ext.query(api.extension.request, { requestId: started.requestId });
    expect(extView).toEqual({
      status: "error",
      code: "phone_session_invalid",
      error: "VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.",
      expiresAt: expect.any(Number),
    });
    const httpRes = await t.fetch(`/v1/session/requests/${started.requestId}`, { headers: { authorization: `Bearer ${key}` } });
    expect(httpRes.status).toBe(200);
    expect(await httpRes.json()).toEqual({
      status: "error",
      code: "phone_session_invalid",
      error: "VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.",
    });
    // Legacy row written before `errorCode` existed: still reads cleanly across HTTP, extension, and app queries.
    const nowSec = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      await ctx.db.insert("sessionRequests", {
        requestId: "r-legacy",
        accountId: ctx.db.normalizeId("accounts", account)!,
        status: "error",
        cookies: null,
        csrfToken: null,
        error: "credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate.",
        createdAt: nowSec,
        expiresAt: nowSec + 120,
        vaultDueAt: null,
        vaultStarted: true,
        phoneAsked: false,
        phoneError: null,
        source: null,
        phones: [],
      });
    });
    expect(await ext.query(api.extension.request, { requestId: "r-legacy" })).toEqual({
      status: "error",
      code: "credentials_rejected",
      error: "credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate.",
      expiresAt: (nowSec + 120) * 1000,
    });
    const legacyHttp = await t.fetch("/v1/session/requests/r-legacy", { headers: { authorization: `Bearer ${key}` } });
    expect(await legacyHttp.json()).toEqual({
      status: "error",
      code: "credentials_rejected",
      error: "credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate.",
    });
    expect(await as.query(api.app.loginResult, { requestId: "r-legacy" })).toEqual({
      status: "error",
      code: "invalid_credentials",
      error: "VTOP rejected the saved password.",
    });
  });
});
