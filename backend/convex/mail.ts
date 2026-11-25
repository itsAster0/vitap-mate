// Forwarding addresses for OTP mail (`/v1/mail-otp/*` of the edge), and the
// mail itself: the mail Worker POSTs each parsed mail to `/internal/mail`,
// which seals it to every install on the address (http.ts `mailIn`) and
// stores the boxes here. An address is the student's from the start, so
// `owner` and `reclaim` are gone: registering again joins the student's
// address.

import { v } from "convex/values";
import {
  type MutationCtx,
  internalMutation,
  internalQuery,
  mutation,
  query,
} from "./_generated/server";
import { badRequest, fail } from "./lib/errors";
import { caller, realCaller } from "./lib/identity";
import { mailboxOfAccount } from "./lib/identity";
import { notify } from "./lib/notify";
import { offerCode } from "./logins";
import {
  CONFIRMATION_TTL_SECS,
  SEALED_TTL_SECS,
  addMember,
  address,
  addressTaken,
  leaveMailbox,
  mailboxRow,
  membersOf,
  newId,
  owned,
  sealedTo,
  touch,
  unknownMailbox,
} from "./lib/mailboxes";
import { internal } from "./_generated/api";
import { seal } from "./lib/mailSeal";
import { log, now } from "./lib/util";

/** The student's address, made now if they have none; this install joins it. */
export const register = mutation({
  args: { flow: v.string() },
  handler: async (ctx, { flow }) => (await joinAddress(ctx, flow, true))!,
});

/** Joins the student's address if they have one, else null: a phone whose address went looks for
 * where its phones moved, and an address the student deleted stays deleted. */
export const join = mutation({
  args: { flow: v.string() },
  handler: async (ctx, { flow }) => joinAddress(ctx, flow, false),
});

async function joinAddress(
  ctx: MutationCtx,
  flow: string,
  make: boolean,
): Promise<{ id: string; address: string } | null> {
  const me = await realCaller(ctx);
  const accountId = me.account._id;
  let mailbox = await mailboxOfAccount(ctx, accountId);
  const at = now();
  if (!mailbox && !make) return null;
  if (!mailbox) {
    let id = newId();
    for (
      let attempt = 0;
      (await addressTaken(ctx, id)) && attempt < 3;
      attempt++
    )
      id = newId();
    if (await addressTaken(ctx, id))
      fail("store_unavailable", "Could not create an address. Try again.");
    const row = await ctx.db.insert("mailboxes", {
      addressId: id,
      accountId,
      confirmation: null,
      confirmationAt: null,
      lastOtpAt: null,
      createdAt: at,
      updatedAt: at,
    });
    mailbox = (await ctx.db.get(row))!;
    log({ flow, regno: me.regno }, "mail-otp: registered a mailbox");
  } else {
    log({ flow, regno: me.regno }, "mail-otp: a device joined a mailbox");
  }
  await addMember(ctx, mailbox.addressId, me.deviceId, accountId);
  // Gmail's confirmation, if it is still kept, sealed now to this install, which joined after
  // Gmail sent it.
  if (
    mailbox.confirmation !== null &&
    mailbox.confirmationAt !== null &&
    at - mailbox.confirmationAt < CONFIRMATION_TTL_SECS
  ) {
    const sealed = await seal(
      me.device.sealKey,
      mailbox.addressId,
      new TextEncoder().encode(mailbox.confirmation),
    );
    if (sealed !== null) {
      for (const old of await ctx.db
        .query("otpInbox")
        .withIndex("by_address_device_kind", (q) =>
          q.eq("addressId", mailbox.addressId).eq("deviceId", me.deviceId).eq("kind", "gmail_confirm"),
        )
        .collect()) {
        await ctx.db.delete(old._id);
      }
      await ctx.db.insert("otpInbox", {
        addressId: mailbox.addressId,
        deviceId: me.deviceId,
        kind: "gmail_confirm",
        sealed,
        receivedAt: mailbox.confirmationAt,
        expiresAt: mailbox.confirmationAt + CONFIRMATION_TTL_SECS,
        accountId,
      });
    }
  }
  return { id: mailbox.addressId, address: address(mailbox.addressId) };
}

/** Leaves the address, or deletes it when last or with `everywhere`. */
export const remove = mutation({
  args: {
    flow: v.string(),
    id: v.string(),
    everywhere: v.optional(v.boolean()),
  },
  handler: async (ctx, { flow, id, everywhere }) => {
    const me = await realCaller(ctx);
    const done = await leaveMailbox(ctx, me.deviceId, id, me.account._id, {
      deleteWhenLast: true,
      everywhere,
    });
    log(
      { flow, regno: me.regno },
      done === "deleted"
        ? "mail-otp: deleted a mailbox"
        : "mail-otp: a device left a mailbox",
    );
    return null;
  },
});

/** This install leaves the address, which stays for the student even with no install left: their
 * Gmail filter still forwards to it, and their next phone joins it. */
export const leave = mutation({
  args: { flow: v.string(), id: v.string() },
  handler: async (ctx, { flow, id }) => {
    const me = await realCaller(ctx);
    await leaveMailbox(ctx, me.deviceId, id, me.account._id, { deleteWhenLast: false });
    log({ flow, regno: me.regno }, "mail-otp: a device left a mailbox");
    return null;
  },
});

/** The installs the address forwards to, for the app's device list. */
export const devices = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    const me = await caller(ctx);
    if (me.demo) return unknownMailbox();
    const { members } = await owned(ctx, me.deviceId, id, me.account?._id ?? null);
    const out = [];
    for (const m of members) {
      const device = await ctx.db
        .query("devices")
        .withIndex("by_deviceId", (q) => q.eq("deviceId", m.deviceId))
        .unique();
      out.push({
        deviceId: m.deviceId,
        label: device?.label ?? null,
        addedAt: m.addedAt,
        seenAt: m.seenAt,
        thisDevice: m.deviceId === me.deviceId,
      });
    }
    return { devices: out };
  },
});

/** Takes another install off the address; it stops getting OTPs. */
export const removeDevice = mutation({
  args: { flow: v.string(), id: v.string(), removeDevice: v.string() },
  handler: async (ctx, { flow, id, removeDevice: target }) => {
    const me = await realCaller(ctx);
    const { members, me: mine } = await owned(ctx, me.deviceId, id, me.account._id);
    await touch(ctx, mine);
    if (!members.some((m) => m.deviceId === target))
      fail("unknown_device", "That device does not use this address.");
    if (target === me.deviceId)
      badRequest("Use remove to take this device off the address.");
    await leaveMailbox(ctx, target, id, me.account._id, { deleteWhenLast: false });
    log(
      { flow, regno: me.regno },
      "mail-otp: a device was removed from a mailbox",
    );
    return null;
  },
});

/** The newest OTP sealed to this install (while it may be collected, 5 minutes), Gmail's latest
 * forwarding confirmation (a day, until an OTP comes) and when the latest OTP was sent. Live: a
 * phone waiting for its code watches this. */
export const inbox = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    const me = await caller(ctx);
    if (me.demo) return unknownMailbox();
    const { mailbox } = await owned(ctx, me.deviceId, id, me.account?._id ?? null);
    const at = now();
    const latest = (kind: "otp" | "gmail_confirm") =>
      ctx.db
        .query("otpInbox")
        .withIndex("by_address_device_kind", (q) =>
          q.eq("addressId", id).eq("deviceId", me.deviceId).eq("kind", kind),
        )
        .order("desc")
        .first();
    const box = await latest("otp");
    const fresh =
      box && Math.max(0, at - box.receivedAt) <= SEALED_TTL_SECS ? box : null;
    const confirm =
      mailbox.confirmation !== null ? await latest("gmail_confirm") : null;
    const kept =
      confirm && at - confirm.receivedAt < CONFIRMATION_TTL_SECS
        ? confirm.sealed
        : null;
    return {
      ...(fresh
        ? { sealed: fresh.sealed, receivedAt: fresh.receivedAt }
        : { sealed: null }),
      confirmation: kept,
      lastOtpAt: mailbox.lastOtpAt,
    };
  },
});

// ---- mail in (http.ts) ------------------------------------------------------------------------

/** Who a mail for `addressId` is sealed to: the installs on it. Null when there is no such
 * address. */
export const recipients = internalQuery({
  args: { addressId: v.string() },
  handler: async (ctx, { addressId }) => {
    const mailbox = await mailboxRow(ctx, addressId);
    if (!mailbox) return null;
    return {
      regno: (await ctx.db.get(mailbox.accountId))?.regno ?? null,
      members: await sealedTo(ctx, mailbox),
    };
  },
});

/** Stores a sealed mail: one box per install, each to its own key. Gmail's confirmation is also
 * kept readable a day for installs that join later, and leaves OTP boxes alone (anyone can make
 * Gmail send one, and it must not hide a code not yet collected). An OTP drops it, as forwarding
 * then works. Each install is woken with a `mail_otp` push, and the code (`otp`) goes on to the
 * student's server sign-ins waiting for it. */
export const deliver = internalMutation({
  args: {
    addressId: v.string(),
    kind: v.union(v.literal("otp"), v.literal("gmail_confirm")),
    sentAt: v.number(),
    plaintext: v.string(),
    boxes: v.array(v.object({ deviceId: v.string(), sealed: v.string() })),
    otp: v.optional(v.string()),
  },
  handler: async (ctx, { addressId, kind, sentAt, plaintext, boxes, otp }) => {
    const mailbox = await mailboxRow(ctx, addressId);
    if (!mailbox) return false;
    // A retried mail older than the last OTP stored: the newer code stays.
    if (mailbox.lastOtpAt !== null && sentAt < mailbox.lastOtpAt) return true;
    log(
      { regno: (await ctx.db.get(mailbox.accountId))?.regno },
      `mail-otp: storing a ${kind} for ${boxes.length} device(s)`,
    );
    const at = now();
    if (kind === "gmail_confirm") {
      await ctx.db.patch(mailbox._id, {
        confirmation: plaintext,
        confirmationAt: at,
        updatedAt: at,
      });
      await ctx.scheduler.runAt((at + CONFIRMATION_TTL_SECS) * 1000, internal.mail.dropConfirmation, {
        mailboxId: mailbox._id,
        confirmationAt: at,
      });
    } else {
      await ctx.db.patch(mailbox._id, {
        lastOtpAt: sentAt,
        confirmation: null,
        confirmationAt: null,
        updatedAt: at,
      });
    }
    // Sealing ran outside this mutation: a phone that left the address, or changed student, since
    // then gets nothing.
    const still = new Set(
      (await sealedTo(ctx, mailbox)).map((m) => m.deviceId),
    );
    for (const { deviceId, sealed } of boxes) {
      if (!still.has(deviceId)) continue;
      const old = await ctx.db
        .query("otpInbox")
        .withIndex("by_address_device_kind", (q) =>
          q
            .eq("addressId", addressId)
            .eq("deviceId", deviceId)
            .eq("kind", kind),
        )
        .collect();
      for (const row of old) await ctx.db.delete(row._id);
      const box = await ctx.db.insert("otpInbox", {
        addressId,
        deviceId,
        kind,
        sealed,
        receivedAt: at,
        expiresAt: at + (kind === "otp" ? SEALED_TTL_SECS : CONFIRMATION_TTL_SECS),
        accountId: mailbox.accountId,
      });
      // `inbox` stops handing out an OTP after SEALED_TTL_SECS, so it goes then; the 15-minute
      // sweep is only a backstop.
      if (kind === "otp")
        await ctx.scheduler.runAt((at + SEALED_TTL_SECS) * 1000, internal.mail.dropBox, { box });
      // The push is only a wake-up: an open app sees the box in `mail:inbox`.
      await notify(ctx, deviceId, "mail_otp", { mailbox: addressId, sealed });
    }
    if (otp !== undefined)
      await offerCode(ctx, mailbox.accountId, otp, sentAt);
    return true;
  },
});

/** Deletes an OTP box once it can no longer be collected; gone already (replaced or left) is
 * fine. */
export const dropBox = internalMutation({
  args: { box: v.id("otpInbox") },
  handler: async (ctx, { box }) => {
    if (await ctx.db.get(box)) await ctx.db.delete(box);
  },
});

/** Clears Gmail's forwarding confirmation at expiry so subscribed `mail:inbox` queries re-run. */
export const dropConfirmation = internalMutation({
  args: { mailboxId: v.id("mailboxes"), confirmationAt: v.number() },
  handler: async (ctx, { mailboxId, confirmationAt }) => {
    const mailbox = await ctx.db.get(mailboxId);
    if (!mailbox || mailbox.confirmationAt !== confirmationAt || now() < confirmationAt + CONFIRMATION_TTL_SECS) return;
    await ctx.db.patch(mailboxId, { confirmation: null, confirmationAt: null });
    const rows = await ctx.db
      .query("otpInbox")
      .withIndex("by_account", (q) => q.eq("accountId", mailbox.accountId))
      .collect();
    for (const row of rows) {
      if (row.addressId === mailbox.addressId && row.kind === "gmail_confirm" && row.receivedAt <= confirmationAt) {
        await ctx.db.delete(row._id);
      }
    }
  },
});

/** Records that mail reached an address still waiting for its student; false when there is no such
 * address at all. */
export const noteUnclaimed = internalMutation({
  args: {
    addressId: v.string(),
    kind: v.union(v.literal("otp"), v.literal("gmail_confirm")),
    sentAt: v.number(),
  },
  handler: async (ctx, { addressId, kind, sentAt }) => {
    const waiting = await ctx.db
      .query("unclaimedMailboxes")
      .withIndex("by_addressId", (q) => q.eq("addressId", addressId))
      .first();
    if (!waiting) return false;
    if (kind === "otp")
      await ctx.db.patch(waiting._id, {
        lastOtpAt: Math.max(waiting.lastOtpAt ?? 0, sentAt),
        updatedAt: now(),
      });
    console.log(`mail-otp: a ${kind} for an address not claimed yet, dropped`);
    return true;
  },
});
