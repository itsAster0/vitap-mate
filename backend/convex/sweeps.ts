// What the Durable Objects' alarms used to drop, swept by crons.ts. Each
// sweep deletes at most `BATCH` rows a run; the next run takes the rest.

import { internalMutation } from "./_generated/server";
import { REFRESH_TTL_SECS } from "./auth";
import { saveRecord } from "./lib/accounts";
import { REQUEST_KEEP_SECS, SESSION_CACHE_SECS } from "./lib/accountState";
import { recordOf } from "./lib/identity";
import { sweepLogins } from "./logins";
import { CONFIRMATION_TTL_SECS, SEALED_TTL_SECS } from "./lib/mailboxes";
import { now } from "./lib/util";

const BATCH = 500;

/** Every minute: expired tickets, pairings, OAuth requests and codes, server sign-ins whose OTP
 * mail never came, and finished session requests (kept 10 minutes for late polls and answers). */
export const minute = internalMutation({
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

/** Every hour: sealed mail past its use (an OTP box after 15 minutes, a confirmation after a
 * day), cached sessions past their week and saved credentials past the
 * student's chosen lifetime. */
export const hour = internalMutation({
  args: {},
  handler: async (ctx) => {
    const at = now();
    const inbox = await ctx.db
      .query("otpInbox")
      .withIndex("by_receivedAt", (q) => q.lte("receivedAt", at - Math.max(15 * 60, SEALED_TTL_SECS)))
      .take(BATCH);
    for (const row of inbox) {
      if (row.kind === "otp" || at - row.receivedAt >= CONFIRMATION_TTL_SECS) await ctx.db.delete(row._id);
    }
    for (const mailbox of await ctx.db.query("mailboxes").collect()) {
      if (mailbox.confirmationAt !== null && at - mailbox.confirmationAt >= CONFIRMATION_TTL_SECS) {
        await ctx.db.patch(mailbox._id, { confirmation: null, confirmationAt: null });
      }
    }
    for (const account of await ctx.db.query("accounts").collect()) {
      const stale = account.sealedSession !== null && at >= (account.sessionSavedAt ?? 0) + SESSION_CACHE_SECS;
      const ttl = account.settings.vaultTtlSecs;
      const expired = account.sealedVault !== null && ttl !== null && account.vaultSavedAt !== null && at >= account.vaultSavedAt + ttl;
      if (!stale && !expired) continue;
      const doc = await recordOf(ctx, account);
      if (stale) doc.clearSession();
      if (expired) doc.clearVault();
      await saveRecord(ctx, account._id, doc);
    }
  },
});

/** Every day: refresh tokens unused for 90 days. */
export const day = internalMutation({
  args: {},
  handler: async (ctx) => {
    const at = now();
    const tokens = await ctx.db
      .query("refreshTokens")
      .withIndex("by_createdAt", (q) => q.lte("createdAt", at - REFRESH_TTL_SECS))
      .take(BATCH);
    for (const row of tokens) await ctx.db.delete(row._id);
  },
});
