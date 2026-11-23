// OAuth for MCP clients (MCP authorization, 2025-06-18), ported from
// edge/src/oauth.ts: dynamic registration, an authorize page that shows a
// pairing code the app approves with `keys:create`, and `/token` swapping the
// one-time code and its PKCE verifier for the access key. These are the
// stored halves; http.ts has the routes. Codes, poll ids and verifiers are
// kept as SHA-256 only. Random values come from the HTTP actions.

import { v } from "convex/values";
import type { Doc } from "./_generated/dataModel";
import { type QueryCtx, internalMutation, internalQuery } from "./_generated/server";
import { toBase64Url } from "./lib/base64";
import { hashesEqual, sha256 } from "./lib/crypto";

/** How long the authorize page waits for the app. */
export const REQUEST_TTL_SECS = 600;
/** How long a code from `/authorize` may be swapped at `/token`. */
export const GRANT_TTL_SECS = 60;
export const MAX_CLIENTS = 2000;

export function hostOf(uri: string): string {
  try {
    const url = new URL(uri);
    return url.host || `${url.protocol}//`;
  } catch {
    return uri;
  }
}

function isLoopback(url: URL): boolean {
  return url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
}

/** A redirect URI a client may register: https, http on a loopback address (RFC 8252), or an
 * app's own scheme; never with a fragment. */
export function redirectUriOk(uri: string): boolean {
  if (uri.length > 512) return false;
  let url: URL;
  try {
    url = new URL(uri);
  } catch {
    return false;
  }
  if (url.hash !== "" || uri.includes("#")) return false;
  if (url.protocol === "https:") return url.hostname !== "";
  if (url.protocol === "http:") return isLoopback(url);
  // Private-use schemes (`cursor://…`, `com.example.app:/cb`); not ones a browser runs or reads
  // files with.
  const scheme = url.protocol.slice(0, -1);
  return /^[a-z][a-z0-9+.-]*$/.test(scheme) && !["javascript", "data", "file", "blob", "vbscript", "about", "ws", "wss", "ftp"].includes(scheme);
}

/** Whether `asked` is one of the registered URIs: exactly, except that a loopback URI may use
 * any port (RFC 8252 §7.3). */
export function redirectMatches(registered: string[], asked: string): boolean {
  if (registered.includes(asked)) return true;
  let url: URL;
  try {
    url = new URL(asked);
  } catch {
    return false;
  }
  if (!isLoopback(url)) return false;
  url.port = "";
  return registered.some((uri) => {
    try {
      const known = new URL(uri);
      if (!isLoopback(known)) return false;
      known.port = "";
      return known.href === url.href;
    } catch {
      return false;
    }
  });
}

/** base64url(SHA-256(verifier)), as PKCE S256 wants it. */
export async function s256(verifier: string): Promise<string> {
  return toBase64Url(await sha256(verifier));
}

/** The client behind a pairing code still waiting for the app; null when the code is not one of
 * these (it may be the extension's). */
export async function pendingApp(ctx: QueryCtx, codeHash: string, at: number): Promise<{ name: string; host: string; row: Doc<"oauthRequests"> } | null> {
  const row = await ctx.db
    .query("oauthRequests")
    .withIndex("by_codeHash", (q) => q.eq("codeHash", codeHash))
    .unique();
  if (!row || row.expiresAt <= at || row.accessKey !== null) return null;
  const client = await clientRow(ctx, row.clientId);
  return client ? { name: client.name, host: hostOf(row.redirectUri), row } : null;
}

async function clientRow(ctx: QueryCtx, clientId: string) {
  return ctx.db
    .query("oauthClients")
    .withIndex("by_clientId", (q) => q.eq("clientId", clientId))
    .unique();
}

/** Stores a client, dropping the one unused longest when full. */
export const register = internalMutation({
  args: { clientId: v.string(), name: v.string(), redirectUris: v.array(v.string()), at: v.number() },
  handler: async (ctx, { clientId, name, redirectUris, at }) => {
    const oldest = await ctx.db.query("oauthClients").withIndex("by_usedAt").order("asc").take(MAX_CLIENTS + 1);
    for (const extra of oldest.slice(0, Math.max(0, oldest.length - MAX_CLIENTS + 1))) await ctx.db.delete(extra._id);
    await ctx.db.insert("oauthClients", { clientId, name, redirectUris, createdAt: at, usedAt: at });
  },
});

export const client = internalQuery({
  args: { clientId: v.string() },
  handler: async (ctx, { clientId }) => {
    const row = await clientRow(ctx, clientId);
    return row && { id: row.clientId, name: row.name, redirectUris: row.redirectUris };
  },
});

/** A request from `/authorize` (checked already) that waits for the app. */
export const open = internalMutation({
  args: { clientId: v.string(), redirectUri: v.string(), challenge: v.string(), state: v.union(v.string(), v.null()), codeHash: v.string(), pollHash: v.string(), at: v.number() },
  handler: async (ctx, args) => {
    const row = await clientRow(ctx, args.clientId);
    if (row) await ctx.db.patch(row._id, { usedAt: args.at });
    await ctx.db.insert("oauthRequests", {
      codeHash: args.codeHash,
      pollHash: args.pollHash,
      clientId: args.clientId,
      redirectUri: args.redirectUri,
      challenge: args.challenge,
      state: args.state,
      expiresAt: args.at + REQUEST_TTL_SECS,
      accessKey: null,
    });
  },
});


/** The authorize page's poll: once approved, the request becomes a code for the client (its hash
 * `grantHash`) and the page is told where to go. */
export const poll = internalMutation({
  args: { pollHash: v.string(), grantHash: v.string(), at: v.number() },
  handler: async (ctx, { pollHash, grantHash, at }) => {
    const row = await ctx.db
      .query("oauthRequests")
      .withIndex("by_pollHash", (q) => q.eq("pollHash", pollHash))
      .unique();
    if (!row || row.expiresAt <= at) return { status: "expired" as const };
    if (row.accessKey === null || row.accountId === undefined) return { status: "pending" as const };
    await ctx.db.delete(row._id);
    await ctx.db.insert("oauthCodes", {
      codeHash: grantHash,
      clientId: row.clientId,
      redirectUri: row.redirectUri,
      challenge: row.challenge,
      accessKey: row.accessKey,
      expiresAt: at + GRANT_TTL_SECS,
      accountId: row.accountId,
    });
    return { status: "ready" as const, redirectUri: row.redirectUri, state: row.state };
  },
});

/** Swaps a code for its key, once: the code is gone after any try. */
export const redeem = internalMutation({
  args: { codeHash: v.string(), clientId: v.string(), redirectUri: v.string(), verifierS256: v.string(), at: v.number() },
  handler: async (ctx, { codeHash, clientId, redirectUri, verifierS256, at }) => {
    const row = await ctx.db
      .query("oauthCodes")
      .withIndex("by_codeHash", (q) => q.eq("codeHash", codeHash))
      .unique();
    if (row) await ctx.db.delete(row._id);
    const bad = { error: "invalid_grant", description: "The code is unknown, used or expired." };
    if (!row || row.expiresAt <= at) return bad;
    if (row.clientId !== clientId) return bad;
    if (row.redirectUri !== redirectUri) return { error: "invalid_grant", description: "redirect_uri does not match the authorization request." };
    if (!hashesEqual(verifierS256, row.challenge)) return { error: "invalid_grant", description: "code_verifier does not match the code_challenge." };
    return { key: row.accessKey };
  },
});
