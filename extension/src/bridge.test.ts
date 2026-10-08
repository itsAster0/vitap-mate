import { describe, expect, it } from "vitest";
import {
  clampPollAfter,
  friendlyLoginError,
  parseAPIError,
  validateCookies,
} from "./bridge";

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
    expect(
      friendlyLoginError(410, '{"code":"TOKEN_INVALID","error":"expired"}'),
    ).toContain("new Token");
    expect(friendlyLoginError(429, '{"code":"RATE_LIMITED"}')).toContain(
      "Too many",
    );
    expect(
      friendlyLoginError(
        200,
        "Bad state: VTOP session is not authenticated after login.",
      ),
    ).toContain("refresh your VTOP login");
  });

  it("parses errors and clamps polling intervals", () => {
    expect(parseAPIError('{"code":"STORE_UNAVAILABLE"}').code).toBe(
      "STORE_UNAVAILABLE",
    );
    expect(clampPollAfter(10)).toBe(500);
    expect(clampPollAfter(9000)).toBe(5000);
    expect(clampPollAfter(undefined)).toBe(1500);
  });
});

import { interpretSessionResponse, isAccessKey, isLoginRedirect } from "./bridge";

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
    });
  });

  it("reads a pending request", () => {
    expect(
      interpretSessionResponse(202, { status: "pending", requestId: "r1", pollAfterMs: 1500 }),
    ).toEqual({ kind: "pending", requestId: "r1", pollAfterMs: 1500 });
    expect(interpretSessionResponse(202, { status: "pending" })).toEqual({
      kind: "pending",
      requestId: undefined,
      pollAfterMs: 1500,
    });
  });

  it("turns errors into friendly messages", () => {
    const unreachable = interpretSessionResponse(200, {
      status: "error",
      code: "phone_unreachable",
      error: "phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone.",
    });
    expect(unreachable.kind).toBe("error");
    expect(unreachable.kind === "error" && unreachable.message).toContain("Reconnect this phone");

    const unknown = interpretSessionResponse(401, { code: "key_unknown", error: "x" });
    expect(unknown.kind === "error" && unknown.message).toContain("access key");

    const expired = interpretSessionResponse(410, { code: "request_expired" });
    expect(expired.kind === "error" && expired.message).toContain("phone");
  });
});

describe("friendly errors for the session API", () => {
  it("explains unknown keys and phone problems", () => {
    expect(friendlyLoginError(401, '{"code":"key_unknown"}')).toContain("Connected apps");
    expect(friendlyLoginError(200, '{"code":"phone_timeout"}')).toContain("phone");
    expect(friendlyLoginError(429, '{"code":"rate_limited"}')).toContain("Too many");
  });

  it("drops the code prefix from rejected phone answers", () => {
    const reply = JSON.stringify({
      status: "error",
      code: "phone_session_invalid",
      error: "phone_session_invalid: VTOP rejected your phone's session.",
    });
    expect(friendlyLoginError(200, reply)).toBe("VTOP rejected your phone's session.");
  });
});

describe("login redirects", () => {
  it("detects VTOP's login page", () => {
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/login")).toBe(true);
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/initialProcess")).toBe(true);
    expect(isLoginRedirect("https://vtop.vitap.ac.in/vtop/content")).toBe(false);
  });
});
