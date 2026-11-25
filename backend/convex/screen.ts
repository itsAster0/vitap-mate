// The Server screen's one live read: the account, this install, what the server keeps, and the
// phones on this install's forwarding address. A change to any of them re-runs this one query,
// where the screen read four.

import { v } from "convex/values";
import type { Id } from "./_generated/dataModel";
import { query } from "./_generated/server";
import { keysOf, phoneOf, phonesOf, signInsOf } from "./lib/accounts";
import { AccountRecord } from "./lib/accountState";
import { caller, deviceFcmToken, recordOf } from "./lib/identity";
import { address, mailboxRow, membersOf } from "./lib/mailboxes";
import { now } from "./lib/util";

/** What a phone with no account (the demo) is shown the server keeps: nothing. */
const NOTHING_KEPT = {
  sessionSavedAt: null,
  sessionSource: null,
  vaultSavedAt: null,
  vaultExpiresAt: null,
  vaultGmail: false,
  credentialsRejectedAt: null,
  forwarding: null,
  signIns: [],
  phones: 1,
  keys: [],
};

/** `{ account, device, status, mailbox }`: `account:view`'s answer, `devices:info`'s, `app:status`'s
 * (null when this phone isn't linked), and with `mailbox` (this install's address) its phones, or
 * `{ gone: true }` when the install isn't on it any more. Live. */
export const view = query({
  args: { mailbox: v.optional(v.string()) },
  handler: async (ctx, { mailbox }) => {
    const me = await caller(ctx);
    const at = now();
    const device = {
      deviceId: me.device.deviceId,
      label: me.device.label,
      platform: me.device.platform,
      pushToken: deviceFcmToken(me.device) !== null,
      createdAt: me.device.createdAt,
    };
    if (!me.account) {
      const demo = new AccountRecord(me.regno, me.device.createdAt);
      const account = demo.view([phoneOf(me.device)], [], me.deviceId, false, at);
      return { account, device, status: NOTHING_KEPT, mailbox: mailbox === undefined ? null : { gone: true } };
    }
    const id = me.account._id;
    const doc = await recordOf(ctx, me.account);
    const phones = await phonesOf(ctx, id);
    const keys = await keysOf(ctx, id);
    const account = doc.view(phones, keys, me.deviceId, doc.mailboxId !== null, at);
    const forwarding = doc.mailboxId === null ? null : address(doc.mailboxId);
    const status = account.thisPhone ? doc.statusView(forwarding, phones.length, keys, await signInsOf(ctx, id)) : null;
    return { account, device, status, mailbox: mailbox === undefined ? null : await phonesOn(ctx, mailbox, me.deviceId, id) };
  },
});

/** The phones on `addressId`, or `{ gone: true }` when it isn't `accountId`'s or `deviceId` isn't
 * one of them. */
async function phonesOn(ctx: Parameters<typeof membersOf>[0], addressId: string, deviceId: string, accountId: Id<"accounts">) {
  const members = (await mailboxRow(ctx, addressId))?.accountId === accountId ? await membersOf(ctx, addressId) : [];
  if (!members.some((m) => m.deviceId === deviceId)) return { gone: true as const };
  const devices = [];
  for (const m of members) {
    const row = await ctx.db
      .query("devices")
      .withIndex("by_deviceId", (q) => q.eq("deviceId", m.deviceId))
      .unique();
    devices.push({ deviceId: m.deviceId, label: row?.label ?? null, addedAt: m.addedAt, seenAt: m.seenAt, thisDevice: m.deviceId === deviceId });
  }
  return { devices };
}
