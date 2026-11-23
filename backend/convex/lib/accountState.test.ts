// Ported from edge/src/account-state.test.ts: the account rules did not change.
import { describe, expect, it } from "vitest";

import {
  MAX_KEY_USES,
  AccountRecord,
  type KeyRecord,
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
  keyUse,
  keysToMove,
  leastRecent,
  parseSettings,
  phoneId,
  phonesForKey,
  plan,
  rfc3339,
  semesterId,
  toHeader,
  validCookies,
  fromFields,
  toFields,
} from "./accountState";
import { sha256Hex } from "./crypto";
import { sha256HexSync } from "./sha256";

const NOW = 1_800_000_000;

function account(): AccountRecord {
  return new AccountRecord("22BCE0001", NOW);
}

function phone(device: string): PhoneRecord {
  return { deviceId: device, label: null, addedAt: NOW, seenAt: NOW };
}

function key(hash: string, deviceId: string): KeyRecord {
  return { hash, label: "k", createdAt: NOW, lastUsedAt: null, deviceId, outings: false, uses: [] };
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
    expect(a.view([], [], null, false, NOW).vaultGmail).toBe(true);
    expect(a.statusView(null, 0, [], []).vaultGmail).toBe(true);
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
    expect(a.view([], [], null, false, NOW).credentialsRejectedAt).toBe(NOW + 5);
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

  it("show keys and phones only to a linked phone", () => {
    const a = account();
    const phones = [phone("d1"), { ...phone("d2"), seenAt: NOW + 5 }];
    const keys = [key("00abcdefgh", "d2"), { ...key("01abcdefgh", "d1"), outings: true }];
    const view = a.view(phones, keys, "d2", true, NOW);
    expect(view.thisPhone).toBe(true);
    expect(view.phoneCount).toBe(2);
    expect(view.phones.map((p) => [p.id, p.thisPhone, p.seenAt])).toEqual([
      [phoneId("d1"), false, NOW],
      [phoneId("d2"), true, NOW + 5],
    ]);
    expect(view.keys.map((k) => [k.id, k.phoneId, k.outings])).toEqual([
      ["00abcdef", phoneId("d2"), false],
      ["01abcdef", phoneId("d1"), true],
    ]);
    const outsider = a.view(phones, keys, "d9", true, NOW);
    expect(outsider).toMatchObject({ thisPhone: false, phones: [], keys: [], phoneCount: 2, linked: true });
    expect(a.statusView(null, 2, keys, []).keys.map((k) => k.id)).toEqual(["00abcdef", "01abcdef"]);
  });

  it("write a key's use at most once a minute, keeping the latest", () => {
    let k = key("00abcdefgh", "d1");
    const first = keyUse(k, NOW, "mcp")!;
    expect(first).toEqual({ lastUsedAt: NOW, uses: [{ at: NOW, via: "mcp" }] });
    k = { ...k, ...first };
    expect(keyUse(k, NOW + 59, "mcp")).toBeNull();
    k = { ...k, ...keyUse(k, NOW + 60, "extension")! };
    expect(k.uses).toEqual([
      { at: NOW + 60, via: "extension" },
      { at: NOW, via: "mcp" },
    ]);
    for (let n = 2; n < 14; n++) k = { ...k, ...keyUse(k, NOW + 60 * n, "extension")! };
    expect(k.uses).toHaveLength(MAX_KEY_USES);
    expect(k.uses[0].at).toBe(NOW + 60 * 13);
  });

  it("make room by the phone seen least recently", () => {
    const phones = Array.from({ length: MAX_PHONES }, (_, n) => ({ ...phone(`d${n}`), seenAt: NOW + 10 - n }));
    expect(leastRecent(phones)).toBe(MAX_PHONES - 1);
    // A phone never seen since it was linked counts from its link.
    expect(leastRecent([{ ...phone("a"), addedAt: NOW + 50, seenAt: 0 }, phone("b")])).toBe(1);
  });

  it("ask only the phone that made a key", () => {
    const phones = [phone("d1"), phone("d2")];
    expect(phonesForKey(key("mine", "d2"), phones)).toEqual(["d2"]);
    expect(phonesForKey(key("mine", "gone"), phones)).toEqual([]);
    expect(phonesForKey(undefined, phones)).toEqual([]);
  });

  it("move another phone's keys and unlinked phones' keys to this one", () => {
    const phones = [phone("old"), phone("other"), phone("new")];
    const keys = [key("k-old", "old"), key("k-other", "other"), key("k-gone", "gone")];
    expect(keysToMove(keys, phones, phoneId("nope"), "new")).toBeNull();
    expect(keysToMove(keys, phones, phoneId("new"), "new")).toBeNull();
    const moved = keysToMove(keys, phones, phoneId("old"), "new")!;
    expect(moved.source?.deviceId).toBe("old");
    expect(moved.moving.map((k) => k.hash)).toEqual(["k-old", "k-gone"]);
    // Without a phone: only the keys no linked phone answers for.
    expect(keysToMove(keys, phones, null, "new")).toEqual({ source: null, moving: [keys[2]] });
  });

  it("round trip through the stored fields", () => {
    const a = account();
    a.saveVault("v", NOW, true);
    a.saveSession("s", NOW, "phone");
    const back = fromFields(a.id, JSON.parse(JSON.stringify(toFields(a))), "abcdefghij");
    expect(toFields(back)).toEqual(toFields(a));
    expect(back.mailboxId).toBe("abcdefghij");
  });

  it("hash phone ids as Web Crypto does", async () => {
    for (const text of ["", "abc", "d".repeat(64), "ü".repeat(100)]) expect(sha256HexSync(text)).toBe(await sha256Hex(text));
  });
});

describe("session requests", () => {
  it("look as clients expect", () => {
    const r = new RequestRecord("r", NOW);
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
