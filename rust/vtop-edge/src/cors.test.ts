import { describe, expect, it } from "vitest";

import type { Env } from "./common";
import { withCors } from "./index";

describe("withCors", () => {
  it("lets the extension send its access key", () => {
    const out = withCors(new Response(null, { status: 204 }), "chrome-extension://abc", { ALLOWED_ORIGINS: "" } as Env);
    expect(out.headers.get("access-control-allow-origin")).toBe("*");
    expect(out.headers.get("access-control-allow-headers")).toContain("Authorization");
  });
});
