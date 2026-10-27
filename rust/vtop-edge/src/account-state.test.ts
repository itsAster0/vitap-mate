import { describe, expect, it } from "vitest";

import {
  MAX_KEY_USES,
  MAX_SIGN_INS,
  ACCOUNT_RATE_PER_MINUTE,
  AccountRecord,
  MAX_KEYS_PER_ACCOUNT,
  MAX_PHONES,
  PHONE_UNREACHABLE,
  POLL_AFTER_MS,
  type PhoneRecord,
  REQUEST_TIMEOUT_SECS,
  RequestRecord,
  SESSION_CACHE_SECS,
  defaultSettings,
  errorCode,
  fromHeader,
  isAccountId,
  keyId,
  keyLabel,
  linkSecret,
  newAppSecret,
  newKey,
  parseSettings,
  phoneId,
  plan,
  rfc3339,
  semesterId,
  toHeader,
  validCookies,
} from "./account-state";
import { hash } from "./shared/tokens";

const NOW = 1_800_000_000;

function account(): AccountRecord {
  return new AccountRecord("22BCE0001", NOW);
}

function phone(device: string, secret: string): PhoneRecord {
  return { deviceId: device, secretHash: hash(secret), label: null, addedAt: NOW, seenAt: NOW };
}

describe("settings and ids", () => {
  it("settings only take offered choices", () => {
    expect(parseSettings(defaultSettings())).toEqual(defaultSettings());
    expect(parseSettings({ ...defaultSettings(), phoneWaitSecs: 11 })).toBeNull();
    expect(parseSettings({ alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 })).not.toBeNull();
    expect(JSON.parse(JSON.stringify(defaultSettings()))).toEqual({
      alwaysUseVault: true,
      phoneWaitSecs: 20,
      vaultTtlSecs: null,
    });
  });

  it("ids and labels", () => {
    expect(isAccountId("23ABC4567")).toBe(true);
    expect(isAccountId("23abc4567")).toBe(false);
    expect(isAccountId("a/b")).toBe(false);
    expect(keyId("abcdef0123456789")).toBe("abcdef01");
    expect(keyLabel("  ")).toBe("Access key");
    expect(keyLabel("x".repeat(80)).length).toBe(60);
    expect(newKey().startsWith("vtm_")).toBe(true);
    expect(newAppSecret().startsWith("vta_")).toBe(true);
    expect(semesterId(" VL2025 ")).toBe("VL2025");
    expect(semesterId("VL 2025")).toBeNull();
    expect(errorCode("phone_unreachable: x")).toBe("phone_unreachable");
    expect(errorCode("plain")).toBe("login_failed");
  });
});

describe("cookies", () => {
  it("from a header match the app's Cookie-Editor shape", () => {
    expect(JSON.parse(JSON.stringify(fromHeader("JSESSIONID=abc; ; bad; SERVERID=x=y")))).toEqual([
      {
        domain: "vtop.vitap.ac.in",
        hostOnly: true,
        httpOnly: false,
        name: "JSESSIONID",
        path: "/",
        sameSite: "unspecified",
        secure: true,
        session: true,
        storeId: "0",
        value: "abc",
        id: 1,
      },
      {
        domain: "vtop.vitap.ac.in",
        hostOnly: true,
        httpOnly: false,
        name: "SERVERID",
        path: "/",
        sameSite: "unspecified",
        secure: true,
        session: true,
        storeId: "0",
        value: "x=y",
        id: 4,
      },
    ]);
  });

  it("round trip through a header", () => {
    expect(toHeader(fromHeader("A=1; B=2"))).toBe("A=1; B=2");
  });

  it("need a name, a value and a domain", () => {
    const good = { domain: "d", name: "n", value: "v" };
    expect(validCookies([good])).toEqual([{ name: "n", value: "v" }]);
    expect(validCookies([])).toBeNull();
    expect(validCookies([{ ...good, name: " " }])).toBeNull();
    expect(validCookies([{ ...good, value: "" }])).toBeNull();
    expect(validCookies([{ ...good, domain: " " }])).toBeNull();
  });
});

describe("accounts", () => {
  it("follow the lifetime of saved credentials", () => {
    const a = account();
    expect(a.usableVault(NOW)).toBeNull();
    expect(a.wantCredentials(NOW)).toBe(true);
    a.mailboxId = "m";
    a.saveVault("v", NOW);
    expect(a.usableVault(NOW + 10 * 86_400)).toBe("v");
    expect(a.wantCredentials(NOW)).toBe(false);
    a.settings.vaultTtlSecs = 86_400;
    expect(a.wantCredentials(NOW)).toBe(true);
    expect(a.usableVault(NOW + 86_399)).toBe("v");
    expect(a.usableVault(NOW + 86_400)).toBeNull();
  });

  it("want credentials again while the server cannot read the OTP", () => {
    const a = account();
    a.saveVault("v", NOW);
    // No forwarding address and no Gmail access.
    expect(a.wantCredentials(NOW)).toBe(true);
    a.saveVault("v", NOW, true);
    expect(a.wantCredentials(NOW)).toBe(false);
    expect(a.view(null, false, NOW).vaultGmail).toBe(true);
    expect(a.statusView(null).vaultGmail).toBe(true);
  });

  it("drop only the Gmail access Google refused", () => {
    const a = account();
    a.saveVault("v", NOW, true);
    a.dropGmail("w");
    expect(a.usableVault(NOW)).toBe("w");
    expect(a.vaultSavedAt).toBe(NOW);
    expect(a.vaultGmail).toBe(false);
    // Nothing to drop it from once the credentials are gone.
    a.clearVault();
    a.dropGmail("x");
    expect(a.sealedVault).toBeNull();
  });

  it("drop rejected credentials until new ones come", () => {
    const a = account();
    a.saveVault("v", NOW);
    a.rejectCredentials(NOW + 5);
    expect(a.usableVault(NOW + 5)).toBeNull();
    expect(a.credentialsRejectedAt).toBe(NOW + 5);
    expect(a.view(null, false, NOW).credentialsRejectedAt).toBe(NOW + 5);
    a.saveVault("w", NOW + 9);
    expect(a.credentialsRejectedAt).toBeNull();
  });

  it("cache sessions for a week", () => {
    const a = account();
    a.saveSession("s", NOW, "server");
    expect(a.cachedSource()).toMatchObject({ from: "server", cached: true });
    expect(a.cachedSession(NOW + SESSION_CACHE_SECS - 1)).toBe("s");
    expect(a.cachedSession(NOW + SESSION_CACHE_SECS)).toBeNull();
    a.clearSession();
    expect(a.cachedSession(NOW)).toBeNull();
  });

  it("plan who is asked", () => {
    const settings = defaultSettings();
    expect(plan(settings, false)).toEqual({ kind: "phone_only" });
    expect(plan(settings, true)).toEqual({ kind: "vault_first" });
    settings.alwaysUseVault = false;
    expect(plan(settings, true)).toEqual({ kind: "phone_first", waitSecs: 20 });
  });

  it("limit, touch and revoke keys", () => {
    const a = account();
    a.addPhone(phone("d1", "vta_x"));
    for (let n = 0; n < MAX_KEYS_PER_ACCOUNT; n++) {
      expect(a.addKey(`${String(n).padStart(2, "0")}abcdefgh`, "k", "d1", NOW)).toBe(true);
    }
    expect(a.addKey("x", "k", "d1", NOW)).toBe(false);
    expect(a.hasKey("03abcdefgh")).toBe(true);
    expect(a.touchKey("03abcdefgh", NOW)).toBe(true);
    expect(a.touchKey("03abcdefgh", NOW + 59)).toBe(false);
    expect(a.touchKey("03abcdefgh", NOW + 60)).toBe(true);
    expect(a.revokeKey("03abcdef")).toBe("03abcdefgh");
    expect(a.hasKey("03abcdefgh")).toBe(false);
    expect(a.revokeKey("")).toBeNull();
    const view = a.view(0, true, NOW);
    expect(view.keys.length).toBe(MAX_KEYS_PER_ACCOUNT - 1);
    expect(view.keys[0].id).toBe("00abcdef");
    expect(a.view(null, true, NOW).keys).toEqual([]);
  });

  it("find phones by app secret", () => {
    const a = account();
    expect(a.phoneBySecret("vta_x")).toBeNull();
    a.addPhone(phone("d1", "vta_x"));
    a.addPhone(phone("d2", "vta_y"));
    expect(a.phoneBySecret(" vta_x ")).toBe(0);
    expect(a.phoneBySecret("vta_y")).toBe(1);
    expect(a.phoneBySecret("vta_z")).toBeNull();
    expect(a.phoneBySecret(undefined)).toBeNull();
  });

  it("link and unlink several phones", () => {
    const a = account();
    for (let n = 0; n < MAX_PHONES; n++) a.addPhone({ ...phone(`d${n}`, `vta_${n}`), seenAt: NOW + n });
    // Linking the same install again replaces it, not adds.
    a.addPhone(phone("d3", "vta_new"));
    expect(a.phones.length).toBe(MAX_PHONES);
    expect(a.phoneBySecret("vta_3")).toBeNull();
    // One more drops the least recently seen (d0).
    a.addPhone(phone("d9", "vta_9"));
    expect(a.phones.length).toBe(MAX_PHONES);
    expect(a.phones.every((p) => p.deviceId !== "d0")).toBe(true);
    const view = a.view(a.phoneBySecret("vta_9"), false, NOW);
    expect(view.phones.length).toBe(MAX_PHONES);
    expect(view.phones[MAX_PHONES - 1].thisPhone).toBe(true);
    expect(a.view(null, false, NOW).phones).toEqual([]);
    expect(a.view(null, false, NOW).phoneCount).toBe(MAX_PHONES);
    expect(a.removePhone(phoneId("d9"))?.deviceId).toBe("d9");
    expect(a.removePhone(phoneId("d9"))).toBeNull();
    expect(a.removePhone("")).toBeNull();
    expect(a.touchPhone(0, NOW + 7200)).toBe(true);
    expect(a.touchPhone(0, NOW + 7201)).toBe(false);
  });

  it("ask only the phone that made a key", () => {
    const a = account();
    a.addPhone(phone("d1", "vta_1"));
    a.addPhone(phone("d2", "vta_2"));
    a.addKey("mine", "k", "d2", NOW);
    expect(a.phonesForKey("mine")).toEqual(["d2"]);
    expect(a.view(0, false, NOW).keys[0].phoneId).toBe(phoneId("d2"));
    a.phoneMoved("d2", "d3");
    a.phones[1].deviceId = "d3";
    expect(a.phonesForKey("mine")).toEqual(["d3"]);
    a.removePhone(phoneId("d3"));
    expect(a.phonesForKey("mine")).toEqual([]);
  });

  it("reset the rate limit each minute", () => {
    const a = account();
    for (let i = 0; i < ACCOUNT_RATE_PER_MINUTE; i++) expect(a.allow(NOW)).toBe(true);
    expect(a.allow(NOW + 59)).toBe(false);
    expect(a.allow(NOW + 60)).toBe(true);
  });

  it("copy deeply when marked loaded", () => {
    const a = account();
    a.addPhone(phone("d1", "vta_1"));
    a.markLoaded();
    a.phones[0].seenAt = NOW + 1;
    a.settings.phoneWaitSecs = 45;
    expect(a.loaded?.phones[0].seenAt).toBe(NOW);
    expect(a.loaded?.settings.phoneWaitSecs).toBe(20);
    expect(a.loaded?.loaded).toBeNull();
  });
});

describe("session requests", () => {
  it("look as clients expect", () => {
    const r = new RequestRecord("r", "h", NOW);
    expect(r.pending(NOW)).toBe(true);
    expect(r.pollView(NOW).status).toBe(202);
    expect(r.pollView(NOW).body).toMatchObject({ stage: "starting" });
    r.phoneAsked = true;
    r.vaultDueAt = NOW + 10;
    expect(r.pollView(NOW).body).toMatchObject({ stage: "phone" });
    expect(r.startedView().serverDueAt).toBe(rfc3339(NOW + 10));
    r.vaultStarted = true;
    expect(r.pollView(NOW).body).toMatchObject({ stage: "server", serverDueAt: null });
    expect(r.pollView(NOW + REQUEST_TIMEOUT_SECS).status).toBe(410);
    r.fail(PHONE_UNREACHABLE);
    expect(r.pollView(NOW)).toMatchObject({ status: 200, body: { code: "phone_unreachable" } });
    r.succeed("JSESSIONID=a", "phone");
    const body = r.pollView(NOW + 1000).body as { status: string; session: { cookies: string }; cookies: { name: string }[] };
    expect(body.status).toBe("ready");
    expect(body.session.cookies).toBe("JSESSIONID=a");
    expect(body.cookies[0].name).toBe("JSESSIONID");
    expect(r.startedView().pollAfterMs).toBe(POLL_AFTER_MS);
  });

  it("write RFC 3339 dates", () => {
    expect(rfc3339(0)).toBe("1970-01-01T00:00:00Z");
    expect(rfc3339(1_709_210_096)).toBe("2024-02-29T12:34:56Z");
  });
});

describe("sign-ins and key uses", () => {
  it("keeps the newest sign-ins, newest first", () => {
    const a = account();
    for (let n = 0; n < 12; n++) a.recordSignIn(NOW + n, n % 2 ? "phone" : "server", n !== 5, n === 5 ? "x" : null);
    expect(a.signIns).toHaveLength(MAX_SIGN_INS);
    expect(a.signIns[0]).toEqual({ at: NOW + 11, by: "phone", ok: true, reason: null });
    expect(a.signIns.at(-1)?.at).toBe(NOW + 2);
    expect(a.signIns.find((s) => s.at === NOW + 5)).toEqual({ at: NOW + 5, by: "phone", ok: false, reason: "x" });
  });

  it("records a key use when its last use is written", () => {
    const a = account();
    a.addPhone(phone("d1", "vta_x"));
    a.addKey("00abcdefgh", "k", "d1", NOW);
    a.touchKey("00abcdefgh", NOW, "mcp");
    a.touchKey("00abcdefgh", NOW + 59, "mcp");
    const key = () => a.keys[0];
    expect(key().uses).toEqual([{ at: NOW, via: "mcp" }]);
    a.touchKey("00abcdefgh", NOW + 60);
    expect(key().uses).toEqual([
      { at: NOW + 60, via: "extension" },
      { at: NOW, via: "mcp" },
    ]);
    for (let n = 2; n < 14; n++) a.touchKey("00abcdefgh", NOW + 60 * n);
    expect(key().uses).toHaveLength(MAX_KEY_USES);
  });
  it("a phone linking again keeps its secret, so its requests in flight still count", () => {
    expect(linkSecret(" vta_mine ", 2)).toBe("vta_mine");
    const fresh = linkSecret("vta_stranger", null);
    expect(fresh.startsWith("vta_")).toBe(true);
    expect(fresh).not.toBe("vta_stranger");
    expect(linkSecret(undefined, null).startsWith("vta_")).toBe(true);
  });
});
