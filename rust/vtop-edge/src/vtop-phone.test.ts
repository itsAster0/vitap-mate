import { describe, expect, it } from "vitest";

import type { Env } from "./common";
import { PACKED_PAGE, phoneVtop } from "./vtop-phone";

const EDGE = "https://edge.example";
const env = { ENGINE_URL: "https://engine.example/", BRIDGE_SECRET: "s3cret" } as Env;

function capture(response: Response) {
  const seen: Request[] = [];
  const fetcher = async (req: Request) => {
    seen.push(req);
    return response.clone();
  };
  return { seen, fetcher };
}

function post(path: string, body: unknown): Request {
  return new Request(`${EDGE}${path}`, { method: "POST", body: JSON.stringify(body) });
}

describe("phoneVtop", () => {
  it("passes a fetch to the engine with the shared secret, body unchanged", async () => {
    const { seen, fetcher } = capture(Response.json({ data: { semesters: [] } }));
    const body = { session: { cookies: "JSESSIONID=a" }, name: "semesters" };
    const res = await phoneVtop(post("/v1/vtop/fetch", body), env, fetcher);
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ data: { semesters: [] } });
    expect(seen[0].url).toBe("https://engine.example/engine/fetch");
    expect(seen[0].headers.get("authorization")).toBe("Bearer s3cret");
    expect(await seen[0].json()).toEqual(body);
  });

  it("passes a packed page on byte for byte", async () => {
    const bytes = new Uint8Array([0x1b, 0x00, 0xff, 0x7f, 0x80]);
    const { fetcher } = capture(new Response(bytes, { headers: { "content-type": PACKED_PAGE } }));
    const res = await phoneVtop(post("/v1/vtop/fetch", { name: "marks" }), env, fetcher);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe(PACKED_PAGE);
    expect(res.headers.get("cache-control")).toContain("no-transform");
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(bytes);
  });

  it("maps each sign-in step to its engine route", async () => {
    const { seen, fetcher } = capture(Response.json({ status: "ok" }));
    for (const path of ["/v1/vtop/login", "/v1/vtop/login/otp", "/v1/vtop/login/resend"]) {
      await phoneVtop(post(path, {}), env, fetcher);
    }
    expect(seen.map((r) => new URL(r.url).pathname)).toEqual([
      "/engine/vtop-login",
      "/engine/vtop-login/otp",
      "/engine/vtop-login/resend",
    ]);
  });

  it("passes the engine's refusals on as they are", async () => {
    const { fetcher } = capture(Response.json({ code: "session_expired", error: "x" }, { status: 401 }));
    const res = await phoneVtop(post("/v1/vtop/fetch", {}), env, fetcher);
    expect(res.status).toBe(401);
    expect(((await res.json()) as { code: string }).code).toBe("session_expired");
  });

  it("refuses GET and oversized bodies", async () => {
    const { fetcher, seen } = capture(new Response("{}"));
    expect((await phoneVtop(new Request(`${EDGE}/v1/vtop/fetch`), env, fetcher)).status).toBe(405);
    const big = new Request(`${EDGE}/v1/vtop/fetch`, { method: "POST", body: "x".repeat(70_000) });
    expect((await phoneVtop(big, env, fetcher)).status).toBe(413);
    expect(seen).toHaveLength(0);
  });

  it("says the engine is down, or slow, instead of failing", async () => {
    const fail = async () => {
      throw new Error("refused");
    };
    expect((await phoneVtop(post("/v1/vtop/fetch", {}), env, fail)).status).toBe(503);
    const hang = (req: Request) =>
      new Promise<Response>((_, reject) => req.signal.addEventListener("abort", () => reject(new Error("aborted"))));
    const res = await phoneVtop(post("/v1/vtop/fetch", {}), env, hang, 10);
    expect(res.status).toBe(504);
  });
});
