// VTOP sessions for key holders (the extension and MCP clients). A request is
// answered from the cache (checked with VTOP by the engine), or left pending
// while the phone that made the key is asked (it sees the request in
// `sessions:pending`, woken by a push) and/or the engine signs in with saved
// credentials, in the order the student's settings choose. The phone answers
// with `sessions:answer`; the engine's sign-in (logins.ts) ends in `loginDone`.
// Expiry transitions and delayed server fallbacks run via scheduled mutations
// and crons. Never log cookies, passwords, keys or tokens.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import { rateLimits } from "./lib/rateLimits";
import type { Doc, Id } from "./_generated/dataModel";
import { type ActionCtx, type MutationCtx, type QueryCtx, action, internalAction, internalMutation, internalQuery, query } from "./_generated/server";
import { accountByRegno, keyOf, phonesOf, recordSignIn, saveRecord } from "./lib/accounts";
import {
  type AccountRecord,
  type KeyVia,
  type Plan,
  RequestRecord,
  type SignInRecord,
  keyUse,
  phonesForKey,
  plan,
  toHeader,
  validCookies,
} from "./lib/accountState";
import { type Credentials, credentialsOf, credentialsValidator, usableText } from "./lib/credentials";
import { engineJson } from "./lib/engine";
import { type LoginOutcome, SEAL, SESSION, type SealVault, type SealedVault, type SessionCheck, type SessionChecked } from "./lib/engineApi";
import { fail } from "./lib/errors";
import { actionCaller, caller, mailboxOfAccount, recordOf } from "./lib/identity";
import { notify } from "./lib/notify";
import { askPhone, insertRequest, pendingRequest, putRequest, requestOf, requestRow } from "./lib/requests";
import { accountId, log, logError, logWarn, now } from "./lib/util";
import { startLogin } from "./logins";

const PHONE_SESSION_INVALID =
  "phone_session_invalid: VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.";
const ACCOUNT_MISMATCH = "account_mismatch: Your phone is signed in to VTOP as a different student.";

// ---- key holders --------------------------------------------------------------------------

export type KeyHolder = { key: Doc<"accessKeys">; account: Doc<"accounts"> };

/** The key's row and its account; null for no such key. */
export async function keyAccount(ctx: QueryCtx, keyHash: string): Promise<KeyHolder | null> {
  const key = await ctx.db
    .query("accessKeys")
    .withIndex("by_keyHash", (q) => q.eq("keyHash", keyHash))
    .unique();
  const account = key && (await ctx.db.get(key.accountId));
  return key && account ? { key, account } : null;
}

/** How a session for this key will be made: `GET /v1/whoami`, and the extension's live
 * `extension:key`. `phone` is the key's own phone, the only one its requests go to. */
export async function keyView(ctx: QueryCtx, { key, account }: KeyHolder) {
  const doc = await recordOf(ctx, account);
  const at = now();
  const vault = doc.usableVault(at);
  const device = await ctx.db
    .query("devices")
    .withIndex("by_deviceId", (q) => q.eq("deviceId", key.deviceId))
    .unique();
  const linked = device?.accountId === account._id;
  return {
    registrationNumber: doc.id,
    keyLabel: key.label,
    keyCreatedAt: key.createdAt,
    keyLastUsedAt: key.lastUsedAt,
    keyOutings: key.outings,
    semesterId: doc.semesterId,
    settings: doc.settings,
    phoneLinked: linked,
    phone: linked ? { label: device.label, notifications: device.fcmToken !== "" } : null,
    savedCredentials: vault !== null,
    vaultSavedAt: vault !== null ? doc.vaultSavedAt : null,
    credentialsRejectedAt: doc.credentialsRejectedAt,
    cachedSession: doc.cachedSession(at) !== null,
  };
}

/** `GET /v1/whoami`: read-only. Null for no such key. */
export const whoami = internalQuery({
  args: { keyHash: v.string() },
  handler: async (ctx, { keyHash }) => {
    const found = await keyAccount(ctx, keyHash);
    return found && (await keyView(ctx, found));
  },
});

/** The account id of a key; null for no such key. */
export const keyHolder = internalQuery({
  args: { keyHash: v.string() },
  handler: async (ctx, { keyHash }) => (await keyAccount(ctx, keyHash))?.account._id ?? null,
});

/** Counts a session call (`limited` past `sessionCall`) and records the key's use, then says what
 * the cache holds. Null for no such key. */
export const start = internalMutation({
  args: { keyHash: v.string(), via: v.union(v.literal("extension"), v.literal("mcp")) },
  handler: async (ctx, { keyHash, via }) => {
    const found = await keyAccount(ctx, keyHash);
    if (!found) return null;
    const { key, account } = found;
    if (!(await rateLimits.limit(ctx, "sessionCall", { key: account._id })).ok) return { limited: true as const };
    const doc = await recordOf(ctx, account);
    const at = now();
    const use = keyUse(keyOf(key), at, via as KeyVia);
    if (use) await ctx.db.patch(key._id, use);
    return { limited: false as const, accountId: account._id, regno: doc.id, cached: doc.cachedSession(at), source: doc.cachedSource() };
  },
});

export const clearSession = internalMutation({
  args: { accountId: v.id("accounts") },
  handler: async (ctx, { accountId: id }) => {
    const account = await ctx.db.get(id);
    if (!account) return;
    const doc = await recordOf(ctx, account);
    doc.clearSession();
    await saveRecord(ctx, id, doc);
  },
});

/** The cached session's cookies and CSRF token when the engine finds it still works; one that
 * expired is dropped. */
export async function cachedCookies(
  ctx: ActionCtx,
  id: Id<"accounts">,
  regno: string,
  sealed: string | null,
  flow: string | null = null,
): Promise<{ cookies: string; csrfToken: string | null } | null> {
  if (sealed === null) return null;
  try {
    const checked = await engineJson<SessionChecked>(SESSION, { sealedSession: sealed, account: regno } satisfies SessionCheck, { flow, regno });
    if (checked.status === "expired") {
      log({ flow, regno }, "account: cached session no longer works");
      await ctx.runMutation(internal.sessions.clearSession, { accountId: id });
      return null;
    }
    return checked.cookies ? { cookies: checked.cookies, csrfToken: checked.csrfToken ?? null } : null;
  } catch (e) {
    logError({ flow, regno }, "account: cannot check the cache", e);
    return null;
  }
}

/** Joins the account's pending request, or opens a new one (asking the key's phone only; a
 * `app:login` asks none). Recorded before any network call, so a session call that comes in
 * meanwhile joins it instead of starting another. */
export const open = internalMutation({
  args: { accountId: v.id("accounts"), keyHash: v.union(v.string(), v.null()), requestId: v.string() },
  handler: async (ctx, { accountId: id, keyHash, requestId }) => {
    const at = now();
    const joined = await pendingRequest(ctx, id, at);
    if (joined) return { joined: joined.request.startedView(), requestId: joined.request.id };
    const account = await ctx.db.get(id);
    if (!account) return fail("not_linked", "This account no longer exists.");
    const doc = await recordOf(ctx, account);
    const request = new RequestRecord(requestId, at);
    if (keyHash !== null) {
      const key = await ctx.db
        .query("accessKeys")
        .withIndex("by_keyHash", (q) => q.eq("keyHash", keyHash))
        .unique();
      request.phones = phonesForKey(key && key.accountId === id ? keyOf(key) : undefined, await phonesOf(ctx, id));
    }
    await insertRequest(ctx, id, request);
    const vault = doc.usableVault(at);
    return { opened: { plan: plan(doc.settings, vault !== null), vault, regno: doc.id }, requestId };
  },
});

/** Asks the request's phones; failing that, ends the request (`orFail`) or keeps why. With
 * `serverAfter`, the server signs in when the phone is silent that long. True when asked. */
export const ask = internalMutation({
  args: { requestId: v.string(), orFail: v.boolean(), serverAfter: v.optional(v.number()) },
  handler: async (ctx, { requestId, orFail, serverAfter }) => {
    const at = now();
    const row = await requestRow(ctx, requestId);
    const account = row && (await ctx.db.get(row.accountId));
    if (!row || !account) return true;
    const request = requestOf(row);
    if (!request.pending(at)) return true;
    const failure = await askPhone(ctx, await recordOf(ctx, account), request, at);
    let asked = true;
    if (failure === null) {
      if (serverAfter !== undefined) {
        request.vaultDueAt = at + serverAfter;
        await ctx.scheduler.runAt(request.vaultDueAt * 1000, internal.sessions.serverDue, { requestId });
      }
    } else {
      asked = false;
      if (orFail) request.fail(failure);
      else request.phoneError = failure;
    }
    await putRequest(ctx, row, request);
    return asked;
  },
});

/** Ends a pending request with `error`, or why its phone could not be asked. */
export const failRequest = internalMutation({
  args: { requestId: v.string(), error: v.optional(v.string()) },
  handler: async (ctx, { requestId, error }) => {
    const row = await requestRow(ctx, requestId);
    if (!row) return;
    const request = requestOf(row);
    if (!request.pending(now())) return;
    request.fail(error ?? request.phoneError ?? "phone_unreachable: Could not reach the phone.");
    await putRequest(ctx, row, request);
  },
});

export const state = internalQuery({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const row = await requestRow(ctx, requestId);
    return row && { ...row, startedView: requestOf(row).startedView(), pending: requestOf(row).pending(now()) };
  },
});

/** Asks the key's phones and/or starts a sign-in, in the order the settings choose. A sign-in
 * that fails asks the phone instead (`loginDone`). */
export async function begin(ctx: ActionCtx, requestId: string, opened: { plan: Plan; vault: string | null; regno: string }): Promise<void> {
  const { regno, vault } = opened;
  switch (opened.plan.kind) {
    case "vault_first":
      if (vault !== null) return startLogin(ctx, regno, vault, requestId, null, false);
      await ctx.runMutation(internal.sessions.ask, { requestId, orFail: true });
      return;
    case "phone_first":
      if (await ctx.runMutation(internal.sessions.ask, { requestId, orFail: false, serverAfter: opened.plan.waitSecs })) return;
      if (vault !== null) return startLogin(ctx, regno, vault, requestId, null, false);
      await ctx.runMutation(internal.sessions.failRequest, { requestId });
      return;
    case "phone_only":
      await ctx.runMutation(internal.sessions.ask, { requestId, orFail: true });
  }
}

/** A poll of the account's request: the status code and body. */
export const poll = internalQuery({
  args: { accountId: v.id("accounts"), requestId: v.string() },
  handler: async (ctx, { accountId: id, requestId }) => {
    const row = await requestRow(ctx, requestId);
    if (!row || row.accountId !== id) return { status: 404, body: { code: "request_not_found", error: "Unknown session request." } };
    return requestOf(row).pollView(now());
  },
});

// ---- timers ---------------------------------------------------------------------------------

/** The phone stayed silent: sign in with saved credentials. */
export const serverDue = internalAction({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const due = await ctx.runQuery(internal.sessions.dueState, { requestId });
    if (!due) return;
    if (due.vault) await startLogin(ctx, due.regno, due.vault, requestId, null, true);
    else await ctx.runMutation(internal.sessions.clearDue, { requestId });
  },
});

export const dueState = internalQuery({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const row = await requestRow(ctx, requestId);
    if (!row) return null;
    const request = requestOf(row);
    const at = now();
    if (!request.pending(at) || request.vaultStarted || request.vaultDueAt === null || at < request.vaultDueAt) return null;
    const account = await ctx.db.get(row.accountId);
    if (!account) return null;
    return { regno: account.regno, vault: (await recordOf(ctx, account)).usableVault(at) };
  },
});

export const clearDue = internalMutation({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const row = await requestRow(ctx, requestId);
    if (row) await ctx.db.patch(row._id, { vaultDueAt: null });
  },
});

/** Marks a pending request expired at `expiresAt` so subscribed live queries (`app:loginResult`,
 * `sessions:pending`) re-run without altering HTTP `410 request_expired` poll semantics. */
export const expireRequest = internalMutation({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const row = await requestRow(ctx, requestId);
    if (!row || row.status !== "pending" || row.expired || now() < row.expiresAt) return;
    await ctx.db.patch(row._id, { expired: true });
  },
});

// ---- the phone's answer ---------------------------------------------------------------------

/** The session requests waiting on this phone, live: the app's handover prompt. Empty for the
 * demo. */
export const pending = query({
  args: { flow: v.optional(v.string()) },
  handler: async (ctx) => {
    const me = await caller(ctx);
    const account = me.account;
    if (!account) return [];
    const at = now();
    const doc = await recordOf(ctx, account);
    const rows = await ctx.db
      .query("sessionRequests")
      .withIndex("by_account", (q) => q.eq("accountId", account._id))
      .collect();
    return rows
      .map(requestOf)
      .filter((r) => r.pending(at) && r.phoneAsked && r.phones.includes(me.deviceId))
      .map((r) => ({
        requestId: r.id,
        expiresAt: r.expiresAt * 1000,
        wantCredentials: doc.wantCredentials(at),
      }));
  },
});

/** Whether install `deviceId`, one of the account's phones, was asked for request `requestId`;
 * the account's forwarding address when it was. */
export const asked = internalQuery({
  args: { accountId: v.id("accounts"), requestId: v.string(), deviceId: v.string() },
  handler: async (ctx, { accountId: id, requestId, deviceId }) => {
    const row = await requestRow(ctx, requestId);
    const account = await ctx.db.get(id);
    if (!account || !row || row.accountId !== id) return fail("request_not_found", "Unknown session request.");
    // The caller is linked to the account (checked from its JWT), so only the request's list matters.
    if (!row.phones.includes(deviceId)) return fail("not_asked", "This phone was not asked for this session.");
    return { regno: account.regno, forwarding: (await mailboxOfAccount(ctx, id)) !== null };
  },
});

/** The phone's checked session (and sealed credentials) for a request: cached even when the
 * client gave up. True when late. */
export const answered = internalMutation({
  args: {
    requestId: v.string(),
    cookies: v.string(),
    sealedSession: v.optional(v.string()),
    vault: v.optional(v.object({ sealed: v.string(), gmail: v.boolean() })),
  },
  handler: async (ctx, { requestId, cookies, sealedSession, vault }) => {
    const at = now();
    const row = await requestRow(ctx, requestId);
    if (!row) return fail("request_not_found", "Unknown session request.");
    const account = await ctx.db.get(row.accountId);
    if (!account) return fail("request_not_found", "Unknown session request.");
    const request = requestOf(row);
    const late = !request.pending(at);
    if (!late) {
      request.succeed(cookies, "phone");
      await putRequest(ctx, row, request);
    }
    const doc = await recordOf(ctx, account);
    if (sealedSession) doc.saveSession(sealedSession, at, "phone");
    await recordSignIn(ctx, account._id, { at, by: "phone", ok: true, reason: null });
    if (vault) doc.saveVault(vault.sealed, at, vault.gmail);
    await saveRecord(ctx, account._id, doc);
    return late;
  },
});

/** Has the engine seal credentials, only when the server can read the code: Gmail access sent
 * along or a forwarding address, as the phone picked (either one from older phones). */
export async function sealCredentials(
  regno: string,
  username: string,
  credentials: Credentials,
  forwarding: boolean,
  flow: string | null = null,
): Promise<{ sealed: string; gmail: boolean } | undefined> {
  const { password, gmail, otp } = credentials;
  if (!password) return undefined;
  const readable = otp === "gmail" ? gmail !== null : (otp === null && gmail !== null) || forwarding;
  if (!readable) {
    log({ flow, regno }, `account: credentials not kept: the server can't read the code (${otp ?? "any"})`);
    return undefined;
  }
  try {
    const sealed = await engineJson<SealedVault>(SEAL, {
      account: regno,
      username,
      password,
      ...(gmail !== null ? { gmail } : {}),
      ...(otp !== null ? { otp } : {}),
    } satisfies SealVault, { flow, regno });
    return { sealed: sealed.sealedVault, gmail: gmail !== null };
  } catch (e) {
    logError({ flow, regno }, "account: sealing credentials failed", e);
    return undefined;
  }
}

/** The phone's answer to a `vtop_cookie_request`: its cookies (and maybe its credentials), or why
 * it has none. An action: the engine checks the cookies first. */
export const answer = action({
  args: {
    flow: v.string(),
    requestId: v.string(),
    cookies: v.optional(v.array(v.object({ name: v.string(), value: v.string(), domain: v.string() }))),
    credentials: v.optional(v.union(credentialsValidator, v.null())),
    username: v.optional(v.string()),
    error: v.optional(v.string()),
  },
  handler: async (ctx, args): Promise<{ ok: true; late?: boolean }> => {
    const me = await actionCaller(ctx);
    if (!me.account) return fail("demo_unavailable", "The demo account can't do this.");
    const requestId = args.requestId.trim();
    const check = await ctx.runQuery(internal.sessions.asked, { accountId: me.account._id, requestId, deviceId: me.deviceId });
    const failed = (args.error ?? "").trim();
    if (failed) {
      await ctx.runMutation(internal.sessions.failRequest, { requestId, error: failed });
      return { ok: true };
    }
    const cookies = validCookies(args.cookies ?? []);
    if (!cookies) return fail("cookies_invalid", "The callback contained invalid cookies.");
    const header = toHeader(cookies);
    let checked: SessionChecked;
    try {
      checked = await engineJson<SessionChecked>(SESSION, { cookies: header } satisfies SessionCheck, { flow: args.flow, regno: check.regno });
    } catch (e) {
      logError({ flow: args.flow, regno: check.regno }, "account: checking a phone answer failed", e);
      return fail("vtop_unreachable", "VTOP is unreachable; try again.");
    }
    switch (checked.status) {
      case "valid":
        if (accountId(checked.registrationNumber ?? "") === check.regno) break;
        await ctx.runMutation(internal.sessions.failRequest, { requestId, error: ACCOUNT_MISMATCH });
        return fail("account_mismatch", "These cookies belong to a different student than the one who asked.");
      case "expired":
        await ctx.runMutation(internal.sessions.failRequest, { requestId, error: PHONE_SESSION_INVALID });
        return fail("session_invalid", "VTOP does not accept these cookies.");
      default:
        logWarn({ flow: args.flow, regno: check.regno }, `account: the engine could not check a phone answer (${String(checked.status)})`);
        return fail("vtop_unreachable", "VTOP is unreachable; try again.");
    }
    const credentials = credentialsOf(args.credentials);
    const username = credentials ? (usableText(credentials.username) ?? usableText(args.username)) : null;
    const vault = credentials && username !== null ? await sealCredentials(check.regno, username, credentials, check.forwarding, args.flow) : undefined;
    const late = await ctx.runMutation(internal.sessions.answered, {
      requestId,
      cookies: header,
      ...(checked.sealedSession ? { sealedSession: checked.sealedSession } : {}),
      ...(vault ? { vault } : {}),
    });
    log({ flow: args.flow, regno: check.regno }, late ? "account: a late phone answer was cached" : "account: a phone answered a session request");
    return { ok: true, late };
  },
});

// ---- the engine's report --------------------------------------------------------------------

export function isLoginOutcome(raw: unknown): raw is LoginOutcome {
  if (raw === null || typeof raw !== "object") return false;
  const outcome = raw as Record<string, unknown>;
  switch (outcome.status) {
    case "ok":
      return typeof outcome.cookies === "string" && typeof outcome.sealedSession === "string";
    case "failed":
      return typeof outcome.reason === "string";
    case "gmail_revoked":
      return typeof outcome.sealedVault === "string";
    case "invalid_credentials":
    case "vault_unusable":
    case "forwarding_required":
      return true;
    default:
      return false;
  }
}

/** A sign-in with saved credentials ended (logins.ts): record it, hand a session to the request,
 * or ask the phone instead. */
export async function loginDone(ctx: MutationCtx, regno: string, requestId: string, outcome: LoginOutcome): Promise<void> {
  const account = await accountByRegno(ctx, regno);
  if (!account) return;
  const doc: AccountRecord = await recordOf(ctx, account);
  const at = now();
  const row = await requestRow(ctx, requestId);
  const request = row && row.accountId === account._id ? requestOf(row) : null;
  // Set when the phone should be asked instead; with the message to end the request with if it
  // cannot be (null: why the phone could not).
  let fallback: { reason: string | null } | null = null;
  let signIn: SignInRecord;
  switch (outcome.status) {
    case "ok":
      signIn = { at, by: "server", ok: true, reason: null };
      doc.saveSession(outcome.sealedSession, at, "server");
      if (request?.pending(at)) request.succeed(outcome.cookies, "server", outcome.csrfToken ?? null);
      break;
    case "invalid_credentials":
      log({ regno: doc.id }, "account: VTOP rejected saved credentials; dropping them");
      signIn = { at, by: "server", ok: false, reason: "VTOP rejected the saved password" };
      doc.rejectCredentials(at);
      fallback = { reason: "credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate." };
      break;
    case "forwarding_required":
      // The engine did not ask VTOP (it would have mailed an OTP nobody reads). The
      // credentials stay: the phone sends them again until the server can read the OTP.
      signIn = { at, by: "server", ok: false, reason: "Neither OTP forwarding nor Gmail is set up" };
      fallback = {
        reason: "forwarding_required: Set up OTP email forwarding or connect Gmail in VITAP Mate to sign in from the server.",
      };
      break;
    case "gmail_revoked":
      log({ regno: doc.id }, "account: Google refused the saved Gmail access; dropping it");
      signIn = { at, by: "server", ok: false, reason: "Google refused the saved Gmail access" };
      doc.dropGmail(outcome.sealedVault);
      fallback = { reason: "gmail_revoked: Gmail access was revoked. Connect Gmail again in VITAP Mate to sign in from the server." };
      break;
    case "vault_unusable":
      signIn = { at, by: "server", ok: false, reason: "The saved password can no longer be used" };
      doc.clearVault();
      fallback = { reason: null };
      break;
    case "failed":
      logWarn({ regno: doc.id }, "account: server sign-in failed for now");
      signIn = { at, by: "server", ok: false, reason: outcome.reason };
      // With no phone to ask (`app:login`), the engine's reason is the answer.
      fallback = { reason: request?.phones.length === 0 ? outcome.reason : null };
      break;
  }
  if (fallback && request?.pending(at)) {
    if (!request.phoneAsked) {
      const failure = await askPhone(ctx, doc, request, at);
      if (failure !== null) request.fail(fallback.reason ?? failure);
    } else if (request.phoneError !== null) {
      request.fail(fallback.reason ?? request.phoneError);
    }
  }
  await saveRecord(ctx, account._id, doc);
  await recordSignIn(ctx, account._id, signIn);
  if (row && request) await putRequest(ctx, row, request);
  if (outcome.status === "invalid_credentials") {
    for (const phone of await phonesOf(ctx, account._id)) await notify(ctx, phone.deviceId, "credentials_rejected");
  }
}

