import { describe, expect, it } from "vitest";

import { proxyVtop, VTOP_ORIGIN } from "./vtop-proxy";

const PROXY = "https://edge.example";

function capture(response: Response) {
  const seen: Request[] = [];
  const fetcher = async (req: Request) => {
    seen.push(req);
    return response.clone();
  };
  return { seen, fetcher };
}

describe("proxyVtop", () => {
  it("sends the request to VTOP as the phone would, cookies and body included", async () => {
    const { seen, fetcher } = capture(new Response("ok"));
    const req = new Request(`${PROXY}/vtop/login?x=1`, {
      method: "POST",
      headers: {
        cookie: "JSESSIONID=a",
        "content-type": "application/x-www-form-urlencoded",
        origin: PROXY,
        referer: `${PROXY}/vtop/open/page`,
        "upgrade-insecure-requests": "1",
        "sec-fetch-user": "?1",
        priority: "u=0, i",
        host: PROXY.replace("https://", ""),
        "cf-connecting-ip": "1.2.3.4",
      },
      body: "a=b",
    });
    await proxyVtop(req, fetcher);
    const sent = seen[0];
    expect(sent.url).toBe(`${VTOP_ORIGIN}/vtop/login?x=1`);
    expect(sent.method).toBe("POST");
    expect(sent.headers.get("cookie")).toBe("JSESSIONID=a");
    expect(sent.headers.get("origin")).toBe(VTOP_ORIGIN);
    expect(sent.headers.get("referer")).toBe(`${VTOP_ORIGIN}/vtop/open/page`);
    // Browser headers the phone sends pass straight through (parity with
    // a direct request); hop/host/Cloudflare headers do not.
    expect(sent.headers.get("upgrade-insecure-requests")).toBe("1");
    expect(sent.headers.get("sec-fetch-user")).toBe("?1");
    expect(sent.headers.get("priority")).toBe("u=0, i");
    expect(sent.headers.get("host")).not.toBe(PROXY.replace("https://", ""));
    expect(sent.headers.get("cf-connecting-ip")).toBeNull();
    expect(sent.redirect).toBe("manual");
    expect(await sent.text()).toBe("a=b");
  });

  it("keeps redirects and cookies on the proxy, so the phone stays on it", async () => {
    const headers = new Headers({ location: `${VTOP_ORIGIN}/vtop/content` });
    headers.append("set-cookie", "JSESSIONID=n; Path=/vtop; Domain=vtop.vitap.ac.in; Secure; HttpOnly");
    headers.append("set-cookie", "SERVERID=s; Domain=.vitap.ac.in; Path=/");
    const { fetcher } = capture(new Response(null, { status: 302, headers }));
    const res = await proxyVtop(new Request(`${PROXY}/vtop/login`), fetcher);
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(`${PROXY}/vtop/content`);
    expect(res.headers.getSetCookie()).toEqual([
      "JSESSIONID=n; Path=/vtop; Secure; HttpOnly",
      "SERVERID=s; Path=/",
    ]);
  });

  it("forwards only VTOP's own paths", async () => {
    const { seen, fetcher } = capture(new Response("ok"));
    const res = await proxyVtop(new Request(`${PROXY}/vtop/../v1/x`), fetcher);
    // URL parsing has already resolved `..`: not a VTOP path, not forwarded.
    expect(res.status).toBe(404);
    expect(seen).toHaveLength(0);
  });
  it("does not forward the phone's address; VTOP sees Cloudflare", async () => {
    const { seen, fetcher } = capture(new Response("ok"));
    await proxyVtop(
      new Request(`${PROXY}/vtop/login`, {
        headers: {
          "cf-connecting-ip": "49.15.2.3",
          "x-forwarded-for": "49.15.2.3",
          "x-real-ip": "49.15.2.3",
          "true-client-ip": "49.15.2.3",
        },
      }),
      fetcher,
    );
    expect(seen[0].headers.get("true-client-ip")).toBeNull();
    expect(seen[0].headers.get("x-forwarded-for")).toBeNull();
    expect(seen[0].headers.get("x-real-ip")).toBeNull();
    expect(seen[0].headers.get("cf-connecting-ip")).toBeNull();
  });

  it("answers 504 when VTOP is too slow, instead of leaving the phone waiting", async () => {
    const hang = (req: Request) =>
      new Promise<Response>((_, reject) => {
        req.signal.addEventListener("abort", () => reject(new Error("aborted")));
      });
    const res = await proxyVtop(new Request(`${PROXY}/vtop/content`), hang, 10);
    expect(res.status).toBe(504);
    expect(((await res.json()) as { code: string }).code).toBe("vtop_timeout");
  });

  it("answers 502 when VTOP can't be reached", async () => {
    const fail = async () => {
      throw new Error("connection refused");
    };
    const res = await proxyVtop(new Request(`${PROXY}/vtop/content`), fail, 1000);
    expect(res.status).toBe(502);
    expect(((await res.json()) as { code: string }).code).toBe("vtop_unreachable");
  });
});
