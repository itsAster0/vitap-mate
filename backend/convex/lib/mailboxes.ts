// Forwarding addresses `otp+<id>@<MAIL_DOMAIN>` and their member devices. Each
// address belongs to a student's account from the start, so a new phone of
// theirs joins the same address.

import type { Doc, Id } from "../_generated/dataModel";
import type { MutationCtx, QueryCtx } from "../_generated/server";
import { randomBytes } from "./crypto";
import { fail } from "./errors";
import { now } from "./util";

export const ID_LEN = 10;
/** No `l`, `o`, `0` or `1`, so an id read aloud or typed is unambiguous. */
export const ID_ALPHABET = "abcdefghijkmnpqrstuvwxyz23456789";
/** How long a sealed box waits to be collected. */
export const SEALED_TTL_SECS = 300;
/** How long Gmail's confirmation is kept readable for phones that join during setup. It holds
 * the student's Gmail address and a live link, so not for good. */
export const CONFIRMATION_TTL_SECS = 24 * 3600;
export const ADDRESS_PREFIX = "otp+";
/** Installs one address forwards to, at most. Adding one more drops the one seen least recently. */
export const MAX_MEMBERS = 5;
/** An install not seen for this long is dropped when another joins. */
export const MEMBER_STALE_SECS = 60 * 24 * 3600;
/** `seenAt` is written at most this often. */
const SEEN_EVERY_SECS = 3600;

export function newId(): string {
  // 32 letters: a byte modulo 32 is uniform.
  return Array.from(
    randomBytes(ID_LEN),
    (byte) => ID_ALPHABET[byte % ID_ALPHABET.length],
  ).join("");
}

export function isId(id: string): boolean {
  return id.length === ID_LEN && [...id].every((c) => ID_ALPHABET.includes(c));
}

export function mailDomain(): string {
  return (process.env.MAIL_DOMAIN ?? "vmate.aster0.dev").trim();
}

export function address(id: string): string {
  return `${ADDRESS_PREFIX}${id}@${mailDomain()}`;
}

export const unknownMailbox = (): never =>
  fail("mailbox_not_found", "No such forwarding address.");

/** Whether an address id is in use, claimed or still waiting for its student. */
export async function addressTaken(
  ctx: QueryCtx,
  addressId: string,
): Promise<boolean> {
  if (await mailboxRow(ctx, addressId)) return true;
  const waiting = await ctx.db
    .query("unclaimedMailboxes")
    .withIndex("by_addressId", (q) => q.eq("addressId", addressId))
    .first();
  return waiting !== null;
}

/** Gives the account the old address waiting under its owner hash, if Gmail may still forward to
 * it ([`giveWayTo`]). Of several waiting (older v2 addresses), the one that got an OTP last; the
 * rest go. */
export async function claimMailbox(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  ownerHash: string,
): Promise<void> {
  const waiting = await ctx.db
    .query("unclaimedMailboxes")
    .withIndex("by_ownerHash", (q) => q.eq("ownerHash", ownerHash))
    .collect();
  if (waiting.length === 0) return;
  const best = waiting.reduce((a, b) => (newer(b, a) ? b : a));
  for (const row of waiting) await ctx.db.delete(row._id);
  if (
    (await giveWayTo(ctx, accountId, best.addressId, best.lastOtpAt)) === "kept"
  )
    return;
  const { _id, _creationTime, ownerHash: _owner, ...fields } = best;
  await ctx.db.insert("mailboxes", { ...fields, accountId });
}

/** Whether `a` was in use after `b`: its last OTP, else its last change. */
function newer(
  a: { lastOtpAt: number | null; updatedAt: number },
  b: { lastOtpAt: number | null; updatedAt: number },
): boolean {
  return (a.lastOtpAt ?? -1) !== (b.lastOtpAt ?? -1)
    ? (a.lastOtpAt ?? -1) > (b.lastOtpAt ?? -1)
    : a.updatedAt > b.updatedAt;
}

/** Makes way for the old address `addressId` (its last OTP at `lastOtpAt`) when the account has
 * another one: "kept" when that one got an OTP since, so Gmail forwards there and it stays.
 * Otherwise it goes ("replaced"): its phones move over (at most [`MAX_MEMBERS`], the most recent),
 * and its sealed mail and waiting engine sign-ins go. "none" when the account has no address. */
export async function giveWayTo(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  addressId: string,
  lastOtpAt: number | null,
): Promise<"none" | "kept" | "replaced"> {
  const theirs = await ctx.db
    .query("mailboxes")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .first();
  if (!theirs || theirs.addressId === addressId) return "none";
  if (theirs.lastOtpAt !== null && theirs.lastOtpAt > (lastOtpAt ?? -1))
    return "kept";
  const already = new Set(
    (await membersOf(ctx, addressId)).map((m) => m.deviceId),
  );
  for (const m of await membersOf(ctx, theirs.addressId)) {
    if (already.has(m.deviceId)) await ctx.db.delete(m._id);
    else await ctx.db.patch(m._id, { addressId });
  }
  await capMembers(ctx, addressId);
  for (const row of await ctx.db
    .query("otpInbox")
    .withIndex("by_address_device_kind", (q) => q.eq("addressId", theirs.addressId))
    .collect()) {
    await ctx.db.delete(row._id);
  }
  await ctx.db.delete(theirs._id);
  return "replaced";
}

/** Keeps the [`MAX_MEMBERS`] installs on `addressId` seen most recently. */
export async function capMembers(
  ctx: MutationCtx,
  addressId: string,
): Promise<void> {
  const members = await membersOf(ctx, addressId);
  members.sort(
    (a, b) => Math.max(b.seenAt, b.addedAt) - Math.max(a.seenAt, a.addedAt),
  );
  for (const m of members.slice(MAX_MEMBERS)) await ctx.db.delete(m._id);
}

export async function mailboxRow(
  ctx: QueryCtx,
  addressId: string,
): Promise<Doc<"mailboxes"> | null> {
  if (!isId(addressId)) return null;
  return ctx.db
    .query("mailboxes")
    .withIndex("by_addressId", (q) => q.eq("addressId", addressId))
    .unique();
}

export async function membersOf(
  ctx: QueryCtx,
  addressId: string,
): Promise<Doc<"mailboxDevices">[]> {
  return ctx.db
    .query("mailboxDevices")
    .withIndex("by_address", (q) => q.eq("addressId", addressId))
    .collect();
}

/** The installs mail to `mailbox` is sealed to: members still registered, and linked to the
 * mailbox's student (or not linked yet, as phones from v2 are until their first call). */
export async function sealedTo(
  ctx: QueryCtx,
  mailbox: Doc<"mailboxes">,
): Promise<{ deviceId: string; sealKey: string }[]> {
  const out = [];
  for (const m of await membersOf(ctx, mailbox.addressId)) {
    const device = await ctx.db
      .query("devices")
      .withIndex("by_deviceId", (q) => q.eq("deviceId", m.deviceId))
      .unique();
    if (
      device &&
      (device.accountId === undefined || device.accountId === mailbox.accountId)
    ) {
      out.push({ deviceId: m.deviceId, sealKey: device.sealKey });
    }
  }
  return out;
}

/** The mailbox named `addressId` with its members, if it is `accountId`'s and the install is one
 * of them. A membership left from another student (a phone that changed hands) gives nothing. */
export async function owned(
  ctx: QueryCtx,
  deviceId: string,
  addressId: string,
  accountId: Id<"accounts"> | null,
) {
  const mailbox = await mailboxRow(ctx, addressId);
  if (!mailbox || mailbox.accountId !== accountId) return unknownMailbox();
  const members = await membersOf(ctx, addressId);
  const me = members.find((m) => m.deviceId === deviceId);
  if (!me) return unknownMailbox();
  return { mailbox, members, me };
}

/** Records a visit, at most hourly, so quiet installs can be told apart. */
export async function touch(
  ctx: MutationCtx,
  member: Doc<"mailboxDevices">,
): Promise<void> {
  const at = now();
  if (at - member.seenAt >= SEEN_EVERY_SECS)
    await ctx.db.patch(member._id, { seenAt: at });
}

/** Adds the install, dropping installs gone quiet, then the least recently seen while there are
 * too many. The new member always stays. */
export async function addMember(
  ctx: MutationCtx,
  addressId: string,
  deviceId: string,
  accountId: Id<"accounts">,
): Promise<void> {
  const at = now();
  let members = await membersOf(ctx, addressId);
  for (const m of members) {
    if (
      m.deviceId === deviceId ||
      at - Math.max(m.seenAt, m.addedAt) >= MEMBER_STALE_SECS
    )
      await ctx.db.delete(m._id);
  }
  members = members.filter(
    (m) =>
      m.deviceId !== deviceId &&
      at - Math.max(m.seenAt, m.addedAt) < MEMBER_STALE_SECS,
  );
  while (members.length >= MAX_MEMBERS) {
    let oldest = 0;
    members.forEach((m, i) => {
      const old = members[oldest];
      if (Math.max(m.seenAt, m.addedAt) < Math.max(old.seenAt, old.addedAt))
        oldest = i;
    });
    await ctx.db.delete(members[oldest]._id);
    members.splice(oldest, 1);
  }
  await ctx.db.insert("mailboxDevices", {
    addressId,
    deviceId,
    addedAt: at,
    seenAt: at,
    accountId,
  });
}

async function dropInbox(
  ctx: MutationCtx,
  addressId: string,
  deviceId: string,
): Promise<void> {
  for (const kind of ["otp", "gmail_confirm"] as const) {
    const rows = await ctx.db
      .query("otpInbox")
      .withIndex("by_address_device_kind", (q) =>
        q.eq("addressId", addressId).eq("deviceId", deviceId).eq("kind", kind),
      )
      .collect();
    for (const row of rows) await ctx.db.delete(row._id);
  }
}

/** Deletes the address, its members and boxes. */
export async function deleteMailbox(
  ctx: MutationCtx,
  mailbox: Doc<"mailboxes">,
): Promise<void> {
  for (const m of await membersOf(ctx, mailbox.addressId)) {
    await dropInbox(ctx, mailbox.addressId, m.deviceId);
    await ctx.db.delete(m._id);
  }
  await ctx.db.delete(mailbox._id);
}

/** Takes the install off the address. With `deleteWhenLast` (the edge's `delete`) the address
 * goes when it was the last one, or with `everywhere`; without it (`leave`) the address stays
 * for the student, whose Gmail filter still forwards to it. Throws `mailbox_not_found` when the
 * install is not on it. */
export async function leaveMailbox(
  ctx: MutationCtx,
  deviceId: string,
  addressId: string,
  accountId: Id<"accounts"> | null,
  opts: { deleteWhenLast: boolean; everywhere?: boolean },
): Promise<"left" | "deleted"> {
  const { mailbox, members, me } = await owned(ctx, deviceId, addressId, accountId);
  if (
    opts.deleteWhenLast &&
    (opts.everywhere === true || members.length <= 1)
  ) {
    await deleteMailbox(ctx, mailbox);
    return "deleted";
  }
  await dropInbox(ctx, addressId, deviceId);
  await ctx.db.delete(me._id);
  await ctx.db.patch(mailbox._id, { updatedAt: now() });
  return "left";
}
