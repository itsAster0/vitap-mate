// Unit tests for pure account state, key, cookie and session request rules.
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
  type RequestPhase,
  SESSION_CACHE_SECS,
  type StoredRequestFields,
  defaultSettings,
  errorCode,
  fromFields,
  fromRequestFields,
  fromHeader,
  isAccountId,
  keyId,
  keyLabel,
  keyUse,
  keysToMove,
  leastRecent,
  parseRequestError,
  parseSettings,
  phoneId,
  phonesForKey,
  plan,
  rfc3339,
  semesterId,
  toFields,
  toHeader,
  toRequestFields,
  validCookies,
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
    expect(parseSettings({ alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 })).toEqual({
      cacheSession: false,
      alwaysUseVault: false,
      phoneWaitSecs: 45,
      vaultTtlSecs: 86_400,
    });
    // Older apps omit cacheSession: preserves current value.
    expect(parseSettings({ alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 }, true)).toEqual({
      cacheSession: true,
      alwaysUseVault: false,
      phoneWaitSecs: 45,
      vaultTtlSecs: 86_400,
    });
    expect(parseSettings({ ...defaultSettings(), cacheSession: "yes" })).toBeNull();
    expect(JSON.parse(JSON.stringify(defaultSettings()))).toEqual({
      cacheSession: false,
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
    expect(errorCode("Error: failed")).toBe("login_failed");
    expect(parseRequestError("phone_unreachable: x")).toEqual({ code: "phone_unreachable", message: "x" });
    expect(parseRequestError("plain")).toEqual({ code: "login_failed", message: "plain" });
    expect(parseRequestError("Error: VTOP returned 500")).toEqual({ code: "login_failed", message: "Error: VTOP returned 500" });
    expect(parseRequestError("Timeout at 12:34: failed")).toEqual({ code: "login_failed", message: "Timeout at 12:34: failed" });
    expect(parseRequestError("https://vtop.vitap.ac.in")).toEqual({ code: "login_failed", message: "https://vtop.vitap.ac.in" });
    expect(parseRequestError("bad_code:")).toEqual({ code: "bad_code", message: "bad_code:" });
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
  it("follow the lifetime of saved credentials and never ask for piggybacked credentials", () => {
    const a = account();
    expect(a.storage(NOW)).toEqual({ kind: "off" });
    expect(a.usableVault(NOW)).toBeNull();
    expect(a.wantCredentials(NOW)).toBe(false);
    a.mailboxId = "m";
    a.saveVault("v", NOW);
    // Saving a vault turns cacheSession on.
    expect(a.settings.cacheSession).toBe(true);
    expect(a.storage(NOW)).toMatchObject({ kind: "offline_sign_in", vault: { sealed: "v", savedAt: NOW, expiresAt: null, gmail: false } });
    expect(a.usableVault(NOW + 10 * 86_400)).toBe("v");
    expect(a.wantCredentials(NOW)).toBe(false);
    a.settings.vaultTtlSecs = 86_400;
    expect(a.wantCredentials(NOW)).toBe(false);
    expect(a.usableVault(NOW + 86_399)).toBe("v");
    expect(a.view([], [], null, false, NOW).vaultExpiresAt).toBe(NOW + 86_400);
    expect(a.usableVault(NOW + 86_400)).toBeNull();
    expect(a.storage(NOW + 86_400)).toEqual({ kind: "session_only", session: null });
  });

  it("model storage state as a discriminated union and apply migration rules at the read boundary", () => {
    // Missing cacheSession with no vault -> off, even if a leftover sealedSession exists.
    const noVault = fromFields(
      "22BCE0001",
      {
        settings: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null },
        semesterId: null,
        sealedVault: null,
        vaultSavedAt: null,
        vaultGmail: false,
        sealedSession: "leftover",
        sessionSavedAt: NOW,
        sessionSource: "phone",
        credentialsRejectedAt: null,
        createdAt: NOW,
        updatedAt: NOW,
      },
      null,
    );
    expect(noVault.settings.cacheSession).toBe(false);
    expect(noVault.storage(NOW)).toEqual({ kind: "off" });
    expect(noVault.cachedSession(NOW)).toBeNull();
    expect(noVault.view([], [], null, false, NOW)).toMatchObject({ sessionSavedAt: null, sessionSource: null, vaultExpiresAt: null });

    // Missing cacheSession with a vault -> on (offline_sign_in) and keeps cached session.
    const withVault = fromFields(
      "22BCE0001",
      {
        settings: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: 86_400 },
        semesterId: null,
        sealedVault: "v",
        vaultSavedAt: NOW,
        vaultGmail: true,
        sealedSession: "s",
        sessionSavedAt: NOW,
        sessionSource: "server",
        credentialsRejectedAt: null,
        createdAt: NOW,
        updatedAt: NOW,
      },
      null,
    );
    expect(withVault.settings.cacheSession).toBe(true);
    expect(withVault.storage(NOW)).toEqual({
      kind: "offline_sign_in",
      session: { sealed: "s", savedAt: NOW, source: "server" },
      vault: { sealed: "v", savedAt: NOW, expiresAt: NOW + 86_400, gmail: true },
      options: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: 86_400 },
    });
    expect(withVault.view([], [], null, false, NOW)).toMatchObject({
      sessionSavedAt: NOW,
      sessionSource: "server",
      vaultExpiresAt: NOW + 86_400,
    });
    expect(withVault.statusView(null, 0, [], [], NOW).vaultGmail).toBe(true);
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

  it("cache sessions for a week only when cacheSession is on", () => {
    const a = account();
    // Default off refuses to cache.
    expect(a.saveSession("s", NOW, "server")).toBe(false);
    expect(a.cachedSession(NOW)).toBeNull();
    a.setSettings({ ...defaultSettings(), cacheSession: true });
    expect(a.saveSession("s", NOW, "server")).toBe(true);
    expect(a.storage(NOW)).toEqual({ kind: "session_only", session: { sealed: "s", savedAt: NOW, source: "server" } });
    expect(a.cachedSource()).toMatchObject({ from: "server", cached: true });
    expect(a.cachedSession(NOW + SESSION_CACHE_SECS - 1)).toBe("s");
    expect(a.cachedSession(NOW + SESSION_CACHE_SECS)).toBeNull();
    // Turning cacheSession off clears session and vault.
    a.setSettings({ ...defaultSettings(), cacheSession: false });
    expect(a.sealedSession).toBeNull();
    expect(a.storage(NOW)).toEqual({ kind: "off" });
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
    expect(a.statusView(null, 2, keys, [], NOW).keys.map((k) => k.id)).toEqual(["00abcdef", "01abcdef"]);
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
    r.markPhoneAsked();
    r.scheduleServer(NOW + 10);
    expect(r.pollView(NOW).body).toMatchObject({ stage: "phone" });
    expect(r.startedView().serverDueAt).toBe(rfc3339(NOW + 10));
    expect(r.startServer(NOW)).toBe(true);
    expect(r.startServer(NOW)).toBe(false);
    expect(r.pollView(NOW).body).toMatchObject({ stage: "server", serverDueAt: null });
    expect(r.pollView(NOW + REQUEST_TIMEOUT_SECS).status).toBe(410);
    r.fail(PHONE_UNREACHABLE);
    expect(r.phase).toEqual({ kind: "error", error: PHONE_UNREACHABLE });
    expect(r.failure()).toEqual(PHONE_UNREACHABLE);
    expect(r.pollView(NOW)).toEqual({
      status: 200,
      body: { status: "error", code: "phone_unreachable", error: PHONE_UNREACHABLE.message },
    });
    r.succeed("JSESSIONID=a", "phone");
    expect(r.phase).toEqual({ kind: "success", cookies: "JSESSIONID=a", csrfToken: null, source: "phone" });
    expect(r.failure()).toBeNull();
    const body = r.pollView(NOW + 1000, false).body as { status: string; session: { cookies: string }; cookies: { name: string }[]; cacheable: boolean };
    expect(body.status).toBe("ready");
    expect(body.session.cookies).toBe("JSESSIONID=a");
    expect(body.cookies[0].name).toBe("JSESSIONID");
    expect(body.cacheable).toBe(false);
    expect(r.scrub()).toBe(true);
    expect(r.scrub()).toBe(false);
    expect(r.phase).toEqual({ kind: "scrubbed", source: "phone" });
    expect(r.pollView(NOW + 1000, false).status).toBe(410);
    expect(r.startedView().pollAfterMs).toBe(POLL_AFTER_MS);
  });

  it("reads both structured and legacy packed error rows cleanly via parseRequestError", () => {
    const baseRow: StoredRequestFields = {
      status: "error",
      cookies: null,
      csrfToken: null,
      errorCode: null,
      error: "forwarding_required: Set up OTP email forwarding.",
      createdAt: NOW,
      expiresAt: NOW + REQUEST_TIMEOUT_SECS,
      vaultDueAt: null,
      vaultStarted: false,
      phoneAsked: false,
      phoneErrorCode: null,
      phoneError: null,
      source: null,
      phones: [],
    };
    const legacy = fromRequestFields("old", baseRow);
    expect(legacy.failure()).toEqual({
      code: "forwarding_required",
      message: "Set up OTP email forwarding.",
    });
    expect(legacy.pollView(NOW)).toEqual({
      status: 200,
      body: { status: "error", code: "forwarding_required", error: "Set up OTP email forwarding." },
    });
    // Non-snake-case legacy messages keep their full text under login_failed.
    const plainLegacy = fromRequestFields("old-plain", { ...baseRow, error: "Error: VTOP returned 500" });
    expect(plainLegacy.failure()).toEqual({ code: "login_failed", message: "Error: VTOP returned 500" });
    // Legacy packed phoneError on a pending row parses cleanly too.
    const pendingLegacy = fromRequestFields("old-pending", {
      ...baseRow,
      status: "pending",
      error: null,
      phoneError: "phone_unreachable: Open VITAP Mate.",
    });
    expect(pendingLegacy.phoneFailure()).toEqual({ code: "phone_unreachable", message: "Open VITAP Mate." });
  });

  it("round-trips every RequestPhase through StoredRequestFields and normalizes conflicting rows", () => {
    const phases: RequestPhase[] = [
      { kind: "pending", stage: { kind: "starting", phoneError: null } },
      { kind: "pending", stage: { kind: "starting", phoneError: PHONE_UNREACHABLE } },
      { kind: "pending", stage: { kind: "phone", vaultDueAt: null, pushSent: false } },
      { kind: "pending", stage: { kind: "phone", vaultDueAt: NOW + 20, pushSent: true } },
      { kind: "pending", stage: { kind: "server", phoneAsked: false, pushSent: false, phoneError: null } },
      { kind: "pending", stage: { kind: "server", phoneAsked: true, pushSent: true, phoneError: null } },
      { kind: "pending", stage: { kind: "server", phoneAsked: false, pushSent: false, phoneError: PHONE_UNREACHABLE } },
      { kind: "expired" },
      { kind: "success", cookies: "JSESSIONID=x", csrfToken: "tok", source: "server" },
      { kind: "scrubbed", source: "phone" },
      { kind: "error", error: { code: "credentials_rejected", message: "Bad password." } },
    ];
    for (const phase of phases) {
      const original = new RequestRecord("r", NOW, NOW + REQUEST_TIMEOUT_SECS, ["dev-1"], phase);
      const roundTripped = fromRequestFields(original.id, toRequestFields(original));
      expect(roundTripped.phase).toEqual(phase);
      expect(roundTripped.phones).toEqual(["dev-1"]);
    }
    // Conflicting stored combinations normalize to one valid phase at the read boundary.
    const wonOverExpired = fromRequestFields("win", {
      status: "success",
      cookies: "a=b",
      csrfToken: null,
      errorCode: "ignored",
      error: "ignored",
      createdAt: NOW,
      expiresAt: NOW + 10,
      vaultDueAt: NOW + 5,
      vaultStarted: true,
      phoneAsked: true,
      phoneError: "ignored",
      source: "phone",
      phones: [],
      expired: true,
    });
    expect(wonOverExpired.phase).toEqual({ kind: "success", cookies: "a=b", csrfToken: null, source: "phone" });
    expect(wonOverExpired.failure()).toBeNull();
    expect(wonOverExpired.phoneFailure()).toBeNull();

    const expiredPending = fromRequestFields("exp", {
      status: "pending",
      cookies: "stale",
      csrfToken: null,
      error: "stale",
      createdAt: NOW,
      expiresAt: NOW + 10,
      vaultDueAt: NOW + 5,
      vaultStarted: false,
      phoneAsked: true,
      phoneError: null,
      source: null,
      phones: ["dev-1"],
      expired: true,
    });
    expect(expiredPending.phase).toEqual({ kind: "expired" });
    expect(expiredPending.pending(NOW)).toBe(false);
    expect(expiredPending.waitingOnPhone(NOW, "dev-1")).toBe(false);
    expect(expiredPending.pollView(NOW).status).toBe(410);
  });

  it("write RFC 3339 dates", () => {
    expect(rfc3339(0)).toBe("1970-01-01T00:00:00Z");
    expect(rfc3339(1_709_210_096)).toBe("2024-02-29T12:34:56Z");
  });
});
