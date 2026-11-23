// Access keys for the extension and MCP clients (`/v1/keys*` of the edge).
// A key is made by a linked phone, so it follows from the same proof; only
// its SHA-256 is kept, and the phone that made it is the one asked for its
// sessions.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import { action, internalMutation, mutation } from "./_generated/server";
import { keyRows, phoneRows, phonesOf, keyOf, unlinkPhone } from "./lib/accounts";
import { KEY_PREFIX, MAX_KEYS_PER_ACCOUNT, keyId, keyLabel, keysToMove } from "./lib/accountState";
import { randomHex, sha256Hex } from "./lib/crypto";
import { badRequest, fail } from "./lib/errors";
import { actionCaller, realCaller } from "./lib/identity";
import { PAIR_TTL_SECS, pairCode } from "./lib/pairing";
import { pendingApp } from "./oauth";
import { env, log, now } from "./lib/util";

/** Where MCP clients connect. */
export function mcpUrl(): string {
  return `${env("PUBLIC_URL")}/mcp`;
}

type Created = { id: string; key: string; mcpUrl: string; app: { name: string; host: string } | null };

/** Makes a key, shown once. With `pairCode` the extension that showed the code collects it
 * (`/v1/pair`), or an MCP client's authorize page gets it, named after the client. An action:
 * the key comes from real randomness. */
export const create = action({
  args: { flow: v.string(), label: v.optional(v.string()), outings: v.optional(v.boolean()), pairCode: v.optional(v.string()) },
  handler: async (ctx, args): Promise<Created> => {
    const me = await actionCaller(ctx);
    if (!me.account) return fail("demo_unavailable", "The demo account can't do this.");
    const pair = args.pairCode === undefined ? undefined : pairCode(args.pairCode);
    if (pair === null) return badRequest("pairCode must be a pairing code.");
    const key = KEY_PREFIX + randomHex(32);
    const created: Created = await ctx.runMutation(internal.keys.store, {
      deviceId: me.deviceId,
      key,
      keyHash: await sha256Hex(key),
      codeHash: pair === undefined ? null : await sha256Hex(pair),
      label: args.label ?? null,
      outings: args.outings === true,
      mcpUrl: mcpUrl(),
    });
    log({ flow: args.flow, regno: me.regno }, created.app ? "keys: made a key for an MCP client" : "keys: made a key");
    return created;
  },
});

export const store = internalMutation({
  args: {
    deviceId: v.string(),
    key: v.string(),
    keyHash: v.string(),
    codeHash: v.union(v.string(), v.null()),
    label: v.union(v.string(), v.null()),
    outings: v.boolean(),
    mcpUrl: v.string(),
  },
  handler: async (ctx, args): Promise<Created> => {
    const me = await realCaller(ctx);
    const accountId = me.account._id;
    const at = now();
    const app = args.codeHash === null ? null : await pendingApp(ctx, args.codeHash, at);
    if ((await keyRows(ctx, accountId)).length >= MAX_KEYS_PER_ACCOUNT) {
      fail("key_limit", "This account already has the most keys allowed; revoke one first.");
    }
    if (app !== null) {
      const request = app.row;
      if (request.expiresAt <= at || request.accessKey !== null) fail("pair_expired", "That code has expired; start connecting again.");
      await ctx.db.patch(request._id, { accessKey: args.key, accountId });
    } else if (args.codeHash !== null) {
      for (const old of await ctx.db
        .query("pairings")
        .withIndex("by_codeHash", (q) => q.eq("codeHash", args.codeHash!))
        .collect()) {
        await ctx.db.delete(old._id);
      }
      await ctx.db.insert("pairings", { codeHash: args.codeHash, key: args.key, expiresAt: at + PAIR_TTL_SECS, accountId });
    }
    await ctx.db.insert("accessKeys", {
      keyHash: args.keyHash,
      accountId,
      label: keyLabel(app?.name ?? args.label ?? undefined),
      createdAt: at,
      lastUsedAt: null,
      deviceId: me.deviceId,
      outings: args.outings,
      uses: [],
    });
    return { id: keyId(args.keyHash), key: args.key, mcpUrl: args.mcpUrl, app: app ? { name: app.name, host: app.host } : null };
  },
});

/** The account's key shown as `id` (its hash's first 8 hex digits). */
async function keyById(ctx: Parameters<typeof keyRows>[0], accountId: Parameters<typeof keyRows>[1], id: string) {
  const key = (await keyRows(ctx, accountId)).find((k) => keyId(k.keyHash) === id.trim());
  return key ?? fail("key_not_found", "No such key on this account.");
}

export const revoke = mutation({
  args: { flow: v.string(), id: v.string() },
  handler: async (ctx, { flow, id }) => {
    const me = await realCaller(ctx);
    await ctx.db.delete((await keyById(ctx, me.account._id, id))._id);
    log({ flow, regno: me.regno }, "keys: revoked a key");
    return null;
  },
});

/** Lets a key apply for and cancel outings, or not. */
export const setOutings = mutation({
  args: { flow: v.string(), id: v.string(), outings: v.boolean() },
  handler: async (ctx, { flow, id, outings }) => {
    const me = await realCaller(ctx);
    await ctx.db.patch((await keyById(ctx, me.account._id, id))._id, { outings });
    log({ flow, regno: me.regno }, "keys: changed a key's outings");
    return null;
  },
});

/** This phone now answers for the keys of phone `from` (which is unlinked) and of phones no
 * longer linked; without `from` only the latter. */
export const move = mutation({
  args: { flow: v.string(), from: v.optional(v.string()) },
  handler: async (ctx, { flow, from }) => {
    const me = await realCaller(ctx);
    const accountId = me.account._id;
    const keys = await keyRows(ctx, accountId);
    const plan = keysToMove(keys.map(keyOf), await phonesOf(ctx, accountId), from === undefined ? null : from, me.deviceId);
    if (plan === null) return fail("phone_not_found", "No such phone on this account.");
    const moving = new Set(plan.moving.map((k) => k.hash));
    for (const key of keys) if (moving.has(key.keyHash)) await ctx.db.patch(key._id, { deviceId: me.deviceId });
    if (plan.source) {
      const source = (await phoneRows(ctx, accountId)).find((row) => row.deviceId === plan.source!.deviceId);
      if (source) await unlinkPhone(ctx, source);
    }
    log({ flow, regno: me.regno }, "keys: moved keys");
    return { moved: moving.size };
  },
});
