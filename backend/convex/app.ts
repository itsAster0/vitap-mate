// A linked phone's view of its account and a sign-in by the server
// (`/v1/app/*` of the edge). The account is the JWT's, so no registration
// number is sent.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import type { Id } from "./_generated/dataModel";
import { type ActionCtx, type QueryCtx, action, internalQuery, mutation, query } from "./_generated/server";
import { keysOf, phonesOf, saveRecord, signInsOf } from "./lib/accounts";
import { errorCode } from "./lib/accountState";
import { fail } from "./lib/errors";
import { actionCaller, caller, realCaller, recordOf } from "./lib/identity";
import { address } from "./lib/mailboxes";
import { log, now } from "./lib/util";
import { requestOf, requestRow } from "./lib/requests";
import { startLogin } from "./logins";
import { cachedCookies } from "./sessions";

/** The app's `SessionState`: without the CSRF token it would not count the session as signed in. */
type Session = { cookies: string; csrf_token: string | null; registration_number: string };

/** Where a server sign-in for the phone stands. */
export type LoginAnswer = { status: "ok"; session: Session } | { status: "pending"; requestId: string } | { status: "error"; code: string; error: string };

const signedIn = (regno: string, cookies: string, csrfToken: string | null): LoginAnswer => ({
  status: "ok",
  session: { cookies, csrf_token: csrfToken, registration_number: regno },
});

/** What the server keeps and its recent activity. */
export const status = query({
  args: { flow: v.optional(v.string()) },
  handler: async (ctx) => {
    const me = await caller(ctx);
    if (!me.account) {
      return {
        sessionSavedAt: null,
        sessionSource: null,
        vaultSavedAt: null,
        vaultExpiresAt: null,
        vaultGmail: false,
        credentialsRejectedAt: null,
        forwarding: null,
        signIns: [],
        phones: 1,
        keys: [],
      };
    }
    const doc = await recordOf(ctx, me.account);
    const id = me.account._id;
    const forwarding = doc.mailboxId === null ? null : address(doc.mailboxId);
    return doc.statusView(forwarding, (await phonesOf(ctx, id)).length, await keysOf(ctx, id), await signInsOf(ctx, id));
  },
});

/** Drops the account's cached VTOP session. */
export const sessionClear = mutation({
  args: { flow: v.string() },
  handler: async (ctx, { flow }) => {
    const me = await realCaller(ctx);
    const doc = await recordOf(ctx, me.account);
    doc.clearSession();
    await saveRecord(ctx, me.account._id, doc);
    log({ flow, regno: me.regno }, "account: cached session cleared");
    return { ok: true };
  },
});

export const loginState = internalQuery({
  args: { accountId: v.id("accounts") },
  handler: async (ctx, { accountId }) => {
    const account = await ctx.db.get(accountId);
    if (!account) return null;
    const doc = await recordOf(ctx, account);
    const at = now();
    return { regno: doc.id, vault: doc.usableVault(at), cached: doc.cachedSession(at) };
  },
});

/** Where the account's sign-in request `requestId` stands. */
async function loginAnswer(ctx: QueryCtx, accountId: Id<"accounts">, requestId: string): Promise<LoginAnswer> {
  const row = await requestRow(ctx, requestId);
  const account = await ctx.db.get(accountId);
  if (!row || !account || row.accountId !== accountId) return { status: "error", code: "request_not_found", error: "Unknown sign-in." };
  const request = requestOf(row);
  switch (request.status) {
    case "success":
      return signedIn(account.regno, request.cookies ?? "", request.csrfToken);
    case "error":
      if (errorCode(request.error ?? "") === "credentials_rejected") return { status: "error", code: "invalid_credentials", error: "VTOP rejected the saved password." };
      return { status: "error", code: "server_login_failed", error: request.error ?? "The server could not sign in." };
    default:
      if (request.pending(now())) return { status: "pending", requestId };
      return { status: "error", code: "vtop_timeout", error: "The server did not sign in in time." };
  }
}

export const loginRequest = internalQuery({
  args: { accountId: v.id("accounts"), requestId: v.string() },
  handler: (ctx, { accountId, requestId }) => loginAnswer(ctx, accountId, requestId),
});

/** A server sign-in `app:login` left pending, live: it waits for VTOP's OTP mail. */
export const loginResult = query({
  args: { flow: v.optional(v.string()), requestId: v.string() },
  handler: async (ctx, { requestId }): Promise<LoginAnswer> => {
    const me = await caller(ctx);
    if (!me.account) return { status: "error", code: "demo_unavailable", error: "The demo account can't do this." };
    return loginAnswer(ctx, me.account._id, requestId);
  },
});

/** Has the server sign the caller in with the saved password, the engine reading the OTP from
 * Gmail at once or the OTP mail arriving later, so the phone sends nothing secret. Answers once
 * the first step is done: signed in, or pending while the OTP mail is awaited (followed with
 * `app:loginResult`). A rejected password is `invalid_credentials`, and the saved one is dropped
 * (`sessions.loginDone`). Throws only when no sign-in can start. */
export async function loginFor(ctx: ActionCtx, flow: string): Promise<LoginAnswer> {
  const me = await actionCaller(ctx);
  if (!me.account) return fail("demo_unavailable", "The demo account can't do this.");
  const id = me.account._id;
  const known = await ctx.runQuery(internal.app.loginState, { accountId: id });
  if (!known?.vault) return fail("no_saved_credentials", "The server keeps no password for this account.");
  const cached = await cachedCookies(ctx, id, known.regno, known.cached, flow);
  if (cached) return signedIn(known.regno, cached.cookies, cached.csrfToken);
  const opened = await ctx.runMutation(internal.sessions.open, { accountId: id, keyHash: null, requestId: crypto.randomUUID() });
  log({ flow, regno: known.regno }, opened.opened ? "account: a server sign-in for the phone" : "account: joined a server sign-in");
  if (opened.opened) await startLogin(ctx, known.regno, known.vault, opened.requestId, flow, true);
  return ctx.runQuery(internal.app.loginRequest, { accountId: id, requestId: opened.requestId });
}

/** `loginFor` for the phone: a failed sign-in throws its code. */
export const login = action({
  args: { flow: v.string() },
  handler: async (ctx, { flow }): Promise<LoginAnswer> => {
    const answer = await loginFor(ctx, flow);
    return answer.status === "error" ? fail(answer.code, answer.error) : answer;
  },
});
