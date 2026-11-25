import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_BRIDGE_URL,
  normalizeBridgeUrl,
  clampPollAfter,
  fetchWithTimeout,
  friendlyLoginError,
  progressText,
  parseAPIError,
  validateCookies,
} from "./bridge";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("bridge helpers", () => {
  it("validates VTOP cookies", () => {
    expect(() =>
      validateCookies([
        { domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "abc" },
      ]),
    ).not.toThrow();
    expect(() =>
      validateCookies([{ domain: "example.com", name: "bad", value: "abc" }]),
    ).toThrow("unexpected domain");
  });

  it("maps machine-readable errors", () => {
    const expired = friendlyLoginError(410, '{"code":"request_expired"}');
    expect(expired).toContain("didn't answer in time");
    expect(expired).toContain("Store on server");
    expect(friendlyLoginError(429, "")).toContain("Too many");
    expect(friendlyLoginError(503, '{"code":"engine_unavailable"}')).toContain("temporarily unavailable");
    expect(
      friendlyLoginError(200, '{"status":"error","code":"forwarding_required","error":"Set up OTP email forwarding."}'),
    ).toBe("Set up OTP email forwarding.");
    expect(friendlyLoginError(200, "not json")).toContain("Could not connect");
    expect(friendlyLoginError(0, "", new DOMException("Aborted", "AbortError"))).toContain("took too long");
    expect(friendlyLoginError(0, "", new DOMException("Timed out", "TimeoutError"))).toContain("took too long");
    expect(friendlyLoginError(0, "", new TypeError("Failed to fetch"))).toContain("Could not reach the backend");
  });

  it("maps fetchWithTimeout timeouts to the timeout message", async () => {
    vi.useFakeTimers();
    vi.stubGlobal(
      "fetch",
      vi.fn((_input: RequestInfo | URL, init?: RequestInit) => {
        return new Promise<Response>((_res, rej) => {
          init?.signal?.addEventListener("abort", () => rej(init.signal?.reason), { once: true });
        });
      }),
    );

    const pending = fetchWithTimeout("https://vmate.test/v1/session", { method: "POST" }, 5000);
    const caught = pending.catch((err) => err);
    vi.advanceTimersByTime(5000);
    const err = await caught;
    expect(err).toBeInstanceOf(DOMException);
    expect((err as DOMException).name).toBe("TimeoutError");
    expect(friendlyLoginError(0, "", err)).toContain("took too long");
  });

  it("parses errors and clamps polling intervals", () => {
    expect(parseAPIError('{"code":"key_unknown"}').code).toBe("key_unknown");
    expect(clampPollAfter(10)).toBe(500);
    expect(clampPollAfter(9000)).toBe(5000);
    expect(clampPollAfter(undefined)).toBe(1500);
  });
});

import {
  PAIR_TTL_MS,
  interpretPairResponse,
  interpretSessionResponse,
  isAccessKey,
  isLoginRedirect,
  newPairCode,
} from "./bridge";

const KEY = `vtm_${"ab".repeat(32)}`;

describe("access keys", () => {
  it("recognises access keys and rejects old FCM tokens", () => {
    expect(isAccessKey(KEY)).toBe(true);
    expect(isAccessKey(` ${KEY} `)).toBe(true);
    expect(isAccessKey("dXk3:APA91bH-long-fcm-token")).toBe(false);
    expect(isAccessKey("vtm_short")).toBe(false);
  });
});

describe("session responses", () => {
  it("reads a ready session", () => {
    expect(
      interpretSessionResponse(200, {
        status: "ready",
        cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "a" }],
      }),
    ).toEqual({
      kind: "ready",
      cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "a" }],
      source: { from: "unknown", cached: false, madeAt: undefined },
    });
  });

  it("reads a pending request", () => {
    expect(
      interpretSessionResponse(202, { status: "pending", requestId: "r1", pollAfterMs: 1500 }),
    ).toEqual({
      kind: "pending",
      requestId: "r1",
      pollAfterMs: 1500,
      stage: "starting",
      serverDueAt: undefined,
    });
    expect(interpretSessionResponse(202, { status: "pending" })).toEqual({
      kind: "pending",
      requestId: undefined,
      pollAfterMs: 1500,
      stage: "starting",
      serverDueAt: undefined,
    });
  });

  it("turns errors into friendly messages", () => {
    const unreachable = interpretSessionResponse(200, {
      status: "error",
      code: "phone_unreachable",
      error: "This key's phone is no longer linked.",
    });
    expect(unreachable.kind).toBe("error");
    expect(unreachable.kind === "error" && unreachable.message).toBe(
      "This key's phone is no longer linked. Open VITAP Mate → Connected apps and move this key to your phone, or pair again.",
    );

    const unknown = interpretSessionResponse(401, { code: "key_unknown", error: "x" });
    expect(unknown.kind === "error" && unknown.message).toContain("access key");

    const expired = interpretSessionResponse(410, { code: "request_expired" });
    expect(expired.kind === "error" && expired.message).toContain("Open VITAP Mate");
    expect(expired.kind === "error" && expired.message).toContain("Store on server");
  });
});

describe("friendly errors for the session API", () => {
  it("explains unknown keys and phone problems", () => {
    expect(friendlyLoginError(401, '{"code":"key_unknown"}')).toContain("Connected apps");
    expect(friendlyLoginError(200, '{"code":"phone_timeout"}')).toContain("phone");
    expect(friendlyLoginError(429, '{"code":"rate_limited"}')).toContain("Too many");
  });

  it("passes clean server error messages through untouched", () => {
    const reply = JSON.stringify({
      status: "error",
      code: "phone_session_invalid",
      error: "VTOP rejected your phone's session.",
    });
    expect(friendlyLoginError(200, reply)).toBe("VTOP rejected your phone's session.");
    expect(friendlyLoginError(200, JSON.stringify({ status: "error", code: "login_failed", error: "note: keep colon" }))).toBe(
      "note: keep colon",
    );
  });
});

describe("login redirects", () => {
  it("detects VTOP's login page", () => {
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/login")).toBe(true);
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/initialProcess")).toBe(true);
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/content")).toBe(false);
  });
});

describe("bridge URL setting", () => {
  it("defaults to the URL from .env", () => {
    expect(DEFAULT_BRIDGE_URL).toBe("https://vmate.aster0.dev");
  });

  it("keeps only an https origin", () => {
    expect(normalizeBridgeUrl("https://bridge.example.com/")).toBe("https://bridge.example.com");
    expect(normalizeBridgeUrl("  https://bridge.example.com/v1/x?y#z ")).toBe("https://bridge.example.com");
    expect(normalizeBridgeUrl("bridge.example.com")).toBe("https://bridge.example.com");
    expect(normalizeBridgeUrl("http://bridge.example.com")).toBeNull();
    expect(normalizeBridgeUrl("")).toBeNull();
    expect(normalizeBridgeUrl("not a url")).toBeNull();
  });

  it("reads the stage of a pending request", () => {
    const outcome = interpretSessionResponse(202, {
      status: "pending",
      requestId: "r",
      stage: "phone",
      serverDueAt: "2026-10-18T10:00:10Z",
    });
    expect(outcome).toMatchObject({ kind: "pending", stage: "phone" });
    if (outcome.kind !== "pending") throw new Error("not pending");
    expect(outcome.serverDueAt).toBe(Date.parse("2026-10-18T10:00:10Z"));
    expect(interpretSessionResponse(202, { status: "pending" })).toMatchObject({
      stage: "starting",
      serverDueAt: undefined,
    });
  });

  it("reads where a session came from", () => {
    expect(
      interpretSessionResponse(200, {
        status: "ready",
        cookies: [],
        source: { from: "server", cached: true, madeAt: "2026-10-18T10:00:00Z" },
      }),
    ).toMatchObject({
      kind: "ready",
      source: { from: "server", cached: true, madeAt: Date.parse("2026-10-18T10:00:00Z") },
    });
    expect(interpretSessionResponse(200, { status: "ready", cookies: [] })).toMatchObject({
      source: { from: "unknown", cached: false },
    });
  });

  it("says who it is waiting for, without a server countdown when storage is off", () => {
    expect(progressText("server", undefined)).toContain("server is signing in");
    expect(progressText("phone", 10_000, 0)).toContain("takes over in 10 s");
    expect(progressText("phone", undefined, 0)).toBe("Waiting for your phone…");
    expect(progressText("phone", 0, 5_000)).toBe("Waiting for your phone…");
    expect(progressText("starting", undefined)).toBe("Starting sign-in…");
    expect(progressText("server", undefined, 0, "waiting_otp")).toBe("Waiting for VTOP's OTP email");
    expect(progressText("server", undefined, 0, "checking_otp")).toContain("Got the OTP");
  });
});

describe("pairing", () => {
  it("makes codes the edge reads", () => {
    expect(newPairCode()).toMatch(/^[0-9A-HJKMNP-TV-Z]{5}-[0-9A-HJKMNP-TV-Z]{5}$/);
    expect(newPairCode((b) => b.fill(255))).toBe("ZZZZZ-ZZZZZ");
    expect(newPairCode((b) => b.fill(0))).toBe("00000-00000");
    expect(newPairCode()).not.toBe(newPairCode());
    expect(PAIR_TTL_MS).toBe(300_000);
  });

  it("reads the edge's replies", () => {
    expect(interpretPairResponse(200, { status: "ready", key: KEY })).toEqual({ kind: "ready", key: KEY });
    expect(interpretPairResponse(202, { status: "pending", pollAfterMs: 2000 })).toEqual({
      kind: "pending",
      pollAfterMs: 2000,
    });
    const bad = interpretPairResponse(400, { code: "bad_request", error: "code must be a pairing code." });
    expect(bad).toEqual({ kind: "error", message: "code must be a pairing code." });
    expect(interpretPairResponse(200, { status: "ready", key: "nope" }).kind).toBe("error");
  });
});
