// Who a phone is, and its tokens.
//
// 1. `auth:proveStart` (signed) hands the install a TLSNotary ticket: 2
//    minutes, one at a time. The verifier (vtop-verifier) takes it
//    (`/internal/proof-ticket`) when the phone opens `/proof`, and reports its
//    verdict (`/internal/proof-done`); the engine gives the regno's owner hash.
// 2. `auth:token` (signed) trades a proved ticket, a refresh token, a v2
//    account token or the demo for an access JWT and a new refresh token. A
//    refresh token is good once: using it replaces it, and showing a replaced
//    one again deletes its whole chain (`refresh_reused`), as only a leak
//    makes that happen. A token is also only good from the install it was
//    issued to, signed by its key. A proved ticket trades in again until it
//    expires, so a lost answer costs no new proof.
// 3. `auth:signOut` deletes the install's refresh tokens, or every phone's.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import type { Doc, Id } from "./_generated/dataModel";
import { type MutationCtx, action, internalMutation, mutation } from "./_generated/server";
import { ensureAccount, linkPhone, phoneRows, revokeDevice, unlinkPhone } from "./lib/accounts";
import { claimMailbox } from "./lib/mailboxes";
import { rateLimits } from "./lib/rateLimits";
import { fcmToken, memberLabel, platform, sealKey } from "./lib/checks";
import { randomBase64Url, randomHex, sha256Hex } from "./lib/crypto";
import { badRequest, fail } from "./lib/errors";
import { DEMO_REGNO, caller, deviceRow } from "./lib/identity";
import { signAccessJwt, verifyV2Token } from "./lib/jwt";
import { type SignedArgs, signedArgs, verifySigned } from "./lib/signed";
import { accountId, log, now, str } from "./lib/util";

/** How long a ticket waits for its proof. */
export const TICKET_TTL_SECS = 120;
/** How long a proved ticket waits for its `auth:token`, and takes it again after a lost answer. */
export const PROVED_TTL_SECS = 1800;
/** How long an inactive demo install and its refresh chain stay before being swept. */
export const DEMO_INACTIVE_SECS = 7 * 86_400;
export const REFRESH_PREFIX = "vrt_";

// ---- the proof ticket -------------------------------------------------------------------

export const proveStart = mutation({
  args: signedArgs,
  handler: async (ctx, args) => {
    const at = now();
    const { deviceId } = await verifySigned("auth:proveStart", args, at);
    // The gap is checked first, so a refused start doesn't spend the hour's allowance.
    for (const name of ["proofGap", "proofHourly", "proofTickets"] as const) {
      const { ok, retryAfter } = await rateLimits.limit(ctx, name, name === "proofTickets" ? {} : { key: deviceId });
      if (!ok) fail("too_many_proofs", "Too many proofs from this phone; wait a little.", { retryAfterMs: retryAfter });
    }
    // A new ticket replaces the last.
    for (const old of await ctx.db
      .query("proofTickets")
      .withIndex("by_device", (q) => q.eq("deviceId", deviceId))
      .collect()) {
      await ctx.db.delete(old._id);
    }
    // 128 bits, as the verifier's binding wants (32 lower-case hex digits).
    const ticket = randomHex(16);
    const expiresAt = at + TICKET_TTL_SECS;
    await ctx.db.insert("proofTickets", { ticket, deviceId, status: "open", expiresAt });
    log({ flow: args.flow }, `auth: proof ticket issued to ${deviceId.slice(0, 8)}`);
    return { ticket, expiresAt: expiresAt * 1000 };
  },
});

async function ticketRow(ctx: MutationCtx, ticket: string): Promise<Doc<"proofTickets"> | null> {
  if (!/^[0-9a-f]{32}$/.test(ticket)) return null;
  return ctx.db
    .query("proofTickets")
    .withIndex("by_ticket", (q) => q.eq("ticket", ticket))
    .unique();
}

/** The verifier starts a proof: an open ticket is used up and names its install; null when it is
 * unknown, used or expired. */
export const takeTicket = internalMutation({
  args: { ticket: v.string() },
  handler: async (ctx, { ticket }) => {
    const row = await ticketRow(ctx, ticket);
    const at = now();
    if (!row || row.status !== "open" || row.expiresAt <= at) return null;
    const { ok, retryAfter } = await rateLimits.limit(ctx, "proofGlobal", {});
    // The ticket stays open: the phone tries again once there is room.
    if (!ok) return { busy: retryAfter };
    // A proof can take a while on a slow VTOP: the sweep must not drop the ticket mid-proof.
    await ctx.db.patch(row._id, { status: "taken", expiresAt: at + TICKET_TTL_SECS });
    return { device: row.deviceId };
  },
});

/** The verifier's verdict on a taken ticket, with the owner hash the engine gave for its regno
 * (none when the engine couldn't say); false when there is no such ticket. */
export const proofDone = internalMutation({
  args: { ticket: v.string(), regno: v.optional(v.string()), ownerHash: v.optional(v.string()), code: v.optional(v.string()) },
  handler: async (ctx, { ticket, regno, ownerHash, code }) => {
    const row = await ticketRow(ctx, ticket);
    if (!row || row.status !== "taken") return false;
    const expiresAt = now() + PROVED_TTL_SECS;
    if (regno && !code) {
      await ctx.db.patch(row._id, { status: "proved", regno: accountId(regno), ...(ownerHash ? { ownerHash } : {}), expiresAt });
      console.log(`[regno=${accountId(regno)}] auth: a proof held`);
    } else {
      await ctx.db.patch(row._id, { status: "failed", code: code ?? "proof_failed", expiresAt });
      console.log("auth: a proof failed");
    }
    return true;
  },
});

// ---- tokens --------------------------------------------------------------------------------

type DeviceFields = { sealKey: string; platform: string; label: string | null; fcmToken: string | null };

/** The `device` of a token request, checked as `/v1/devices/register` checked it. */
function deviceOf(raw: unknown): DeviceFields {
  if (raw === null || typeof raw !== "object") return badRequest("device is required.");
  const body = raw as Record<string, unknown>;
  const seal = sealKey(str(body.sealKey));
  if (!seal.ok) return badRequest(seal.message);
  if (seal.value === null) return badRequest("sealKey is required.");
  const kind = platform(str(body.platform));
  if (!kind.ok) return badRequest(kind.message);
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  return { sealKey: seal.value, platform: kind.value!, label: memberLabel(str(body.label)), fcmToken: fcm.value };
}

/** Registers the install, or updates its keys, label and platform (and FCM token when sent). */
async function upsertDevice(ctx: MutationCtx, deviceId: string, signKey: string, device: DeviceFields): Promise<Doc<"devices">> {
  const found = await deviceRow(ctx, deviceId);
  const fields = { sealKey: device.sealKey, platform: device.platform, label: device.label ?? found?.label ?? null };
  if (found) {
    await ctx.db.patch(found._id, { ...fields, ...(device.fcmToken !== null ? { fcmToken: device.fcmToken } : {}) });
    return (await ctx.db.get(found._id))!;
  }
  const id = await ctx.db.insert("devices", {
    deviceId,
    signKey,
    ...fields,
    ...(device.fcmToken !== null ? { fcmToken: device.fcmToken } : {}),
    createdAt: now(),
  });
  return (await ctx.db.get(id))!;
}

type Grant =
  | { kind: "ticket"; ticket: string; device: DeviceFields }
  | { kind: "refresh"; hash: string }
  | { kind: "v2"; regno: string; iat: number; device: DeviceFields }
  | { kind: "demo"; device: DeviceFields };

type Asked =
  | { kind: "refresh"; token: string }
  | { kind: "ticket"; ticket: string; device: DeviceFields }
  | { kind: "v2"; token: string; device: DeviceFields }
  | { kind: "demo"; device: DeviceFields };

/** What a token request asks for: exactly one of the four. */
function grantOf(body: Record<string, unknown>): Asked {
  const kinds = [body.ticket !== undefined, body.refreshToken !== undefined, body.v2AccountToken !== undefined, body.demo !== undefined];
  if (kinds.filter(Boolean).length !== 1) badRequest("Send exactly one of ticket, refreshToken, v2AccountToken or demo.");
  if (body.refreshToken !== undefined) {
    const token = str(body.refreshToken) ?? "";
    if (!token.startsWith(REFRESH_PREFIX)) fail("refresh_invalid", "Unknown or revoked refresh token; prove the account again.");
    return { kind: "refresh", token };
  }
  const device = deviceOf(body.device);
  if (body.ticket !== undefined) return { kind: "ticket", ticket: str(body.ticket) ?? "", device };
  if (body.v2AccountToken !== undefined) return { kind: "v2", token: str(body.v2AccountToken) ?? "", device };
  if (body.demo !== true) badRequest("demo must be true.");
  return { kind: "demo", device };
}

/** `auth:token`: an action, as only actions can sign the ES256 JWT. */
export const token = action({
  args: signedArgs,
  handler: async (ctx, args: SignedArgs): Promise<{ jwt: string; jwtExpiresAt: number; refreshToken: string; regno: string; account: string }> => {
    const at = now();
    const call = await verifySigned("auth:token", args, at);
    const asked = grantOf(call.body);
    let grant: Grant;
    switch (asked.kind) {
      case "refresh":
        grant = { kind: "refresh", hash: await sha256Hex(asked.token) };
        break;
      case "v2": {
        const claims = await verifyV2Token(asked.token, call.deviceId, at);
        grant = { kind: "v2", regno: accountId(claims.sub), iat: claims.iat, device: asked.device };
        break;
      }
      default:
        grant = asked;
    }
    const refreshToken = REFRESH_PREFIX + randomBase64Url(32);
    const granted: Granted = await ctx.runMutation(internal.auth.grant, {
      deviceId: call.deviceId,
      signKey: call.signKey,
      grant,
      newHash: await sha256Hex(refreshToken),
      chainId: randomHex(16),
    });
    // Refusals after rate-limiting or state mutation (revoking a chain, consuming a failed ticket)
    // come back as values: throwing inside the mutation would roll those writes back.
    if ("refused" in granted) {
      const { code, message } = REFUSALS[granted.refused];
      return fail(code, message);
    }
    if ("proofFailed" in granted) return fail(granted.proofFailed, "VTOP's page did not prove who you are.");
    const { jwt, expiresAt } = await signAccessJwt(
      { sub: call.deviceId, regno: granted.regno, acct: granted.account, ...(granted.demo ? { demo: true as const } : {}) },
      at,
    );
    log({ flow: args.flow, regno: granted.regno }, `auth: issued tokens (${grant.kind})`);
    return { jwt, jwtExpiresAt: expiresAt, refreshToken, regno: granted.regno, account: granted.account };
  },
});

/** Deletes all but the last token `row`'s chain used before it. The one just used may come back
 * from a lost answer; the one before shows a stolen copy, as its reuse revokes the chain. Older
 * ones prove nothing more, so a chain stays three rows however long the phone keeps it. */
async function dropUsedBefore(ctx: MutationCtx, row: Doc<"refreshTokens">): Promise<void> {
  const used = (
    await ctx.db
      .query("refreshTokens")
      .withIndex("by_chain", (q) => q.eq("chainId", row.chainId))
      .collect()
  )
    .filter((old) => old._id !== row._id && old.usedAt !== null)
    .sort((a, b) => b._creationTime - a._creationTime);
  for (const old of used.slice(1)) await ctx.db.delete(old._id);
}

async function deleteChain(ctx: MutationCtx, chainId: string): Promise<void> {
  for (const row of await ctx.db
    .query("refreshTokens")
    .withIndex("by_chain", (q) => q.eq("chainId", chainId))
    .collect()) {
    await ctx.db.delete(row._id);
  }
}

const REFUSALS = {
  refresh_invalid: { code: "refresh_invalid", message: "Unknown or revoked refresh token; prove the account again." },
  refresh_reused: { code: "refresh_reused", message: "This refresh token was used already; prove the account again." },
  device_removed: { code: "device_removed", message: "This phone was removed from the account; sign in again." },
  challenge_invalid: { code: "challenge_invalid", message: "Unknown, used or expired ticket; start the proof again." },
  proof_pending: { code: "proof_pending", message: "The proof has not finished yet." },
  v2_not_live: { code: "token_invalid", message: "The edge did not hold this account token; prove the account instead." },
  v2_used: { code: "token_invalid", message: "This account token was traded in already; prove the account instead." },
} as const;

type Granted = { regno: string; account: string; demo: boolean } | { refused: keyof typeof REFUSALS } | { proofFailed: string };

const deviceFieldsValidator = v.object({
  sealKey: v.string(),
  platform: v.string(),
  label: v.union(v.string(), v.null()),
  fcmToken: v.union(v.string(), v.null()),
});

const grantValidator = v.union(
  v.object({ kind: v.literal("ticket"), ticket: v.string(), device: deviceFieldsValidator }),
  v.object({ kind: v.literal("refresh"), hash: v.string() }),
  v.object({ kind: v.literal("v2"), regno: v.string(), iat: v.number(), device: deviceFieldsValidator }),
  v.object({ kind: v.literal("demo"), device: deviceFieldsValidator }),
);

/** Checks the grant and stores the new refresh token: in the old one's chain for a refresh, else
 * in a new chain. Who it is for, as the JWT names them. */
export const grant = internalMutation({
  args: { deviceId: v.string(), signKey: v.string(), grant: grantValidator, newHash: v.string(), chainId: v.string() },
  handler: async (ctx, args): Promise<Granted> => {
    const at = now();
    const { deviceId, grant } = args;
    switch (grant.kind) {
      case "demo": {
        const perDevice = await rateLimits.limit(ctx, "tokenDemoDevice", { key: deviceId });
        if (!perDevice.ok) fail("rate_limited", "Too many demo token requests; try again later.", { retryAfterMs: perDevice.retryAfter });
        const global = await rateLimits.limit(ctx, "tokenDemoGlobal", {});
        if (!global.ok) fail("rate_limited", "Too many demo token requests; try again later.", { retryAfterMs: global.retryAfter });
        break;
      }
      case "refresh": {
        const perDevice = await rateLimits.limit(ctx, "tokenRefresh", { key: deviceId });
        if (!perDevice.ok) fail("rate_limited", "Too many refresh requests; try again later.", { retryAfterMs: perDevice.retryAfter });
        break;
      }
      case "ticket": {
        const perDevice = await rateLimits.limit(ctx, "tokenTicket", { key: deviceId });
        if (!perDevice.ok) fail("rate_limited", "Too many token requests; try again later.", { retryAfterMs: perDevice.retryAfter });
        break;
      }
      case "v2": {
        const perDevice = await rateLimits.limit(ctx, "tokenV2", { key: deviceId });
        if (!perDevice.ok) fail("rate_limited", "Too many token requests; try again later.", { retryAfterMs: perDevice.retryAfter });
        break;
      }
    }
    const store = (chainId: string, accountRow: Id<"accounts"> | null) =>
      ctx.db.insert("refreshTokens", { hash: args.newHash, deviceId, chainId, ...(accountRow ? { accountId: accountRow } : {}), usedAt: null, createdAt: at });
    // An install keeps one refresh token: a new chain (proof, trade-in, demo) ends its older ones,
    // so a token it lost or leaked stops working.
    const startChain = async (accountRow: Id<"accounts"> | null) => {
      await revokeDevice(ctx, deviceId);
      await store(args.chainId, accountRow);
    };

    switch (grant.kind) {
      case "refresh": {
        const invalid = () => ({ refused: "refresh_invalid" as const });
        const row = await ctx.db
          .query("refreshTokens")
          .withIndex("by_hash", (q) => q.eq("hash", grant.hash))
          .unique();
        if (!row || row.deviceId !== deviceId) return invalid();
        if (row.usedAt !== null) {
          // The answer to the last refresh may have been lost (a dropped socket, the app killed,
          // a phone offline for days): the same install may ask again with the token it still
          // holds while the token that replaced it is unused. That one goes, and the chain
          // carries on. No time limit: every refresh is signed by the device key too, so the
          // token alone is no use to anyone else.
          const later = (
            await ctx.db
              .query("refreshTokens")
              .withIndex("by_chain", (q) => q.eq("chainId", row.chainId))
              .collect()
          ).filter((r) => r._creationTime > row._creationTime);
          if (later.length === 1 && later[0].usedAt === null) {
            const account = row.accountId === undefined ? null : await ctx.db.get(row.accountId);
            const device = await deviceRow(ctx, deviceId);
            if (!device || (row.accountId !== undefined && (!account || device.accountId !== account._id))) {
              await deleteChain(ctx, row.chainId);
              return { refused: "device_removed" };
            }
            await ctx.db.delete(later[0]._id);
            if (account) await linkPhone(ctx, account, device);
            else await ctx.db.patch(device._id, { demoExpiresAt: at + DEMO_INACTIVE_SECS });
            await store(row.chainId, account?._id ?? null);
            return account ? { regno: account.regno, account: account._id, demo: false } : { regno: DEMO_REGNO, account: "demo", demo: true };
          }
          await deleteChain(ctx, row.chainId);
          // Unlink too so any live 24-hour JWT stops working at once and OTPs stop going here.
          const leaked = await deviceRow(ctx, deviceId);
          if (leaked) await unlinkPhone(ctx, leaked);
          console.warn("auth: a replaced refresh token came back; its chain is revoked");
          return { refused: "refresh_reused" };
        }
        const device = await deviceRow(ctx, deviceId);
        if (!device) {
          await deleteChain(ctx, row.chainId);
          return { refused: "device_removed" };
        }
        await dropUsedBefore(ctx, row);
        if (row.accountId === undefined) {
          await ctx.db.patch(row._id, { usedAt: at });
          await ctx.db.patch(device._id, { demoExpiresAt: at + DEMO_INACTIVE_SECS });
          await store(row.chainId, null);
          return { regno: DEMO_REGNO, account: "demo", demo: true };
        }
        const account = await ctx.db.get(row.accountId);
        if (!account || device.accountId !== account._id) {
          await deleteChain(ctx, row.chainId);
          return { refused: "device_removed" };
        }
        await ctx.db.patch(row._id, { usedAt: at });
        await linkPhone(ctx, account, device);
        await store(row.chainId, account._id);
        return { regno: account.regno, account: account._id, demo: false };
      }
      case "ticket": {
        const row = /^[0-9a-f]{32}$/.test(grant.ticket)
          ? await ctx.db
              .query("proofTickets")
              .withIndex("by_ticket", (q) => q.eq("ticket", grant.ticket))
              .unique()
          : null;
        if (!row || row.deviceId !== deviceId || row.expiresAt <= at) return { refused: "challenge_invalid" };
        if (row.status === "open" || row.status === "taken") return { refused: "proof_pending" };
        if (row.status === "failed" || !row.regno) {
          await ctx.db.delete(row._id);
          return { proofFailed: row.code ?? "proof_failed" };
        }
        // A proved ticket stays until it expires: on a weak link this call's answer may be lost,
        // and the install trades the ticket in again instead of proving again. Only the install
        // it was issued to can (every call is signed by its key), and each trade starts a chain
        // that ends the last one.
        const device = await upsertDevice(ctx, deviceId, args.signKey, grant.device);
        const account = await ensureAccount(ctx, row.regno, row.ownerHash ?? null);
        await linkPhone(ctx, account, device);
        await startChain(account._id);
        return { regno: account.regno, account: account._id, demo: false };
      }
      case "v2": {
        const live = await ctx.db
          .query("v2Tokens")
          .withIndex("by_device_iat", (q) => q.eq("deviceId", deviceId).eq("iat", grant.iat))
          .unique();
        const account = live && (await ctx.db.get(live.accountId));
        if (!live || !account || account.regno !== grant.regno) return { refused: "v2_not_live" };
        if (live.usedAt !== null) return { refused: "v2_used" };
        await ctx.db.patch(live._id, { usedAt: at });
        const device = await upsertDevice(ctx, deviceId, args.signKey, grant.device);
        if (account.ownerHash !== null) await claimMailbox(ctx, account._id, account.ownerHash);
        await linkPhone(ctx, account, device);
        await startChain(account._id);
        return { regno: account.regno, account: account._id, demo: false };
      }
      case "demo": {
        // The demo is no account: an install that was a student's phone leaves it.
        const device = await upsertDevice(ctx, deviceId, args.signKey, grant.device);
        await unlinkPhone(ctx, device);
        await ctx.db.patch(device._id, { demoExpiresAt: at + DEMO_INACTIVE_SECS });
        await startChain(null);
        return { regno: DEMO_REGNO, account: "demo", demo: true };
      }
    }
  },
});

export const signOut = mutation({
  args: { flow: v.string(), everywhere: v.optional(v.boolean()) },
  handler: async (ctx, { flow, everywhere }) => {
    const me = await caller(ctx);
    // Everywhere: the other phones also come off the account (and its forwarding address), so
    // their JWTs stop at once, not when they expire.
    if (everywhere && me.account) {
      for (const phone of await phoneRows(ctx, me.account._id)) {
        if (phone.deviceId !== me.deviceId) await unlinkPhone(ctx, phone);
      }
    }
    // This phone leaves the account too: its JWT stops now, and it no longer gets the OTPs.
    if (me.account) await unlinkPhone(ctx, me.device);
    else await revokeDevice(ctx, me.deviceId);
    log({ flow, regno: me.regno }, everywhere ? "auth: signed out everywhere" : "auth: signed out");
    return null;
  },
});
