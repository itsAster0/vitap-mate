import { describe, expect, it } from "vitest";

import { accountObject } from "../test/account-harness";
import { AccountRecord, RequestRecord } from "./account-state";
import { gmailGrantOf, keyVia } from "./account";

function linked(): AccountRecord {
  const at = Math.floor(Date.now() / 1000);
  const a = new AccountRecord("22BCE0001", at);
  a.addPhone({ deviceId: "d1", label: null, addedAt: at, seenAt: at });
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
    expect(rig.load()!.signIns.map((s) => s.reason)).toEqual(["VTOP down", "Neither OTP forwarding nor Gmail is set up"]);
    // The credentials stay for when the server can read the OTP.
    expect(rig.load()!.sealedVault).toBe("sealed");
  });

  it("keeps the password when Google refuses the Gmail access", async () => {
    const rig = accountObject();
    const doc = linked();
    doc.saveVault("sealed", doc.vaultSavedAt!, true);
    rig.seed(doc);
    await rig.account.loginDone({
      account: "22BCE0001",
      requestId: "r1",
      outcome: { status: "gmail_revoked", sealedVault: "without-gmail" },
    });
    const after = rig.load()!;
    expect(after.sealedVault).toBe("without-gmail");
    expect(after.vaultGmail).toBe(false);
    expect(after.signIns[0].reason).toBe("Google refused the saved Gmail access");
  });

  it("records a phone's answer as a phone sign-in", async () => {
    const rig = accountObject(() =>
      Response.json({ status: "valid", registrationNumber: "22BCE0001", cookies: "A=1", sealedSession: "s" }),
    );
    rig.seed(linked());
    const request = new RequestRecord("req1", Math.floor(Date.now() / 1000));
    request.phones = ["d1"];
    rig.inner.putRequest(request);
    const response = await rig.account.answer("d1", {
      requestId: "req1",
      cookies: [{ name: "JSESSIONID", value: "a", domain: "vtop.vitap.ac.in" }],
    });
    expect(response.status).toBe(200);
    expect(rig.load()!.signIns[0]).toMatchObject({ by: "phone", ok: true });
  });
});

describe("session answers", () => {
  it("come only from a phone that was asked", async () => {
    const rig = accountObject();
    const doc = linked();
    const at = Math.floor(Date.now() / 1000);
    doc.addPhone({ deviceId: "d2", label: null, addedAt: at, seenAt: at });
    rig.seed(doc);
    const request = new RequestRecord("req1", at);
    request.phones = ["d1"];
    rig.inner.putRequest(request);
    const cookies = [{ name: "JSESSIONID", value: "a", domain: "vtop.vitap.ac.in" }];
    const response = await rig.account.answer("d2", { requestId: "req1", cookies });
    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({ code: "not_asked" });
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
    doc.addPhone({ deviceId: "d2", label: null, addedAt: at, seenAt: at });
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

describe("Gmail access with credentials", () => {
  it("is read only when usable", () => {
    expect(gmailGrantOf({ refreshToken: " r ", clientId: "c" })).toEqual({
      refreshToken: "r",
      clientId: "c",
      deleteAfterReading: false,
    });
    expect(gmailGrantOf({ refreshToken: "r", clientId: "c", clientSecret: "s", deleteAfterReading: true })).toEqual({
      refreshToken: "r",
      clientId: "c",
      clientSecret: "s",
      deleteAfterReading: true,
    });
    expect(gmailGrantOf({ refreshToken: "", clientId: "c" })).toBeNull();
    expect(gmailGrantOf({ refreshToken: "r" })).toBeNull();
    expect(gmailGrantOf(null)).toBeNull();
  });

  /** A phone answering a request with credentials, for a student without
   * a forwarding address. */
  async function answerWith(credentials: Record<string, unknown>) {
    const sealed: unknown[] = [];
    const INDEX = { getByName: () => ({ get: async () => null }) };
    const rig = accountObject(
      (path, body) => {
        if (path === "/engine/seal") {
          sealed.push(body);
          return Response.json({ sealedVault: "sealed-new" });
        }
        return Response.json({
          status: "valid",
          registrationNumber: "22BCE0001",
          cookies: "A=1",
          sealedSession: "s",
          ownerHash: "owner",
        });
      },
      { INDEX },
    );
    const doc = linked();
    doc.clearVault();
    rig.seed(doc);
    const request = new RequestRecord("req1", Math.floor(Date.now() / 1000));
    request.phones = ["d1"];
    rig.inner.putRequest(request);
    const response = await rig.account.answer("d1", {
      requestId: "req1",
      cookies: [{ name: "JSESSIONID", value: "a", domain: "vtop.vitap.ac.in" }],
      credentials,
    });
    expect(response.status).toBe(200);
    return { sealed, after: rig.load()! };
  }

  it("lets the server keep credentials without forwarding", async () => {
    const gmail = { refreshToken: "r", clientId: "c", clientSecret: "s" };
    const { sealed, after } = await answerWith({ username: "student", password: "pw", gmail });
    expect(sealed).toEqual([
      { account: "22BCE0001", username: "student", password: "pw", gmail: { ...gmail, deleteAfterReading: false } },
    ]);
    expect(after.sealedVault).toBe("sealed-new");
    expect(after.vaultGmail).toBe(true);
  });

  it("is required when there is no forwarding address", async () => {
    const { sealed, after } = await answerWith({ username: "student", password: "pw" });
    expect(sealed).toEqual([]);
    expect(after.sealedVault).toBeNull();
  });
});

describe("clearing the sign-in history", () => {
  const checked = { status: "valid", registrationNumber: "22BCE0001", ownerHash: "o" } as const;
  const device = (id: string) => ({ id, signKey: "k", sealKey: "s", label: null, platform: "android", fcmToken: "" });

  it("drops every sign-in, for a linked phone only", async () => {
    const rig = accountObject();
    const doc = linked();
    doc.recordSignIn(10, "server", true);
    doc.recordSignIn(20, "phone", false, "VTOP down");
    rig.seed(doc);
    const call = (id: string) => rig.account.app("clear-sign-ins", { checked, mailboxId: null, body: {}, device: device(id) });
    expect((await call("d9")).status).toBe(403);
    expect(rig.load()!.signIns).toHaveLength(2);
    expect((await call("d1")).status).toBe(204);
    expect(rig.load()!.signIns).toEqual([]);
    // New sign-ins are kept again.
    await rig.account.loginDone({ account: "22BCE0001", requestId: "r1", outcome: { status: "failed", reason: "x" } });
    expect(rig.load()!.signIns).toHaveLength(1);
  });

  it("unlinks a phone that deletes itself, keeping the rest", async () => {
    const rig = accountObject();
    const doc = linked();
    doc.addPhone({ deviceId: "d2", label: null, addedAt: 1, seenAt: 1 });
    rig.seed(doc);
    await rig.account.forgetPhone("d1");
    await rig.account.forgetPhone("d9");
    expect(rig.load()!.phones.map((p) => p.deviceId)).toEqual(["d2"]);
  });
});
