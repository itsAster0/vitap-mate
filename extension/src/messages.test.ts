import { describe, expect, it } from "vitest";
import type { Release } from "./live";
import {
  isBackgroundMessage,
  isCheckVtopSessionReply,
  isExtensionMessage,
  isLoginReply,
  isPageMessage,
  isRelease,
  isStateMessage,
} from "./messages";
import { createIdleState } from "./state";

const RELEASE: Release = {
  version: "3.0.2",
  sha: "abcdef1",
  subject: "fix: extension",
  url: "https://github.com/itsAster0/vitap-mate/releases/tag/extension-latest",
  zipUrl: "https://example.com/extension.zip",
  xpiUrl: null,
  publishedAt: 2000,
  checkedAt: 3000,
};

describe("message runtime guards", () => {
  it("accepts every valid BackgroundMessage", () => {
    expect(isBackgroundMessage({ type: "OPEN_EXTENSION_PANEL" })).toBe(true);
    expect(isBackgroundMessage({ type: "CANCEL_LOGIN" })).toBe(true);
    expect(isBackgroundMessage({ type: "GET_LOGIN_STATE" })).toBe(true);
    expect(isBackgroundMessage({ type: "CHECK_VTOP_SESSION" })).toBe(true);
    expect(isBackgroundMessage({ type: "LOGIN_WITH_TOKEN" })).toBe(true);
    expect(isBackgroundMessage({ type: "LOGIN_WITH_TOKEN", source: "popup" })).toBe(true);
    expect(isBackgroundMessage({ type: "LOGIN_WITH_TOKEN", source: "content" })).toBe(true);
    expect(isBackgroundMessage({ type: "LOGIN_WITH_TOKEN", source: "resign" })).toBe(true);
    expect(isBackgroundMessage({ type: "RELEASE_SEEN", release: null })).toBe(true);
    expect(isBackgroundMessage({ type: "RELEASE_SEEN", release: RELEASE })).toBe(true);
  });

  it("rejects malformed or unknown BackgroundMessage payloads", () => {
    for (const bad of [
      null,
      undefined,
      "LOGIN_WITH_TOKEN",
      42,
      {},
      { type: "UNKNOWN" },
      { type: "LOGIN_WITH_TOKEN", source: "evil" },
      { type: "RELEASE_SEEN" },
      { type: "RELEASE_SEEN", release: { version: "1.0.0" } },
      { type: "VTOP_MATE_HIDE_LOADING" },
    ]) {
      expect(isBackgroundMessage(bad)).toBe(false);
    }
  });

  it("validates StateMessage with LoginState shape", () => {
    expect(isStateMessage({ type: "VTOP_MATE_STATE", state: createIdleState() })).toBe(true);
    expect(isStateMessage({ type: "VTOP_MATE_STATE", state: { phase: "bogus", text: "", log: [] } })).toBe(false);
    expect(isStateMessage({ type: "VTOP_MATE_STATE" })).toBe(false);
    expect(isStateMessage({ type: "CANCEL_LOGIN" })).toBe(false);
  });

  it("validates PageMessage variants and rejects invalid fields", () => {
    expect(isPageMessage({ type: "VTOP_MATE_LOADING", text: "Signing in" })).toBe(true);
    expect(
      isPageMessage({ type: "VTOP_MATE_LOADING", text: "Waiting", serverDueAt: 5000, tone: "active" }),
    ).toBe(true);
    expect(isPageMessage({ type: "VTOP_MATE_LOADING", text: "Signed in", tone: "ok" })).toBe(true);
    expect(isPageMessage({ type: "VTOP_MATE_ERROR", text: "Failed" })).toBe(true);
    expect(isPageMessage({ type: "VTOP_MATE_HIDE_LOADING" })).toBe(true);

    expect(isPageMessage({ type: "VTOP_MATE_LOADING", text: 123 })).toBe(false);
    expect(isPageMessage({ type: "VTOP_MATE_LOADING", text: "ok", tone: "error" })).toBe(false);
    expect(isPageMessage({ type: "VTOP_MATE_LOADING", text: "ok", serverDueAt: "soon" })).toBe(false);
    expect(isPageMessage({ type: "VTOP_MATE_ERROR" })).toBe(false);
    expect(isPageMessage({ type: "OTHER" })).toBe(false);
  });

  it("validates the combined union and reply helpers", () => {
    expect(isExtensionMessage({ type: "CANCEL_LOGIN" })).toBe(true);
    expect(isExtensionMessage({ type: "VTOP_MATE_STATE", state: createIdleState() })).toBe(true);
    expect(isExtensionMessage({ type: "VTOP_MATE_HIDE_LOADING" })).toBe(true);
    expect(isExtensionMessage({ type: "NOPE" })).toBe(false);

    expect(isRelease(RELEASE)).toBe(true);
    expect(isRelease({ ...RELEASE, publishedAt: "bad" })).toBe(false);

    expect(isLoginReply({ ok: true })).toBe(true);
    expect(isLoginReply({ ok: false, error: "boom" })).toBe(true);
    expect(isLoginReply({ ok: false })).toBe(false);
    expect(isLoginReply(null)).toBe(false);

    expect(isCheckVtopSessionReply({ signedIn: true })).toBe(true);
    expect(isCheckVtopSessionReply({ signedIn: false })).toBe(true);
    expect(isCheckVtopSessionReply({ signedIn: "yes" })).toBe(false);
  });
});
