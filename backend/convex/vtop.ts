// VTOP on the engine for a phone on the Server or Udp route (edge/src/
// vtop-phone.ts): the engine signs in and fetches, and hands back the parsed
// page only. Bodies go to the engine as they came. A JSON answer comes back
// as its value (an error answer as a ConvexError with its `code`), a page the
// engine packed with the phone's page dictionary as an ArrayBuffer.

import { ConvexError, type Value } from "convex/values";
import { type ActionCtx, action } from "./_generated/server";
import { fail } from "./lib/errors";
import { actionCaller } from "./lib/identity";
import { rateLimits } from "./lib/rateLimits";
import { enginePost } from "./lib/engine";
import { signedArgs, verifySigned } from "./lib/signed";
import { type Trace, log, logError, logWarn, now } from "./lib/util";

/** Bodies are a session and a page name, or a username and password. */
const MAX_BODY_BYTES = 64 * 1024;
/** A page packed with the page dictionary (`vtop_shared::page_dictionary`): Brotli already,
 * which only the phone can unpack. */
export const PACKED_PAGE = "application/vnd.vmate.page+br";
/** Over the engine's own VTOP limits (25 s a page, 28 s a sign-in), so the phone hears the
 * engine's reason instead of a bare timeout. */
export const ENGINE_TIMEOUT_MS = 32_000;

/** Passes `body` to the engine's `enginePath` and returns its answer. */
export async function passToEngine(enginePath: string, body: string, trace: Trace, timeoutMs = ENGINE_TIMEOUT_MS): Promise<Value> {
  if (new TextEncoder().encode(body).byteLength > MAX_BODY_BYTES) fail("too_large", "The request is too large.");
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), timeoutMs);
  let answer: Response;
  let raw: ArrayBuffer;
  try {
    answer = await enginePost(enginePath, body, trace, abort.signal);
    // Read here, under the timer.
    raw = await answer.arrayBuffer();
  } catch (e) {
    if (abort.signal.aborted) {
      logWarn(trace, `vtop: ${enginePath} timed out`);
      fail("vtop_timeout", "VTOP took too long to answer.");
    }
    logError(trace, `vtop: engine unreachable for ${enginePath}`, e);
    return fail("engine_unavailable", "The server can't reach VTOP right now.");
  } finally {
    clearTimeout(timer);
  }
  if (answer.headers.get("content-type") === PACKED_PAGE && answer.ok) return raw;
  let json: unknown;
  try {
    json = JSON.parse(new TextDecoder().decode(raw));
  } catch {
    json = null;
  }
  if (answer.ok) return json as Value;
  const data = json as { code?: unknown; error?: unknown } | null;
  if (data && typeof data.code === "string") throw new ConvexError(data as Record<string, Value>);
  logError(trace, `vtop: the engine answered ${enginePath} with ${answer.status} and no error code`);
  return fail(answer.status === 404 ? "not_found" : "engine_unavailable", `The engine answered ${answer.status}.`);
}

/** A JWT call's body as the engine wants it: the args without `flow`. */
async function phoneCall(ctx: ActionCtx, args: Record<string, Value>, enginePath: string): Promise<Value> {
  const me = await actionCaller(ctx);
  if (me.demo) fail("demo_unavailable", "The demo account can't do this.");
  const { flow, ...body } = args;
  const trace = { flow: typeof flow === "string" ? flow : null, regno: me.regno };
  log(trace, `vtop: ${enginePath}`);
  return passToEngine(enginePath, JSON.stringify(body), trace);
}

export const fetch = action({
  handler: (ctx, args: Record<string, Value>) => phoneCall(ctx, args, "/engine/fetch"),
});

export const fetchBatch = action({
  handler: (ctx, args: Record<string, Value>) => phoneCall(ctx, args, "/engine/fetch-batch"),
});

/** A server-route sign-in, before any account: signed, and it proves nothing (the phone proves
 * afterwards with the session it gets back). */
function signedLogin(functionName: string, enginePath: string) {
  return action({
    args: signedArgs,
    handler: async (ctx, args) => {
      await verifySigned(functionName, args, now());
      let username: unknown = null;
      try {
        username = (JSON.parse(args.payload) as { username?: unknown }).username;
      } catch {
        // The engine refuses a body that isn't JSON; the limits still count it in total.
      }
      // Per username first: an action doesn't roll back, so one throttled student must not use up
      // everyone's allowance.
      const checks: Array<["loginGlobal" | "loginUser", string | undefined]> = [];
      if (typeof username === "string") checks.push(["loginUser", username.trim().toUpperCase()]);
      checks.push(["loginGlobal", undefined]);
      for (const [name, key] of checks) {
        const { ok, retryAfter } = await rateLimits.limit(ctx, name, key === undefined ? {} : { key });
        if (!ok) fail("too_many_sign_ins", "Too many sign-ins; wait a little.", { retryAfterMs: retryAfter });
      }
      log({ flow: args.flow }, `vtop: ${enginePath}`);
      return passToEngine(enginePath, args.payload, { flow: args.flow });
    },
  });
}

export const login = signedLogin("vtop:login", "/engine/vtop-login");
export const loginOtp = signedLogin("vtop:loginOtp", "/engine/vtop-login/otp");
export const loginResend = signedLogin("vtop:loginResend", "/engine/vtop-login/resend");
