// Rows past their time, swept by crons.ts. Each sweep reads only rows that are due, through an
// index, and at most `BATCH` a run; the next run takes the rest.

import { internalMutation } from "./_generated/server";
import { DEMO_INACTIVE_SECS } from "./auth";
import { revokeDevice, saveRecord } from "./lib/accounts";
import { REQUEST_KEEP_SECS, SESSION_CACHE_SECS } from "./lib/accountState";
import { recordOf } from "./lib/identity";
import { sweepLogins } from "./logins";
import { CONFIRMATION_TTL_SECS, SEALED_TTL_SECS } from "./lib/mailboxes";
import { now } from "./lib/util";

const BATCH = 500;

/** Every 5 minutes: expired tickets, pairings, OAuth requests and codes, server sign-ins whose OTP
 * mail never came, and finished session requests (kept 10 minutes for late polls and answers). */
export const fiveMinutes = internalMutation({
  args: {},
  handler: async (ctx) => {
    const at = now();
    for (const table of ["proofTickets", "pairings", "oauthRequests", "oauthCodes"] as const) {
      const rows = await ctx.db
        .query(table)
        .withIndex("by_expiresAt", (q) => q.lte("expiresAt", at))
        .take(BATCH);
      for (const row of rows) await ctx.db.delete(row._id);
    }
    await sweepLogins(ctx, at, BATCH);
    const requests = await ctx.db
      .query("sessionRequests")
      .withIndex("by_createdAt", (q) => q.lte("createdAt", at - REQUEST_KEEP_SECS))
      .take(BATCH);
    for (const row of requests) await ctx.db.delete(row._id);
  },
});

/** Every 15 minutes: sealed mail past its use (an OTP box after 5 minutes, a confirmation after
 * a day), cached sessions past their week and saved credentials past the student's chosen
 * lifetime. */
export const quarter = internalMutation({
  args: {},
  handler: async (ctx) => {
    const at = now();
    const expired = await ctx.db
      .query("otpInbox")
      .withIndex("by_expiresAt", (q) => q.gte("expiresAt", 0).lte("expiresAt", at))
      .take(BATCH);
    for (const row of expired) await ctx.db.delete(row._id);
    // Legacy rows written before `expiresAt` existed sit at `expiresAt === undefined` in
    // `by_expiresAt`; delete expired ones and stamp live ones so they leave this bucket immediately.
    const legacy = await ctx.db
      .query("otpInbox")
      .withIndex("by_expiresAt", (q) => q.eq("expiresAt", undefined))
      .take(BATCH);
    for (const row of legacy) {
      const expiresAt = row.receivedAt + (row.kind === "otp" ? SEALED_TTL_SECS : CONFIRMATION_TTL_SECS);
      if (expiresAt <= at) await ctx.db.delete(row._id);
      else await ctx.db.patch(row._id, { expiresAt });
    }
    const confirmations = await ctx.db
      .query("mailboxes")
      .withIndex("by_confirmationAt", (q) => q.gte("confirmationAt", 0).lte("confirmationAt", at - CONFIRMATION_TTL_SECS))
      .take(BATCH);
    for (const mailbox of confirmations) await ctx.db.patch(mailbox._id, { confirmation: null, confirmationAt: null });
    const due = await ctx.db
      .query("accounts")
      .withIndex("by_sweepAt", (q) => q.gte("sweepAt", 0).lte("sweepAt", at))
      .take(BATCH);
    for (const account of due) {
      const doc = await recordOf(ctx, account);
      if (doc.sealedSession !== null && (!doc.settings.cacheSession || at >= (doc.sessionSavedAt ?? 0) + SESSION_CACHE_SECS)) {
        doc.clearSession();
      }
      const ttl = doc.settings.vaultTtlSecs;
      if (doc.sealedVault !== null && ttl !== null && doc.vaultSavedAt !== null && at >= doc.vaultSavedAt + ttl) {
        doc.clearVault();
      }
      // Saving sets the next `sweepAt` (or unsets it), so the row leaves this range either way.
      await saveRecord(ctx, account._id, doc);
    }
    // Pre-migration accounts without a vault still carry a future `sweepAt` for their unconsented
    // `sealedSession`; drop it now instead of waiting up to 7 days.
    const future = await ctx.db
      .query("accounts")
      .withIndex("by_sweepAt", (q) => q.gt("sweepAt", at))
      .take(BATCH);
    for (const account of future) {
      if (account.sealedSession === null) continue;
      const doc = await recordOf(ctx, account);
      if (doc.settings.cacheSession) continue;
      doc.clearSession();
      await saveRecord(ctx, account._id, doc);
    }
    // Inactive demo refresh tokens (no accountId) and demo devices: swept bounded by index so
    // real students' non-expiring refresh tokens and linked devices are never touched.
    const staleDemoTokens = await ctx.db
      .query("refreshTokens")
      .withIndex("by_account_createdAt", (q) => q.eq("accountId", undefined).lte("createdAt", at - DEMO_INACTIVE_SECS))
      .take(BATCH);
    const sweptDemoDevices = new Set<string>();
    for (const row of staleDemoTokens) {
      await ctx.db.delete(row._id);
      sweptDemoDevices.add(row.deviceId);
    }
    for (const deviceId of sweptDemoDevices) {
      const stillActive = await ctx.db
        .query("refreshTokens")
        .withIndex("by_device", (q) => q.eq("deviceId", deviceId))
        .first();
      if (stillActive) continue;
      const device = await ctx.db
        .query("devices")
        .withIndex("by_deviceId", (q) => q.eq("deviceId", deviceId))
        .unique();
      if (device && device.accountId === undefined && !device.linkedOnce) await ctx.db.delete(device._id);
    }
    const staleDemoDevices = await ctx.db
      .query("devices")
      .withIndex("by_demoExpiresAt", (q) => q.gte("demoExpiresAt", 0).lte("demoExpiresAt", at))
      .take(BATCH);
    for (const device of staleDemoDevices) {
      if (device.accountId !== undefined) {
        await ctx.db.patch(device._id, { demoExpiresAt: undefined });
        continue;
      }
      await revokeDevice(ctx, device.deviceId);
      if (device.linkedOnce) await ctx.db.patch(device._id, { demoExpiresAt: undefined });
      else await ctx.db.delete(device._id);
    }
  },
});
