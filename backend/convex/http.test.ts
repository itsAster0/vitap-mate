// HTTP routes: access keys and pairing, OAuth 2.1 for MCP clients, `/mcp`,
// gateway keys, and internal token checks.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, TOKENS, advance, codeOf, engine, phone, prove, setup, type T } from "../test/kit";
import { api, internal } from "./_generated/api";
import { REQUEST_TIMEOUT_SECS, SCRUB_REQUEST_SECS } from "./lib/accountState";
import { sha256Hex } from "./lib/crypto";
import { clientIp, proxyHops } from "./lib/httpKit";
import { pairCode } from "./lib/pairing";
import { MAX_CLIENTS, redirectMatches, redirectUriOk, s256 } from "./oauth";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

const post = (t: T, path: string, body: unknown, headers: Record<string, string> = {}) =>
  t.fetch(path, { method: "POST", headers: { "content-type": "application/json", ...headers }, body: typeof body === "string" ? body : JSON.stringify(body) });
const json = async (r: Response) => (await r.json()) as Record<string, unknown>;
const VERIFIER_TOKEN = "verifier-test";

/** Runs immediate scheduled jobs (`runAfter(0)`), keeping the clock where it is. */
async function drain(t: T) {
  const at = Date.now();
  vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
  vi.setSystemTime(at);
  await t.finishAllScheduledFunctions(() => vi.advanceTimersByTime(0));
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(at);
}

describe("basics", () => {
  it("answer healthz, not_found and CORS preflights", async () => {
    const t = await setup();
    expect(await json(await t.fetch("/healthz"))).toMatchObject({ service: "vmate-convex" });
    const missing = await t.fetch("/nope");
    expect(missing.status).toBe(404);
    expect((await json(missing)).code).toBe("not_found");
    const pre = await t.fetch("/v1/session", { method: "OPTIONS" });
    expect(pre.status).toBe(204);
    expect(pre.headers.get("access-control-allow-origin")).toBe("*");
  });

  it("guard the internal routes with their own tokens", async () => {
    const t = await setup();
    const report = { host: "gw.test", ports: [443], keys: [{ id: "k1", publicKey: "A".repeat(43), from: 1, until: 2 }] };
    expect((await post(t, "/internal/udp-keys", report)).status).toBe(401);
    expect((await post(t, "/internal/udp-keys", report, { authorization: `Bearer ${TOKENS.MAIL_TOKEN}` })).status).toBe(401);
    expect(await codeOf(t.query(api.udp.keys, {}))).toBe("udp_unavailable");
    expect((await post(t, "/internal/udp-keys", report, { authorization: `Bearer ${TOKENS.GATEWAY_TOKEN}` })).status).toBe(204);
    expect(await t.query(api.udp.keys, {})).toEqual(report);
    expect((await post(t, "/internal/udp-keys", { host: "x" }, { authorization: `Bearer ${TOKENS.GATEWAY_TOKEN}` })).status).toBe(400);
    // The gateway's token opens nothing else.
    expect((await post(t, "/internal/proof-ticket", { ticket: "0".repeat(32) }, { authorization: `Bearer ${TOKENS.GATEWAY_TOKEN}` })).status).toBe(401);
    // Nor does the engine's: only the verifier's opens the proof routes.
    expect((await post(t, "/internal/proof-ticket", { ticket: "0".repeat(32) }, { authorization: `Bearer ${TOKENS.INTERNAL_TOKEN}` })).status).toBe(401);
    expect((await post(t, "/internal/udp-keys", report, { authorization: `Bearer ${TOKENS.INTERNAL_TOKEN}` })).status).toBe(401);
  });

  it("let the verifier's token open the proof routes only", async () => {
    const t = await setup();
    vi.stubEnv("VERIFIER_TOKEN", VERIFIER_TOKEN);
    const verifier = { authorization: `Bearer ${VERIFIER_TOKEN}` };
    const ticket = { ticket: "0".repeat(32) };
    expect((await post(t, "/internal/proof-ticket", ticket, verifier)).status).toBe(404);
    expect((await post(t, "/internal/proof-done", { ...ticket, code: "proof_failed" }, verifier)).status).toBe(404);
    expect((await post(t, "/internal/proof-ticket", ticket, { authorization: `Bearer ${TOKENS.MAIL_TOKEN}` })).status).toBe(401);
    const report = { host: "gw.test", ports: [443], keys: [{ id: "k1", publicKey: "A".repeat(43), from: 1, until: 2 }] };
    expect((await post(t, "/internal/udp-keys", report, verifier)).status).toBe(401);
    expect((await post(t, "/internal/mail", { addressId: "kkkkkkkkkk", receivedAt: 1 }, verifier)).status).toBe(401);
    // Unset, no token works for it.
    vi.stubEnv("VERIFIER_TOKEN", "");
    expect((await post(t, "/internal/proof-ticket", ticket, { authorization: "Bearer " })).status).toBe(401);
  });

  it("let the verifier take a ticket and report its verdict, with the engine's owner hash", async () => {
    const t = await setup();
    vi.stubEnv("VERIFIER_TOKEN", VERIFIER_TOKEN);
    const calls = engine((path, body) => (path === "/engine/owner-hash" ? Response.json({ ownerHash: `owner-of-${(body as { registrationNumber: string }).registrationNumber}` }) : undefined));
    const p = await phone(1);
    const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    const auth = { authorization: `Bearer ${VERIFIER_TOKEN}` };
    expect(await json(await post(t, "/internal/proof-ticket", { ticket }, auth))).toEqual({ device: p.deviceId });
    expect((await post(t, "/internal/proof-done", { ticket }, auth)).status).toBe(400);
    // A hash the verifier sends is not taken: only the engine holds the key.
    expect((await post(t, "/internal/proof-done", { ticket, regno: REGNO.toLowerCase(), ownerHash: "forged" }, auth)).status).toBe(204);
    expect(calls.map((c) => [c.path, c.body, c.headers.get("authorization")])).toEqual([["/engine/owner-hash", { registrationNumber: REGNO }, `Bearer ${TOKENS.INTERNAL_TOKEN}`]]);
    const tokens = await t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device }));
    expect(tokens.regno).toBe(REGNO);
    const account = await t.run((ctx) => ctx.db.query("accounts").first());
    expect(account?.ownerHash).toBe(`owner-of-${REGNO}`);
  });

  it("prove an account without an owner hash when the engine can't give one", async () => {
    const t = await setup();
    engine(() => Response.json({ code: "unavailable" }, { status: 503 }));
    const p = await phone(1);
    const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    const auth = { authorization: `Bearer ${TOKENS.VERIFIER_TOKEN}` };
    await post(t, "/internal/proof-ticket", { ticket }, auth);
    expect((await post(t, "/internal/proof-done", { ticket, regno: REGNO }, auth)).status).toBe(204);
    expect((await t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device }))).regno).toBe(REGNO);
    expect((await t.run((ctx) => ctx.db.query("accounts").first()))?.ownerHash).toBeNull();
  });
});

describe("access keys", () => {
  it("are made by a linked phone, shown once, and answer whoami until revoked", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f", label: "Laptop" });
    expect(made.key).toMatch(/^vtm_[0-9a-f]{64}$/);
    expect(made.mcpUrl).toBe("https://vmate.test/mcp");
    const who = await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${made.key}` } });
    expect(await json(who)).toMatchObject({ registrationNumber: REGNO, keyLabel: "Laptop", keyOutings: false, phoneLinked: true });
    await as.mutation(api.keys.setOutings, { flow: "f", id: made.id, outings: true });
    expect((await json(await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${made.key}` } }))).keyOutings).toBe(true);
    expect(await codeOf(as.mutation(api.keys.setOutings, { flow: "f", id: "nope", outings: true }))).toBe("key_not_found");
    // Renamed: the new name shows; a blank one is refused.
    await as.mutation(api.keys.rename, { flow: "f", id: made.id, label: "  Claude on my laptop " });
    expect((await json(await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${made.key}` } }))).keyLabel).toBe("Claude on my laptop");
    expect(await codeOf(as.mutation(api.keys.rename, { flow: "f", id: made.id, label: "  " }))).toBe("bad_request");
    await as.mutation(api.keys.revoke, { flow: "f", id: made.id });
    expect((await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${made.key}` } })).status).toBe(401);
    expect(await codeOf(as.mutation(api.keys.revoke, { flow: "f", id: made.id }))).toBe("key_not_found");
    const rows = await t.run((ctx) => ctx.db.query("accessKeys").collect());
    expect(rows).toHaveLength(0);
  });

  it("stop at ten", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    for (let i = 0; i < 10; i++) await as.action(api.keys.create, { flow: "f" });
    expect(await codeOf(as.action(api.keys.create, { flow: "f" }))).toBe("key_limit");
  });

  it("go to the extension that showed the pairing code, once", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    expect(pairCode("abcde-fghjk")).toBe("ABCDEFGHJK");
    expect(pairCode("oil00-00000")).toBe("01100000000".slice(0, 10));
    expect(pairCode("short")).toBeNull();
    expect((await post(t, "/v1/pair", { code: "bad" })).status).toBe(400);
    const pendingPair = await post(t, "/v1/pair", { code: "ABCDE-FGHJK" });
    expect(pendingPair.status).toBe(202);
    expect(pendingPair.headers.get("cache-control")).toBe("no-store");
    expect(await as.query(api.pair.info, { code: "ABCDE-FGHJK" })).toEqual({ kind: "browser" });
    expect(await codeOf(as.query(api.pair.info, { code: "bad" }))).toBe("bad_request");
    const made = await as.action(api.keys.create, { flow: "f", pairCode: "abcde fghjk" });
    const ready = await post(t, "/v1/pair", { code: "ABCDEFGHJK" });
    expect(ready.headers.get("cache-control")).toBe("no-store");
    expect(await json(ready)).toEqual({ status: "ready", key: made.key });
    expect((await post(t, "/v1/pair", { code: "ABCDEFGHJK" })).status).toBe(202);
    expect(await codeOf(as.action(api.keys.create, { flow: "f", pairCode: "nope" }))).toBe("bad_request");
  });

  it("show an AI app's address only when our engine states it with the internal token", async () => {
    for (const [token, expected] of [
      [TOKENS.INTERNAL_TOKEN, { address: "198.51.100.4", seen: false }],
      ["wrong", null],
    ] as const) {
      const t = await setup();
      const { as } = await prove(t, await phone(1));
      await as.mutation(api.devices.update, { flow: "f", fcmToken: "fcm-tok" });
      const made = await as.action(api.keys.create, { flow: "f" });
      const started = await t.fetch("/v1/session", {
        method: "POST",
        headers: {
          authorization: `Bearer ${made.key}`,
          "x-vtop-client": "mcp",
          "x-vtop-internal": token,
          "x-vtop-asker-ip": "198.51.100.4",
          "x-forwarded-for": "203.0.113.9",
        },
      });
      expect(started.status).toBe(202);
      const [listed] = await as.query(api.sessions.pending, { flow: "f" });
      expect(listed.asker).toEqual({ keyLabel: expect.any(String), via: "mcp", ip: expected });
    }
  });

  it("ask the phone that made the key for a session, never store sealedSession when off, scrub after 30 s, and discard late answers", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    await as.mutation(api.devices.update, { flow: "f", fcmToken: "fcm-tok" });
    const made = await as.action(api.keys.create, { flow: "f" });
    const calls = engine((path, body) => {
      if (path === "/engine/session") {
        const b = body as { cookies?: string; seal?: boolean };
        const cookies = b.cookies ?? "JSESSIONID=abc";
        return Response.json({
          status: "valid",
          registrationNumber: REGNO,
          cookies,
          ...(b.seal === false ? {} : { sealedSession: "sealed" }),
          ownerHash: "o",
        });
      }
      if (path === "/engine/seal") return Response.json({ sealedVault: "should-not-seal" });
    });
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    const started = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}`, "x-forwarded-for": "203.0.113.7" } });
    expect(started.status).toBe(202);
    expect(started.headers.get("cache-control")).toBe("no-store");
    const pending = await json(started);
    expect(pending).toMatchObject({ status: "pending", stage: "phone", pushSent: true });
    // The phone sees the request live with wantCredentials: false, pushSent: true and who asked from where.
    expect(await as.query(api.sessions.pending, { flow: "f" })).toEqual([
      {
        requestId: pending.requestId,
        wantCredentials: false,
        pushSent: true,
        askedAt: expect.any(Number),
        expiresAt: expect.any(Number),
        asker: { keyLabel: expect.any(String), via: "extension", ip: { address: "203.0.113.7", seen: false } },
      },
    ]);
    // Piggybacked credentials are ignored and /engine/session is called with seal: false.
    const answered = await as.action(api.sessions.answer, {
      flow: "f",
      requestId: pending.requestId as string,
      cookies: [{ name: "JSESSIONID", value: "abc", domain: "vtop.vitap.ac.in" }],
      credentials: { username: REGNO, password: "pw", gmail: { refreshToken: "rt", clientId: "cid" } },
    });
    expect(answered).toEqual({ ok: true, late: false });
    expect(calls.find((c) => c.path === "/engine/session")?.body).toEqual({ cookies: "JSESSIONID=abc", seal: false });
    expect(calls.some((c) => c.path === "/engine/seal")).toBe(false);
    // Nothing written to accounts.sealedSession or sealedVault.
    const acctRow = await t.run(async (ctx) => (await ctx.db.get(ctx.db.normalizeId("accounts", account)!))!);
    expect(acctRow.sealedSession).toBeNull();
    expect(acctRow.sealedVault).toBeNull();

    const polled = await t.fetch(`/v1/session/requests/${pending.requestId}`, { headers: { authorization: `Bearer ${made.key}` } });
    expect(polled.headers.get("cache-control")).toBe("no-store");
    expect(await json(polled)).toMatchObject({
      status: "ready",
      cacheable: false,
      session: { cookies: "JSESSIONID=abc" },
      source: { from: "phone", cached: false },
    });

    // Before 30 s, cookies are still present; after 30 s, scrubRequest nulls cookies and csrfToken.
    vi.advanceTimersByTime((SCRUB_REQUEST_SECS - 1) * 1000);
    await t.finishInProgressScheduledFunctions();
    const beforeScrub = await t.run(async (ctx) =>
      (await ctx.db.query("sessionRequests").collect()).find((r) => r.requestId === pending.requestId),
    );
    expect(beforeScrub?.cookies).toBe("JSESSIONID=abc");
    vi.advanceTimersByTime(2_000);
    await t.finishInProgressScheduledFunctions();
    const scrubbedRow = await t.run(async (ctx) =>
      (await ctx.db.query("sessionRequests").collect()).find((r) => r.requestId === pending.requestId),
    );
    expect(scrubbedRow?.cookies).toBeNull();
    expect(scrubbedRow?.csrfToken).toBeNull();
    // Polling after scrub returns 410 request_expired (never empty cookies), and extension:request returns not_found.
    const afterScrub = await t.fetch(`/v1/session/requests/${pending.requestId}`, { headers: { authorization: `Bearer ${made.key}` } });
    expect(afterScrub.status).toBe(410);
    const ext = t.withIdentity({ subject: await sha256Hex(made.key), kind: "key" });
    expect(await ext.query(api.extension.request, { requestId: pending.requestId as string })).toEqual({ status: "not_found" });

    // Late answer when storage is off is discarded (writes neither cookies nor sealedSession nor signIns).
    const started2 = await json(await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } }));
    const signInsBefore = await as.query(api.account.signIns, { flow: "f" });
    vi.advanceTimersByTime(REQUEST_TIMEOUT_SECS * 1000);
    await t.finishInProgressScheduledFunctions();
    const lateAns = await as.action(api.sessions.answer, {
      flow: "f",
      requestId: started2.requestId as string,
      cookies: [{ name: "JSESSIONID", value: "late", domain: "vtop.vitap.ac.in" }],
    });
    expect(lateAns).toEqual({ ok: true, late: true });
    const lateRow = await t.run(async (ctx) =>
      (await ctx.db.query("sessionRequests").collect()).find((r) => r.requestId === started2.requestId),
    );
    expect(lateRow?.cookies).toBeNull();
    expect(await as.query(api.account.signIns, { flow: "f" })).toEqual(signInsBefore);

    // With storage on, phone answers ARE cached and served with cacheable: true.
    await as.mutation(api.account.setSettings, {
      flow: "f",
      settings: { cacheSession: true, alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null },
    });
    const started3 = await json(await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } }));
    await as.action(api.sessions.answer, {
      flow: "f",
      requestId: started3.requestId as string,
      cookies: [{ name: "JSESSIONID", value: "cached1", domain: "vtop.vitap.ac.in" }],
    });
    const polled3 = await json(await t.fetch(`/v1/session/requests/${started3.requestId}`, { headers: { authorization: `Bearer ${made.key}` } }));
    expect(polled3).toMatchObject({ status: "ready", cacheable: true, source: { from: "phone", cached: false } });
    const cached = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    expect(cached.headers.get("cache-control")).toBe("no-store");
    expect(await json(cached)).toMatchObject({ status: "ready", cacheable: true, source: { from: "phone", cached: true } });
    expect((await t.fetch("/v1/session/expire", { method: "POST", headers: { authorization: `Bearer ${made.key}` } })).status).toBe(204);
    expect(await as.query(api.sessions.pending, { flow: "f" })).toEqual([]);
    const other = await phone(2);
    const otherTokens = await prove(t, other, "23XYZ0002");
    expect(await otherTokens.as.query(api.sessions.pending, { flow: "f" })).toEqual([]);
    expect(await codeOf(otherTokens.as.action(api.sessions.answer, { flow: "f", requestId: pending.requestId as string, error: "x" }))).toBe("request_not_found");
  });

  it("evicts MCP RAM cache when a key is revoked", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f", label: "MCP" });
    const keyHash = await sha256Hex(made.key);
    const calls = engine((path) => (path === "/engine/mcp/evict" ? Response.json({ evicted: 1 }) : undefined));
    await as.mutation(api.keys.revoke, { flow: "rev-1", id: made.id });
    await drain(t);
    expect(calls).toHaveLength(1);
    expect(calls[0].path).toBe("/engine/mcp/evict");
    expect(calls[0].body).toEqual({ keyHashes: [keyHash] });
  });

  it("skips a stale sealedSession in sessions:start when cacheSession is off, and reports pushSent: false without FCM token", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    await t.run(async (ctx) => {
      await ctx.db.patch(ctx.db.normalizeId("accounts", account)!, {
        sealedSession: "stale",
        sessionSavedAt: Math.floor(Date.now() / 1000),
        sessionSource: "phone",
      });
    });
    const calls = engine(() => Response.json({ status: "valid", cookies: "SHOULD_NOT_BE_USED" }));
    const res = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    expect(res.status).toBe(202);
    expect(await json(res)).toMatchObject({ status: "pending", stage: "phone", pushSent: false });
    expect(calls).toHaveLength(0);
  });

  it("let the engine sign in with saved credentials, and record it", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f" });
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      const row = (await ctx.db.get(id))!;
      await ctx.db.patch(id, {
        settings: { ...row.settings, cacheSession: true },
        sealedVault: "vault",
        vaultSavedAt: Math.floor(Date.now() / 1000),
      });
    });
    const ok = { status: "ok", cookies: "a=b", sealedSession: "s", csrfToken: "c" };
    const calls = engine((path) => (path === "/engine/login" ? Response.json(ok) : undefined));
    const started = await json(await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } }));
    expect(started).toMatchObject({ status: "pending", stage: "server" });
    await drain(t);
    const login = calls.find((c) => c.path === "/engine/login")!;
    expect(login.body).toEqual({ account: REGNO, sealedVault: "vault", forwarding: false });
    expect(login.headers.get("x-vtop-regno")).toBe(REGNO);
    const polled = await json(await t.fetch(`/v1/session/requests/${started.requestId}`, { headers: { authorization: `Bearer ${made.key}` } }));
    expect(polled).toMatchObject({ status: "ready", source: { from: "server" } });
    const status = await as.query(api.app.status, { flow: "f" });
    expect(status.signIns[0]).toMatchObject({ by: "server", ok: true });
    expect(status.sessionSource).toBe("server");
    // A rejected password drops the vault and is recorded.
    await as.mutation(api.app.sessionClear, { flow: "f" });
    engine((path) => (path === "/engine/login" ? Response.json({ status: "invalid_credentials" }) : undefined));
    await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    await drain(t);
    const after = await as.query(api.app.status, { flow: "f" });
    expect(after.vaultSavedAt).toBeNull();
    expect(after.signIns[0]).toMatchObject({ by: "server", ok: false, reason: "VTOP rejected the saved password" });
    expect(after.signIns).toHaveLength(2);
  });

  it("are rate limited per account", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    engine(() => undefined);
    const ask = () => t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    for (let i = 0; i < 60; i++) await ask();
    expect((await ask()).status).toBe(429);
    advance(60);
    expect((await ask()).status).toBe(202);
    expect((await t.fetch("/v1/session", { method: "POST", headers: { authorization: "Bearer vtm_nope" } })).status).toBe(401);
  });
});

describe("OAuth for MCP clients", () => {
  it("takes https, loopback and app-scheme redirect URIs only, loopback on any port", () => {
    expect(redirectUriOk("https://claude.ai/cb")).toBe(true);
    expect(redirectUriOk("http://127.0.0.1:3000/cb")).toBe(true);
    expect(redirectUriOk("cursor://anysphere/cb")).toBe(true);
    expect(redirectUriOk("http://example.com/cb")).toBe(false);
    expect(redirectUriOk("javascript:alert(1)")).toBe(false);
    expect(redirectUriOk("https://a.test/cb#x")).toBe(false);
    expect(redirectMatches(["http://localhost:1/cb"], "http://localhost:9/cb")).toBe(true);
    expect(redirectMatches(["https://a.test/cb"], "https://a.test/cb2")).toBe(false);
  });

  it("gives the client the key the app made, named after it, once", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const reg = await post(t, "/register", { redirect_uris: ["https://claude.ai/cb"], client_name: "Claude" });
    expect(reg.status).toBe(201);
    const { client_id } = (await reg.json()) as { client_id: string };
    expect((await post(t, "/register", { redirect_uris: ["http://evil.test/cb"] })).status).toBe(400);
    const six = Array.from({ length: 6 }, (_, i) => `https://oauth-redirect.googleusercontent.com/r/${i}`);
    expect((await post(t, "/register", { redirect_uris: six, client_name: "Gemini" })).status).toBe(201);
    const tooMany = Array.from({ length: 21 }, (_, i) => `https://a.test/cb/${i}`);
    expect((await post(t, "/register", { redirect_uris: tooMany })).status).toBe(400);
    const verifier = "v".repeat(43);
    const query = new URLSearchParams({
      response_type: "code",
      client_id,
      redirect_uri: "https://claude.ai/cb",
      code_challenge: await s256(verifier),
      code_challenge_method: "S256",
      state: "st",
    });
    const pageRes = await t.fetch(`/authorize?${query}`);
    expect(pageRes.status).toBe(200);
    const html = await pageRes.text();
    const code = /aria-label="Pairing code">([0-9A-Z-]{11})</.exec(html)![1];
    const poll = /poll:"([0-9a-f]{64})"/.exec(html)![1];
    expect((await post(t, "/authorize/poll", { poll })).status).toBe(202);
    expect(await as.query(api.pair.info, { code })).toEqual({ kind: "mcp", name: "Claude", host: "claude.ai" });
    const made = await as.action(api.keys.create, { flow: "f", pairCode: code, label: "ignored" });
    expect(made.app).toEqual({ name: "Claude", host: "claude.ai" });
    const ready = await json(await post(t, "/authorize/poll", { poll }));
    const redirect = new URL(ready.redirect as string);
    expect(redirect.searchParams.get("state")).toBe("st");
    expect(redirect.searchParams.get("iss")).toBe("https://vmate.test");
    const form = (v: string) =>
      t.fetch("/token", {
        method: "POST",
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({ grant_type: "authorization_code", code: redirect.searchParams.get("code")!, client_id, redirect_uri: "https://claude.ai/cb", code_verifier: v }).toString(),
      });
    const bad = await form("w".repeat(43));
    expect(await json(bad)).toMatchObject({ error: "invalid_grant" });
    // The code is gone after any try.
    expect(await json(await form(verifier))).toMatchObject({ error: "invalid_grant" });
    const views = await as.query(api.account.view, { flow: "f" });
    expect(views.keys[0].label).toBe("Claude");
  });

  it("rejects expired or already-used OAuth codes with pair_expired and forces outings off", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const { client_id } = (await (await post(t, "/register", { redirect_uris: ["https://claude.ai/cb"], client_name: "Claude" })).json()) as { client_id: string };
    const open = async () => {
      const q = new URLSearchParams({
        response_type: "code",
        client_id,
        redirect_uri: "https://claude.ai/cb",
        code_challenge: await s256("v".repeat(43)),
        code_challenge_method: "S256",
      });
      const html = await (await t.fetch(`/authorize?${q}`)).text();
      return /aria-label="Pairing code">([0-9A-Z-]{11})</.exec(html)![1];
    };
    // Approving an MCP OAuth code forces outings off even when requested on, and a second use fails.
    const usedCode = await open();
    await as.action(api.keys.create, { flow: "f", pairCode: usedCode, outings: true });
    const view = await as.query(api.account.view, { flow: "f" });
    expect(view.keys[0].outings).toBe(false);
    expect(await codeOf(as.action(api.keys.create, { flow: "f", pairCode: usedCode }))).toBe("pair_expired");
    // An expired OAuth code fails with pair_expired instead of minting an extension pairing.
    const expiredCode = await open();
    advance(601);
    expect(await codeOf(as.action(api.keys.create, { flow: "f", pairCode: expiredCode }))).toBe("pair_expired");
    expect((await as.query(api.account.view, { flow: "f" })).keys).toHaveLength(1);
    expect(await t.run((ctx) => ctx.db.query("pairings").collect())).toEqual([]);
  });

  it("resolves client IP from X-Forwarded-For using PROXY_HOPS from the end and ignores cf-connecting-ip", async () => {
    const req = (headers: Record<string, string>) => new Request("https://vmate.test/x", { headers });
    expect(clientIp(req({ "cf-connecting-ip": "9.9.9.9" }), 1)).toBeNull();
    expect(clientIp(req({ "cf-connecting-ip": "9.9.9.9", "x-forwarded-for": "1.2.3.4, 10.0.0.1" }), 1)).toBe("10.0.0.1");
    expect(clientIp(req({ "x-forwarded-for": "1.2.3.4, 10.0.0.1" }), 2)).toBe("1.2.3.4");
    expect(clientIp(req({ "x-forwarded-for": "1.2.3.4, 10.0.0.1" }), 3)).toBe("1.2.3.4");
    expect(clientIp(req({ "x-forwarded-for": " , " }), 1)).toBeNull();

    vi.stubEnv("PROXY_HOPS", "1");
    expect(proxyHops()).toBe(1);
    vi.stubEnv("PROXY_HOPS", "2");
    expect(proxyHops()).toBe(2);
    for (const bad of ["", "0", "-1", "1.5", "abc"]) {
      vi.stubEnv("PROXY_HOPS", bad);
      expect(() => proxyHops()).toThrow();
    }
  });

  it("rate limits /register, /authorize, /authorize/poll, /token and /v1/pair per IP", async () => {
    const t = await setup();
    // Spoofed cf-connecting-ip and leftmost XFF entries do not bypass the limit (PROXY_HOPS=1).
    const ip1 = { "cf-connecting-ip": "198.51.100.1", "x-forwarded-for": "198.51.100.9, 203.0.113.1" };
    const ip1Spoofed = { "cf-connecting-ip": "198.51.100.2", "x-forwarded-for": "198.51.100.88, 203.0.113.1" };
    const ip2 = { "x-forwarded-for": "203.0.113.2" };
    // /register: per-IP limit blocks ip1 while ip2 still registers.
    let clientId = "";
    for (let i = 0; i < 10; i++) {
      const r = await post(t, "/register", { redirect_uris: ["https://a.test/cb"] }, ip1);
      expect(r.status).toBe(201);
      clientId = ((await r.json()) as { client_id: string }).client_id;
    }
    expect((await post(t, "/register", { redirect_uris: ["https://a.test/cb"] }, ip1Spoofed)).status).toBe(429);
    expect((await post(t, "/register", { redirect_uris: ["https://a.test/cb"] }, ip2)).status).toBe(201);
    // /authorize: per-IP limit.
    const authUrl = `/authorize?${new URLSearchParams({
      response_type: "code",
      client_id: clientId,
      redirect_uri: "https://a.test/cb",
      code_challenge: await s256("v".repeat(43)),
      code_challenge_method: "S256",
    })}`;
    for (let i = 0; i < 15; i++) expect((await t.fetch(authUrl, { headers: ip1 })).status).toBe(200);
    expect((await t.fetch(authUrl, { headers: ip1 })).status).toBe(429);
    expect((await t.fetch(authUrl, { headers: ip2 })).status).toBe(200);
    // /authorize/poll: per-IP limit.
    const pollBody = { poll: "a".repeat(64) };
    for (let i = 0; i < 30; i++) expect((await post(t, "/authorize/poll", pollBody, ip1)).status).toBe(410);
    expect((await post(t, "/authorize/poll", pollBody, ip1)).status).toBe(429);
    expect((await post(t, "/authorize/poll", pollBody, ip2)).status).toBe(410);
    // /token: per-IP limit.
    const tokenInit = (headers: Record<string, string>) => ({
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded", ...headers },
      body: new URLSearchParams({
        grant_type: "authorization_code",
        code: "bad",
        client_id: clientId,
        redirect_uri: "https://a.test/cb",
        code_verifier: "v".repeat(43),
      }).toString(),
    });
    for (let i = 0; i < 15; i++) expect((await t.fetch("/token", tokenInit(ip1))).status).toBe(400);
    expect((await t.fetch("/token", tokenInit(ip1))).status).toBe(429);
    expect((await t.fetch("/token", tokenInit(ip2))).status).toBe(400);
    // /v1/pair: per-IP limit.
    for (let i = 0; i < 30; i++) expect((await post(t, "/v1/pair", { code: "ABCDE-FGHJK" }, ip1)).status).toBe(202);
    expect((await post(t, "/v1/pair", { code: "ABCDE-FGHJK" }, ip1)).status).toBe(429);
    expect((await post(t, "/v1/pair", { code: "ABCDE-FGHJK" }, ip2)).status).toBe(202);
  });

  it("checks the verifier and hands out the key as a bearer token", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const { client_id } = (await (await post(t, "/register", { redirect_uris: ["http://localhost:1234/cb"] })).json()) as { client_id: string };
    const verifier = "x".repeat(50);
    const html = await (
      await t.fetch(`/authorize?${new URLSearchParams({ response_type: "code", client_id, redirect_uri: "http://localhost:9999/cb", code_challenge: await s256(verifier), code_challenge_method: "S256" })}`)
    ).text();
    const code = /aria-label="Pairing code">([0-9A-Z-]{11})</.exec(html)![1];
    const poll = /poll:"([0-9a-f]{64})"/.exec(html)![1];
    const made = await as.action(api.keys.create, { flow: "f", pairCode: code });
    const redirect = new URL((await json(await post(t, "/authorize/poll", { poll }))).redirect as string);
    const token = await t.fetch("/token", {
      method: "POST",
      body: new URLSearchParams({ grant_type: "authorization_code", code: redirect.searchParams.get("code")!, client_id, redirect_uri: "http://localhost:9999/cb", code_verifier: verifier }).toString(),
    });
    expect(await json(token)).toEqual({ access_token: made.key, token_type: "Bearer" });
    advance(601);
    expect((await post(t, "/authorize/poll", { poll })).status).toBe(410);
  });

  it("sends nothing to an unknown client or redirect", async () => {
    const t = await setup();
    expect((await t.fetch("/authorize?client_id=vmc_nope&redirect_uri=https://a.test/cb")).status).toBe(400);
    const { client_id } = (await (await post(t, "/register", { redirect_uris: ["https://a.test/cb"] })).json()) as { client_id: string };
    expect((await t.fetch(`/authorize?client_id=${client_id}&redirect_uri=https://b.test/cb`)).status).toBe(400);
    const back = await t.fetch(`/authorize?client_id=${client_id}&redirect_uri=https://a.test/cb&response_type=token`);
    expect(back.status).toBe(302);
    expect(new URL(back.headers.get("location")!).searchParams.get("error")).toBe("unsupported_response_type");
    const longState = "s".repeat(760);
    const okLong = await t.fetch(`/authorize?${new URLSearchParams({ response_type: "code", client_id, redirect_uri: "https://a.test/cb", code_challenge: await s256("v".repeat(43)), code_challenge_method: "S256", state: longState })}`);
    expect(okLong.status).toBe(200);
    const tooLong = await t.fetch(`/authorize?${new URLSearchParams({ response_type: "code", client_id, redirect_uri: "https://a.test/cb", code_challenge: await s256("v".repeat(43)), code_challenge_method: "S256", state: "s".repeat(4097) })}`);
    expect(tooLong.status).toBe(302);
    expect(new URL(tooLong.headers.get("location")!).searchParams.get("error")).toBe("invalid_request");
  });
});

describe("/mcp", () => {
  it("tells a client with no key where to get one, and passes a known key on to the engine", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    const calls = engine((path) => (path.startsWith("/mcp") ? Response.json({ jsonrpc: "2.0" }) : undefined));
    // OPTIONS preflight is answered directly with CORS (including DELETE) without hitting the engine.
    for (const path of ["/mcp", `/mcp/${made.key}`]) {
      const pre = await t.fetch(path, { method: "OPTIONS" });
      expect(pre.status).toBe(204);
      expect(pre.headers.get("access-control-allow-origin")).toBe("*");
      expect(pre.headers.get("access-control-allow-methods")).toBe("GET, POST, DELETE, OPTIONS");
      expect(pre.headers.get("access-control-expose-headers")).toBe("WWW-Authenticate");
    }
    expect(calls).toHaveLength(0);
    const none = await t.fetch("/mcp", { method: "POST", body: "{}" });
    expect(none.status).toBe(401);
    expect(none.headers.get("www-authenticate")).toContain("/.well-known/oauth-protected-resource/mcp");
    expect(none.headers.get("access-control-allow-origin")).toBe("*");
    expect(none.headers.get("access-control-expose-headers")).toBe("WWW-Authenticate");
    const badKey = await t.fetch("/mcp", { method: "POST", body: "{}", headers: { authorization: "Bearer vtm_unknown" } });
    expect(badKey.status).toBe(401);
    expect(badKey.headers.get("access-control-allow-origin")).toBe("*");
    const ok = await t.fetch(`/mcp/${made.key}`, {
      method: "POST",
      body: "{}",
      headers: { "cf-connecting-ip": "9.9.9.9", "x-forwarded-for": "8.8.8.8, 1.2.3.4" },
    });
    expect(ok.status).toBe(200);
    expect(ok.headers.get("access-control-allow-origin")).toBe("*");
    expect(ok.headers.get("access-control-allow-methods")).toBe("GET, POST, DELETE, OPTIONS");
    expect(calls.at(-1)?.path).toBe(`/mcp/${made.key}`);
    expect(calls.at(-1)?.headers.get("x-forwarded-for")).toBe("1.2.3.4");
    // Forwarded DELETE and upstream 401 also carry CORS (and WWW-Authenticate on 401).
    engine(() => new Response(null, { status: 204 }));
    const del = await t.fetch(`/mcp/${made.key}`, { method: "DELETE" });
    expect(del.status).toBe(204);
    expect(del.headers.get("access-control-allow-origin")).toBe("*");
    engine(() => Response.json({ error: "revoked" }, { status: 401 }));
    const upstream401 = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "{}" });
    expect(upstream401.status).toBe(401);
    expect(upstream401.headers.get("access-control-allow-origin")).toBe("*");
    expect(upstream401.headers.get("www-authenticate")).toContain("/.well-known/oauth-protected-resource/mcp");
    // Oversized body rejected before reaching the engine, with CORS.
    const big = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "x".repeat(65 * 1024) });
    expect(big.status).toBe(413);
    expect(big.headers.get("access-control-allow-origin")).toBe("*");
    expect((await json(big)).code).toBe("too_large");
    // Engine timeout returns 504 vtop_timeout, with CORS.
    engine(() => {
      throw new DOMException("The operation was aborted due to timeout", "TimeoutError");
    });
    const slow = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "{}" });
    expect(slow.status).toBe(504);
    expect(slow.headers.get("access-control-allow-origin")).toBe("*");
    expect((await json(slow)).code).toBe("vtop_timeout");
    // Engine unreachable returns 503 engine_unavailable, with CORS.
    engine(() => {
      throw new TypeError("connect refused");
    });
    const down = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "{}" });
    expect(down.status).toBe(503);
    expect(down.headers.get("access-control-allow-origin")).toBe("*");
    expect((await json(down)).code).toBe("engine_unavailable");
  });

  it("caps OAuth clients via oauthMeta counter and evicts the least-recently-used", async () => {
    const t = await setup();
    await t.run(async (ctx) => {
      await ctx.db.insert("oauthClients", { clientId: "vmc_oldest", name: "Old", redirectUris: ["https://a.test/cb"], createdAt: 1, usedAt: 1 });
      await ctx.db.insert("oauthClients", { clientId: "vmc_newer", name: "New", redirectUris: ["https://a.test/cb"], createdAt: 2, usedAt: 2 });
      await ctx.db.insert("oauthMeta", { count: MAX_CLIENTS });
    });
    await t.mutation(internal.oauth.register, { clientId: "vmc_latest", name: "Latest", redirectUris: ["https://a.test/cb"], at: 3 });
    const ids = await t.run(async (ctx) => (await ctx.db.query("oauthClients").collect()).map((c) => c.clientId));
    expect(ids).not.toContain("vmc_oldest");
    expect(ids).toContain("vmc_newer");
    expect(ids).toContain("vmc_latest");
    const meta = await t.run((ctx) => ctx.db.query("oauthMeta").first());
    expect(meta?.count).toBe(MAX_CLIENTS);
  });

  it("scrubs /mcp/<key> from logged error messages", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    engine((path) => {
      throw new TypeError(`fetch failed for https://engine.test${path}`);
    });
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      const res = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "{}" });
      expect(res.status).toBe(503);
      const line = String(spy.mock.calls[0]?.[0]);
      expect(line).toContain("/mcp/…");
      expect(line).not.toContain(made.key);
    } finally {
      spy.mockRestore();
    }
  });
});

describe("CORS and URL decoding", () => {
  it("keeps CORS headers on thrown errors and answers 400 for malformed URL encoding", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    const badReq = await t.fetch("/v1/session/requests/%E0%A4%A", {
      headers: { authorization: `Bearer ${made.key}`, origin: "chrome-extension://abc" },
    });
    expect(badReq.status).toBe(400);
    expect((await json(badReq)).code).toBe("bad_request");
    expect(badReq.headers.get("access-control-allow-origin")).toBe("*");
    const badGuide = await t.fetch("/guides/%E0%A4%A");
    expect(badGuide.status).toBe(400);
    expect((await json(badGuide)).code).toBe("bad_request");
  });

  it("does not touch accounts.updatedAt on sessions:start", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    const before = await t.run(async (ctx) => (await ctx.db.query("accounts").first())!.updatedAt);
    advance(120);
    engine(() => undefined);
    const res = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    expect(res.status).toBe(202);
    const after = await t.run(async (ctx) => (await ctx.db.query("accounts").first())!.updatedAt);
    expect(after).toBe(before);
  });

  it("validates sessions:answer credentials, guards missing requests in answered, and sets default timeout on enginePost", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    await expect(as.action(api.sessions.answer, { flow: "f", requestId: "r", credentials: { password: 123 } as never })).rejects.toThrow();
    expect(await codeOf(t.mutation(internal.sessions.answered, { requestId: "missing", cookies: "a=b" }))).toBe("request_not_found");
    const { enginePost } = await import("./lib/engine");
    let seenSignal: AbortSignal | null | undefined;
    vi.stubGlobal("fetch", async (_input: string | URL | Request, init?: RequestInit) => {
      seenSignal = init?.signal;
      return Response.json({ ok: true });
    });
    await enginePost("/engine/test", { a: 1 });
    expect(seenSignal).toBeInstanceOf(AbortSignal);
  });
});
