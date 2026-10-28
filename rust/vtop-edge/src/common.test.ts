import { describe, expect, it } from "vitest";

import { benchResponse, type Env, healthBody, mcpUrl, unknownDevice } from "./common";
import { EDGE_VERSION } from "./version";

describe("mcpUrl", () => {
  it("is the public URL's /mcp", () => {
    expect(mcpUrl({ PUBLIC_URL: "https://h.example" } as Env)).toBe("https://h.example/mcp");
    expect(mcpUrl({ PUBLIC_URL: "https://h.example/" } as Env)).toBe("https://h.example/mcp");
  });
});

describe("unknownDevice", () => {
  it("tells the app to register this install again", async () => {
    const response = unknownDevice();
    expect(response.status).toBe(404);
    expect(await response.json()).toEqual({ code: "unknown_device", error: "This install is not registered." });
  });
});

describe("benchResponse", () => {
  it("sends the asked size, capped, uncached", async () => {
    const r = benchResponse(new URL("https://e.example/v1/bench?size=300"));
    expect((await r.arrayBuffer()).byteLength).toBe(300);
    expect(r.headers.get("cache-control")).toBe("no-store");
    expect((await benchResponse(new URL("https://e.example/v1/bench?size=99999")).arrayBuffer()).byteLength).toBe(20_000);
    expect((await benchResponse(new URL("https://e.example/v1/bench?size=x")).arrayBuffer()).byteLength).toBe(0);
  });
});

describe("healthBody", () => {
  it("names the service, version and region", () => {
    expect(healthBody({ colo: "SIN" })).toEqual({ service: "vtop-edge", status: "ok", version: EDGE_VERSION, colo: "SIN" });
  });

  it("has no region without a string colo", () => {
    expect(healthBody(undefined).colo).toBeNull();
    expect(healthBody({ colo: 5 }).colo).toBeNull();
  });
});
