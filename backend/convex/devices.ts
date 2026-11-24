// The install itself (`/v1/devices/*` of the edge). It is registered by
// `auth:token`; these change, show and delete it.

import { v } from "convex/values";
import { mutation, query } from "./_generated/server";
import { dropMail, revokeDevice } from "./lib/accounts";
import { fcmToken, memberLabel, sealKey } from "./lib/checks";
import { badRequest } from "./lib/errors";
import { caller } from "./lib/identity";
import { leaveMailbox } from "./lib/mailboxes";
import { log } from "./lib/util";

export const update = mutation({
  args: { flow: v.string(), fcmToken: v.optional(v.string()), label: v.optional(v.string()), sealKey: v.optional(v.string()) },
  handler: async (ctx, args) => {
    const me = await caller(ctx);
    const changes: { sealKey?: string; fcmToken?: string; label?: string } = {};
    const seal = sealKey(args.sealKey);
    if (!seal.ok) badRequest(seal.message);
    else if (seal.value !== null) changes.sealKey = seal.value;
    const fcm = fcmToken(args.fcmToken);
    if (!fcm.ok) badRequest(fcm.message);
    else if (fcm.value !== null) changes.fcmToken = fcm.value;
    const label = memberLabel(args.label);
    if (label !== null) changes.label = label;
    if (Object.keys(changes).length === 0) badRequest("Nothing to update.");
    await ctx.db.patch(me.device._id, changes);
    log({ flow: args.flow, regno: me.regno }, "device: updated");
    return null;
  },
});

/** The install as the backend keeps it: no keys or token, only whether it has a push token. */
export const info = query({
  args: { flow: v.optional(v.string()) },
  handler: async (ctx) => {
    const { device } = await caller(ctx);
    return {
      deviceId: device.deviceId,
      label: device.label,
      platform: device.platform,
      pushToken: Boolean(device.fcmToken),
      createdAt: device.createdAt,
    };
  },
});

/** The install deletes itself: with `mailbox` it leaves that address first (deleting it when it
 * was the last install on it); its row, FCM token, sealed mail and refresh tokens go, which also
 * takes it off its account's phones. Its access keys stay, with no phone to ask. */
export const remove = mutation({
  args: { flow: v.string(), mailbox: v.optional(v.string()) },
  handler: async (ctx, { flow, mailbox }) => {
    const me = await caller(ctx);
    if (mailbox) {
      try {
        await leaveMailbox(ctx, me.deviceId, mailbox, me.account?._id ?? null, { deleteWhenLast: true });
      } catch {
        // Not on that address (any more): leaving it is what was wanted. It throws before writing.
      }
    }
    await revokeDevice(ctx, me.deviceId);
    await dropMail(ctx, me.deviceId);
    await ctx.db.delete(me.device._id);
    log({ flow, regno: me.regno }, "device: an install deleted itself");
    return null;
  },
});
