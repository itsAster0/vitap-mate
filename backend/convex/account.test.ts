// The account, the install and VTOP on the engine, as a linked phone uses them;
// plus the sweeps that replaced the Durable Objects' alarms.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, T0, advance, codeOf, engine, phone, prove, setup } from "../test/kit";
import { api, internal } from "./_generated/api";
import type { Id } from "./_generated/dataModel";
import { guideFiles } from "../dev/guideFiles";
import { recordSignIn } from "./lib/accounts";
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
    expect(await codeOf(as.mutation(api.account.setSettings, { flow: "f", settings: { alwaysUseVault: true, phoneWaitSecs: 11 } }))).toBe("bad_request");
    await as.mutation(api.account.setSemester, { flow: "f", semesterId: " AP2026275 " });
    expect(await codeOf(as.mutation(api.account.setSemester, { flow: "f", semesterId: "bad id" }))).toBe("bad_request");
    const view = await as.query(api.account.view, { flow: "f" });
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
      await ctx.db.patch(id, { sealedVault: "v", vaultSavedAt: at });
      await ctx.db.insert("signIns", { accountId: id, at, by: "server", ok: true, reason: null });
    });
    expect((await as.query(api.app.status, { flow: "f" })).signIns).toHaveLength(1);
    expect((await as.query(api.account.view, { flow: "f" })).savedCredentials).toBe(true);
    await as.mutation(api.account.forgetCredentials, { flow: "f" });
    expect((await as.query(api.account.view, { flow: "f" })).savedCredentials).toBe(false);
    await as.mutation(api.account.clearSignIns, { flow: "f" });
    expect((await as.query(api.app.status, { flow: "f" })).signIns).toEqual([]);
  });

  it("saves credentials only when the server can read the OTP", async () => {
    const t = await setup();
    const { as } = await prove(t, await phone(1));
    const calls = engine((path) => (path === "/engine/seal" ? Response.json({ sealedVault: "sv" }) : undefined));
    const password = { username: "22bce0001", password: "pw" };
    // No forwarding address and no Gmail access: nothing is sealed.
    expect(await as.action(api.account.saveCredentials, { flow: "f", credentials: password })).toEqual({ savedCredentials: false, forwarding: false });
    expect(calls).toHaveLength(0);
    const gmail = { refreshToken: "rt", clientId: "cid" };
    expect(await as.action(api.account.saveCredentials, { flow: "f", label: "Pixel", credentials: { ...password, gmail, otp: "gmail" } })).toEqual({
      savedCredentials: true,
      forwarding: false,
    });
    expect(calls[0].body).toEqual({ account: REGNO, username: "22bce0001", password: "pw", gmail: { ...gmail, deleteAfterReading: false }, otp: "gmail" });
    expect(calls[0].headers.get("x-vtop-flow")).toBe("f");
    const view = await as.query(api.account.view, { flow: "f" });
    expect(view).toMatchObject({ savedCredentials: true, vaultGmail: true, vaultSavedAt: T0 });
    expect(view.phones[0].label).toBe("Pixel");
    // With a forwarding address, a password alone is enough; a failed seal keeps the old vault.
    await as.mutation(api.mail.register, { flow: "f" });
    engine(() => new Response(null, { status: 500 }));
    expect(await as.action(api.account.saveCredentials, { flow: "f", credentials: password })).toEqual({ savedCredentials: true, forwarding: true });
    expect(await codeOf(as.action(api.account.saveCredentials, { flow: "f", credentials: null }))).toBe("bad_request");
  });

  it("refuses the demo's credentials", async () => {
    const t = await setup();
    const p = await phone(1);
    const demo = await t.action(api.auth.token, await p.signed("auth:token", { demo: true, device: p.device }));
    const as = t.withIdentity({ subject: p.deviceId, regno: demo.regno, acct: demo.account, demo: true });
    expect(await codeOf(as.action(api.account.saveCredentials, { flow: "f", credentials: { username: "u", password: "p" } }))).toBe("demo_unavailable");
  });

  it("keeps the latest ten sign-ins, newest first", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    await t.run(async (ctx) => {
      const id = account as Id<"accounts">;
      for (let n = 0; n < 12; n++) await recordSignIn(ctx, id, { at: 100 + n, by: n % 2 ? "phone" : "server", ok: n !== 5, reason: n === 5 ? "x" : null });
    });
    const { signIns } = await as.query(api.app.status, { flow: "f" });
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
    expect(await codeOf(proved[0].tokens.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
    const view = await proved[5].tokens.as.query(api.account.view, { flow: "f" });
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
    const other = (await tb.as.query(api.account.view, { flow: "f" })).phones.find((p) => !p.thisPhone)!;
    expect(await codeOf(tb.as.mutation(api.account.removeDevice, { flow: "f", id: "nope" }))).toBe("phone_not_found");
    await tb.as.mutation(api.account.removeDevice, { flow: "f", id: other.id });
    expect(await codeOf(ta.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
    expect((await tb.as.query(api.account.view, { flow: "f" })).phoneCount).toBe(1);
  });

  it("moves an old phone's keys to this one", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    await ta.as.action(api.keys.create, { flow: "f" });
    advance(10);
    const tb = await prove(t, b);
    const old = (await tb.as.query(api.account.view, { flow: "f" })).phones.find((p) => !p.thisPhone)!;
    expect(await codeOf(tb.as.mutation(api.keys.move, { flow: "f", from: "nope" }))).toBe("phone_not_found");
    expect(await tb.as.mutation(api.keys.move, { flow: "f", from: old.id })).toEqual({ moved: 1 });
    expect(await codeOf(ta.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
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
    expect(await codeOf(as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken })))).toBe("refresh_invalid");
    // Proving again starts a new account.
    advance(10);
    const again = await prove(t, p);
    expect((await again.as.query(api.account.view, { flow: "f" })).registrationNumber).toBe(REGNO);
  });
});

describe("the install", () => {
  it("changes its token, label and seal key, and says what it holds", async () => {
    const t = await setup();
    const p = await phone(1);
    const { as } = await prove(t, p);
    expect(await codeOf(as.mutation(api.devices.update, { flow: "f" }))).toBe("bad_request");
    expect(await codeOf(as.mutation(api.devices.update, { flow: "f", sealKey: "x" }))).toBe("bad_request");
    await as.mutation(api.devices.update, { flow: "f", fcmToken: "fcm", label: " Pixel 9 " });
    expect(await as.query(api.devices.info, { flow: "f" })).toEqual({
      deviceId: p.deviceId,
      label: "Pixel 9",
      platform: "android",
      pushToken: true,
      createdAt: Math.floor(Date.now() / 1000),
    });
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
    expect(await codeOf(as.action(api.vtop.fetch, { flow: "f", junk: "x".repeat(70_000) }))).toBe("too_large");
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
  it("drop expired tickets, stale sessions and unused refresh tokens", async () => {
    const t = await setup();
    const { as, account } = await prove(t, await phone(1));
    await t.mutation(api.auth.proveStart, await (await phone(2)).signed("auth:proveStart", {}));
    const at = Math.floor(Date.now() / 1000);
    await t.run(async (ctx) => {
      await ctx.db.patch(ctx.db.normalizeId("accounts", account)!, { sealedSession: "s", sessionSavedAt: at, sessionSource: "phone" });
    });
    advance(121);
    await t.mutation(internal.sweeps.minute, {});
    expect(await t.run((ctx) => ctx.db.query("proofTickets").collect())).toHaveLength(0);
    advance(7 * 86_400);
    await t.mutation(internal.sweeps.quarter, {});
    expect((await as.query(api.app.status, { flow: "f" })).sessionSavedAt).toBeNull();
    advance(90 * 86_400);
    await t.mutation(internal.sweeps.day, {});
    expect(await t.run((ctx) => ctx.db.query("refreshTokens").collect())).toHaveLength(0);
  });
});

describe("FCM", () => {
  it("sends data as strings, and nothing without a service account", async () => {
    const { message, sendFcm } = await import("./push");
    expect(message("tok", { seq: 3, kind: "mail_otp", nested: { a: 1 } })).toEqual({
      message: { token: "tok", data: { seq: "3", kind: "mail_otp", nested: '{"a":1}' }, android: { priority: "HIGH", ttl: "120s" } },
    });
    vi.stubEnv("FIREBASE_CREDENTIALS_JSON", "");
    expect(await sendFcm("tok", {})).toBe("off");
  });
});
