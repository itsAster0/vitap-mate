import { describe, expect, it, vi } from "vitest";

import { accountObject } from "../test/account-harness";
import { AccountRecord } from "./account-state";
import { hash } from "./shared/tokens";

const now = () => Math.floor(Date.now() / 1000);

function linked(): AccountRecord {
  const at = now();
  const a = new AccountRecord("22BCE0001", at);
  a.addPhone({ deviceId: "d1", secretHash: hash("vta_x"), label: null, addedAt: at, seenAt: at });
  a.addKey("00abcdefgh", "laptop", "d1", at);
  a.touchKey("00abcdefgh", at, "mcp");
  a.saveVault("sealed", 1000);
  a.saveSession("sess", 2000, "server");
  a.recordSignIn(3000, "server", true);
  a.mailboxId = "k3f9x2ab7q";
  a.settings = { ...a.settings, vaultTtlSecs: 86400 };
  return a;
}

const body = (secret = "vta_x") => ({ appSecret: secret, registrationNumber: "22BCE0001" });

describe("app status", () => {
  it("needs this phone's app secret", async () => {
    const rig = accountObject();
    rig.seed(linked());
    const response = await rig.account.appStatus(body("vta_wrong"));
    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({ code: "not_linked" });
  });

  it("reports what the server keeps and recent activity", async () => {
    const rig = accountObject(undefined, { MAIL_DOMAIN: "aster0.dev" });
    rig.seed(linked());
    const response = await rig.account.appStatus(body());
    expect(response.status).toBe(200);
    const status = (await response.json()) as Record<string, unknown>;
    expect(status).toMatchObject({
      sessionSavedAt: 2000,
      sessionSource: "server",
      vaultSavedAt: 1000,
      vaultExpiresAt: 1000 + 86400,
      credentialsRejectedAt: null,
      forwarding: "vtop-otp-k3f9x2ab7q@aster0.dev",
      signIns: [{ at: 3000, by: "server", ok: true, reason: null }],
      phones: 1,
    });
    const keys = status.keys as Array<Record<string, unknown>>;
    expect(keys).toHaveLength(1);
    expect(keys[0]).toMatchObject({ label: "laptop", uses: [{ via: "mcp" }] });
    expect(typeof keys[0].id).toBe("string");
  });

  it("clears the cached session", async () => {
    const rig = accountObject();
    rig.seed(linked());
    expect((await rig.account.appSessionClear(body())).status).toBe(200);
    expect(rig.load()!.sealedSession).toBeNull();
    const status = (await (await rig.account.appStatus(body())).json()) as Record<string, unknown>;
    expect(status.sessionSavedAt).toBeNull();
    expect(status.forwarding).toBeNull();
  });
});

describe("app login", () => {
  /** A linked account whose saved password is still usable. */
  const withVault = () => {
    const a = linked();
    a.saveVault("sealed", now());
    return a;
  };

  /** An engine that takes sign-ins, keeping each request id. */
  function engine() {
    const started: string[] = [];
    const fake = (path: string, body: unknown) => {
      if (path !== "/engine/login") return new Response(null, { status: 503 });
      started.push((body as { requestId: string }).requestId);
      return new Response(null, { status: 202 });
    };
    return { started, fake };
  }

  it("needs a saved password", async () => {
    const rig = accountObject();
    rig.seed(linked()); // its password is past the chosen lifetime
    const response = await rig.account.appLogin(body());
    expect(response.status).toBe(409);
    expect(await response.json()).toMatchObject({ code: "no_saved_credentials" });
  });

  it("signs in on the server and answers with the session", async () => {
    const { started, fake } = engine();
    const rig = accountObject(fake);
    rig.seed(withVault());
    const answer = rig.account.appLogin(body());
    await vi.waitFor(() => expect(started).toHaveLength(1));
    await rig.account.loginDone({
      account: "22BCE0001",
      requestId: started[0],
      outcome: { status: "ok", cookies: "JSESSIONID=s", sealedSession: "sealed-s", csrfToken: "c" },
    });
    const response = await answer;
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({
      status: "ok",
      session: { cookies: "JSESSIONID=s", csrf_token: "c", registration_number: "22BCE0001" },
    });
  });

  it("hands out the cached session with its CSRF token", async () => {
    const rig = accountObject((path) =>
      path === "/engine/session"
        ? Response.json({ status: "valid", cookies: "JSESSIONID=c", csrfToken: "t" })
        : new Response(null, { status: 503 }),
    );
    const a = withVault();
    a.saveSession("sealed-c", now(), "server");
    rig.seed(a);
    const response = await rig.account.appLogin(body());
    expect(await response.json()).toEqual({
      status: "ok",
      session: { cookies: "JSESSIONID=c", csrf_token: "t", registration_number: "22BCE0001" },
    });
  });

  it("drops a rejected password and does not try it again", async () => {
    const { started, fake } = engine();
    const rig = accountObject(fake);
    rig.seed(withVault());
    const answer = rig.account.appLogin(body());
    await vi.waitFor(() => expect(started).toHaveLength(1));
    await rig.account.loginDone({ account: "22BCE0001", requestId: started[0], outcome: { status: "invalid_credentials" } });
    const response = await answer;
    expect(response.status).toBe(401);
    expect(await response.json()).toMatchObject({ code: "invalid_credentials" });
    // The next ask finds no password and starts nothing.
    expect((await rig.account.appLogin(body())).status).toBe(409);
    expect(started).toHaveLength(1);
  });
});
