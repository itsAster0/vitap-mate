// The extension collects the key the app made for its pairing code, once.

import { v } from "convex/values";
import { internalMutation, query } from "./_generated/server";
import { sha256Hex } from "./lib/crypto";
import { badRequest } from "./lib/errors";
import { realCaller } from "./lib/identity";
import { pairCode } from "./lib/pairing";
import { now } from "./lib/util";
import { pendingApp } from "./oauth";

export const take = internalMutation({
  args: { codeHash: v.string() },
  handler: async (ctx, { codeHash }) => {
    const row = await ctx.db
      .query("pairings")
      .withIndex("by_codeHash", (q) => q.eq("codeHash", codeHash))
      .first();
    if (!row) return null;
    await ctx.db.delete(row._id);
    return row.expiresAt > now() ? row.key : null;
  },
});

/** Whose pairing code it is, before the app approves it: an MCP app waiting on its OAuth sign-in,
 * or else the browser extension. The phone asks over its socket, with its JWT. */
export const info = query({
  args: { code: v.string() },
  handler: async (ctx, { code }) => {
    await realCaller(ctx);
    const pair = pairCode(code);
    if (pair === null) return badRequest("code must be a pairing code.");
    const app = await pendingApp(ctx, await sha256Hex(pair), now());
    return app ? { kind: "mcp" as const, name: app.name, host: app.host } : { kind: "browser" as const };
  },
});
