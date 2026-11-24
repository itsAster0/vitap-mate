// Server sign-ins with saved credentials, one engine step per call: the first step runs at once,
// one that stops at VTOP's OTP resumes when the forwarded mail comes, and the sweep ends one
// whose mail never came. Also the gateway's way in (`/v1/app/login` with the phone's JWT).

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, TOKENS, advance, codeOf, engine, phone, prove, setup, type T } from "../test/kit";
import { api, internal } from "./_generated/api";
import { REQUEST_TIMEOUT_SECS } from "./lib/accountState";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

const OK = { status: "ok", cookies: "a=b", sealedSession: "s", csrfToken: "c" };
const SESSION = { cookies: "a=b", csrf_token: "c", registration_number: REGNO };

const mail = (t: T, otp: string) =>
  t.fetch("/internal/mail", {
    method: "POST",
    headers: { authorization: `Bearer ${TOKENS.MAIL_TOKEN}`, "content-type": "application/json" },
    body: JSON.stringify({ addressId: addressOf.get(t), receivedAt: Date.now(), otp }),
  });
const addressOf = new WeakMap<T, string>();

/** A proved student with a forwarding address and a saved password. */
async function student(t: T) {
  const proved = await prove(t, await phone(1));
  const box = await proved.as.mutation(api.mail.register, { flow: "f" });
  addressOf.set(t, box.id);
  await t.run(async (ctx) => {
    const id = ctx.db.normalizeId("accounts", proved.account)!;
    await ctx.db.patch(id, { sealedVault: "vault", vaultSavedAt: Math.floor(Date.now() / 1000) });
  });
  return proved;
}

/** Runs immediate scheduled jobs (`runAfter(0)`), keeping the clock where it is. */
async function drain(t: T) {
  const at = Date.now();
  vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
  vi.setSystemTime(at);
  await t.finishAllScheduledFunctions(() => vi.advanceTimersByTime(0));
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(at);
}

/** An engine whose first step stops at the OTP and takes `code` only. */
function otpEngine(code: string, first?: () => Promise<unknown>) {
  const answer = async (step: Record<string, unknown>) => {
    if (step.sealedVault === undefined) return Response.json(step.otp === code ? OK : { status: "otp_needed", sealedState: "st", since: 0 });
    await first?.();
    return Response.json({ status: "otp_needed", sealedState: "st", since: Math.floor(Date.now() / 1000) - 120 });
  };
  return engine((path, body) => (path === "/engine/login" ? answer(body as Record<string, unknown>) : undefined));
}

describe("server sign-ins", () => {
  it("stop at the OTP and resume when its mail comes, with no one polling", async () => {
    const t = await setup();
    const { as } = await student(t);
    const calls = otpEngine("654321");
    const started = await as.action(api.app.login, { flow: "f" });
    expect(started).toMatchObject({ status: "pending" });
    const requestId = (started as { requestId: string }).requestId;
    expect(calls.find((c) => c.path === "/engine/login")!.body).toEqual({ account: REGNO, sealedVault: "vault", forwarding: true });
    expect(await as.query(api.app.loginResult, { requestId })).toEqual({ status: "pending", requestId });
    // An older mail's code leaves it waiting; the next one signs in.
    await mail(t, "111111");
    await drain(t);
    expect(await as.query(api.app.loginResult, { requestId })).toEqual({ status: "pending", requestId });
    advance(5);
    await mail(t, "654321");
    await drain(t);
    expect(calls.filter((c) => c.path === "/engine/login").at(-1)!.body).toEqual({ account: REGNO, sealedState: "st", otp: "654321" });
    expect(await as.query(api.app.loginResult, { requestId })).toEqual({ status: "ok", session: SESSION });
    expect(await t.run((ctx) => ctx.db.query("logins").collect())).toEqual([]);
    const status = await as.query(api.app.status, { flow: "f" });
    expect(status.signIns[0]).toMatchObject({ by: "server", ok: true });
    expect(status.sessionSource).toBe("server");
  });

  it("keep a code that comes while the first step still runs", async () => {
    const t = await setup();
    const { as } = await student(t);
    otpEngine("222222", () => mail(t, "222222"));
    const { requestId } = (await as.action(api.app.login, { flow: "f" })) as { requestId: string };
    await drain(t);
    expect(await as.query(api.app.loginResult, { requestId })).toEqual({ status: "ok", session: SESSION });
  });

  it("answer at once when the first step signs in whole, and drop a rejected password", async () => {
    const t = await setup();
    const { as } = await student(t);
    engine((path) => (path === "/engine/login" ? Response.json(OK) : undefined));
    expect(await as.action(api.app.login, { flow: "f" })).toEqual({ status: "ok", session: SESSION });
    await as.mutation(api.app.sessionClear, { flow: "f" });
    engine((path) => (path === "/engine/login" ? Response.json({ status: "invalid_credentials" }) : undefined));
    expect(await codeOf(as.action(api.app.login, { flow: "f" }))).toBe("invalid_credentials");
    expect((await as.query(api.app.status, { flow: "f" })).vaultSavedAt).toBeNull();
  });

  it("end when the OTP mail never comes", async () => {
    const t = await setup();
    const { as } = await student(t);
    otpEngine("654321");
    await as.action(api.app.login, { flow: "f" });
    // Ends with its request: nobody waits for it after that.
    advance(REQUEST_TIMEOUT_SECS - 1);
    await t.mutation(internal.sweeps.fiveMinutes, {});
    expect(await t.run((ctx) => ctx.db.query("logins").collect())).toHaveLength(1);
    advance(1);
    await t.mutation(internal.sweeps.fiveMinutes, {});
    expect(await t.run((ctx) => ctx.db.query("logins").collect())).toEqual([]);
    const { signIns } = await as.query(api.app.status, { flow: "f" });
    expect(signIns[0]).toMatchObject({ by: "server", ok: false, reason: "otp_not_found: the OTP mail did not arrive" });
  });

  it("ignore late OTP mail once expired, even before the 5-minute sweep runs", async () => {
    const t = await setup();
    const { as } = await student(t);
    const calls = otpEngine("654321");
    await as.action(api.app.login, { flow: "f" });
    const initialCalls = calls.length;
    advance(REQUEST_TIMEOUT_SECS);
    await mail(t, "654321");
    await drain(t);
    expect(calls).toHaveLength(initialCalls);
  });

  it("start nothing for a request that is no longer waiting", async () => {
    const t = await setup();
    const { as } = await student(t);
    otpEngine("654321");
    const { requestId } = (await as.action(api.app.login, { flow: "f" })) as { requestId: string };
    // Opened once already: a second start is turned away.
    expect(await t.mutation(internal.logins.open, { requestId })).toBeNull();
    expect(await t.run((ctx) => ctx.db.query("logins").collect())).toHaveLength(1);
  });

  it("serve the UDP gateway over HTTP with the phone's JWT", async () => {
    const t = await setup();
    const { as } = await student(t);
    otpEngine("654321");
    const started = await as.fetch("/v1/app/login", { method: "POST", body: JSON.stringify({ flow: "udp-1" }) });
    expect(started.status).toBe(202);
    const { requestId } = (await started.json()) as { requestId: string };
    expect((await as.fetch(`/v1/app/login/${requestId}`)).status).toBe(202);
    await mail(t, "654321");
    await drain(t);
    const done = await as.fetch(`/v1/app/login/${requestId}`);
    expect([done.status, await done.json()]).toEqual([200, { status: "ok", session: SESSION }]);
    expect((await as.fetch("/v1/app/login/nope")).status).toBe(404);
    const anonymous = await t.fetch("/v1/app/login", { method: "POST", body: "{}" });
    expect([anonymous.status, ((await anonymous.json()) as { code: string }).code]).toEqual([401, "unauthenticated"]);
    const badEnc = await as.fetch("/v1/app/login/%E0%A4%A");
    expect(badEnc.status).toBe(400);
    await as.mutation(api.app.sessionClear, { flow: "f" });
    // Timed-out sign-in returns 504 vtop_timeout.
    const slow = await as.fetch("/v1/app/login", { method: "POST", body: JSON.stringify({ flow: "udp-2" }) });
    const slowId = ((await slow.json()) as { requestId: string }).requestId;
    advance(REQUEST_TIMEOUT_SECS);
    const timedOut = await as.fetch(`/v1/app/login/${slowId}`);
    expect([timedOut.status, ((await timedOut.json()) as { code: string }).code]).toEqual([504, "vtop_timeout"]);
    // No saved credentials -> 409, demo -> 403, removed device -> 401.
    await as.mutation(api.account.forgetCredentials, { flow: "f" });
    const noCreds = await as.fetch("/v1/app/login", { method: "POST", body: "{}" });
    expect([noCreds.status, ((await noCreds.json()) as { code: string }).code]).toEqual([409, "no_saved_credentials"]);
    const p2 = await phone(2);
    const demo = await t.action(api.auth.token, await p2.signed("auth:token", { demo: true, device: p2.device }));
    const asDemo = t.withIdentity({ subject: p2.deviceId, regno: demo.regno, acct: demo.account, demo: true });
    const demoRes = await asDemo.fetch("/v1/app/login", { method: "POST", body: "{}" });
    expect([demoRes.status, ((await demoRes.json()) as { code: string }).code]).toEqual([403, "demo_unavailable"]);
    // Restore vault and verify invalid_credentials -> 401.
    await t.run(async (ctx) => {
      const row = (await ctx.db.query("accounts").first())!;
      await ctx.db.patch(row._id, { sealedVault: "vault", vaultSavedAt: Math.floor(Date.now() / 1000) });
    });
    engine((path) => (path === "/engine/login" ? Response.json({ status: "invalid_credentials" }) : undefined));
    const rejected = await as.fetch("/v1/app/login", { method: "POST", body: "{}" });
    expect([rejected.status, ((await rejected.json()) as { code: string }).code]).toEqual([401, "invalid_credentials"]);
    await as.mutation(api.auth.signOut, { flow: "f" });
    const removed = await as.fetch("/v1/app/login", { method: "POST", body: "{}" });
    expect([removed.status, ((await removed.json()) as { code: string }).code]).toEqual([401, "device_removed"]);
  });

  it("marks a pending login request expired at its TTL so live app:loginResult re-runs", async () => {
    const t = await setup();
    const { as } = await student(t);
    otpEngine("654321");
    vi.useFakeTimers({ now: Date.now(), toFake: ["Date", "setTimeout", "clearTimeout"] });
    const started = (await as.action(api.app.login, { flow: "f" })) as { requestId: string };
    expect(await as.query(api.app.loginResult, { requestId: started.requestId })).toEqual({ status: "pending", requestId: started.requestId });
    vi.advanceTimersByTime(REQUEST_TIMEOUT_SECS * 1000);
    await t.finishInProgressScheduledFunctions();
    const row = await t.run((ctx) => ctx.db.query("sessionRequests").first());
    expect(row?.expired).toBe(true);
    expect(await as.query(api.app.loginResult, { requestId: started.requestId })).toMatchObject({ status: "error", code: "vtop_timeout" });
  });
});
