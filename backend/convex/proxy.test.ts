import { describe, expect, it } from "vitest";
import { proxyOrigin } from "./proxy";

describe("proxy", () => {
  it("hand out only an https origin", () => {
    expect(proxyOrigin("https://vmate-proxy.aster0.dev")).toBe("https://vmate-proxy.aster0.dev");
    expect(proxyOrigin("https://vmate-proxy.aster0.dev/")).toBe("https://vmate-proxy.aster0.dev");
    for (const bad of [undefined, "", "http://x.example", "https://x.example/vtop", "not a url"]) {
      expect(proxyOrigin(bad)).toBeNull();
    }
  });
});
