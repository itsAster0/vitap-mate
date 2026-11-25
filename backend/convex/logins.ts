// Server sign-ins with saved credentials, one engine step per call (`/engine/login`). The first
// step signs in whole (Gmail, or no OTP asked) or stops at VTOP's OTP; a stopped one waits here
// until the student's forwarded OTP mail comes (`mail.deliver` schedules `resume`) or
// `LOGIN_WAIT_SECS` pass (`sweeps.fiveMinutes`). Each outcome goes to `sessions.loginDone`. Nothing
// waits on mail or polls.

import { v } from "convex/values";
import { internal } from "./_generated/api";
import type { Doc, Id } from "./_generated/dataModel";
import { type ActionCtx, type MutationCtx, internalAction, internalMutation, internalQuery } from "./_generated/server";
import { enginePost } from "./lib/engine";
import { LOGIN, type LoginOutcome } from "./lib/engineApi";
import { mailboxOfAccount } from "./lib/identity";
import { putRequest, requestOf, requestRow } from "./lib/requests";
import { logError, now } from "./lib/util";
import { isLoginOutcome, loginDone } from "./sessions";

/** How long a sign-in waits for its OTP mail. */
export const LOGIN_WAIT_SECS = 300;
/** Mail dates come from VTOP's clock; allow it to run a little behind. */
const CLOCK_SLACK_SECS = 120;
const OTP_NOT_FOUND: LoginOutcome = { status: "failed", code: "otp_not_found", reason: "the OTP mail did not arrive" };

/** Mutually exclusive phase of a server sign-in row. */
export type LoginPhase =
  | { kind: "starting"; nextOtp: string | null }
  | { kind: "waiting_otp"; sealedState: string }
  | { kind: "checking_otp"; sealedState: string; nextOtp: string | null };

const CLEAR_LEGACY = { sealedState: undefined, busy: undefined, next: undefined } as const;

/** Reads `row.phase`, or translates a pre-phase row left in flight during deploy. */
export function phaseOf(row: Doc<"logins">): LoginPhase {
  if (row.phase) return row.phase;
  if (row.sealedState == null) return { kind: "starting", nextOtp: row.next ?? null };
  if (row.busy) return { kind: "checking_otp", sealedState: row.sealedState, nextOtp: row.next ?? null };
  return { kind: "waiting_otp", sealedState: row.sealedState };
}

/** How an engine step ended. */
type Stepped = LoginOutcome | { status: "otp_needed"; sealedState: string; since: number };

const steppedValidator = v.union(
  v.object({ status: v.literal("ok"), cookies: v.string(), sealedSession: v.string(), csrfToken: v.optional(v.string()) }),
  v.object({ status: v.literal("invalid_credentials") }),
  v.object({ status: v.literal("vault_unusable") }),
  v.object({ status: v.literal("forwarding_required") }),
  v.object({ status: v.literal("gmail_revoked"), sealedVault: v.string() }),
  v.object({ status: v.literal("failed"), code: v.optional(v.string()), reason: v.string() }),
  v.object({ status: v.literal("otp_needed"), sealedState: v.string(), since: v.number() }),
);

/** Strips any extra engine fields so the value always satisfies `steppedValidator`. */
function normalizeOutcome(outcome: LoginOutcome): LoginOutcome {
  switch (outcome.status) {
    case "ok":
      return {
        status: "ok",
        cookies: outcome.cookies,
        sealedSession: outcome.sealedSession,
        ...(typeof outcome.csrfToken === "string" ? { csrfToken: outcome.csrfToken } : {}),
      };
    case "gmail_revoked":
      return { status: "gmail_revoked", sealedVault: outcome.sealedVault };
    case "failed":
      return {
        status: "failed",
        ...(typeof outcome.code === "string" && outcome.code ? { code: outcome.code } : {}),
        reason: outcome.reason,
      };
    case "invalid_credentials":
    case "vault_unusable":
    case "forwarding_required":
      return { status: outcome.status };
  }
}

/** One engine step; an engine that can't be reached, or answers oddly, is a failed sign-in. */
async function step(body: { account: string; sealedVault?: string; forwarding?: boolean; sealedState?: string; otp?: string }, flow: string | null): Promise<Stepped> {
  const trace = { flow, regno: body.account };
  try {
    const response = await enginePost(LOGIN, body, trace);
    const answer = response.status === 200 ? ((await response.json()) as Record<string, unknown>) : null;
    if (answer?.status === "otp_needed" && typeof answer.sealedState === "string" && typeof answer.since === "number") {
      return { status: "otp_needed", sealedState: answer.sealedState, since: answer.since };
    }
    if (isLoginOutcome(answer)) return normalizeOutcome(answer);
    logError(trace, `account: the engine's sign-in step answered ${response.status}`);
  } catch (e) {
    logError(trace, "account: engine unreachable for a sign-in", e);
  }
  return { status: "failed", code: "server_login_failed", reason: "The server could not reach the engine." };
}

/** Marks the request's server sign-in started and keeps a row that takes OTP mail from now on,
 * so a code that comes before the first step answers is not missed. Null for no such request. */
export const open = internalMutation({
  args: { requestId: v.string() },
  handler: async (ctx, { requestId }) => {
    const row = await requestRow(ctx, requestId);
    if (!row) return null;
    const request = requestOf(row);
    const at = now();
    // Answered, failed or timed out since the caller looked, or started already: signing in
    // now would only mail the student an OTP nobody waits for.
    if (!request.pending(at) || request.vaultStarted) return null;
    request.vaultStarted = true;
    await putRequest(ctx, row, request);
    const loginId = await ctx.db.insert("logins", {
      accountId: row.accountId,
      requestId,
      since: at - CLOCK_SLACK_SECS,
      phase: { kind: "starting", nextOtp: null },
      // No longer than its request: a sign-in nobody waits for doesn't take later codes.
      expiresAt: Math.min(at + LOGIN_WAIT_SECS, request.expiresAt),
    });
    return { loginId, forwarding: (await mailboxOfAccount(ctx, row.accountId)) !== null };
  },
});

/** Starts a sign-in for `requestId` with the sealed `vault`: here (`inline`, the caller waits for
 * the first step) or scheduled. */
export async function startLogin(ctx: ActionCtx, regno: string, vault: string, requestId: string, flow: string | null, inline: boolean): Promise<void> {
  const opened = await ctx.runMutation(internal.logins.open, { requestId });
  if (!opened) return;
  const args = { loginId: opened.loginId, regno, requestId, vault, forwarding: opened.forwarding, flow };
  if (inline) await first(ctx, args);
  else await ctx.scheduler.runAfter(0, internal.logins.firstStep, args);
}

type First = { loginId: Id<"logins">; regno: string; requestId: string; vault: string; forwarding: boolean; flow: string | null };

async function first(ctx: ActionCtx, { loginId, regno, requestId, vault, forwarding, flow }: First): Promise<void> {
  const stepped = await step({ account: regno, sealedVault: vault, forwarding }, flow);
  await ctx.runMutation(internal.logins.stepped, { loginId, regno, requestId, stepped });
}

export const firstStep = internalAction({
  args: { loginId: v.id("logins"), regno: v.string(), requestId: v.string(), vault: v.string(), forwarding: v.boolean(), flow: v.union(v.string(), v.null()) },
  handler: first,
});

export const waiting = internalQuery({
  args: { loginId: v.id("logins") },
  handler: async (ctx, { loginId }) => {
    const row = await ctx.db.get(loginId);
    const account = row && (await ctx.db.get(row.accountId));
    if (!row || !account || row.expiresAt <= now()) return null;
    const phase = phaseOf(row);
    if (phase.kind === "starting") return null;
    return { sealedState: phase.sealedState, regno: account.regno, requestId: row.requestId };
  },
});

/** The next step, with a code from the student's mail. */
export const resume = internalAction({
  args: { loginId: v.id("logins"), otp: v.string() },
  handler: async (ctx, { loginId, otp }) => {
    const row = await ctx.runQuery(internal.logins.waiting, { loginId });
    if (!row) return;
    const stepped = await step({ account: row.regno, sealedState: row.sealedState, otp }, null);
    await ctx.runMutation(internal.logins.stepped, { loginId, regno: row.regno, requestId: row.requestId, stepped });
  },
});

/** Keeps a sign-in that waits for its OTP (sending it a code that came during the step), or
 * ends it. A late outcome, after the sweep ended the row, still caches its session. */
export const stepped = internalMutation({
  args: { loginId: v.id("logins"), regno: v.string(), requestId: v.string(), stepped: steppedValidator },
  handler: async (ctx, { loginId, regno, requestId, stepped: answer }) => {
    const row = await ctx.db.get(loginId);
    if (answer.status !== "otp_needed") {
      if (row) await ctx.db.delete(loginId);
      await loginDone(ctx, regno, requestId, answer);
      return;
    }
    if (!row) return;
    if (row.expiresAt <= now()) {
      await ctx.db.delete(loginId);
      await loginDone(ctx, regno, requestId, OTP_NOT_FOUND);
      return;
    }
    const phase = phaseOf(row);
    const queued = phase.kind === "waiting_otp" ? null : phase.nextOtp;
    if (queued !== null) {
      await ctx.db.patch(loginId, {
        since: answer.since,
        phase: { kind: "checking_otp", sealedState: answer.sealedState, nextOtp: null },
        ...CLEAR_LEGACY,
      });
      await ctx.scheduler.runAfter(0, internal.logins.resume, { loginId, otp: queued });
    } else {
      await ctx.db.patch(loginId, {
        since: answer.since,
        phase: { kind: "waiting_otp", sealedState: answer.sealedState },
        ...CLEAR_LEGACY,
      });
    }
  },
});

/** A code from the student's forwarded mail, sent at `sentAt`, for each of their sign-ins
 * waiting on one. */
export async function offerCode(ctx: MutationCtx, accountId: Id<"accounts">, otp: string, sentAt: number): Promise<void> {
  const at = now();
  const rows = await ctx.db
    .query("logins")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  for (const row of rows) {
    if (row.expiresAt <= at || sentAt < row.since) continue;
    const phase = phaseOf(row);
    switch (phase.kind) {
      case "starting":
        await ctx.db.patch(row._id, { phase: { kind: "starting", nextOtp: otp }, ...CLEAR_LEGACY });
        break;
      case "checking_otp":
        await ctx.db.patch(row._id, { phase: { kind: "checking_otp", sealedState: phase.sealedState, nextOtp: otp }, ...CLEAR_LEGACY });
        break;
      case "waiting_otp":
        await ctx.db.patch(row._id, { phase: { kind: "checking_otp", sealedState: phase.sealedState, nextOtp: null }, ...CLEAR_LEGACY });
        await ctx.scheduler.runAfter(0, internal.logins.resume, { loginId: row._id, otp });
        break;
    }
  }
}

/** Ends sign-ins whose OTP mail never came; at most `limit` a run. */
export async function sweepLogins(ctx: MutationCtx, at: number, limit: number): Promise<void> {
  const rows = await ctx.db
    .query("logins")
    .withIndex("by_expiresAt", (q) => q.lte("expiresAt", at))
    .take(limit);
  for (const row of rows) {
    await ctx.db.delete(row._id);
    const account = await ctx.db.get(row.accountId);
    if (account) await loginDone(ctx, account.regno, row.requestId, OTP_NOT_FOUND);
  }
}
