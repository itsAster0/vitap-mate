import { describe, expect, it } from "vitest";

import { handles } from "./account";

describe("account routes", () => {
  it("are the ones the edge hands to accounts", () => {
    for (const path of [
      "/v1/whoami",
      "/v1/session",
      "/v1/session/requests/abc",
      "/v1/session/answer",
      "/v1/link",
      "/v1/keys",
      "/v1/account",
      "/v1/account/credentials-rejected",
    ]) {
      expect(handles(path), path).toBe(true);
    }
    // `/v1/data/*` is routed in index.ts, before account routes.
    for (const path of ["/cookie", "/cookie/callback", "/v1/mail-otp/register", "/mcp", "/healthz", "/v1/data/refresh", "/v1/data/action"]) {
      expect(handles(path), path).toBe(false);
    }
  });
});
