// Account management, device lifecycle, engine-proxied VTOP calls, guides,
// scheduled expiry transitions, sweeps and FCM pushes.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, T0, advance, codeOf, engine, phone, prove, setup } from "../test/kit";
import { api, internal } from "./_generated/api";
import type { Id } from "./_generated/dataModel";
import { guideFiles } from "../dev/guideFiles";
import { recordSignIn } from "./lib/accounts";
import { REQUEST_TIMEOUT_SECS, SESSION_CACHE_SECS } from "./lib/accountState";
import { DEMO_INACTIVE_SECS } from "./auth";
import { CONFIRMATION_TTL_SECS, SEALED_TTL_SECS } from "./lib/mailboxes";
import { sha256Hex } from "./lib/crypto";
import { GUIDE_FILES } from "./lib/guideFiles";
import { PACKED_PAGE } from "./vtop";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("the account", () => {
  it("takes the offered settings and a semester", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    await as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 } });
    expect(await codeOf(as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: true, phoneWaitSecs: 11, vaultTtlSecs: null } }))).toBe("bad_request");
    await expect(as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: "yes" } as never })).rejects.toThrow();
    await as.mutation(api.account.setSemester, { flow: "f", semesterId: " AP2026275 " });
    expect(await codeOf(as.mutation(api.account.setSemester, { flow: "f", semesterId: "bad id" }))).toBe("bad_request");
    const view = await as.query(api.account.view, {});
    expect(view).toMatchObject({ settings: { alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 }, semesterId: "AP2026275", forwarding: false });
    // The same semester again (every new JWT sends it) writes nothing.
    const before = await t.run(async (ctx) => (await ctx.db.query("accounts").first())!.updatedAt);
    advance(60);
    await as.mutation(api.account.setSemester, { flow: "f", semesterId: "AP2026275" });
    expect(await t.run(async (ctx) => (await ctx.db.query("accounts").first())!.updatedAt)).toBe(before);
  });

  it("forgets credentials and clears sign-ins", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    const at = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      const row = (await ctx.db.get(id))!;
      await ctx.db.patch(id, {
        settings: { ...row.settings, cacheSession: true },
        sealedVault: "v",
        vaultSavedAt: at,
      });
      await ctx.db.insert("signIns", { accountId: id, at, by: "server", ok: true, reason: null });
    });
    expect((await as.query(api.app.status, {})).signIns).toHaveLength(1);
    expect(await as.query(api.account.signIns, {})).toHaveLength(1);
    expect((await as.query(api.account.view, {})).savedCredentials).toBe(true);
    await as.mutation(api.account.forgetCredentials, { flow: "f" });
    expect((await as.query(api.account.view, {})).savedCredentials).toBe(false);
    await as.mutation(api.account.clearSignIns, { flow: "f" });
    expect((await as.query(api.app.status, {})).signIns).toEqual([]);
    expect(await as.query(api.account.signIns, {})).toEqual([]);
  });

  it("saves credentials only when the server can read the OTP, and turns cacheSession on", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    expect((await as.query(api.account.view, {})).settings.cacheSession).toBe(false);
    const calls = engine((path) => (path === "/engine/seal" ? Response.json({ sealedVault: "sv" }) : undefined));
    const password = { username: "22bce0001", password: "pw" };
    // No forwarding address and no Gmail access: nothing is sealed, cacheSession stays false.
    expect(await as.action(api.account.saveCredentials, { flow: "f", credentials: password })).toEqual({ savedCredentials: false, forwarding: false });
    expect(calls).toHaveLength(0);
    expect((await as.query(api.account.view, {})).settings.cacheSession).toBe(false);
    const gmail = { refreshToken: "rt", clientId: "cid", clientSecret: null, deleteAfterReading: false };
    expect(await as.action(api.account.saveCredentials, { flow: "f", label: "Pixel", credentials: { ...password, gmail, otp: "gmail" } })).toEqual({
      savedCredentials: true,
      forwarding: false,
    });
    expect(calls[0].body).toEqual({ account: REGNO, username: "22bce0001", password: "pw", gmail: { refreshToken: "rt", clientId: "cid", deleteAfterReading: false }, otp: "gmail" });
    expect(calls[0].headers.get("x-vtop-flow")).toBe("f");
    const view = await as.query(api.account.view, {});
    expect(view).toMatchObject({
      savedCredentials: true,
      vaultGmail: true,
      vaultSavedAt: T0,
      vaultExpiresAt: null,
      sessionSavedAt: null,
      sessionSource: null,
      settings: { cacheSession: true },
    });
    expect(view.phones[0].label).toBe("Pixel");
    // Older apps calling setSettings without cacheSession preserve cacheSession: true.
    await as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 } });
    expect(await as.query(api.account.view, {})).toMatchObject({
      settings: { cacheSession: true, alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 },
      vaultExpiresAt: T0 + 86_400,
    });
    // With a forwarding address, a password alone is enough; a failed seal keeps the old vault.
    await as.mutation(api.mail.register, { flow: "f" });
    engine(() => new Response(null, { status: 500 }));
    expect(await as.action(api.account.saveCredentials, { flow: "f", credentials: password })).toEqual({ savedCredentials: true, forwarding: true });
    await expect(as.action(api.account.saveCredentials, { flow: "f", credentials: null as never })).rejects.toThrow();
  });

  it("turns storage off in one mutation: clears session, vault, logins, and evicts MCP RAM cache", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f", label: "Claude" });
    const keyHash = await sha256Hex(made.key);
    const at = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      const row = (await ctx.db.get(id))!;
      await ctx.db.patch(id, {
        settings: { ...row.settings, cacheSession: true },
        sealedVault: "sv",
        vaultSavedAt: at,
        sealedSession: "ss",
        sessionSavedAt: at,
        sessionSource: "phone",
      });
      await ctx.db.insert("logins", {
        accountId: id,
        requestId: "r-inflight",
        since: at,
        phase: { kind: "waiting_otp", sealedState: "st" },
        expiresAt: at + 300,
      });
    });
    const calls = engine((path) => (path === "/engine/mcp/evict" ? Response.json({ evicted: 1 }) : undefined));
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    await as.mutation(api.account.setSettings, {
      flow: "turn-off",
      settings: { cacheSession: false, alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null },
    });
    // Immediately after the single mutation, session, vault and logins are all gone.
    const after = await t.run(async (ctx) => ({
      account: (await ctx.db.get(ctx.db.normalizeId("accounts", account)!))!,
      logins: await ctx.db.query("logins").collect(),
    }));
    expect(after.account.settings.cacheSession).toBe(false);
    expect(after.account.sealedSession).toBeNull();
    expect(after.account.sealedVault).toBeNull();
    expect(after.logins).toEqual([]);
    // The scheduled eviction notifies the engine with the account's key hashes.
    await t.finishAllScheduledFunctions(() => vi.advanceTimersByTime(100));
    expect(calls).toHaveLength(1);
    expect(calls[0].path).toBe("/engine/mcp/evict");
    expect(calls[0].body).toEqual({ keyHashes: [keyHash] });
    expect(calls[0].headers.get("x-vtop-flow")).toBe("turn-off");
    expect(calls[0].headers.get("x-vtop-regno")).toBe(REGNO);
  });

  it("refuses the demo's credentials and returns empty signIns for demo", async () => {
    const t = await setup();
    const p = await phone(1);
    const demo = await t.action(api.auth.token, await p.signed("auth:token", { demo: true, device: p.device }));
    const as = t.withIdentity({ subject: p.deviceId, regno: demo.regno, acct: demo.account, demo: true });
    expect(await codeOf(as.action(api.account.saveCredentials, { flow: "f", credentials: { username: "u", password: "p" } }))).toBe("demo_unavailable");
    expect(await as.query(api.account.signIns, {})).toEqual([]);
  });

  it("keeps the latest ten sign-ins, newest first", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    await t.run(async (ctx) => {
      const id = account as Id<"accounts">;
      for (let n = 0; n < 12; n++) await recordSignIn(ctx, id, { at: 100 + n, by: n % 2 ? "phone" : "server", ok: n !== 5, reason: n === 5 ? "x" : null });
    });
    const { signIns } = await as.query(api.app.status, {});
    expect(signIns.map((s) => s.at)).toEqual([111, 110, 109, 108, 107, 106, 105, 104, 103, 102]);
    expect(signIns.find((s) => s.at === 105)).toEqual({ at: 105, by: "phone", ok: false, reason: "x" });
    expect(await t.run((ctx) => ctx.db.query("signIns").collect())).toHaveLength(10);
  });

  it("links at most five phones, dropping the one seen least recently", async () => {
    const t = await setup();
    const proved: Array<{ p: Awaited<ReturnType<typeof phone>>; tokens: Awaited<ReturnType<typeof prove>> }> = [];
    for (let n = 1; n <= 6; n++) {
      const p = await phone(n);
      proved.push({ p, tokens: await prove(t, p) });
      advance(10);
    }
    expect(await codeOf(proved[0].tokens.as.query(api.account.view, {}))).toBe("device_removed");
    const view = await proved[5].tokens.as.query(api.account.view, {});
    expect(view.phoneCount).toBe(5);
    expect(view.phones.filter((p) => p.thisPhone)).toHaveLength(1);
    const first = (await t.run((ctx) => ctx.db.query("devices").collect())).find((d) => d.deviceId === proved[0].p.deviceId);
    // The install stays, unlinked, and links again on its next proof.
    expect(first?.accountId).toBeUndefined();
    expect(await codeOf(t.action(api.auth.token, await proved[0].p.signed("auth:token", { refreshToken: proved[0].tokens.refreshToken })))).toBe("refresh_invalid");
  });

  it("removes a phone by its short id", async () => {
    const t = await setup();
    const ta = await prove(t, await phone(1));
    advance(10);
    const tb = await prove(t, await phone(2));
    const other = (await tb.as.query(api.account.view, {})).phones.find((p) => !p.thisPhone)!;
    expect(await codeOf(tb.as.mutation(api.account.removeDevice, { flow: "f", id: "nope" }))).toBe("phone_not_found");
    await tb.as.mutation(api.account.removeDevice, { flow: "f", id: other.id });
    expect(await codeOf(ta.as.query(api.account.view, {}))).toBe("device_removed");
    expect((await tb.as.query(api.account.view, {})).phoneCount).toBe(1);
  });

  it("moves another phone's keys to this one and leaves that phone signed in", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    await ta.as.action(api.keys.create, { flow: "f" });
    advance(10);
    const tb = await prove(t, b);
    const old = (await tb.as.query(api.account.view, {})).phones.find((p) => !p.thisPhone)!;
    expect(await codeOf(tb.as.mutation(api.keys.move, { flow: "f", from: "nope" }))).toBe("phone_not_found");
    expect(await tb.as.mutation(api.keys.move, { flow: "f", from: old.id })).toEqual({ moved: 1 });
    const after = await ta.as.query(api.account.view, {});
    expect(after.phones).toHaveLength(2);
    const mine = after.phones.find((p) => !p.thisPhone)!.id;
    expect(after.keys.map((k) => k.phoneId)).toEqual([mine]);
  });

  it("deletes itself: keys go, and every phone loses access", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, refreshToken } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f" });
    await as.mutation(api.mail.register, { flow: "f" });
    await as.mutation(api.account.remove, { flow: "f" });
    const left = await t.run(async (ctx) => {
      const counts: Record<string, number> = {};
      for (const table of ["accounts", "accessKeys", "signIns", "sessionRequests", "mailboxes", "mailboxDevices", "otpInbox", "refreshTokens", "v2Tokens", "pairings", "oauthCodes"] as const) {
        counts[table] = (await ctx.db.query(table).collect()).length;
      }
      return counts;
    });
    expect(Object.values(left).every((n) => n === 0)).toBe(true);
    expect((await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${made.key}` } })).status).toBe(401);
    expect(await codeOf(as.query(api.account.view, {}))).toBe("device_removed");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken })))).toBe("refresh_invalid");
    // Proving again starts a new account.
    advance(10);
    const again = await prove(t, p);
    expect((await again.as.query(api.account.view, {})).registrationNumber).toBe(REGNO);
  });
});

describe("the install", () => {
  it("changes its token, label and seal key, and says what it holds", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as } = await prove(t, p);
    const made = await as.action(api.keys.create, { flow: "f" });
    const ext = t.withIdentity({ subject: await sha256Hex(made.key), kind: "key" });
    // Registered without an FCM token: field is absent (not "") and reads as off everywhere.
    expect((await t.run((ctx) => ctx.db.query("devices").first()))?.fcmToken).toBeUndefined();
    expect((await as.query(api.devices.info, {})).pushToken).toBe(false);
    expect((await as.query(api.screen.view, {})).device.pushToken).toBe(false);
    expect((await ext.query(api.extension.key, {}))?.phone?.notifications).toBe(false);
    expect(await t.query(internal.push.tokenOf, { deviceId: p.deviceId })).toBeNull();

    expect(await codeOf(as.mutation(api.devices.update, { flow: "f" }))).toBe("bad_request");
    expect(await codeOf(as.mutation(api.devices.update, { flow: "f", sealKey: "x" }))).toBe("bad_request");
    await as.mutation(api.devices.update, { flow: "f", fcmToken: "fcm", label: " Pixel 9 " });
    expect(await as.query(api.devices.info, {})).toEqual({
      deviceId: p.deviceId,
      label: "Pixel 9",
      platform: "android",
      pushToken: true,
      createdAt: Math.floor(Date.now() / 1000),
    });
    expect((await ext.query(api.extension.key, {}))?.phone?.notifications).toBe(true);
    expect(await t.query(internal.push.tokenOf, { deviceId: p.deviceId })).toBe("fcm");

    // Clearing a dead token unsets the optional field instead of writing "".
    await t.mutation(internal.push.clearToken, { deviceId: p.deviceId, token: "other" });
    expect(await t.query(internal.push.tokenOf, { deviceId: p.deviceId })).toBe("fcm");
    await t.mutation(internal.push.clearToken, { deviceId: p.deviceId, token: "fcm" });
    expect((await t.run((ctx) => ctx.db.query("devices").first()))?.fcmToken).toBeUndefined();
    expect((await as.query(api.devices.info, {})).pushToken).toBe(false);
    expect((await ext.query(api.extension.key, {}))?.phone?.notifications).toBe(false);
    expect(await t.query(internal.push.tokenOf, { deviceId: p.deviceId })).toBeNull();

    // Legacy row with fcmToken: "" still reads as absent at the boundary.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("devices").first())!;
      await ctx.db.patch(row._id, { fcmToken: "" });
    });
    expect((await as.query(api.devices.info, {})).pushToken).toBe(false);
    expect((await as.query(api.screen.view, {})).device.pushToken).toBe(false);
    expect((await ext.query(api.extension.key, {}))?.phone?.notifications).toBe(false);
    expect(await t.query(internal.push.tokenOf, { deviceId: p.deviceId })).toBeNull();
  });

  it("drops mailbox memberships and otpInbox rows when removed", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    const box = await as.mutation(api.mail.register, { flow: "f" });
    await t.run(async (ctx) => {
      await ctx.db.insert("otpInbox", {
        addressId: box.id,
        deviceId: p.deviceId,
        kind: "otp",
        sealed: "box",
        receivedAt: Math.floor(Date.now() / 1000),
        accountId: ctx.db.normalizeId("accounts", account)!,
      });
    });
    await as.mutation(api.devices.remove, { flow: "f" });
    expect(await t.run((ctx) => ctx.db.query("mailboxDevices").collect())).toEqual([]);
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });
});

describe("the Server screen", () => {
  it("reads in one query what the four did, and says when this phone left its address", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as } = await prove(t, p);
    const box = await as.mutation(api.mail.register, { flow: "f" });
    const screen = await as.query(api.screen.view, { mailbox: box.id });
    expect(screen.account).toEqual(await as.query(api.account.view, {}));
    expect(screen.device).toEqual(await as.query(api.devices.info, {}));
    expect(screen.status).toEqual(await as.query(api.app.status, {}));
    expect(screen.mailbox).toEqual(await as.query(api.mail.devices, { id: box.id }));
    expect((await as.query(api.screen.view, {})).mailbox).toBeNull();
    await as.mutation(api.mail.leave, { flow: "f", id: box.id });
    expect((await as.query(api.screen.view, { mailbox: box.id })).mailbox).toEqual({ gone: true });
  });
});

describe("VTOP on the engine", () => {
  it("passes the body on and answers JSON, packed pages and the engine's errors", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const calls = engine((path, body) => {
      if (path === "/engine/fetch") return Response.json({ data: { semesters: [] } });
      if (path === "/engine/fetch-batch" && (body as { dictionary?: string }).dictionary) {
        return new Response(new Uint8Array([1, 2, 3]), { headers: { "content-type": PACKED_PAGE } });
      }
      if (path === "/engine/fetch-batch") return Response.json({ code: "session_expired", error: "Sign in again." }, { status: 401 });
    });
    const session = { cookies: "a=b", csrf_token: "c", registration_number: REGNO };
    expect(await as.action(api.vtop.fetch, { flow: "f9", session, name: "semesters", args: {} })).toEqual({ data: { semesters: [] } });
    expect(calls[0].body).toEqual({ session, name: "semesters", args: {} });
    expect(calls[0].headers.get("x-vtop-regno")).toBe(REGNO);
    expect(calls[0].headers.get("x-vtop-flow")).toBe("f9");
    const packed = await as.action(api.vtop.fetchBatch, { flow: "f", session, semester: "S", pages: [{ name: "x" }], dictionary: "d" });
    expect(new Uint8Array(packed as ArrayBuffer)).toEqual(new Uint8Array([1, 2, 3]));
    expect(await codeOf(as.action(api.vtop.fetchBatch, { flow: "f", session, semester: "S", pages: [] }))).toBe("session_expired");
    expect(await codeOf(as.action(api.vtop.fetch, { flow: "f", session: { cookies: "x".repeat(70_000) }, name: "semesters" }))).toBe("too_large");
    await expect(as.action(api.vtop.fetch, { flow: "f", junk: 1 } as never)).rejects.toThrow();
    await expect(as.action(api.vtop.fetchBatch, { flow: "f", session, pages: "nope" } as never)).rejects.toThrow();
  });

  it("signs in on the server before any account, signed by the install", async () => {
    const t = await setup();
    const p = await phone(1);
    const calls = engine((path) => (path === "/engine/vtop-login" ? Response.json({ status: "otp_required" }) : undefined));
    const body = { username: "u", password: "p" };
    expect(await t.action(api.vtop.login, await p.signed("vtop:login", body))).toEqual({ status: "otp_required" });
    expect(calls[0].body).toEqual(body);
    expect(await codeOf(t.action(api.vtop.login, await p.signed("vtop:loginOtp", body)))).toBe("bad_signature");
    // Per username, however many device keys ask: 5 at once, then about one a minute.
    for (let i = 0; i < 4; i++) await t.action(api.vtop.login, await (await phone(10 + i)).signed("vtop:login", body));
    expect(await codeOf(t.action(api.vtop.login, await (await phone(20)).signed("vtop:login", { username: " U ", password: "p" })))).toBe("too_many_sign_ins");
    expect(await t.action(api.vtop.login, await p.signed("vtop:login", { username: "someone-else", password: "p" }))).toEqual({ status: "otp_required" });
  });

  it("app:login needs saved credentials", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    engine(() => undefined);
    expect(await codeOf(as.action(api.app.login, { flow: "f" }))).toBe("no_saved_credentials");
    expect(await as.mutation(api.app.sessionClear, { flow: "f" })).toEqual({ ok: true });
  });
});

describe("guides", () => {
  it("are bundled from guides/ as it is now (else run bun dev/gen-guides.ts)", async () => {
    const files = await guideFiles();
    expect(Object.keys(GUIDE_FILES).sort()).toEqual(files.map((f) => f.path).sort());
    for (const f of files) expect(GUIDE_FILES[f.path]).toEqual({ type: f.type, etag: f.etag, body: f.base64 });
  });

  it("serve JSON and images over HTTP, with an ETag", async () => {
    const t = await setup();
    const json = await t.fetch("/guides/forwarding.json");
    expect(json.headers.get("content-type")).toBe("application/json");
    expect(json.headers.get("cache-control")).toBe("public, max-age=3600");
    expect((await json.json()).schema).toBe(1);
    const image = await t.fetch("/guides/forwarding/1-add.webp");
    expect(image.headers.get("content-type")).toBe("image/webp");
    const bytes = new Uint8Array(await image.arrayBuffer());
    expect(new TextDecoder().decode(bytes.slice(8, 12))).toBe("WEBP");
    const again = await t.fetch("/guides/forwarding/1-add.webp", { headers: { "if-none-match": image.headers.get("etag")! } });
    expect(again.status).toBe(304);
    expect((await t.fetch("/guides/_headers")).status).toBe(404);
    expect((await t.fetch("/guides/nope.json")).status).toBe(404);
  });

  it("serve the edge's JSON", async () => {
    const t = await setup();
    expect((await t.query(api.guides.updates, {})).schema).toBe(1);
    expect((await t.query(api.guides.forwarding, {})).schema).toBe(1);
    expect(Object.keys((await t.query(api.guides.demo, {})).pages).length).toBeGreaterThan(0);
  });
});

describe("sweeps", () => {
  it("drop expired tickets and stale sessions, and keep refresh tokens", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    await t.mutation(api.auth.proveStart, await (await phone(2)).signed("auth:proveStart", {}));
    const at = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      const id = ctx.db.normalizeId("accounts", account)!;
      const row = (await ctx.db.get(id))!;
      await ctx.db.patch(id, {
        settings: { ...row.settings, cacheSession: true },
        sealedSession: "s",
        sessionSavedAt: at,
        sessionSource: "phone",
      });
    });
    advance(121);
    await t.mutation(internal.sweeps.fiveMinutes, {});
    // The traded-in ticket waits out its time for a lost answer; the open one went.
    const tickets = () => t.run((ctx) => ctx.db.query("proofTickets").collect());
    expect((await tickets()).map((r) => r.status)).toEqual(["proved"]);
    // Written without `sweepAt`, as rows from before it were: the one-off backfill sets it.
    expect(await t.mutation(internal.migrate.setSweepAt, {})).toBe(1);
    const row = () => t.run(async (ctx) => (await ctx.db.get(ctx.db.normalizeId("accounts", account)!))!);
    expect((await row()).sweepAt).toBe(at + 7 * 86_400);
    advance(7 * 86_400);
    await t.mutation(internal.sweeps.quarter, {});
    expect((await as.query(api.app.status, {})).sessionSavedAt).toBeNull();
    expect((await row()).sealedSession).toBeNull();
    expect((await row()).sweepAt).toBeUndefined();
    advance(180 * 86_400);
    await t.mutation(internal.sweeps.fiveMinutes, {});
    await t.mutation(internal.sweeps.quarter, {});
    expect(await tickets()).toHaveLength(0);
    expect(await t.run((ctx) => ctx.db.query("refreshTokens").collect())).toHaveLength(1);
  });

  it("drops unconsented legacy sealedSession where effective cacheSession is false while keeping vault holders", async () => {
    const t = await setup();
    const noVault = await prove(t, await phone(1), "22BCE0001");
    const withVault = await prove(t, await phone(2), "22BCE0002");
    const at = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      // Legacy row without cacheSession and without vault, with a future sweepAt.
      await ctx.db.patch(ctx.db.normalizeId("accounts", noVault.account)!, {
        settings: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null },
        sealedVault: null,
        sealedSession: "unconsented",
        sessionSavedAt: at,
        sessionSource: "phone",
        sweepAt: at + SESSION_CACHE_SECS,
      });
      // Legacy row without cacheSession but WITH a vault -> effective cacheSession is true.
      await ctx.db.patch(ctx.db.normalizeId("accounts", withVault.account)!, {
        settings: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null },
        sealedVault: "vault",
        vaultSavedAt: at,
        sealedSession: "kept-session",
        sessionSavedAt: at,
        sessionSource: "server",
        sweepAt: at + SESSION_CACHE_SECS,
      });
    });
    // Even before sweep, noVault reads as off.
    expect((await noVault.as.query(api.account.view, {})).settings.cacheSession).toBe(false);
    expect((await noVault.as.query(api.account.view, {})).sessionSavedAt).toBeNull();
    expect((await withVault.as.query(api.account.view, {})).settings.cacheSession).toBe(true);
    expect((await withVault.as.query(api.account.view, {})).sessionSavedAt).toBe(at);

    // Quarter sweep immediately drops noVault's leftover sealedSession and leaves withVault intact.
    await t.mutation(internal.sweeps.quarter, {});
    const rows = await t.run(async (ctx) => ({
      noVault: (await ctx.db.get(ctx.db.normalizeId("accounts", noVault.account)!))!,
      withVault: (await ctx.db.get(ctx.db.normalizeId("accounts", withVault.account)!))!,
    }));
    expect(rows.noVault.sealedSession).toBeNull();
    expect(rows.noVault.sessionSavedAt).toBeNull();
    expect(rows.noVault.sweepAt).toBeUndefined();
    expect(rows.withVault.sealedSession).toBe("kept-session");
    expect(rows.withVault.sealedVault).toBe("vault");
  });

  it("expires saved vault and cached session server-side at sweepAt so live queries re-run", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    const made = await as.action(api.keys.create, { flow: "f" });
    const ext = t.withIdentity({ subject: await sha256Hex(made.key), kind: "key" });
    engine((path) => (path === "/engine/seal" ? Response.json({ sealedVault: "sv" }) : undefined));
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    await as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: 86_400 } });
    await as.action(api.account.saveCredentials, {
      flow: "f",
      credentials: { username: REGNO, password: "pw", gmail: { refreshToken: "rt", clientId: "cid" }, otp: "gmail" },
    });
    // Also store a cached session (expires in 7 days, after the 24h vault).
    await t.mutation(internal.sessions.answered, {
      requestId: await t.run(async (ctx) => {
        const id = ctx.db.normalizeId("accounts", account)!;
        await ctx.db.insert("sessionRequests", {
          requestId: "r-exp",
          accountId: id,
          status: "pending",
          cookies: null,
          csrfToken: null,
          error: null,
          createdAt: Math.floor(Date.now() / 1000),
          expiresAt: Math.floor(Date.now() / 1000) + 120,
          vaultDueAt: null,
          vaultStarted: false,
          phoneAsked: true,
          phoneError: null,
          source: null,
          phones: [],
        });
        return "r-exp";
      }),
      cookies: "JSESSIONID=a",
      sealedSession: "ss",
    });
    expect((await as.query(api.account.view, {})).savedCredentials).toBe(true);
    expect((await ext.query(api.extension.key, {}))?.cachedSession).toBe(true);
    // At 24h: vault clears server-side without running sweeps.quarter; cached session stays.
    vi.advanceTimersByTime(86_400 * 1000);
    await t.finishInProgressScheduledFunctions();
    expect((await as.query(api.account.view, {})).savedCredentials).toBe(false);
    expect((await as.query(api.screen.view, {})).account.savedCredentials).toBe(false);
    expect((await ext.query(api.extension.key, {}))?.savedCredentials).toBe(false);
    expect((await ext.query(api.extension.key, {}))?.cachedSession).toBe(true);
    // At 7d: cached session clears server-side too.
    vi.advanceTimersByTime((SESSION_CACHE_SECS - 86_400) * 1000);
    await t.finishInProgressScheduledFunctions();
    expect((await ext.query(api.extension.key, {}))?.cachedSession).toBe(false);
    expect((await as.query(api.screen.view, {})).status?.sessionSavedAt).toBeNull();
  });

  it("sweeps expired otpInbox rows even behind 500 young gmail_confirm rows", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    const box = await as.mutation(api.mail.register, { flow: "f" });
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date"] });
    const t0 = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      const accountId = ctx.db.normalizeId("accounts", account)!;
      for (let i = 0; i < 500; i++) {
        await ctx.db.insert("otpInbox", {
          addressId: box.id,
          deviceId: `dev-${i}`,
          kind: "gmail_confirm",
          sealed: "c",
          receivedAt: t0,
          expiresAt: t0 + CONFIRMATION_TTL_SECS,
          accountId,
        });
      }
    });
    vi.advanceTimersByTime(10_000);
    await t.mutation(internal.mail.deliver, {
      addressId: box.id,
      kind: "otp",
      sentAt: t0 + 10,
      plaintext: "123456",
      boxes: [{ deviceId: p.deviceId, sealed: "o" }],
    });
    vi.advanceTimersByTime((SEALED_TTL_SECS + 1) * 1000);
    await t.mutation(internal.sweeps.quarter, {});
    const remaining = await t.run((ctx) => ctx.db.query("otpInbox").collect());
    expect(remaining.filter((r) => r.kind === "otp")).toHaveLength(0);
    expect(remaining.filter((r) => r.kind === "gmail_confirm")).toHaveLength(500);
    // After 24h the confirmations expire via `by_expiresAt` as well.
    vi.advanceTimersByTime((CONFIRMATION_TTL_SECS - SEALED_TTL_SECS) * 1000);
    await t.mutation(internal.sweeps.quarter, {});
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("sweeps legacy otpInbox rows without expiresAt via by_expiresAt even behind 500 live gmail_confirm rows", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as, account } = await prove(t, p);
    const box = await as.mutation(api.mail.register, { flow: "f" });
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date"] });
    const t0 = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      const accountId = ctx.db.normalizeId("accounts", account)!;
      for (let i = 0; i < 500; i++) {
        await ctx.db.insert("otpInbox", {
          addressId: box.id,
          deviceId: `dev-${i}`,
          kind: "gmail_confirm",
          sealed: "live-confirm",
          receivedAt: t0 - 10,
          expiresAt: t0 - 10 + CONFIRMATION_TTL_SECS,
          accountId,
        });
      }
      await ctx.db.insert("otpInbox", {
        addressId: box.id,
        deviceId: p.deviceId,
        kind: "otp",
        sealed: "old-otp",
        receivedAt: t0,
        accountId,
      });
      await ctx.db.insert("otpInbox", {
        addressId: box.id,
        deviceId: p.deviceId,
        kind: "gmail_confirm",
        sealed: "old-confirm",
        receivedAt: t0,
        accountId,
      });
    });
    vi.advanceTimersByTime((SEALED_TTL_SECS + 1) * 1000);
    await t.mutation(internal.sweeps.quarter, {});
    const afterOtp = await t.run((ctx) => ctx.db.query("otpInbox").collect());
    expect(afterOtp.filter((r) => r.kind === "otp")).toHaveLength(0);
    // Live legacy gmail_confirm was stamped with expiresAt so it left the undefined bucket.
    const stamped = afterOtp.find((r) => r.deviceId === p.deviceId && r.kind === "gmail_confirm");
    expect(stamped?.expiresAt).toBe(t0 + CONFIRMATION_TTL_SECS);
    vi.advanceTimersByTime(CONFIRMATION_TTL_SECS * 1000);
    // 501 confirmations expired (> BATCH=500), so two sweep runs clear them all.
    await t.mutation(internal.sweeps.quarter, {});
    await t.mutation(internal.sweeps.quarter, {});
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("sweeps inactive demo devices and refresh tokens without touching active demo or real accounts", async () => {
    const t = await setup();
    const real = await phone(1);
    const staleDemo = await phone(2);
    const activeDemo = await phone(3);
    const legacyDemo = await phone(4);
    await prove(t, real);
    await t.action(api.auth.token, await staleDemo.signed("auth:token", { demo: true, device: staleDemo.device }));
    const activeTokens = await t.action(api.auth.token, await activeDemo.signed("auth:token", { demo: true, device: activeDemo.device }));
    // Simulate a legacy demo install created before demoExpiresAt existed.
    await t.action(api.auth.token, await legacyDemo.signed("auth:token", { demo: true, device: legacyDemo.device }));
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("devices").collect()).find((d) => d.deviceId === legacyDemo.deviceId);
      await ctx.db.patch(row!._id, { demoExpiresAt: undefined });
    });

    // Active demo refreshes halfway through the window, extending its expiry.
    advance(Math.floor(DEMO_INACTIVE_SECS / 2));
    await t.action(api.auth.token, await activeDemo.signed("auth:token", { refreshToken: activeTokens.refreshToken }));

    advance(DEMO_INACTIVE_SECS - Math.floor(DEMO_INACTIVE_SECS / 2) + 1);
    await t.mutation(internal.sweeps.quarter, {});

    const devices = await t.run((ctx) => ctx.db.query("devices").collect());
    const tokens = await t.run((ctx) => ctx.db.query("refreshTokens").collect());
    expect(devices.map((d) => d.deviceId).sort()).toEqual([real.deviceId, activeDemo.deviceId].sort());
    expect(tokens.some((r) => r.deviceId === real.deviceId)).toBe(true);
    expect(tokens.some((r) => r.deviceId === activeDemo.deviceId)).toBe(true);
    expect(tokens.some((r) => r.deviceId === staleDemo.deviceId || r.deviceId === legacyDemo.deviceId)).toBe(false);
  });
});

describe("FCM", () => {
  it("sends data as strings, passes timeouts, clears cached token on 401 and retries within request TTL", async () => {
    const { RETRY_DELAYS_MS, message, sendFcm } = await import("./push");
    expect(message("tok", { seq: 3, kind: "mail_otp", nested: { a: 1 } })).toEqual({
      message: { token: "tok", data: { seq: "3", kind: "mail_otp", nested: '{"a":1}' }, android: { priority: "HIGH", ttl: "120s" } },
    });
    expect(RETRY_DELAYS_MS.reduce((a, b) => a + b, 0)).toBeLessThan(REQUEST_TIMEOUT_SECS * 1000);
    vi.stubEnv("FIREBASE_CREDENTIALS_JSON", "");
    expect(await sendFcm("tok", {})).toBe("off");

    // Mint a real RS256 private key to test token caching, timeout signal and 401 cache clear.
    const pair = (await crypto.subtle.generateKey(
      { name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256" },
      true,
      ["sign"],
    )) as CryptoKeyPair;
    const pkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));
    const pem = `-----BEGIN PRIVATE KEY-----\n${btoa(String.fromCharCode(...pkcs8))}\n-----END PRIVATE KEY-----`;
    vi.stubEnv(
      "FIREBASE_CREDENTIALS_JSON",
      JSON.stringify({ client_email: "fcm@test.iam", private_key: pem, project_id: "proj", token_uri: "https://oauth.test/token" }),
    );
    vi.stubEnv("FCM_URL", "https://fcm.test");
    let tokenCalls = 0;
    const fcmStatuses = [200, 401, 200, 200];
    const signals: Array<AbortSignal | undefined> = [];
    const fcmUrls: string[] = [];
    vi.stubGlobal("fetch", async (input: string | URL | Request, init?: RequestInit) => {
      signals.push(init?.signal ?? undefined);
      const url = typeof input === "string" || input instanceof URL ? String(input) : input.url;
      if (url === "https://oauth.test/token") {
        tokenCalls += 1;
        return Response.json({ access_token: `tok-${tokenCalls}`, expires_in: 3600 });
      }
      fcmUrls.push(url);
      return new Response("{}", { status: fcmStatuses.shift() ?? 200 });
    });
    expect(await sendFcm("dev-tok", { kind: "mail_otp" })).toBe("sent");
    expect(tokenCalls).toBe(1);
    expect(await sendFcm("dev-tok", { kind: "mail_otp" })).toBe("failed");
    expect(tokenCalls).toBe(1);
    // After 401 cleared the cached token, the next call mints a fresh one.
    expect(await sendFcm("dev-tok", { kind: "mail_otp" })).toBe("sent");
    expect(tokenCalls).toBe(2);
    // Empty FCM_URL is treated as unset and uses Google's FCM API origin.
    vi.stubEnv("FCM_URL", "");
    expect(await sendFcm("dev-tok", { kind: "mail_otp" })).toBe("sent");
    expect(fcmUrls.at(-1)).toBe("https://fcm.googleapis.com/v1/projects/proj/messages:send");
    expect(signals.every((s) => s instanceof AbortSignal)).toBe(true);
  });

  it("fails fast when INTERNAL_TOKEN is unset or blank", async () => {
    const { enginePost } = await import("./lib/engine");
    vi.stubEnv("ENGINE_URL", "https://engine.test");
    vi.stubEnv("INTERNAL_TOKEN", "");
    expect(() => enginePost("/engine/fetch", {})).toThrow("INTERNAL_TOKEN is not configured");
    vi.stubEnv("INTERNAL_TOKEN", "   ");
    expect(() => enginePost("/engine/fetch", {})).toThrow("INTERNAL_TOKEN is not configured");
  });
});
