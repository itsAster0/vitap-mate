// The HTTP routes kept from the edge: access keys and pairing, OAuth for MCP
// clients (ported from edge/src/oauth.test.ts and pair.test.ts), `/mcp`, the
// gateway's keys, and the internal tokens.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, TOKENS, advance, codeOf, engine, phone, prove, setup, type T } from "../test/kit";
import { api } from "./_generated/api";
import { pairCode } from "./lib/pairing";
import { redirectMatches, redirectUriOk, s256 } from "./oauth";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

const post = (t: T, path: string, body: unknown, headers: Record<string, string> = {}) =>
  t.fetch(path, { method: "POST", headers: { "content-type": "application/json", ...headers }, body: typeof body === "string" ? body : JSON.stringify(body) });
const json = async (r: Response) => (await r.json()) as Record<string, unknown>;

/** Runs what is scheduled (the sign-in step `runAfter(0)` queues), keeping the clock where it is. */
async function drain(t: T) {
  const at = Date.now();
  vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
  vi.setSystemTime(at);
  await t.finishAllScheduledFunctions(vi.runAllTimers);
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
    expect((await post(t, "/internal/proof-ticket", { ticket: "0".repeat(32) }, { authorization: `Bearer ${TOKENS.INTERNAL_TOKEN}` })).status).toBe(404);
  });

  it("let the engine take a ticket and report its verdict over HTTP", async () => {
    const t = await setup();
    const p = await phone(1);
    const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    const auth = { authorization: `Bearer ${TOKENS.INTERNAL_TOKEN}` };
    expect(await json(await post(t, "/internal/proof-ticket", { ticket }, auth))).toEqual({ device: p.deviceId });
    expect((await post(t, "/internal/proof-done", { ticket }, auth)).status).toBe(400);
    expect((await post(t, "/internal/proof-done", { ticket, regno: REGNO, ownerHash: "o" }, auth)).status).toBe(204);
    const tokens = await t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device }));
    expect(tokens.regno).toBe(REGNO);
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
    expect((await post(t, "/v1/pair", { code: "ABCDE-FGHJK" })).status).toBe(202);
    expect(await as.query(api.pair.info, { code: "ABCDE-FGHJK" })).toEqual({ kind: "browser" });
    expect(await codeOf(as.query(api.pair.info, { code: "bad" }))).toBe("bad_request");
    const made = await as.action(api.keys.create, { flow: "f", pairCode: "abcde fghjk" });
    const ready = await post(t, "/v1/pair", { code: "ABCDEFGHJK" });
    expect(await json(ready)).toEqual({ status: "ready", key: made.key });
    expect((await post(t, "/v1/pair", { code: "ABCDEFGHJK" })).status).toBe(202);
    expect(await codeOf(as.action(api.keys.create, { flow: "f", pairCode: "nope" }))).toBe("bad_request");
  });

  it("ask the phone that made the key for a session, and take its answer", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f" });
    engine((path, body) => {
      if (path === "/engine/session") {
        const cookies = (body as { cookies?: string }).cookies ?? "JSESSIONID=abc";
        return Response.json({ status: "valid", registrationNumber: REGNO, cookies, sealedSession: "sealed", ownerHash: "o" });
      }
    });
    const started = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    expect(started.status).toBe(202);
    const pending = await json(started);
    expect(pending).toMatchObject({ status: "pending", stage: "phone" });
    // The phone sees the request live; a phone of another student does not.
    expect(await as.query(api.sessions.pending, { flow: "f" })).toEqual([
      { requestId: pending.requestId, wantCredentials: true, expiresAt: expect.any(Number) },
    ]);
    const answered = await as.action(api.sessions.answer, {
      flow: "f",
      requestId: pending.requestId as string,
      cookies: [{ name: "JSESSIONID", value: "abc", domain: "vtop.vitap.ac.in" }],
    });
    expect(answered).toEqual({ ok: true, late: false });
    const polled = await t.fetch(`/v1/session/requests/${pending.requestId}`, { headers: { authorization: `Bearer ${made.key}` } });
    expect(await json(polled)).toMatchObject({ status: "ready", session: { cookies: "JSESSIONID=abc" }, source: { from: "phone", cached: false } });
    // The answer is cached now.
    const cached = await t.fetch("/v1/session", { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
    expect(await json(cached)).toMatchObject({ status: "ready", source: { from: "phone", cached: true } });
    expect((await t.fetch("/v1/session/expire", { method: "POST", headers: { authorization: `Bearer ${made.key}` } })).status).toBe(204);
    expect(await as.query(api.sessions.pending, { flow: "f" })).toEqual([]);
    const other = await phone(2);
    const otherTokens = await prove(t, other, "23XYZ0002");
    expect(await otherTokens.as.query(api.sessions.pending, { flow: "f" })).toEqual([]);
    expect(await codeOf(otherTokens.as.action(api.sessions.answer, { flow: "f", requestId: pending.requestId as string, error: "x" }))).toBe("request_not_found");
  });

  it("let the engine sign in with saved credentials, and record it", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f" });
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      await ctx.db.patch(id, { sealedVault: "vault", vaultSavedAt: Math.floor(Date.now() / 1000) });
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
  });
});

describe("/mcp", () => {
  it("tells a client with no key where to get one, and passes a known key on to the engine", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    const calls = engine((path) => (path.startsWith("/mcp") ? Response.json({ jsonrpc: "2.0" }) : undefined));
    const none = await t.fetch("/mcp", { method: "POST", body: "{}" });
    expect(none.status).toBe(401);
    expect(none.headers.get("www-authenticate")).toContain("/.well-known/oauth-protected-resource/mcp");
    expect((await t.fetch("/mcp", { method: "POST", body: "{}", headers: { authorization: "Bearer vtm_unknown" } })).status).toBe(401);
    const ok = await t.fetch(`/mcp/${made.key}`, { method: "POST", body: "{}", headers: { "cf-connecting-ip": "1.2.3.4" } });
    expect(ok.status).toBe(200);
    expect(calls.at(-1)?.path).toBe(`/mcp/${made.key}`);
    expect(calls.at(-1)?.headers.get("x-forwarded-for")).toBe("1.2.3.4");
  });
});
