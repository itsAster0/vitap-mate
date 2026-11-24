// `jwt` functions: the socket's access JWT names the install (`sub`), its
// student (`regno`) and account row (`acct`). Every call also checks that
// the install still exists and is linked to that account, so removing a
// phone ends its access at once, not when its JWT runs out.

import type { UserIdentity } from "convex/server";
import { internal } from "../_generated/api";
import type { Doc, Id } from "../_generated/dataModel";
import type { ActionCtx, QueryCtx } from "../_generated/server";
import { type AccountRecord, fromFields } from "./accountState";
import { fail } from "./errors";

export const DEMO_REGNO = "DEMO2026";

export type Caller = {
  deviceId: string;
  device: Doc<"devices">;
  regno: string;
  demo: boolean;
  /** Null for the demo, which has no account row. */
  account: Doc<"accounts"> | null;
};

export type Claims = { deviceId: string; regno: string; acct: string; demo: boolean };

export function claimsOf(identity: UserIdentity | null): Claims {
  if (identity === null) fail("unauthenticated", "Sign in first.");
  // An access key's JWT (the extension's) never acts as a phone.
  if (identity.kind !== undefined) fail("unauthenticated", "Sign in first.");
  const regno = identity.regno;
  const acct = identity.acct;
  if (typeof regno !== "string" || typeof acct !== "string") fail("unauthenticated", "Sign in first.");
  return { deviceId: identity.subject, regno, acct, demo: identity.demo === true };
}

const removed = (): never => fail("device_removed", "This phone was removed from the account; sign in again.");

export async function deviceRow(ctx: QueryCtx, deviceId: string): Promise<Doc<"devices"> | null> {
  return ctx.db
    .query("devices")
    .withIndex("by_deviceId", (q) => q.eq("deviceId", deviceId))
    .unique();
}

export async function callerOf(ctx: QueryCtx, claims: Claims): Promise<Caller> {
  const device = await deviceRow(ctx, claims.deviceId);
  if (!device) return removed();
  if (claims.demo) {
    const live = await ctx.db
      .query("refreshTokens")
      .withIndex("by_device", (q) => q.eq("deviceId", claims.deviceId))
      .first();
    if (!live || live.accountId !== undefined) return removed();
    return { deviceId: claims.deviceId, device, regno: DEMO_REGNO, demo: true, account: null };
  }
  const id = ctx.db.normalizeId("accounts", claims.acct);
  const account = id && (await ctx.db.get(id));
  if (!account || account.regno !== claims.regno || device.accountId !== account._id) return removed();
  return { deviceId: claims.deviceId, device, regno: account.regno, demo: false, account };
}

/** The caller of a query or mutation. */
export async function caller(ctx: QueryCtx): Promise<Caller> {
  return callerOf(ctx, claimsOf(await ctx.auth.getUserIdentity()));
}

/** As `caller`, refusing the demo account: it changes no server state. */
export async function realCaller(ctx: QueryCtx): Promise<Caller & { account: Doc<"accounts"> }> {
  const found = await caller(ctx);
  if (found.account === null) fail("demo_unavailable", "The demo account can't do this.");
  return found as Caller & { account: Doc<"accounts"> };
}

/** The caller of an action, checked by an internal query. */
export async function actionCaller(ctx: ActionCtx): Promise<Caller> {
  const claims = claimsOf(await ctx.auth.getUserIdentity());
  return ctx.runQuery(internal.identity.check, claims);
}

/** The student's forwarding address row, if they have one. */
export async function mailboxOfAccount(ctx: QueryCtx, accountId: Id<"accounts">): Promise<Doc<"mailboxes"> | null> {
  return ctx.db
    .query("mailboxes")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .first();
}

/** The record of an account row, for the rules in lib/accountState.ts. */
export async function recordOf(ctx: QueryCtx, account: Doc<"accounts">): Promise<AccountRecord> {
  const { _id, _creationTime, regno, ownerHash, ...fields } = account;
  return fromFields(regno, fields, (await mailboxOfAccount(ctx, _id))?.addressId ?? null);
}

export type AccountId = Id<"accounts">;
