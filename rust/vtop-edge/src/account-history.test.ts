import { describe, expect, it } from "vitest";

import { accountObject } from "../test/account-harness";
import { AccountRecord, RequestRecord } from "./account-state";
import { keyVia } from "./account";
import { hash } from "./shared/tokens";

function linked(): AccountRecord {
  const at = Math.floor(Date.now() / 1000);
  const a = new AccountRecord("22BCE0001", at);
  a.addPhone({ deviceId: "d1", secretHash: hash("vta_x"), label: null, addedAt: at, seenAt: at });
  a.saveVault("sealed", at);
  return a;
}

describe("sign-in history", () => {
  it("records server sign-ins, ok and rejected", async () => {
    const rig = accountObject();
    rig.seed(linked());
    await rig.account.loginDone({ account: "22BCE0001", requestId: "r1", outcome: { status: "ok", cookies: "A=1", sealedSession: "s" } });
    await rig.account.loginDone({ account: "22BCE0001", requestId: "r2", outcome: { status: "invalid_credentials" } });
    const signIns = rig.load()!.signIns;
    expect(signIns.map(({ by, ok, reason }) => ({ by, ok, reason }))).toEqual([
      { by: "server", ok: false, reason: "VTOP rejected the saved password" },
      { by: "server", ok: true, reason: null },
    ]);
  });

  it("records why other server sign-ins failed", async () => {
    const rig = accountObject();
    rig.seed(linked());
    await rig.account.loginDone({ account: "22BCE0001", requestId: "r1", outcome: { status: "forwarding_required" } });
    await rig.account.loginDone({ account: "22BCE0001", requestId: "r2", outcome: { status: "failed", reason: "VTOP down" } });
    expect(rig.load()!.signIns.map((s) => s.reason)).toEqual(["VTOP down", "OTP forwarding is not set up"]);
  });

  it("records a phone's answer as a phone sign-in", async () => {
    const rig = accountObject(() =>
      Response.json({ status: "valid", registrationNumber: "22BCE0001", cookies: "A=1", sealedSession: "s" }),
    );
    rig.seed(linked());
    const request = new RequestRecord("req1", "tok", Math.floor(Date.now() / 1000));
    rig.inner.putRequest(request);
    const response = await rig.account.answer({
      requestId: "req1",
      responseToken: "tok",
      cookies: [{ name: "JSESSIONID", value: "a", domain: "vtop.vitap.ac.in" }],
    });
    expect(response.status).toBe(200);
    expect(rig.load()!.signIns[0]).toMatchObject({ by: "phone", ok: true });
  });
});

describe("keyVia", () => {
  it("is mcp only when the request says so", () => {
    expect(keyVia(new Request("https://e/v1/session", { headers: { "x-vtop-client": "mcp" } }))).toBe("mcp");
    expect(keyVia(new Request("https://e/v1/session"))).toBe("extension");
  });
});

describe("rejected password", () => {
  function devices(fail = false) {
    const appended: Array<{ device: string; event: unknown }> = [];
    const DEVICE = {
      getByName: (device: string) => ({
        append: async (event: unknown) => {
          if (fail) throw new Error("down");
          appended.push({ device, event });
          return 1;
        },
      }),
    };
    return { appended, DEVICE };
  }

  it("tells every linked phone", async () => {
    const { appended, DEVICE } = devices();
    const rig = accountObject(undefined, { DEVICE });
    const doc = linked();
    const at = Math.floor(Date.now() / 1000);
    doc.addPhone({ deviceId: "d2", secretHash: hash("vta_y"), label: null, addedAt: at, seenAt: at });
    rig.seed(doc);
    const response = await rig.account.loginDone({
      account: "22BCE0001",
      requestId: "r1",
      outcome: { status: "invalid_credentials" },
    });
    expect(response.status).toBeLessThan(300);
    expect(appended).toEqual([
      { device: "d1", event: { kind: "credentials_rejected" } },
      { device: "d2", event: { kind: "credentials_rejected" } },
    ]);
  });

  it("a phone that cannot be told does not fail the sign-in report", async () => {
    const { DEVICE } = devices(true);
    const rig = accountObject(undefined, { DEVICE });
    rig.seed(linked());
    const response = await rig.account.loginDone({
      account: "22BCE0001",
      requestId: "r1",
      outcome: { status: "invalid_credentials" },
    });
    expect(response.status).toBeLessThan(300);
    expect(rig.load()!.credentialsRejectedAt).not.toBeNull();
  });
});
