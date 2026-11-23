// The student's account as a linked phone sees and changes it (`/v1/account*`
// of the edge). The JWT names the student, so no session or account token is
// sent, and every caller is a linked phone.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import { type MutationCtx, action, internalMutation, internalQuery, mutation, query } from "./_generated/server";
import { deleteOwned, keysOf, phoneOf, phoneRows, phonesOf, saveRecord, unlinkPhone } from "./lib/accounts";
import { AccountRecord, parseSettings, phoneId, semesterId } from "./lib/accountState";
import { deleteMailbox } from "./lib/mailboxes";
import { badRequest, fail } from "./lib/errors";
import { memberLabel } from "./lib/checks";
import { credentialsOf, usableText } from "./lib/credentials";
import { type Caller, actionCaller, caller, mailboxOfAccount, realCaller, recordOf } from "./lib/identity";
import { sealCredentials } from "./sessions";
import { log, now } from "./lib/util";

/** The account view: `registrationNumber, createdAt, settings, semesterId, linked,
 * savedCredentials, vaultSavedAt, vaultGmail, credentialsRejectedAt, forwarding, phoneLinked,
 * phoneCount, thisPhone, phones[], keys[]`. Live. */
export const view = query({
  args: { flow: v.optional(v.string()) },
  handler: async (ctx) => {
    const me = await caller(ctx);
    const at = now();
    if (!me.account) {
      const demo = new AccountRecord(me.regno, me.device.createdAt);
      return demo.view([phoneOf(me.device)], [], me.deviceId, false, at);
    }
    const doc = await recordOf(ctx, me.account);
    const id = me.account._id;
    return doc.view(await phonesOf(ctx, id), await keysOf(ctx, id), me.deviceId, doc.mailboxId !== null, at);
  },
});

/** Runs `change` on the caller's account record and saves it. */
async function change(ctx: MutationCtx, flow: string, what: string, edit: (doc: AccountRecord, me: Caller) => void | Promise<void>): Promise<null> {
  const me = await realCaller(ctx);
  const doc = await recordOf(ctx, me.account);
  await edit(doc, me);
  await saveRecord(ctx, me.account._id, doc);
  log({ flow, regno: me.regno }, `account: ${what}`);
  return null;
}

/** Saves the student's VTOP password on the server, as `/v1/link` did with its credentials: the
 * engine seals them (`/engine/seal`) and the account keeps the sealed vault. Only when the server
 * can read the OTP (Gmail access sent along, or a forwarding address); otherwise nothing is kept.
 * `label` renames this phone. Answers `{ savedCredentials, forwarding }`. */
export const saveCredentials = action({
  args: { flow: v.string(), label: v.optional(v.string()), credentials: v.any() },
  handler: async (ctx, args): Promise<{ savedCredentials: boolean; forwarding: boolean }> => {
    const me = await actionCaller(ctx);
    if (!me.account) return fail("demo_unavailable", "The demo account can't do this.");
    const credentials = credentialsOf(args.credentials);
    if (!credentials) return badRequest("credentials is required.");
    const forwarding = await ctx.runQuery(internal.account.hasForwarding, { accountId: me.account._id });
    const username = usableText(credentials.username);
    const vault = username === null ? undefined : await sealCredentials(me.regno, username, credentials, forwarding, args.flow);
    const saved: boolean = await ctx.runMutation(internal.account.keepVault, {
      accountId: me.account._id,
      deviceId: me.deviceId,
      label: memberLabel(args.label) ?? undefined,
      ...(vault ? { vault } : {}),
    });
    log({ flow: args.flow, regno: me.regno }, vault ? "account: credentials saved" : "account: credentials not saved");
    return { savedCredentials: saved, forwarding };
  },
});

export const hasForwarding = internalQuery({
  args: { accountId: v.id("accounts") },
  handler: async (ctx, { accountId }) => (await mailboxOfAccount(ctx, accountId)) !== null,
});

/** Keeps the sealed vault (and the phone's new label); true when the account has usable saved
 * credentials afterwards. */
export const keepVault = internalMutation({
  args: { accountId: v.id("accounts"), deviceId: v.string(), label: v.optional(v.string()), vault: v.optional(v.object({ sealed: v.string(), gmail: v.boolean() })) },
  handler: async (ctx, { accountId, deviceId, label, vault }) => {
    const account = await ctx.db.get(accountId);
    if (!account) return false;
    const doc = await recordOf(ctx, account);
    const at = now();
    if (vault) {
      doc.saveVault(vault.sealed, at, vault.gmail);
      await saveRecord(ctx, accountId, doc);
    }
    if (label !== undefined) {
      const phone = (await phoneRows(ctx, accountId)).find((row) => row.deviceId === deviceId);
      if (phone) await ctx.db.patch(phone._id, { label });
    }
    return doc.usableVault(at) !== null;
  },
});

export const forgetCredentials = mutation({
  args: { flow: v.string() },
  handler: (ctx, { flow }) => change(ctx, flow, "credentials forgotten", (doc) => doc.clearVault()),
});

export const setSettings = mutation({
  args: { flow: v.string(), settings: v.any() },
  handler: (ctx, { flow, settings }) =>
    change(ctx, flow, "settings changed", (doc) => {
      const parsed = parseSettings(settings);
      if (!parsed) badRequest("settings must use the offered choices.");
      else doc.settings = parsed;
    }),
});

export const setSemester = mutation({
  args: { flow: v.string(), semesterId: v.string() },
  handler: async (ctx, args) => {
    const semester = semesterId(args.semesterId);
    if (semester === null) return badRequest("semesterId is required.");
    // Sent with every new JWT: the same semester writes nothing, so live views don't run again.
    const me = await realCaller(ctx);
    if ((await recordOf(ctx, me.account)).semesterId === semester) return null;
    return change(ctx, args.flow, "semester set", (doc) => {
      doc.semesterId = semester;
    });
  },
});

/** Unlinks a phone (its id from `account:view`); its JWT stops working at once. */
export const removeDevice = mutation({
  args: { flow: v.string(), id: v.string() },
  handler: async (ctx, { flow, id }) => {
    const me = await realCaller(ctx);
    const phone = (await phoneRows(ctx, me.account._id)).find((row) => phoneId(row.deviceId) === id.trim());
    if (!phone) return fail("phone_not_found", "No such phone on this account.");
    await unlinkPhone(ctx, phone);
    log({ flow, regno: me.regno }, "account: a phone was removed");
    return null;
  },
});

export const clearSignIns = mutation({
  args: { flow: v.string() },
  handler: async (ctx, { flow }) => {
    const me = await realCaller(ctx);
    await deleteOwned(ctx, "signIns", me.account._id);
    log({ flow, regno: me.regno }, "account: sign-in history cleared");
    return null;
  },
});

/** Deletes the account and every row it owns: keys, sign-ins, session requests, pairings, OAuth
 * grants, its forwarding address and sealed mail. Every phone is unlinked and loses its refresh
 * tokens, so its JWT stops working at once. */
export const remove = mutation({
  args: { flow: v.string() },
  handler: async (ctx, { flow }) => {
    const me = await realCaller(ctx);
    const id = me.account._id;
    for (const phone of await phoneRows(ctx, id)) await unlinkPhone(ctx, phone);
    const mailbox = await mailboxOfAccount(ctx, id);
    if (mailbox) await deleteMailbox(ctx, mailbox);
    for (const table of ["accessKeys", "signIns", "sessionRequests", "pairings", "oauthCodes", "oauthRequests", "v2Tokens", "refreshTokens", "otpInbox", "mailboxDevices", "logins"] as const) {
      await deleteOwned(ctx, table, id);
    }
    await ctx.db.delete(id);
    log({ flow, regno: me.regno }, "account: deleted at the user's request");
    return null;
  },
});
