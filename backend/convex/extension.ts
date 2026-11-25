// The browser extension's live view over the socket: its key, a session request's progress and a
// pairing code's approval. (It asks GitHub for the newest build itself.) The extension signs in with a key JWT
// from `POST /v1/key/token`. The VTOP session and new keys still go out over HTTP, once
// (`GET /v1/session/requests/{id}`, `POST /v1/pair`), so they never sit in a query's results.
// The HTTP routes stay for older builds and the MCP.

import { v } from "convex/values";
import type { Doc } from "./_generated/dataModel";
import { internalMutation, query, type QueryCtx } from "./_generated/server";
import { rateLimits } from "./lib/rateLimits";
import { requestOf, requestRow } from "./lib/requests";
import { now } from "./lib/util";
import { phaseOf } from "./logins";
import { type KeyHolder, keyAccount, keyView } from "./sessions";

/** The key behind the caller's key JWT; null for a phone's JWT, none, or a revoked key. */
async function keyCaller(ctx: QueryCtx): Promise<KeyHolder | null> {
  const identity = await ctx.auth.getUserIdentity();
  if (identity?.kind !== "key") return null;
  return keyAccount(ctx, identity.subject);
}

/** Checks a key before `POST /v1/key/token` signs a JWT for it: null for no such key. Not a use
 * of the key, so opening the popup doesn't show up in Connected apps. */
export const tokenFor = internalMutation({
  args: { keyHash: v.string() },
  handler: async (ctx, { keyHash }) => {
    const found = await keyAccount(ctx, keyHash);
    if (!found) return null;
    const limited = !(await rateLimits.limit(ctx, "keyToken", { key: found.account._id })).ok;
    return { limited };
  },
});

/** The key's account and how a session will be made; null once the key is revoked, so an open
 * popup shows it at once. */
export const key = query({
  args: {},
  handler: async (ctx) => {
    const found = await keyCaller(ctx);
    return found && (await keyView(ctx, found));
  },
});

/** What the server's sign-in for a request is doing, from its `logins` row. */
async function loginStep(ctx: QueryCtx, row: Doc<"sessionRequests">) {
  const login = (
    await ctx.db
      .query("logins")
      .withIndex("by_account", (q) => q.eq("accountId", row.accountId))
      .collect()
  ).find((l) => l.requestId === row.requestId);
  if (!login) return null;
  const phase = phaseOf(login);
  return phase.kind === "starting" ? ("signing_in" as const) : phase.kind;
}

/** A session request's progress, never its cookies: the extension fetches those once over HTTP
 * when this says `ready`. Times in ms. `expiresAt` is the extension's to enforce: a query doesn't
 * run again as time passes. */
export const request = query({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const found = await keyCaller(ctx);
    if (!found) return { status: "key_unknown" as const };
    const row = await requestRow(ctx, requestId);
    if (!row || row.accountId !== found.account._id) return { status: "not_found" as const };
    const r = requestOf(row);
    const expiresAt = r.expiresAt * 1000;
    const phase = r.phaseAt(now());
    switch (phase.kind) {
      case "success":
        return { status: "ready" as const, from: phase.source, expiresAt };
      case "error":
        return { status: "error" as const, code: phase.error.code, error: phase.error.message, expiresAt };
      case "expired":
      case "scrubbed":
        return { status: "not_found" as const };
      case "pending":
        return {
          status: "pending" as const,
          stage: phase.stage.kind,
          serverDueAt: phase.stage.kind === "phone" && phase.stage.vaultDueAt !== null ? phase.stage.vaultDueAt * 1000 : null,
          pushSent: phase.stage.kind === "phone" || phase.stage.kind === "server" ? phase.stage.pushSent : false,
          login: phase.stage.kind === "server" ? await loginStep(ctx, row) : null,
          expiresAt,
        };
    }
  },
});

/** Whether the app approved the pairing code with this SHA-256 (of the code without its dash, as
 * `keys:create` hashes it). No sign-in: only the extension knows the code. Never the key. */
export const pairing = query({
  args: { codeHash: v.string() },
  handler: async (ctx, { codeHash }) => {
    if (!/^[0-9a-f]{64}$/.test(codeHash)) return { ready: false };
    const row = await ctx.db
      .query("pairings")
      .withIndex("by_codeHash", (q) => q.eq("codeHash", codeHash))
      .first();
    return { ready: row !== null && row.expiresAt > now() };
  },
});
