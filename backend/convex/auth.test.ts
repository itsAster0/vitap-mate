// Identity: the TLSNotary ticket, tokens of every kind, refresh rotation with
// reuse detection, and revoking that ends access at once.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, TOKENS, T0, advance, codeOf, phone, prove, setup } from "../test/kit";
import { ensureAccount } from "./lib/accounts";
import { api, internal } from "./_generated/api";
import { issueV2Token } from "./lib/jwt";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("signed calls", () => {
  it("refuse a bad signature, an off clock and a body that is not JSON", async () => {
    const t = await setup();
    const p = await phone(1);
    const good = await p.signed("auth:proveStart", {});
    expect(await codeOf(t.mutation(api.auth.proveStart, { ...good, payload: "{ }" }))).toBe("bad_signature");
    expect(await codeOf(t.mutation(api.auth.proveStart, await p.signed("auth:token", {})))).toBe("bad_signature");
    expect(await codeOf(t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}, T0 - 121)))).toBe("stale_time");
    expect(await codeOf(t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", "x" as unknown as object)))).toBe("bad_request");
  });
});

describe("the proof ticket", () => {
  it("is 128 bits, one at a time, and rate limited per install", async () => {
    const t = await setup();
    const p = await phone(1);
    const first = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    expect(first.ticket).toMatch(/^[0-9a-f]{32}$/);
    expect(first.expiresAt).toBe((T0 + 120) * 1000);
    expect(await codeOf(t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {})))).toBe("too_many_proofs");
    advance(5);
    const second = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    // The new ticket replaced the first.
    expect(await t.mutation(internal.auth.takeTicket, { ticket: first.ticket })).toBeNull();
    expect(await t.mutation(internal.auth.takeTicket, { ticket: second.ticket })).toEqual({ device: p.deviceId });
    // Used up once taken.
    expect(await t.mutation(internal.auth.takeTicket, { ticket: second.ticket })).toBeNull();
    // 20 an hour, however spaced.
    for (let i = 2; i < 20; i++) {
      advance(5);
      await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    }
    advance(5);
    expect(await codeOf(t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {})))).toBe("too_many_proofs");
    advance(3600);
    expect((await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}))).ticket).toMatch(/^[0-9a-f]{32}$/);
    // Another install has its own allowance.
    expect((await t.mutation(api.auth.proveStart, await (await phone(2)).signed("auth:proveStart", {}))).ticket).toBeTruthy();
  });

  it("expires, and is good only for its own install once proved", async () => {
    const t = await setup();
    const p = await phone(1);
    const other = await phone(2);
    const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device })))).toBe("proof_pending");
    await t.mutation(internal.auth.takeTicket, { ticket });
    expect(await t.mutation(internal.auth.proofDone, { ticket, regno: "22bce0001", ownerHash: "o" })).toBe(true);
    expect(await codeOf(t.action(api.auth.token, await other.signed("auth:token", { ticket, device: other.device })))).toBe("challenge_invalid");
    const late = await phone(3);
    const lateTicket = (await t.mutation(api.auth.proveStart, await late.signed("auth:proveStart", {}))).ticket;
    advance(121);
    expect(await t.mutation(internal.auth.takeTicket, { ticket: lateTicket })).toBeNull();
  });

  it("hands on the engine's failure and consumes the failed ticket", async () => {
    const t = await setup();
    const p = await phone(1);
    const { ticket } = await t.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}));
    expect(await t.mutation(internal.auth.proofDone, { ticket, code: "proof_failed" })).toBe(false);
    await t.mutation(internal.auth.takeTicket, { ticket });
    await t.mutation(internal.auth.proofDone, { ticket, code: "proof_failed" });
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device })))).toBe("proof_failed");
    expect(await t.run((ctx) => ctx.db.query("proofTickets").collect())).toEqual([]);
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device })))).toBe("challenge_invalid");
  });
});

describe("tokens", () => {
  it("a proved ticket makes the account, the device row and a linked phone", async () => {
    const t = await setup();
    const p = await phone(1);
    const tokens = await prove(t, p, "22bce0001");
    expect(tokens.regno).toBe(REGNO);
    expect(tokens.jwt.split(".")).toHaveLength(3);
    expect(tokens.jwtExpiresAt).toBe((T0 + 24 * 3600) * 1000);
    expect(tokens.refreshToken).toMatch(/^vrt_[A-Za-z0-9_-]{43}$/);
    const payload = JSON.parse(atob(tokens.jwt.split(".")[1].replace(/-/g, "+").replace(/_/g, "/"))) as Record<string, unknown>;
    expect(payload).toMatchObject({ sub: p.deviceId, regno: REGNO, acct: tokens.account, aud: "vmate", iss: "https://vmate.test" });
    const view = await tokens.as.query(api.account.view, { flow: "f" });
    expect(view).toMatchObject({ registrationNumber: REGNO, thisPhone: true, phoneCount: 1 });
    const rows = await t.run(async (ctx) => ({
      devices: await ctx.db.query("devices").collect(),
      refresh: await ctx.db.query("refreshTokens").collect(),
    }));
    expect(rows.devices[0]).toMatchObject({ deviceId: p.deviceId, accountId: tokens.account, sealKey: p.sealKey, linkedAt: T0 });
    expect(rows.refresh[0]).toMatchObject({ accountId: tokens.account, usedAt: null });
    expect(rows.refresh[0].hash).not.toContain("vrt_");
  });

  it("asks for exactly one grant and a good device", async () => {
    const t = await setup();
    const p = await phone(1);
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", {})))).toBe("bad_request");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { demo: true, refreshToken: "vrt_x", device: p.device })))).toBe("bad_request");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { demo: true, device: { ...p.device, sealKey: "x" } })))).toBe("bad_request");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { demo: true, device: { ...p.device, platform: "Not ok!" } })))).toBe("bad_request");
  });

  it("rotate refresh tokens, and a reused one revokes its whole chain", async () => {
    const t = await setup();
    const p = await phone(1);
    const first = await prove(t, p);
    const second = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: first.refreshToken }));
    expect(second.refreshToken).not.toBe(first.refreshToken);
    const third = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: second.refreshToken }));
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: first.refreshToken })))).toBe("refresh_reused");
    // The newest token went with the chain, and the live 24-hour JWT stops working at once.
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: third.refreshToken })))).toBe("refresh_invalid");
    expect(await codeOf(first.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
    expect(await t.run((ctx) => ctx.db.query("refreshTokens").collect())).toHaveLength(0);
  });

  it("keeps a chain to three rows however often it is refreshed", async () => {
    const t = await setup();
    const p = await phone(1);
    let token = (await prove(t, p)).refreshToken;
    const seen = [token];
    for (let i = 0; i < 6; i++) {
      token = (await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: token }))).refreshToken;
      seen.push(token);
    }
    expect(await t.run((ctx) => ctx.db.query("refreshTokens").collect())).toHaveLength(3);
    // The one before the last used still shows a stolen copy.
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: seen[4] })))).toBe("refresh_reused");
    expect(await t.run((ctx) => ctx.db.query("refreshTokens").collect())).toHaveLength(0);
  });

  it("a new proof ends the install's older refresh tokens", async () => {
    const t = await setup();
    const p = await phone(1);
    const lost = await prove(t, p);
    advance(3600); // past the proof rate limit
    const fresh = await prove(t, p);
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: lost.refreshToken })))).toBe("refresh_invalid");
    expect((await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: fresh.refreshToken }))).regno).toBe(fresh.regno);
  });

  it("a refresh token works only from its own install and has no time limit", async () => {
    const t = await setup();
    const p = await phone(1);
    const thief = await phone(2);
    const tokens = await prove(t, p);
    expect(await codeOf(t.action(api.auth.token, await thief.signed("auth:token", { refreshToken: tokens.refreshToken })))).toBe("refresh_invalid");
    // Still works after more than 90 days, including a retry after a lost rotation reply.
    advance(365 * 86_400);
    const rotated = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: tokens.refreshToken }));
    expect(rotated.regno).toBe(REGNO);
    advance(120 * 86_400);
    const retried = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: tokens.refreshToken }));
    expect(retried.regno).toBe(REGNO);
    expect(retried.refreshToken).not.toBe(rotated.refreshToken);
  });

  it("keeps both phones signed in with valid refresh and forwarding when a second phone proves the same account", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    advance(10);
    const tb = await prove(t, b);
    expect(await tb.as.mutation(api.mail.register, { flow: "f" })).toEqual(box);
    // Both phones keep working JWTs, independent refresh chains, and mailbox membership.
    expect((await ta.as.query(api.account.view, { flow: "f" })).phoneCount).toBe(2);
    expect((await tb.as.query(api.account.view, { flow: "f" })).phoneCount).toBe(2);
    expect((await ta.as.query(api.screen.view, { mailbox: box.id })).mailbox).toMatchObject({ devices: expect.any(Array) });
    expect((await tb.as.query(api.screen.view, { mailbox: box.id })).mailbox).toMatchObject({ devices: expect.any(Array) });
    const ta2 = await t.action(api.auth.token, await a.signed("auth:token", { refreshToken: ta.refreshToken }));
    const tb2 = await t.action(api.auth.token, await b.signed("auth:token", { refreshToken: tb.refreshToken }));
    expect(ta2.regno).toBe(REGNO);
    expect(tb2.regno).toBe(REGNO);
    expect((await ta.as.query(api.mail.devices, { flow: "f", id: box.id })).devices).toHaveLength(2);
    expect((await tb.as.query(api.mail.devices, { flow: "f", id: box.id })).devices).toHaveLength(2);
  });

  it("caps the proofs the engine runs, not the tickets fresh device keys ask for", async () => {
    const t = await setup();
    const tickets = [];
    for (let i = 0; i < 21; i++) tickets.push((await t.mutation(api.auth.proveStart, await (await phone(100 + i)).signed("auth:proveStart", {}))).ticket);
    for (const ticket of tickets.slice(0, 20)) expect(await t.mutation(internal.auth.takeTicket, { ticket })).toHaveProperty("device");
    // The 21st waits, its ticket still open for another try.
    expect(await t.mutation(internal.auth.takeTicket, { ticket: tickets[20] })).toHaveProperty("busy");
    advance(60);
    expect(await t.mutation(internal.auth.takeTicket, { ticket: tickets[20] })).toHaveProperty("device");
  });

  it("lets an install ask again with the token it holds when the last answer was lost", async () => {
    const t = await setup();
    const p = await phone(1);
    const first = await prove(t, p);
    // This answer never reaches the phone, which then stays offline for two days.
    await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: first.refreshToken }));
    advance(2 * 86_400);
    const again = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: first.refreshToken }));
    expect(again.regno).toBe(REGNO);
    const next = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: again.refreshToken }));
    // Once the chain moved on, the old token is a reuse again.
    advance(20);
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: first.refreshToken })))).toBe("refresh_reused");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: next.refreshToken })))).toBe("refresh_invalid");
  });

  it("signing out everywhere ends the other phones' JWTs at once", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    await ta.as.mutation(api.auth.signOut, { flow: "f", everywhere: true });
    expect(await codeOf(tb.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
  });

  it("signing out deletes this install's chain, or every phone's", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    await ta.as.mutation(api.auth.signOut, { flow: "f" });
    expect(await codeOf(t.action(api.auth.token, await a.signed("auth:token", { refreshToken: ta.refreshToken })))).toBe("refresh_invalid");
    const tb2 = await t.action(api.auth.token, await b.signed("auth:token", { refreshToken: tb.refreshToken }));
    await tb.as.mutation(api.auth.signOut, { flow: "f", everywhere: true });
    expect(await codeOf(t.action(api.auth.token, await b.signed("auth:token", { refreshToken: tb2.refreshToken })))).toBe("refresh_invalid");
  });

  it("removing a phone ends its access at once, JWT and refresh token both", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    const view = await ta.as.query(api.account.view, { flow: "f" });
    const other = view.phones.find((ph) => !ph.thisPhone)!;
    await ta.as.mutation(api.account.removeDevice, { flow: "f", id: other.id });
    expect(await codeOf(tb.as.query(api.account.view, { flow: "f" }))).toBe("device_removed");
    expect(await codeOf(t.action(api.auth.token, await b.signed("auth:token", { refreshToken: tb.refreshToken })))).toBe("refresh_invalid");
  });

  it("a deleted install can't refresh", async () => {
    const t = await setup();
    const p = await phone(1);
    const tokens = await prove(t, p);
    await tokens.as.mutation(api.devices.remove, { flow: "f" });
    expect(await codeOf(tokens.as.query(api.devices.info, { flow: "f" }))).toBe("device_removed");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: tokens.refreshToken })))).toBe("refresh_invalid");
  });

  it("a trade-in hands over the old address waiting under the account's owner hash", async () => {
    const t = await setup();
    const p = await phone(1);
    const token = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: p.deviceId, iat: T0 - 10, exp: T0 + 1000 });
    await t.run(async (ctx) => {
      const account = await ensureAccount(ctx, REGNO, null);
      await ctx.db.patch(account._id, { ownerHash: "owner-v2" });
      await ctx.db.insert("v2Tokens", { deviceId: p.deviceId, iat: T0 - 10, accountId: account._id, usedAt: null });
      await ctx.db.insert("unclaimedMailboxes", { addressId: "kkkkkkkkkk", ownerHash: "owner-v2", confirmation: null, confirmationAt: null, lastOtpAt: 5, createdAt: 1, updatedAt: 1 });
      // An address another phone made on v3 meanwhile: Gmail still forwards to the old one.
      await ctx.db.insert("mailboxes", { addressId: "nnnnnnnnnn", accountId: account._id, confirmation: null, confirmationAt: null, lastOtpAt: null, createdAt: 2, updatedAt: 2 });
    });
    const tokens = await t.action(api.auth.token, await p.signed("auth:token", { v2AccountToken: token, device: p.device }));
    const as = t.withIdentity({ subject: p.deviceId, regno: tokens.regno, acct: tokens.account });
    expect((await as.mutation(api.mail.register, { flow: "f" })).id).toBe("kkkkkkkkkk");
    const left = await t.run((ctx) => ctx.db.query("mailboxes").collect());
    expect(left.map((m) => m.addressId)).toEqual(["kkkkkkkkkk"]);
  });

  it("trade a live v2 account token in once", async () => {
    const t = await setup();
    const p = await phone(1);
    const token = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: "22bce0001", dev: p.deviceId, iat: T0 - 10, exp: T0 + 1000 });
    // Good only while the edge held it live (imported by the backfill).
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { v2AccountToken: token, device: p.device })))).toBe("token_invalid");
    await t.run(async (ctx) => {
      const account = await ensureAccount(ctx, REGNO, null);
      await ctx.db.insert("v2Tokens", { deviceId: p.deviceId, iat: T0 - 10, accountId: account._id, usedAt: null });
    });
    const tokens = await t.action(api.auth.token, await p.signed("auth:token", { v2AccountToken: token, device: p.device }));
    expect(tokens.regno).toBe(REGNO);
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { v2AccountToken: token, device: p.device })))).toBe("token_invalid");
    const other = await phone(2);
    expect(await codeOf(t.action(api.auth.token, await other.signed("auth:token", { v2AccountToken: token, device: other.device })))).toBe("token_invalid");
    const old = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: other.deviceId, iat: T0 - 2000, exp: T0 - 1 });
    expect(await codeOf(t.action(api.auth.token, await other.signed("auth:token", { v2AccountToken: old, device: other.device })))).toBe("token_expired");
    const forged = await issueV2Token("another secret", { sub: REGNO, dev: other.deviceId, iat: T0, exp: T0 + 10 });
    expect(await codeOf(t.action(api.auth.token, await other.signed("auth:token", { v2AccountToken: forged, device: other.device })))).toBe("token_invalid");
  });

  it("the demo gets DEMO2026, reads, and changes nothing", async () => {
    const t = await setup();
    const p = await phone(1);
    const tokens = await t.action(api.auth.token, await p.signed("auth:token", { demo: true, device: p.device }));
    expect(tokens).toMatchObject({ regno: "DEMO2026", account: "demo" });
    const as = t.withIdentity({ subject: p.deviceId, regno: "DEMO2026", acct: "demo", demo: true });
    expect((await as.query(api.account.view, { flow: "f" })).registrationNumber).toBe("DEMO2026");
    expect((await as.query(api.app.status, { flow: "f" })).signIns).toEqual([]);
    expect(await codeOf(as.mutation(api.mail.register, { flow: "f" }))).toBe("demo_unavailable");
    expect(await codeOf(as.mutation(api.account.setSemester, { flow: "f", semesterId: "AP1" }))).toBe("demo_unavailable");
    expect(await codeOf(as.action(api.keys.create, { flow: "f" }))).toBe("demo_unavailable");
    expect(await codeOf(as.action(api.vtop.fetch, { flow: "f", session: { cookies: "a=b" }, name: "semesters" }))).toBe("demo_unavailable");
    const refreshed = await t.action(api.auth.token, await p.signed("auth:token", { refreshToken: tokens.refreshToken }));
    expect(refreshed.regno).toBe("DEMO2026");
    // Signing out ends the demo's 24-hour JWT at once too.
    await as.mutation(api.auth.signOut, { flow: "f" });
    expect(await codeOf(as.query(api.devices.info, { flow: "f" }))).toBe("device_removed");
    expect(await codeOf(t.action(api.auth.token, await p.signed("auth:token", { refreshToken: refreshed.refreshToken })))).toBe("refresh_invalid");
  });

  it("no JWT, no access", async () => {
    const t = await setup();
    expect(await codeOf(t.query(api.devices.info, { flow: "f" }))).toBe("unauthenticated");
  });

  it("rate limits demo grants per install and globally, and refresh rotations per install", async () => {
    const t = await setup();
    const p1 = await phone(1);
    for (let i = 0; i < 5; i++) {
      await t.action(api.auth.token, await p1.signed("auth:token", { demo: true, device: p1.device }));
    }
    expect(await codeOf(t.action(api.auth.token, await p1.signed("auth:token", { demo: true, device: p1.device })))).toBe("rate_limited");
    // Another install still gets demo tokens until the global bucket fills (40 total, 5 already used).
    for (let i = 0; i < 35; i++) {
      const pi = await phone(50 + i);
      await t.action(api.auth.token, await pi.signed("auth:token", { demo: true, device: pi.device }));
    }
    const extra = await phone(99);
    expect(await codeOf(t.action(api.auth.token, await extra.signed("auth:token", { demo: true, device: extra.device })))).toBe("rate_limited");

    // Per-device refresh rate limit (capacity 15).
    const r = await phone(2);
    let rt = (await prove(t, r)).refreshToken;
    for (let i = 0; i < 15; i++) {
      rt = (await t.action(api.auth.token, await r.signed("auth:token", { refreshToken: rt }))).refreshToken;
    }
    expect(await codeOf(t.action(api.auth.token, await r.signed("auth:token", { refreshToken: rt })))).toBe("rate_limited");
    // Another install can still refresh.
    const other = await phone(3);
    const otherTokens = await prove(t, other);
    expect((await t.action(api.auth.token, await other.signed("auth:token", { refreshToken: otherTokens.refreshToken }))).regno).toBe(REGNO);
  });

  it("rate limits ticket and v2 grants per install", async () => {
    const t = await setup();
    const p1 = await phone(1);
    const p2 = await phone(2);

    // Ticket grant: per-device limit (capacity 15).
    for (let i = 0; i < 15; i++) {
      expect(await codeOf(t.action(api.auth.token, await p1.signed("auth:token", { ticket: "0".repeat(32), device: p1.device })))).toBe("challenge_invalid");
    }
    expect(await codeOf(t.action(api.auth.token, await p1.signed("auth:token", { ticket: "0".repeat(32), device: p1.device })))).toBe("rate_limited");
    // Another install is unaffected.
    expect(await codeOf(t.action(api.auth.token, await p2.signed("auth:token", { ticket: "0".repeat(32), device: p2.device })))).toBe("challenge_invalid");

    // v2 grant: per-device limit (capacity 15).
    const v2p1 = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: p1.deviceId, iat: T0 - 10, exp: T0 + 1000 });
    const v2p2 = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: p2.deviceId, iat: T0 - 10, exp: T0 + 1000 });
    for (let i = 0; i < 15; i++) {
      expect(await codeOf(t.action(api.auth.token, await p1.signed("auth:token", { v2AccountToken: v2p1, device: p1.device })))).toBe("token_invalid");
    }
    expect(await codeOf(t.action(api.auth.token, await p1.signed("auth:token", { v2AccountToken: v2p1, device: p1.device })))).toBe("rate_limited");
    // Another install is unaffected.
    expect(await codeOf(t.action(api.auth.token, await p2.signed("auth:token", { v2AccountToken: v2p2, device: p2.device })))).toBe("token_invalid");
  });
});
