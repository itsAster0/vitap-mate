// What the mailbox objects store, and the checks on what clients send. No
// Worker APIs here, so it is tested on its own.

import { randomBytes } from "@noble/hashes/utils.js";

import { isPublicKey } from "./shared/mail-seal";

// ---- forwarding addresses: `otp+<id>@<domain>` --------------------------------
// One Email Routing rule (`otp@<domain>` → this Worker) catches every plus
// address; a subdomain can have no catch-all.

export const ID_LEN = 10;
/** No `l`, `o`, `0` or `1`, so an id read aloud or typed is unambiguous. */
export const ID_ALPHABET = "abcdefghijkmnpqrstuvwxyz23456789";
/** How long a sealed box waits to be collected. */
export const SEALED_TTL_SECS = 300;
export const MAX_FCM_TOKEN_LEN = 4096;
export const ADDRESS_PREFIX = "otp+";

export function newId(): string {
  // 32 letters: a byte modulo 32 is uniform.
  return Array.from(randomBytes(ID_LEN), (byte) => ID_ALPHABET[byte % ID_ALPHABET.length]).join("");
}

export function isId(id: string): boolean {
  return id.length === ID_LEN && [...id].every((c) => ID_ALPHABET.includes(c));
}

export function address(id: string, domain: string): string {
  return `${ADDRESS_PREFIX}${id}@${domain}`;
}

/** The mailbox id in a recipient address, if it is one of ours. */
export function idFromAddress(recipient: string): string | null {
  const local = recipient.trim().split("@")[0].replace(/[A-Z]/g, (c) => c.toLowerCase());
  if (!local.startsWith(ADDRESS_PREFIX)) return null;
  const id = local.slice(ADDRESS_PREFIX.length);
  return isId(id) ? id : null;
}

// ---- mailboxes ------------------------------------------------------------------

/** Installs one address forwards to, at most. Adding one more drops the
 * one seen least recently. */
export const MAX_MEMBERS = 5;
/** An install not seen for this long is dropped when another joins. */
export const MEMBER_STALE_SECS = 60 * 24 * 3600;
/** `seenAt` is written at most this often, so polling stays read-only. */
const SEEN_EVERY_SECS = 3600;
export const MAX_LABEL_LEN = 60;

/** One install an address forwards to and its pending box, sealed to the
 * install's own `sealKey`, so each opens only what was sealed for it. */
export interface MailboxMember {
  deviceId: string;
  sealed: string | null;
  sealedAt: number | null;
  addedAt: number;
  /** When this install last used the address. */
  seenAt: number;
}

/** One forwarding address and the installs it forwards to. */
export interface MailboxDoc {
  id: string;
  ownerHash: string | null;
  members: MailboxMember[];
  createdAt: number;
  updatedAt: number;
}

/** The latest box, while it may still be collected. */
export function freshSealed(member: MailboxMember, now: number): { sealed: string; at: number } | null {
  const { sealed, sealedAt: at } = member;
  if (sealed === null || at === null) return null;
  return Math.max(0, now - at) <= SEALED_TTL_SECS ? { sealed, at } : null;
}

/** The index of this install among the members, or null. */
export function memberOf(doc: MailboxDoc, deviceId: string): number | null {
  const index = doc.members.findIndex((m) => m.deviceId === deviceId);
  return index < 0 ? null : index;
}

/** The index of the member seen least recently. */
function leastRecent<T extends { seenAt: number; addedAt: number }>(items: T[]): number {
  let oldest = 0;
  items.forEach((item, i) => {
    const old = items[oldest];
    if (Math.max(item.seenAt, item.addedAt) < Math.max(old.seenAt, old.addedAt)) oldest = i;
  });
  return oldest;
}

/** Adds `member`, replacing an earlier entry for the same install. Installs
 * gone quiet are dropped, then the least recently seen while there are too
 * many. The new member always stays. */
export function addMember(doc: MailboxDoc, member: MailboxMember, now: number): void {
  doc.members = doc.members
    .filter((m) => m.deviceId !== member.deviceId)
    .filter((m) => now - Math.max(m.seenAt, m.addedAt) < MEMBER_STALE_SECS);
  while (doc.members.length >= MAX_MEMBERS) doc.members.splice(leastRecent(doc.members), 1);
  doc.members.push(member);
}

/** Records a visit; true when that changed what is stored. */
export function touch(doc: MailboxDoc, index: number, now: number): boolean {
  const member = doc.members[index];
  if (now - member.seenAt < SEEN_EVERY_SECS) return false;
  member.seenAt = now;
  return true;
}

/** Drops boxes that can no longer be collected; true when any was. */
export function clearExpired(doc: MailboxDoc, now: number): boolean {
  let changed = false;
  for (const member of doc.members) {
    if (member.sealed !== null && freshSealed(member, now) === null) {
      member.sealed = null;
      member.sealedAt = null;
      changed = true;
    }
  }
  return changed;
}

/** When the last pending box expires, for the cleanup alarm. */
export function nextExpiry(doc: MailboxDoc): number | null {
  const times = doc.members.flatMap((m) => (m.sealedAt === null ? [] : [m.sealedAt]));
  return times.length ? Math.max(...times) + SEALED_TTL_SECS + 1 : null;
}

/** A label as sent by the app (a device's or a key's): trimmed, short, or
 * none. */
export function memberLabel(raw: string | undefined | null): string | null {
  const label = raw?.trim();
  return label ? [...label].slice(0, MAX_LABEL_LEN).join("") : null;
}

// ---- engine sign-ins waiting for an OTP -------------------------------------------

/** How long an engine sign-in may wait for forwarded OTPs. */
export const WAITER_TTL_SECS = 300;

/** An engine sign-in waiting for its student's next OTP mail. Codes come
 * sealed to its key; the edge keeps nothing it can read. */
export interface Waiter {
  publicKey: string;
  ownerHash: string;
  /** Mail sent before this (unix seconds) is not for this sign-in. */
  since: number;
  expiresAt: number;
  results: string[];
}

export function wants(waiter: Waiter, sentAt: number, now: number): boolean {
  return now < waiter.expiresAt && sentAt >= waiter.since;
}

// ---- input checks -------------------------------------------------------------------

/** Ok with the token (or null when none was sent), or the error message. */
export type Checked = { ok: true; value: string | null } | { ok: false; message: string };

export function fcmToken(raw: string | undefined): Checked {
  if (raw === undefined) return { ok: true, value: null };
  const token = raw.trim();
  if (!token || token.length > MAX_FCM_TOKEN_LEN) return { ok: false, message: "fcmToken is not an FCM token." };
  return { ok: true, value: token };
}

/** An X25519 key (`sealKey`), stored without padding. */
export function sealKey(raw: string | undefined): Checked {
  if (raw === undefined) return { ok: true, value: null };
  const key = raw.trim().replace(/=+$/, "");
  return isPublicKey(key) ? { ok: true, value: key } : { ok: false, message: "sealKey must be a base64url X25519 key." };
}

export const MAX_PLATFORM_LEN = 20;

/** `platform` as the app sends it ("android", "ios"). */
export function platform(raw: string | undefined): Checked {
  const value = raw?.trim().toLowerCase() ?? "";
  return /^[a-z0-9_-]+$/.test(value) && value.length <= MAX_PLATFORM_LEN
    ? { ok: true, value }
    : { ok: false, message: "platform is required (android, ios, ...)." };
}
