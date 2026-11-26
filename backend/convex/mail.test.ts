// Forwarding addresses and mail delivery from the mail Worker (`/internal/mail`).

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, TOKENS, advance, codeOf, phone, prove, setup, type T } from "../test/kit";
import { api, internal } from "./_generated/api";
import type { Id } from "./_generated/dataModel";
import { CONFIRMATION_TTL_SECS, claimMailbox } from "./lib/mailboxes";
import { open } from "./lib/mailSeal";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

const mail = (t: T, body: object, token = TOKENS.MAIL_TOKEN) =>
  t.fetch("/internal/mail", { method: "POST", headers: { authorization: `Bearer ${token}`, "content-type": "application/json" }, body: JSON.stringify(body) });

const text = async (secret: string, id: string, sealed: string | null) => new TextDecoder().decode((await open(secret, id, sealed!))!);

describe("addresses", () => {
  it("belong to the student: a second phone joins the same one", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    expect(box.address).toBe(`otp+${box.id}@vmate.test`);
    advance(10);
    const tb = await prove(t, b);
    expect(await tb.as.mutation(api.mail.register, { flow: "f" })).toEqual(box);
    const { devices } = await ta.as.query(api.mail.devices, { id: box.id });
    expect(devices.map((d) => [d.deviceId, d.thisDevice])).toEqual([
      [a.deviceId, true],
      [b.deviceId, false],
    ]);
    const rows = await t.run((ctx) => ctx.db.query("mailboxDevices").collect());
    expect(rows.every((r) => r.accountId === ta.account)).toBe(true);
  });

  it("keeps an active phone on the address when a second phone joins months later", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    // Phone a stays active and refreshes past MEMBER_STALE_SECS (60 days).
    advance(61 * 86_400);
    await t.action(api.auth.token, await a.signed("auth:token", { refreshToken: ta.refreshToken }));
    const tb = await prove(t, b);
    expect(await tb.as.mutation(api.mail.register, { flow: "f" })).toEqual(box);
    const { devices } = await ta.as.query(api.mail.devices, { id: box.id });
    expect(devices.map((d) => d.deviceId).sort()).toEqual([a.deviceId, b.deviceId].sort());
    expect((await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "555555" })).status).toBe(200);
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).sealed).toBeTruthy();
    expect((await tb.as.query(api.mail.inbox, { id: box.id })).sealed).toBeTruthy();
  });

  it("leave keeps the address; remove deletes it when last; others can be taken off", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    await tb.as.mutation(api.mail.register, { flow: "f" });
    expect(await codeOf(ta.as.mutation(api.mail.removeDevice, { flow: "f", id: box.id, removeDevice: a.deviceId }))).toBe("bad_request");
    expect(await codeOf(ta.as.mutation(api.mail.removeDevice, { flow: "f", id: box.id, removeDevice: "f".repeat(32) }))).toBe("unknown_device");
    await ta.as.mutation(api.mail.removeDevice, { flow: "f", id: box.id, removeDevice: b.deviceId });
    expect(await codeOf(tb.as.query(api.mail.inbox, { id: box.id }))).toBe("mailbox_not_found");
    await ta.as.mutation(api.mail.leave, { flow: "f", id: box.id });
    // Still the student's, with no install on it: the next phone joins it.
    expect((await tb.as.mutation(api.mail.register, { flow: "f" })).id).toBe(box.id);
    await tb.as.mutation(api.mail.remove, { flow: "f", id: box.id });
    expect(await t.run((ctx) => ctx.db.query("mailboxes").collect())).toHaveLength(0);
    expect(await codeOf(tb.as.mutation(api.mail.remove, { flow: "f", id: box.id }))).toBe("mailbox_not_found");
  });

  it("evicts the least recently seen member when a sixth device joins", async () => {
    const t = await setup();
    const phones = [];
    let boxId = "";
    for (let n = 1; n <= 5; n++) {
      const p = await phone(n);
      const proved = await prove(t, p);
      boxId = (await proved.as.mutation(api.mail.register, { flow: "f" })).id;
      phones.push({ p, proved });
      advance(10);
    }
    // Touch the oldest member so the second one becomes least recently seen.
    advance(3600);
    await t.action(api.auth.token, await phones[0].p.signed("auth:token", { refreshToken: phones[0].proved.refreshToken }));
    advance(10);
    const sixth = await phone(6);
    const proved6 = await prove(t, sixth);
    await proved6.as.mutation(api.mail.register, { flow: "f" });
    const { devices } = await proved6.as.query(api.mail.devices, { id: boxId });
    expect(devices).toHaveLength(5);
    expect(devices.map((d) => d.deviceId)).not.toContain(phones[1].p.deviceId);
    expect(devices.map((d) => d.deviceId)).toContain(sixth.deviceId);
  });
});

describe("mail in", () => {
  it("seals an OTP to each install, keeps it 5 minutes and logs a mail_otp event", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    await tb.as.mutation(api.mail.register, { flow: "f" });
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    const sentMs = Date.now() - 5000;
    const res = await mail(t, { addressId: box.id, receivedAt: sentMs, otp: "401295", gmailConfirm: null, from: "noreply.sdc@vitap.ac.in", subject: "OTP" });
    expect(res.status).toBe(200);
    for (const [p, as] of [
      [a, ta.as],
      [b, tb.as],
    ] as const) {
      const inbox = await as.query(api.mail.inbox, { id: box.id });
      expect(JSON.parse(await text(p.sealSecret, box.id, inbox.sealed))).toEqual({ kind: "otp", code: "401295", sentAt: Math.floor(sentMs / 1000) });
      expect(inbox.lastOtpAt).toBe(Math.floor(sentMs / 1000));
    }
    advance(301);
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).sealed).toBeNull();
    // Then the boxes themselves go, without waiting for the 15-minute sweep.
    await t.finishAllScheduledFunctions(vi.runAllTimers);
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("keeps Gmail's confirmation a day for phones that join later, until an OTP comes", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    const confirm = { code: "123456789", link: "https://mail-settings.google.com/mail/vf-abc", requester: "s@gmail.com" };
    expect((await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: null, gmailConfirm: confirm })).status).toBe(200);
    const first = await ta.as.query(api.mail.inbox, { id: box.id });
    expect(first.sealed).toBeNull();
    expect(JSON.parse(await text(a.sealSecret, box.id, first.confirmation))).toMatchObject({ kind: "gmail_confirm", ...confirm });
    advance(60);
    const tb = await prove(t, b);
    await tb.as.mutation(api.mail.register, { flow: "f" });
    // Joining again replaces the previous gmail_confirm row instead of duplicating it.
    await tb.as.mutation(api.mail.join, { flow: "f" });
    const bConfirmRows = await t.run(async (ctx) =>
      (await ctx.db.query("otpInbox").collect()).filter((r) => r.deviceId === b.deviceId && r.kind === "gmail_confirm"),
    );
    expect(bConfirmRows).toHaveLength(1);
    const joined = await tb.as.query(api.mail.inbox, { id: box.id });
    expect(JSON.parse(await text(b.sealSecret, box.id, joined.confirmation)).code).toBe("123456789");
    await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "111111" });
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmation).toBeNull();
  });

  it("takes mail for an old address waiting for its student, and keeps the newest OTP", async () => {
    const t = await setup();
    await t.run((ctx) =>
      ctx.db.insert("unclaimedMailboxes", { addressId: "kkkkkkkkkk", ownerHash: "o", confirmation: null, confirmationAt: null, lastOtpAt: null, createdAt: 1, updatedAt: 1 }),
    );
    // No bounce, which could make Gmail turn the forwarding off.
    const res = await mail(t, { addressId: "kkkkkkkkkk", receivedAt: Date.now(), otp: "123456" });
    expect(res.status).toBe(200);
    expect((await t.run((ctx) => ctx.db.query("unclaimedMailboxes").first()))?.lastOtpAt).toBe(Math.floor(Date.now() / 1000));

    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    const newer = Date.now();
    await mail(t, { addressId: box.id, receivedAt: newer, otp: "222222" });
    // A queued retry of an older mail comes in after the newer one.
    await mail(t, { addressId: box.id, receivedAt: newer - 60_000, otp: "111111" });
    const inbox = await ta.as.query(api.mail.inbox, { id: box.id });
    expect(JSON.parse(await text(a.sealSecret, box.id, inbox.sealed)).code).toBe("222222");
  });

  it("a phone another student signs in on stops getting the first one's OTPs", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    advance(3600);
    await prove(t, a, "23ABC0002");
    await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "444444" });
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("a membership left on another student's address gives that student's phone nothing", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    advance(10);
    const tb = await prove(t, b, "23ABC0002");
    // As a backfill could leave it: b on a's address, under a's account.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("mailboxDevices").collect())[0];
      await ctx.db.insert("mailboxDevices", { ...row, _id: undefined, _creationTime: undefined, deviceId: b.deviceId } as never);
    });
    for (const call of [
      () => tb.as.query(api.mail.devices, { id: box.id }),
      () => tb.as.query(api.mail.inbox, { id: box.id }),
      () => tb.as.mutation(api.mail.remove, { flow: "f", id: box.id, everywhere: true }),
      () => tb.as.mutation(api.mail.removeDevice, { flow: "f", id: box.id, removeDevice: a.deviceId }),
    ]) {
      expect(await codeOf(call())).toBe("mailbox_not_found");
    }
    expect((await tb.as.query(api.screen.view, { mailbox: box.id })).mailbox).toEqual({ gone: true });
    expect((await ta.as.query(api.mail.devices, { id: box.id })).devices).toHaveLength(2);
    expect(await t.mutation(internal.migrate.dropStrayMembers, {})).toBe(1);
    expect((await ta.as.query(api.mail.devices, { id: box.id })).devices).toHaveLength(1);
  });

  it("drops a box sealed to a phone that changed student before delivery", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    // Sealed for the address's phones, then the phone signs in as someone else.
    advance(3600);
    await prove(t, a, "23ABC0002");
    await t.mutation(internal.mail.deliver, {
      addressId: box.id,
      kind: "otp",
      sentAt: Date.now(),
      plaintext: "",
      boxes: [{ deviceId: a.deviceId, sealed: "x" }],
    });
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("takes a phone migrated onto another student's address off it when it proves", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    advance(10);
    await prove(t, b, "23ABC0002");
    // As the backfill leaves it: b on a's address from v2, and not linked to anyone yet.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("devices").collect()).find((d) => d.deviceId === b.deviceId);
      await ctx.db.patch(row!._id, { accountId: undefined });
      await ctx.db.insert("mailboxDevices", { addressId: box.id, deviceId: b.deviceId, addedAt: 1, seenAt: 1, accountId: ta.account });
    });
    advance(3600);
    await prove(t, b, "23ABC0002");
    const members = await t.run((ctx) => ctx.db.query("mailboxDevices").collect());
    expect(members.map((m) => m.deviceId)).toEqual([a.deviceId]);
  });

  it("join finds the student's address and never makes one", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    expect(await ta.as.mutation(api.mail.join, { flow: "f" })).toBeNull();
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    advance(10);
    const tb = await prove(t, b);
    expect(await tb.as.mutation(api.mail.join, { flow: "f" })).toEqual(box);
  });

  it("claims the old address used last, unless the current one got an OTP since", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    const waiting = (addressId: string, lastOtpAt: number | null) =>
      t.run((ctx) => ctx.db.insert("unclaimedMailboxes", { addressId, ownerHash: "o", confirmation: null, confirmationAt: null, lastOtpAt, createdAt: 1, updatedAt: 1 }));
    const claim = () => t.run((ctx) => claimMailbox(ctx, ta.account as Id<"accounts">, "o"));
    const addresses = () => t.run(async (ctx) => (await ctx.db.query("mailboxes").collect()).map((m) => m.addressId));
    const left = () => t.run(async (ctx) => (await ctx.db.query("unclaimedMailboxes").collect()).length);
    // Two old addresses: the one with the later OTP wins over the new one, and both rows go.
    await waiting("kkkkkkkkkk", 5);
    await waiting("mmmmmmmmmm", 9);
    await claim();
    expect(await addresses()).toEqual(["mmmmmmmmmm"]);
    expect(await left()).toBe(0);
    const members = await t.run((ctx) => ctx.db.query("mailboxDevices").collect());
    expect(members.map((m) => [m.addressId, m.deviceId])).toEqual([["mmmmmmmmmm", a.deviceId]]);
    // The current one got an OTP after this old one: it stays, and the old one goes.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("mailboxes").collect())[0];
      await ctx.db.patch(row._id, { lastOtpAt: 20 });
    });
    await waiting("kkkkkkkkkk", 10);
    await claim();
    expect(await addresses()).toEqual(["mmmmmmmmmm"]);
    expect(await left()).toBe(0);
  });

  it("stops sealing OTPs to a phone taken off the account", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    await tb.as.mutation(api.mail.register, { flow: "f" });
    const other = (await ta.as.query(api.account.view, {})).phones.find((ph) => !ph.thisPhone)!;
    await ta.as.mutation(api.account.removeDevice, { flow: "f", id: other.id });
    await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "333333" });
    const boxes = await t.run((ctx) => ctx.db.query("otpInbox").collect());
    expect(boxes.map((r) => r.deviceId)).toEqual([a.deviceId]);
  });

  it("answers 404 for an unknown address, 401 without MAIL_TOKEN and 400 for no code", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    expect((await mail(t, { addressId: "aaaaaaaaaa", receivedAt: Date.now(), otp: "123456" })).status).toBe(404);
    expect((await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "123456" }, TOKENS.INTERNAL_TOKEN)).status).toBe(401);
    const bad = await mail(t, { addressId: box.id, receivedAt: Date.now() });
    expect(bad.status).toBe(400);
    expect(((await bad.json()) as { code: string }).code).toBe("bad_request");
  });

  it("drops Gmail's confirmation at 24h via scheduled mutation so live mail:inbox re-runs", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    const confirm = { code: "123456789", link: "https://mail-settings.google.com/mail/vf-abc", requester: "s@gmail.com" };
    expect((await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: null, gmailConfirm: confirm })).status).toBe(200);
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmation).not.toBeNull();
    vi.advanceTimersByTime(CONFIRMATION_TTL_SECS * 1000);
    await t.finishInProgressScheduledFunctions();
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmation).toBeNull();
    const row = await t.run((ctx) => ctx.db.query("mailboxes").first());
    expect(row?.confirmation).toBeNull();
    expect(await t.run((ctx) => ctx.db.query("otpInbox").collect())).toEqual([]);
  });

  it("join with fromId returns null when the account still uses that same address, and joins when it moved", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    advance(10);
    const tb = await prove(t, b);
    const box1 = await ta.as.mutation(api.mail.register, { flow: "f" });
    await tb.as.mutation(api.mail.register, { flow: "f" });
    await ta.as.mutation(api.mail.removeDevice, { flow: "f", id: box1.id, removeDevice: b.deviceId });
    // Removed from box1 on purpose: passing fromId=box1.id must not rejoin it.
    expect(await tb.as.mutation(api.mail.join, { flow: "f", fromId: box1.id })).toBeNull();
    const afterRefused = await ta.as.query(api.mail.devices, { id: box1.id });
    expect(afterRefused.devices.map((d) => d.deviceId)).toEqual([a.deviceId]);
    // Without fromId (older apps or NeedsRejoin), join still joins box1.
    expect(await tb.as.mutation(api.mail.join, { flow: "f" })).toEqual(box1);
    // When the student deletes box1 and registers a new address box2, fromId=box1.id joins box2.
    await ta.as.mutation(api.mail.remove, { flow: "f", id: box1.id, everywhere: true });
    const box2 = await ta.as.mutation(api.mail.register, { flow: "f" });
    expect(box2.id).not.toBe(box1.id);
    expect(await tb.as.mutation(api.mail.join, { flow: "f", fromId: box1.id })).toEqual(box2);
  });

  it("keeps confirmedAt past dropConfirmation and returns it from mail:inbox", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmedAt).toBeNull();
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    const confirmMs = Date.now() - 2000;
    const confirmSec = Math.floor(confirmMs / 1000);
    const confirm = { code: "123456789", link: "https://mail-settings.google.com/mail/vf-abc", requester: "s@gmail.com" };
    expect((await mail(t, { addressId: box.id, receivedAt: confirmMs, otp: null, gmailConfirm: confirm })).status).toBe(200);
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmedAt).toBe(confirmSec);
    // After 24h, dropConfirmation clears confirmation but keeps confirmedAt.
    vi.advanceTimersByTime(CONFIRMATION_TTL_SECS * 1000);
    await t.finishInProgressScheduledFunctions();
    const afterExpiry = await ta.as.query(api.mail.inbox, { id: box.id });
    expect(afterExpiry.confirmation).toBeNull();
    expect(afterExpiry.confirmedAt).toBe(confirmSec);
    // A second phone joining after 24h still sees confirmedAt.
    const tb = await prove(t, b);
    await tb.as.mutation(api.mail.join, { flow: "f" });
    const bInbox = await tb.as.query(api.mail.inbox, { id: box.id });
    expect(bInbox.confirmation).toBeNull();
    expect(bInbox.confirmedAt).toBe(confirmSec);
    // An OTP clears confirmation but keeps confirmedAt.
    await mail(t, { addressId: box.id, receivedAt: Date.now(), otp: "987654" });
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmedAt).toBe(confirmSec);
    // Legacy rows written before confirmedAt existed remain valid.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("mailboxes").collect())[0];
      await ctx.db.patch(row._id, { confirmedAt: undefined });
    });
    expect((await ta.as.query(api.mail.inbox, { id: box.id })).confirmedAt).toBeNull();
  });

  it("ignores a gmail_confirm older than the stored confirmation's sentAt", async () => {
    const t = await setup();
    const a = await phone(1);
    const ta = await prove(t, a);
    const box = await ta.as.mutation(api.mail.register, { flow: "f" });
    const newerMs = Date.now();
    const newerSec = Math.floor(newerMs / 1000);
    await mail(t, {
      addressId: box.id,
      receivedAt: newerMs,
      otp: null,
      gmailConfirm: { code: "222222222", link: "https://mail-settings.google.com/mail/vf-new", requester: "s@gmail.com" },
    });
    // An older retried gmail_confirm arrives later: must not replace the newer one.
    await mail(t, {
      addressId: box.id,
      receivedAt: newerMs - 60_000,
      otp: null,
      gmailConfirm: { code: "111111111", link: "https://mail-settings.google.com/mail/vf-old", requester: "old@gmail.com" },
    });
    const inbox = await ta.as.query(api.mail.inbox, { id: box.id });
    expect(JSON.parse(await text(a.sealSecret, box.id, inbox.confirmation)).code).toBe("222222222");
    expect(inbox.confirmedAt).toBe(newerSec);
    // A newer gmail_confirm still replaces it.
    advance(30);
    const newestMs = Date.now();
    await mail(t, {
      addressId: box.id,
      receivedAt: newestMs,
      otp: null,
      gmailConfirm: { code: "333333333", link: "https://mail-settings.google.com/mail/vf-newest", requester: "s@gmail.com" },
    });
    const latest = await ta.as.query(api.mail.inbox, { id: box.id });
    expect(JSON.parse(await text(a.sealSecret, box.id, latest.confirmation)).code).toBe("333333333");
    expect(latest.confirmedAt).toBe(Math.floor(newestMs / 1000));
  });
});
