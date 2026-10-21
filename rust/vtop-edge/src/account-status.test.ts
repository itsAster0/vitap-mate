import { describe, expect, it } from "vitest";

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
