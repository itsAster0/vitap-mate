import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import api from "../API.md?raw";
import type { Env } from "./common";
import worker from "./index";
import { APP_OPS } from "./account";
import { ROUTES, type Route, withRouteTable } from "./routes";
import { PHONE_ROUTES } from "./vtop-phone";

type Documented = { method: string | null; path: string };

/** `/a/<id>?x=1` and `/a/{id}` alike. */
function normal(path: string): string {
  return path.split("?")[0].replace(/<[^>]+>|\{[^}]+\}/g, "{}");
}

/** The routes API.md documents: `### \`METHOD /path\`` headings and the
 * first cell of each table headed `Route`. A cell may list several, and
 * `…/x` is the one before it plus `/x`. `/guides/*` is Static Assets. */
function documented(markdown: string): Documented[] {
  const out: Documented[] = [];
  let inRoutes = false;
  for (const line of markdown.split("\n")) {
    const heading = /^###\s+`([^`]+)`/.exec(line);
    let cells: string[] = [];
    if (heading) {
      cells = [heading[1]];
    } else if (line.startsWith("|")) {
      const first = line.split("|")[1].trim();
      if (first === "Route") inRoutes = true;
      else if (inRoutes && !first.startsWith("---")) cells = [...first.matchAll(/`([^`]+)`/g)].map((m) => m[1]);
    } else {
      inRoutes = false;
    }
    let previous: string | null = null;
    for (const cell of cells) {
      const match = /^(?:(GET|POST|DELETE|OPTIONS)\s+)?(\/\S*|…\/\S*)$/.exec(cell);
      if (!match) continue;
      const path: string = match[2].startsWith("…") ? `${previous}${match[2].slice(1)}` : match[2];
      previous = path;
      if (!path.startsWith("/guides/")) out.push({ method: match[1] ?? null, path: normal(path) });
    }
  }
  return out;
}

const fits = (route: Route, doc: Documented) =>
  normal(route.path) === doc.path && (doc.method === null || route.method === "ANY" || doc.method === route.method);

describe("ROUTES and API.md", () => {
  const docs = documented(api);

  it("finds the documented routes", () => {
    expect(docs.length).toBeGreaterThan(40);
  });

  it("documents every route", () => {
    const missing = ROUTES.filter((r) => !docs.some((d) => fits(r, d))).map((r) => `${r.method} ${r.path}`);
    expect(missing).toEqual([]);
  });

  it("lists every documented route", () => {
    const extra = docs.filter((d) => !ROUTES.some((r) => fits(r, d))).map((d) => `${d.method ?? "*"} ${d.path}`);
    expect(extra).toEqual([]);
  });

  it("has no route twice", () => {
    const keys = ROUTES.map((r) => `${r.method} ${r.path}`);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("has a one-line summary for every route", () => {
    expect(ROUTES.filter((r) => !r.summary.trim() || r.summary.includes("\n")).map((r) => r.path)).toEqual([]);
  });

  it("lists the phone and app routes the handlers have", () => {
    const listed = new Set(ROUTES.map((r) => r.path));
    expect([...Object.keys(PHONE_ROUTES), ...Object.keys(APP_OPS)].filter((p) => !listed.has(p))).toEqual([]);
  });

  it("has an up-to-date route table (else run bun run docs)", () => {
    expect(withRouteTable(api), "API.md's route table is stale: run bun run docs").toBe(api);
  });
});

describe("ROUTES and the dispatch", () => {
  // Nothing leaves the test: the VTOP proxy and the engine get this.
  beforeEach(() => {
    vi.stubGlobal("fetch", async () => new Response("{}", { status: 200 }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  const env = {
    MAIL_DOMAIN: "mail.test",
    ENGINE_URL: "https://engine.test",
    PUBLIC_URL: "https://edge.test",
    ALLOWED_ORIGINS: "",
    INTERNAL_TOKEN: "internal",
  } as unknown as Env;

  /** Whether the Worker dispatched the request somewhere: an unknown route
   * gets `404 not_found` (or `405`) before any binding is touched, so a
   * missing binding's throw (`500 internal_error`, or a throw) means it got
   * past the dispatch. */
  async function dispatched(method: string, path: string): Promise<boolean> {
    const url = `https://edge.test${path.replace(/\{[^}]+\}/g, "x").replace(/\*$/, "x")}`;
    try {
      const res = await worker.fetch(new Request(url, { method, body: method === "GET" ? undefined : "{}" }), env);
      if (res.status === 405) return false;
      if (res.status !== 404) return true;
      const body = (await res.json().catch(() => ({}))) as { code?: string };
      return body.code !== "not_found";
    } catch {
      return true;
    }
  }

  it("answers every route", async () => {
    const unanswered: string[] = [];
    for (const route of ROUTES) {
      const method = route.method === "ANY" ? "GET" : route.method;
      if (!(await dispatched(method, route.path))) unanswered.push(`${route.method} ${route.path}`);
    }
    expect(unanswered).toEqual([]);
  });

  it("refuses routes it does not have", async () => {
    expect(await dispatched("POST", "/v1/nothing")).toBe(false);
    expect(await dispatched("POST", "/v1/events")).toBe(false);
    expect(await dispatched("POST", "/v1/account/nothing")).toBe(false);
    expect(await dispatched("GET", "/v1/link")).toBe(false);
  });
});

describe("error replies", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", async () => new Response("{}", { status: 200 }));
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  // No Durable Object bindings: a route that reaches one throws.
  const env = {
    MAIL_DOMAIN: "mail.test",
    ENGINE_URL: "https://engine.test",
    PUBLIC_URL: "https://edge.test",
    ALLOWED_ORIGINS: "",
    INTERNAL_TOKEN: "internal",
  } as unknown as Env;

  const call = (method: string, path: string, init: { body?: string; headers?: Record<string, string> } = {}) =>
    worker.fetch(new Request(`https://edge.test${path}`, { method, ...init }), env);

  /** `{ code, error }`, both strings, as JSON with `status`. */
  async function expectError(res: Response, status: number, code: string): Promise<Record<string, unknown>> {
    expect(res.status).toBe(status);
    expect(res.headers.get("content-type")).toContain("application/json");
    const body = (await res.json()) as Record<string, unknown>;
    expect(body).toMatchObject({ code, error: expect.any(String) });
    expect((body.error as string).length).toBeGreaterThan(0);
    return body;
  }

  const device = { "x-vm-device": "0".repeat(32), "x-vm-time": "1", "x-vm-sig": "x" };

  it("auth none", async () => {
    await expectError(await call("POST", "/v1/nothing", { body: "{}" }), 404, "not_found");
    await expectError(await call("POST", "/v1/pair", { body: "not json" }), 400, "bad_request");
    await expectError(await call("POST", "/v1/pair/info", { body: "{}" }), 400, "bad_request");
    await expectError(await call("GET", "/v1/pair"), 405, "method_not_allowed");
    await expectError(await call("GET", "/authorize/poll"), 405, "method_not_allowed");
    // The pairing page reads `status`; `code` and `error` come with it.
    const poll = await expectError(await call("POST", "/authorize/poll", { body: "{}" }), 400, "bad_request");
    expect(poll.status).toBe("expired");
  });

  it("auth signed", async () => {
    await expectError(await call("POST", "/v1/devices/info", { body: "{}" }), 401, "bad_signature");
    await expectError(await call("POST", "/v1/mail-otp/inbox", { body: "{}" }), 401, "bad_signature");
    await expectError(await call("POST", "/v1/link", { body: "{}" }), 401, "bad_signature");
    await expectError(await call("POST", "/v1/vtop/fetch", { body: "{}" }), 401, "bad_signature");
    await expectError(await call("POST", "/v1/devices/register", { body: "nope" }), 400, "bad_request");
    await expectError(await call("GET", "/v1/devices/info"), 405, "method_not_allowed");
    await expectError(await call("POST", "/v1/events", { body: "{}" }), 405, "method_not_allowed");
    await expectError(await call("GET", "/v1/vtop/fetch"), 405, "method_not_allowed");
  });

  it("auth key", async () => {
    await expectError(await call("GET", "/v1/whoami"), 401, "key_unknown");
    await expectError(await call("POST", "/v1/session", { headers: { authorization: "Bearer nope" } }), 401, "key_unknown");
    const mcp = await call("POST", "/mcp", { body: "{}" });
    await expectError(mcp, 401, "unauthorized");
    expect(mcp.headers.get("www-authenticate")).toContain("resource_metadata=");
  });

  it("auth internal", async () => {
    for (const path of ["/internal/mail-otp/await", "/internal/udp-keys", "/internal/accounts/login-done"]) {
      await expectError(await call("POST", path, { body: "{}" }), 401, "unauthorized");
    }
    const internal = { authorization: "Bearer internal" };
    await expectError(await call("POST", "/internal/udp-keys", { body: "{}", headers: internal }), 400, "bad_request");
    await expectError(await call("POST", "/internal/accounts/nothing", { body: "{}", headers: internal }), 404, "not_found");
    await expectError(await call("POST", "/internal/nothing", { body: "{}", headers: internal }), 404, "not_found");
  });

  it("answers a throw with 500 internal_error, with CORS on account routes", async () => {
    await expectError(await call("POST", "/v1/devices/info", { body: "{}", headers: device }), 500, "internal_error");
    const linked = await call("POST", "/v1/link", { body: "{}", headers: { ...device, origin: "https://x.test" } });
    await expectError(linked, 500, "internal_error");
    expect(linked.headers.get("access-control-allow-origin")).toBe("*");
  });
});
