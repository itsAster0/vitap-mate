// Shared message contract for background <-> popup <-> content, with runtime guards.
// content.ts imports only types from here so it stays a classic (non-module) script.

import type { Release } from "./live";
import { isLoginState, type LoginState } from "./state";

export type LoginSource = "popup" | "content" | "resign";

export type LoginMessage = {
  type: "LOGIN_WITH_TOKEN";
  /** `resign`: the page script, after VTOP signed the browser out. */
  source?: LoginSource;
};

/** Sent from the popup or content script to the background worker. */
export type BackgroundMessage =
  | LoginMessage
  | { type: "OPEN_EXTENSION_PANEL" }
  | { type: "CANCEL_LOGIN" }
  | { type: "GET_LOGIN_STATE" }
  | { type: "CHECK_VTOP_SESSION" }
  | { type: "RELEASE_SEEN"; release: Release | null };

/** Sent from the background worker to the popup on every state change. */
export type StateMessage = { type: "VTOP_MATE_STATE"; state: LoginState };

/** Sent from the background worker to the VTOP tab. */
export type PageMessage =
  | { type: "VTOP_MATE_LOADING"; text: string; hint?: string; serverDueAt?: number; tone?: "active" | "ok" }
  | { type: "VTOP_MATE_ERROR"; text: string }
  | { type: "VTOP_MATE_HIDE_LOADING" };

/** Every runtime message exchanged between background, popup, and content script. */
export type ExtensionMessage = BackgroundMessage | StateMessage | PageMessage;

export type LoginReply = { ok: true } | { ok: false; error: string };
export type CheckVtopSessionReply = { signedIn: boolean };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

export function isRelease(value: unknown): value is Release {
  if (!isRecord(value)) return false;
  return (
    typeof value.version === "string" &&
    typeof value.sha === "string" &&
    isNullableString(value.subject) &&
    typeof value.url === "string" &&
    isNullableString(value.zipUrl) &&
    isNullableString(value.xpiUrl) &&
    typeof value.publishedAt === "number" &&
    typeof value.checkedAt === "number"
  );
}

export function isBackgroundMessage(value: unknown): value is BackgroundMessage {
  if (!isRecord(value) || typeof value.type !== "string") return false;
  switch (value.type) {
    case "OPEN_EXTENSION_PANEL":
    case "CANCEL_LOGIN":
    case "GET_LOGIN_STATE":
    case "CHECK_VTOP_SESSION":
      return true;
    case "LOGIN_WITH_TOKEN":
      return (
        value.source === undefined ||
        value.source === "popup" ||
        value.source === "content" ||
        value.source === "resign"
      );
    case "RELEASE_SEEN":
      return value.release === null || isRelease(value.release);
    default:
      return false;
  }
}

export function isStateMessage(value: unknown): value is StateMessage {
  return isRecord(value) && value.type === "VTOP_MATE_STATE" && isLoginState(value.state);
}

export function isPageMessage(value: unknown): value is PageMessage {
  if (!isRecord(value) || typeof value.type !== "string") return false;
  switch (value.type) {
    case "VTOP_MATE_LOADING":
      return (
        typeof value.text === "string" &&
        (value.hint === undefined || typeof value.hint === "string") &&
        (value.serverDueAt === undefined || typeof value.serverDueAt === "number") &&
        (value.tone === undefined || value.tone === "active" || value.tone === "ok")
      );
    case "VTOP_MATE_ERROR":
      return typeof value.text === "string";
    case "VTOP_MATE_HIDE_LOADING":
      return true;
    default:
      return false;
  }
}

export function isExtensionMessage(value: unknown): value is ExtensionMessage {
  return isBackgroundMessage(value) || isStateMessage(value) || isPageMessage(value);
}

export function isLoginReply(value: unknown): value is LoginReply {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  return value.ok ? true : typeof value.error === "string";
}

export function isCheckVtopSessionReply(value: unknown): value is CheckVtopSessionReply {
  return isRecord(value) && typeof value.signedIn === "boolean";
}
