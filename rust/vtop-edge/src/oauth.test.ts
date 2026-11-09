import { describe, expect, it } from "vitest";

import { accountObject } from "../test/account-harness";
import { memorySql } from "../test/memory-sql";
import { AccountRecord } from "./account-state";
import type { Env } from "./common";
import worker from "./index";
import { MAX_CLIENTS, OAuth, handle, pairInfo, redirectMatches, redirectUriOk, s256 } from "./oauth";

const BASE = "https://vmate.example";
const VERIFIER = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const REDIRECT = "https://claude.ai/api/mcp/auth_callback";

/** One `OAuth` object over in-memory SQLite, as `env.OAUTH`. */
function rig() {
  let alarm: number | null = null;
  const storage = {
    sql: memorySql(),
    getAlarm: async () => alarm,
    setAlarm: async (at: number) => void (alarm = at),
  };
  const object = new OAuth({ storage } as unknown as DurableObjectState, {} as Env);
  const OAUTH = { getByName: () => object };
  const env = { OAUTH, PUBLIC_URL: BASE } as unknown as Env;
  const call = async (method: string, path: string, body?: BodyInit, type = "application/json") => {
    const req = new Request(`${BASE}${path}`, { method, body, headers: body ? { "content-type": type } : {} });
    return (await handle(req, env, new URL(req.url).pathname))!;
  };
  return { object, env, OAUTH, call, alarm: () => alarm };
}

async function registered(r: ReturnType<typeof rig>, uris = [REDIRECT]): Promise<string> {
  const res = await r.call("POST", "/register", JSON.stringify({ client_name: "Claude", redirect_uris: uris, token_endpoint_auth_method: "client_secret_post" }));
  expect(res.status).toBe(201);
  const body = (await res.json()) as Record<string, unknown>;
  expect(body.token_endpoint_auth_method).toBe("none");
  expect(body).not.toHaveProperty("client_secret");
  return body.client_id as string;
}

function authorizePath(clientId: string, extra: Record<string, string> = {}): string {
  const params = new URLSearchParams({
    response_type: "code",
    client_id: clientId,
    redirect_uri: REDIRECT,
    code_challenge: s256(VERIFIER),
    code_challenge_method: "S256",
    state: "xyz",
    resource: `${BASE}/mcp`,
    ...extra,
  });
  return `/authorize?${params}`;
}

/** The pairing code and poll id the authorize page shows. */
async function opened(r: ReturnType<typeof rig>, clientId: string) {
  const res = await r.call("GET", authorizePath(clientId));
  expect(res.status).toBe(200);
  expect(res.headers.get("content-security-policy")).toContain("frame-ancestors 'none'");
  const html = await res.text();
  const code = /aria-label="Pairing code">([0-9A-Z]{5}-[0-9A-Z]{5})</.exec(html)![1];
  const poll = /poll:"([0-9a-f]{64})"/.exec(html)![1];
  expect(html).toContain("<svg");
  expect(html).toContain("Connect Claude");
  return { code, poll };
}

function tokenBody(fields: Record<string, string>): string {
  return new URLSearchParams({ grant_type: "authorization_code", redirect_uri: REDIRECT, code_verifier: VERIFIER, ...fields }).toString();
}

describe("metadata", () => {
  it("names this Worker as the authorization server", async () => {
    const r = rig();
    for (const path of ["/.well-known/oauth-protected-resource", "/.well-known/oauth-protected-resource/mcp"]) {
      const res = await r.call("GET", path);
      expect(res.headers.get("access-control-allow-origin")).toBe("*");
      expect(await res.json()).toMatchObject({ resource: `${BASE}/mcp`, authorization_servers: [BASE] });
    }
    const res = await r.call("GET", "/.well-known/oauth-authorization-server");
    expect(await res.json()).toMatchObject({
      issuer: BASE,
      authorization_endpoint: `${BASE}/authorize`,
      token_endpoint: `${BASE}/token`,
      registration_endpoint: `${BASE}/register`,
      code_challenge_methods_supported: ["S256"],
    });
    expect((await r.call("OPTIONS", "/token")).status).toBe(204);
  });

  it("answers /mcp without a token with where to get one", async () => {
    const env = { PUBLIC_URL: BASE } as unknown as Env;
    const res = await worker.fetch(new Request(`${BASE}/mcp`, { method: "POST", body: "{}" }), env);
    expect(res.status).toBe(401);
    expect(res.headers.get("www-authenticate")).toBe(
      `Bearer resource_metadata="${BASE}/.well-known/oauth-protected-resource/mcp"`,
    );
  });
});

describe("registration", () => {
  it("takes https, loopback and app-scheme redirect URIs only", () => {
    expect(redirectUriOk("https://claude.ai/cb")).toBe(true);
    expect(redirectUriOk("http://localhost:3334/callback")).toBe(true);
    expect(redirectUriOk("http://127.0.0.1/cb")).toBe(true);
    expect(redirectUriOk("cursor://anysphere.cursor-retrieval/oauth/callback")).toBe(true);
    expect(redirectUriOk("http://evil.example/cb")).toBe(false);
    expect(redirectUriOk("https://claude.ai/cb#x")).toBe(false);
    expect(redirectUriOk("javascript:alert(1)")).toBe(false);
    expect(redirectUriOk("data:text/html,hi")).toBe(false);
    expect(redirectUriOk("not a uri")).toBe(false);
  });

  it("matches redirect URIs exactly, loopback on any port", () => {
    expect(redirectMatches(["https://a.example/cb"], "https://a.example/cb")).toBe(true);
    expect(redirectMatches(["https://a.example/cb"], "https://a.example/cb2")).toBe(false);
    expect(redirectMatches(["https://a.example/cb"], "https://a.example/cb?x=1")).toBe(false);
    expect(redirectMatches(["http://localhost:3000/cb"], "http://localhost:51234/cb")).toBe(true);
    expect(redirectMatches(["http://localhost:3000/cb"], "http://localhost:51234/other")).toBe(false);
    expect(redirectMatches(["http://localhost:3000/cb"], "http://127.0.0.1:3000/cb")).toBe(false);
  });

  it("refuses bad metadata", async () => {
    const r = rig();
    expect((await r.call("POST", "/register", "nope")).status).toBe(400);
    expect((await r.call("POST", "/register", JSON.stringify({ redirect_uris: [] }))).status).toBe(400);
    const res = await r.call("POST", "/register", JSON.stringify({ redirect_uris: ["http://evil.example/cb"] }));
    expect(await res.json()).toMatchObject({ error: "invalid_redirect_uri" });
    const grants = await r.call("POST", "/register", JSON.stringify({ redirect_uris: [REDIRECT], grant_types: ["client_credentials"] }));
    expect(grants.status).toBe(400);
  });

  it("keeps at most MAX_CLIENTS, dropping the one unused longest", async () => {
    const r = rig();
    const first = await r.object.register("old", [REDIRECT], 1);
    const second = await r.object.register("used", [REDIRECT], 2);
    for (let i = 2; i < MAX_CLIENTS; i++) await r.object.register(`c${i}`, [REDIRECT], 10);
    await r.object.open(second, REDIRECT, s256(VERIFIER), null, 20);
    await r.object.register("new", [REDIRECT], 30);
    expect(await r.object.client(first)).toBeNull();
    expect(await r.object.client(second)).not.toBeNull();
  });
});

describe("authorize, approve in the app, token", () => {
  it("gives the client the key the app made, named after it, once", async () => {
    const r = rig();
    const clientId = await registered(r);
    const { code, poll } = await opened(r, clientId);
    expect(r.alarm()).not.toBeNull();

    expect(await (await pairInfo({ code }, r.env)).json()).toEqual({ kind: "mcp", name: "Claude", host: "claude.ai" });
    expect(await (await pairInfo({ code: "ABCDE-12345" }, r.env)).json()).toEqual({ kind: "browser" });

    const pending = await r.call("POST", "/authorize/poll", JSON.stringify({ poll }));
    expect(pending.status).toBe(202);

    // The app: Pair a browser with the page's code.
    const INDEX = { getByName: () => ({ put: async () => {} }) };
    const account = accountObject(undefined, { OAUTH: r.OAUTH, INDEX, PUBLIC_URL: BASE });
    const at = Math.floor(Date.now() / 1000);
    const doc = new AccountRecord("22BCE0001", at);
    doc.addPhone({ deviceId: "d1", label: null, addedAt: at, seenAt: at });
    const made = (await account.inner.linkedOp("create-key", doc, 0, { label: "Browser", pairCode: code, outings: false }, at)) as Response;
    expect(made.status).toBe(201);
    const { key, app } = (await made.json()) as { key: string; app: unknown };
    expect(app).toEqual({ name: "Claude", host: "claude.ai" });
    expect(doc.keys[0]).toMatchObject({ label: "Claude", outings: false });
    // Approved once only.
    expect(await (await pairInfo({ code }, r.env)).json()).toEqual({ kind: "browser" });

    const ready = await r.call("POST", "/authorize/poll", JSON.stringify({ poll }));
    expect(ready.status).toBe(200);
    const redirect = new URL(((await ready.json()) as { redirect: string }).redirect);
    expect(`${redirect.origin}${redirect.pathname}`).toBe(REDIRECT);
    expect(redirect.searchParams.get("state")).toBe("xyz");
    expect(redirect.searchParams.get("iss")).toBe(BASE);
    const grant = redirect.searchParams.get("code")!;
    // The request is gone once it became a code.
    expect((await r.call("POST", "/authorize/poll", JSON.stringify({ poll }))).status).toBe(410);

    const wrongClient = await r.call("POST", "/token", tokenBody({ code: grant, client_id: "vmc_other" }), "application/x-www-form-urlencoded");
    expect(await wrongClient.json()).toMatchObject({ error: "invalid_grant" });
    // Any try uses the code up.
    const again = await r.call("POST", "/token", tokenBody({ code: grant, client_id: clientId }), "application/x-www-form-urlencoded");
    expect(again.status).toBe(400);
  });

  it("checks the verifier and redirect_uri, and hands out a bearer token", async () => {
    const r = rig();
    const clientId = await registered(r);
    const grantFor = async () => {
      const { code, poll } = await opened(r, clientId);
      expect(await r.object.approve(code.replace("-", ""), "vtm_secret", Math.floor(Date.now() / 1000))).toBe(true);
      const ready = (await (await r.call("POST", "/authorize/poll", JSON.stringify({ poll }))).json()) as { redirect: string };
      return new URL(ready.redirect).searchParams.get("code")!;
    };
    const form = "application/x-www-form-urlencoded";
    const badVerifier = await r.call("POST", "/token", tokenBody({ code: await grantFor(), client_id: clientId, code_verifier: "x".repeat(43) }), form);
    expect(await badVerifier.json()).toMatchObject({ error: "invalid_grant" });
    const badRedirect = await r.call("POST", "/token", tokenBody({ code: await grantFor(), client_id: clientId, redirect_uri: "https://claude.ai/other" }), form);
    expect(await badRedirect.json()).toMatchObject({ error: "invalid_grant" });
    const ok = await r.call("POST", "/token", tokenBody({ code: await grantFor(), client_id: clientId }), form);
    expect(ok.status).toBe(200);
    expect(ok.headers.get("cache-control")).toBe("no-store");
    expect(await ok.json()).toEqual({ access_token: "vtm_secret", token_type: "Bearer" });
    const refresh = await r.call("POST", "/token", new URLSearchParams({ grant_type: "refresh_token", refresh_token: "x" }).toString(), form);
    expect(await refresh.json()).toMatchObject({ error: "unsupported_grant_type" });
  });

  it("sends nothing to an unknown client or redirect, and errors back to a known one", async () => {
    const r = rig();
    const clientId = await registered(r);
    expect((await r.call("GET", authorizePath("vmc_nope"))).status).toBe(400);
    expect((await r.call("GET", authorizePath(clientId, { redirect_uri: "https://evil.example/cb" }))).status).toBe(400);
    const cases: Array<Record<string, string>> = [{ code_challenge_method: "plain" }, { code_challenge: "" }, { response_type: "token" }, { resource: "https://evil.example/mcp" }];
    for (const extra of cases) {
      const res = await r.call("GET", authorizePath(clientId, extra));
      expect(res.status).toBe(302);
      const back = new URL(res.headers.get("location")!);
      expect(back.searchParams.get("error")).toBeTruthy();
      expect(back.searchParams.get("state")).toBe("xyz");
    }
  });

  it("lets an expired request go, and the app's late approval fails", async () => {
    const r = rig();
    const clientId = await registered(r);
    const at = 1000;
    const { pairCode, poll } = await r.object.open(clientId, REDIRECT, s256(VERIFIER), null, at);
    expect(await r.object.pending(pairCode, at + 601)).toBeNull();
    expect(await r.object.approve(pairCode, "vtm_k", at + 601)).toBe(false);
    expect(await r.object.poll(poll, BASE, at + 601)).toEqual({ status: "expired" });
  });
});
