// Reading and writing `accounts` rows as `AccountRecord`s, and the rows that
// hang off an account: its phones (linked `devices`), keys and sign-ins.

import { internal } from "../_generated/api";
import type { Doc, Id } from "../_generated/dataModel";
import type { MutationCtx, QueryCtx } from "../_generated/server";
import { AccountRecord, type KeyRecord, MAX_PHONES, MAX_SIGN_INS, type PhoneRecord, type SignInRecord, leastRecent, toFields } from "./accountState";
import { claimMailbox, touch } from "./mailboxes";
import { now } from "./util";

export async function accountByRegno(ctx: QueryCtx, regno: string): Promise<Doc<"accounts"> | null> {
  return ctx.db
    .query("accounts")
    .withIndex("by_regno", (q) => q.eq("regno", regno))
    .unique();
}

/** Writes the record back with `updatedAt`, and schedules server-side expiry at `sweepAt` so live
 * queries re-run when a cached session or saved vault expires. */
export async function saveRecord(ctx: MutationCtx, id: Id<"accounts">, record: AccountRecord): Promise<void> {
  record.updatedAt = now();
  const fields = toFields(record);
  const prev = await ctx.db.get(id);
  await ctx.db.patch(id, fields);
  if (fields.sweepAt !== undefined && fields.sweepAt !== prev?.sweepAt) {
    await ctx.scheduler.runAt(fields.sweepAt * 1000, internal.account.expire, { accountId: id });
  }
}

/** The student's account, made on first sight. */
export async function ensureAccount(ctx: MutationCtx, regno: string, ownerHash: string | null): Promise<Doc<"accounts">> {
  const found = await accountByRegno(ctx, regno);
  let id: Id<"accounts">;
  if (found) {
    if (ownerHash !== null && found.ownerHash !== ownerHash) await ctx.db.patch(found._id, { ownerHash });
    id = found._id;
  } else {
    id = await ctx.db.insert("accounts", { regno, ownerHash, ...toFields(new AccountRecord(regno, now())) });
  }
  if (ownerHash !== null) await claimMailbox(ctx, id, ownerHash);
  return (await ctx.db.get(id))!;
}

// ---- phones ---------------------------------------------------------------------------

export async function phoneRows(ctx: QueryCtx, accountId: Id<"accounts">): Promise<Doc<"devices">[]> {
  return ctx.db
    .query("devices")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
}

export function phoneOf(row: Doc<"devices">): PhoneRecord {
  const addedAt = row.linkedAt ?? row.createdAt;
  return { deviceId: row.deviceId, label: row.label, addedAt, seenAt: row.seenAt ?? addedAt };
}

/** The account's phones, oldest link first. */
export async function phonesOf(ctx: QueryCtx, accountId: Id<"accounts">): Promise<PhoneRecord[]> {
  return (await phoneRows(ctx, accountId)).map(phoneOf).sort((a, b) => a.addedAt - b.addedAt);
}

/** Deletes every refresh token of the install, so it can't get another JWT. */
export async function revokeDevice(ctx: MutationCtx, deviceId: string): Promise<void> {
  const rows = await ctx.db
    .query("refreshTokens")
    .withIndex("by_device", (q) => q.eq("deviceId", deviceId))
    .collect();
  for (const row of rows) await ctx.db.delete(row._id);
}

/** Takes the install off its account: ends its refresh tokens and takes it off the account's
 * forwarding address, so a removed or lost phone stops getting the student's OTPs. */
export async function unlinkPhone(ctx: MutationCtx, device: Doc<"devices">): Promise<void> {
  await ctx.db.patch(device._id, { accountId: undefined, linkedAt: undefined, seenAt: undefined });
  await revokeDevice(ctx, device.deviceId);
  // Even unlinked: a backfilled membership can sit on an install that was never linked here.
  await dropMail(ctx, device.deviceId);
}

/** Takes the install off every forwarding address, and deletes its mail, except `keep`'s: a phone
 * that changes student must not get the last one's OTPs. By install, not by account, so rows
 * migrated under an account the phone was never linked to go too. Kept memberships are marked
 * seen (hourly at most), so an active phone isn't dropped as stale when another joins. */
export async function dropMail(ctx: MutationCtx, deviceId: string, keep?: Id<"accounts">): Promise<void> {
  const members = await ctx.db
    .query("mailboxDevices")
    .withIndex("by_device", (q) => q.eq("deviceId", deviceId))
    .collect();
  for (const row of members) {
    if (row.accountId !== keep) await ctx.db.delete(row._id);
    else await touch(ctx, row);
  }
  const boxes = await ctx.db
    .query("otpInbox")
    .withIndex("by_device", (q) => q.eq("deviceId", deviceId))
    .collect();
  for (const row of boxes) if (row.accountId !== keep) await ctx.db.delete(row._id);
}

/** Links the install as one of the account's phones (linking again marks it seen). The phone seen
 * least recently makes room past `MAX_PHONES`, and loses its refresh tokens. */
export async function linkPhone(ctx: MutationCtx, account: Doc<"accounts">, device: Doc<"devices">): Promise<void> {
  const at = now();
  const again = device.accountId === account._id;
  // Another student's phone until now: it leaves their account and forwarding address first.
  if (!again && device.accountId !== undefined) await unlinkPhone(ctx, device);
  await ctx.db.patch(device._id, {
    accountId: account._id,
    linkedAt: again ? (device.linkedAt ?? at) : at,
    seenAt: at,
    linkedOnce: true,
    demoExpiresAt: undefined,
  });
  // Linking again too: the backfill can leave an older student's membership on this install.
  await dropMail(ctx, device.deviceId, account._id);
  const others = (await phoneRows(ctx, account._id)).filter((row) => row.deviceId !== device.deviceId);
  while (others.length >= MAX_PHONES) {
    const oldest = leastRecent(others.map(phoneOf));
    await unlinkPhone(ctx, others[oldest]);
    others.splice(oldest, 1);
  }
}

// ---- keys -----------------------------------------------------------------------------

export async function keyRows(ctx: QueryCtx, accountId: Id<"accounts">): Promise<Doc<"accessKeys">[]> {
  const rows = await ctx.db
    .query("accessKeys")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  return rows.sort((a, b) => a.createdAt - b.createdAt);
}

export function keyOf(row: Doc<"accessKeys">): KeyRecord {
  return { hash: row.keyHash, label: row.label, createdAt: row.createdAt, lastUsedAt: row.lastUsedAt, deviceId: row.deviceId, outings: row.outings, uses: row.uses };
}

export async function keysOf(ctx: QueryCtx, accountId: Id<"accounts">): Promise<KeyRecord[]> {
  return (await keyRows(ctx, accountId)).map(keyOf);
}

// ---- sign-ins -------------------------------------------------------------------------

/** The account's latest sign-ins, newest first. */
export async function signInsOf(ctx: QueryCtx, accountId: Id<"accounts">): Promise<SignInRecord[]> {
  const rows = await ctx.db
    .query("signIns")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .order("desc")
    .take(MAX_SIGN_INS);
  return rows.map(({ at, by, ok, reason }) => ({ at, by, ok, reason }));
}

/** Keeps a sign-in, dropping the oldest past `MAX_SIGN_INS`. */
export async function recordSignIn(ctx: MutationCtx, accountId: Id<"accounts">, signIn: SignInRecord): Promise<void> {
  await ctx.db.insert("signIns", { accountId, ...signIn });
  const old = await ctx.db
    .query("signIns")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .order("desc")
    .collect();
  for (const row of old.slice(MAX_SIGN_INS)) await ctx.db.delete(row._id);
}

/** Tables whose rows belong to an account and carry a `by_account` index. */
export const ACCOUNT_OWNED_TABLES = [
  "accessKeys",
  "signIns",
  "sessionRequests",
  "pairings",
  "oauthCodes",
  "oauthRequests",
  "v2Tokens",
  "refreshTokens",
  "otpInbox",
  "mailboxDevices",
  "logins",
] as const;

export type AccountOwnedTable = (typeof ACCOUNT_OWNED_TABLES)[number];

/** Deletes every row of the account in `table` (all are indexed `by_account`). */
export async function deleteOwned(ctx: MutationCtx, table: AccountOwnedTable, accountId: Id<"accounts">): Promise<void> {
  const rows = await ctx.db
    .query(table)
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  for (const row of rows) await ctx.db.delete(row._id);
}
