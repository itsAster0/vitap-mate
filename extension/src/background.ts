import {
  DEFAULT_BRIDGE_URL,
  fetchWithTimeout,
  friendlyLoginError,
  interpretSessionResponse,
  isAccessKey,
  isLoginRedirect,
  normalizeBridgeUrl,
  progressText,
  type CookieEditorCookie,
  type SessionSource,
} from "./bridge";
import { VTOP_DOMAIN, installCookies } from "./cookies";

const VTOP_CONTENT_URL = "https://vtop.vitap.ac.in/vtop/content";

type LoginMessage = {
  type: "LOGIN_WITH_TOKEN";
  source?: "popup" | "content";
};

type OpenPanelMessage = {
  type: "OPEN_EXTENSION_PANEL";
};

type CancelMessage = {
  type: "CANCEL_LOGIN";
};

type StateRequest = {
  type: "GET_LOGIN_STATE";
};

type CheckRequest = {
  type: "CHECK_VTOP_SESSION";
};

type BackgroundMessage =
  | LoginMessage
  | OpenPanelMessage
  | CancelMessage
  | StateRequest
  | CheckRequest;

/** Where a sign-in is. `phone` and `server` are the two ways a session is made. */
export type LoginPhase =
  | "idle"
  | "check"
  | "request"
  | "phone"
  | "server"
  | "cookies"
  | "open"
  | "done"
  | "error";

export type LoginState = {
  phase: LoginPhase;
  /** The step's text, as shown on the VTOP page too. */
  text: string;
  /** Which way the session came, once known. */
  via?: "browser" | "phone" | "server";
  /** Where the session the server handed out came from. */
  source?: SessionSource;
  startedAt?: number;
  finishedAt?: number;
  /** When the wait for a session gives up (ms). */
  deadline?: number;
  /** When the server takes over from a phone that hasn't answered (ms). */
  serverDueAt?: number;
  error?: string;
  /** The step that failed, so the popup can mark it. */
  failedPhase?: LoginPhase;
  /** What happened, with times, for the popup's activity list. */
  log: { at: number; text: string }[];
};

/** Sent to the popup on every change. */
export type StateMessage = { type: "VTOP_MATE_STATE"; state: LoginState };

/** The last finished sign-in, kept for the popup. */
export type LastLogin = { at: number; via: string; ms: number; source?: SessionSource };

const POLL_TIMEOUT_MS = 120000;
const START_TIMEOUT_MS = 15000;
const STATUS_TIMEOUT_MS = 10000;
const CANCELLED = "Sign-in cancelled.";
let loginInFlight: Promise<void> | null = null;
let cancel: AbortController | null = null;
let state: LoginState = { phase: "idle", text: "", log: [] };

/** Waits, but stops early when the sign-in is cancelled. */
function sleep(milliseconds: number, signal: AbortSignal) {
  return new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    signal.addEventListener("abort", () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

function throwIfCancelled(signal: AbortSignal) {
  if (signal.aborted) throw new Error(CANCELLED);
}

async function getActiveTabId() {
  const tabs = await chrome.tabs.query({
    active: true,
    currentWindow: true,
  });

  return tabs[0]?.id;
}

async function sendToTab(tabId: number | undefined, message: unknown) {
  if (!tabId) return;

  try {
    await chrome.tabs.sendMessage(tabId, message);
  } catch {
    // Content script may not be loaded on the current tab. Ignore.
  }
}

async function broadcast() {
  const message: StateMessage = { type: "VTOP_MATE_STATE", state };
  try {
    await chrome.runtime.sendMessage(message);
  } catch {
    // The popup is closed. It asks for the state when it opens.
  }
}

/**
 * Moves the sign-in on: updates the state the popup draws, logs a new
 * step, and shows `text` on the VTOP page.
 */
async function report(
  tabId: number | undefined,
  text: string,
  change: Partial<LoginState> = {},
) {
  const newStep = change.phase !== undefined && change.phase !== state.phase;
  state = { ...state, ...change, text };
  if (newStep || state.log.length === 0) {
    state.log = [...state.log, { at: Date.now(), text }];
  }
  await sendToTab(tabId, { type: "VTOP_MATE_LOADING", text });
  await broadcast();
}

async function goToVtopContent(tabId?: number) {
  if (tabId) {
    await chrome.tabs.update(tabId, {
      url: VTOP_CONTENT_URL,
    });
  } else {
    await chrome.tabs.create({
      url: VTOP_CONTENT_URL,
    });
  }
}

async function readSessionOutcome(response: Response) {
  const text = await response.text();

  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(friendlyLoginError(response.status, text));
  }

  return interpretSessionResponse(response.status, json);
}

async function getSession(
  base: string,
  key: string,
  tabId: number | undefined,
  signal: AbortSignal,
) {
  const headers = { Authorization: `Bearer ${key}` };

  await report(tabId, "Asking VITAP Mate for a session…", { phase: "request" });
  let response: Response;
  try {
    response = await fetchWithTimeout(
      `${base}/v1/session`,
      { method: "POST", headers },
      START_TIMEOUT_MS,
    );
  } catch (err) {
    throw new Error(friendlyLoginError(0, "", err));
  }

  let outcome = await readSessionOutcome(response);
  const deadline = Date.now() + POLL_TIMEOUT_MS;

  while (outcome.kind === "pending" && Date.now() < deadline) {
    throwIfCancelled(signal);
    const requestId = outcome.requestId;
    if (!requestId) {
      throw new Error("The sign-in service sent an unexpected reply.");
    }

    const phase = outcome.stage === "starting" ? "request" : outcome.stage;
    await report(tabId, progressText(outcome.stage, outcome.serverDueAt), {
      phase,
      via: phase === "request" ? state.via : phase,
      deadline,
      serverDueAt: outcome.serverDueAt,
    });
    await sleep(outcome.pollAfterMs, signal);
    throwIfCancelled(signal);

    let statusResponse: Response;
    try {
      statusResponse = await fetchWithTimeout(
        `${base}/v1/session/requests/${encodeURIComponent(requestId)}`,
        { method: "GET", headers },
        STATUS_TIMEOUT_MS,
      );
    } catch (err) {
      throw new Error(friendlyLoginError(0, "", err));
    }

    outcome = await readSessionOutcome(statusResponse);
  }

  if (outcome.kind === "ready") {
    const { from, cached } = outcome.source;
    const via = from === "unknown" ? state.via : from;
    const text = cached
      ? `Reusing a session your ${from === "unknown" ? "account" : from} made earlier`
      : `Got a new session from your ${from === "server" ? "server sign-in" : "phone"}`;
    await report(tabId, text, { phase: "cookies", via, source: outcome.source });
    return outcome.cookies;
  }
  if (outcome.kind === "error") throw new Error(outcome.message);

  throw new Error(friendlyLoginError(200, JSON.stringify({ code: "phone_timeout" })));
}

async function install(cookies: CookieEditorCookie[], tabId: number | undefined) {
  await report(tabId, "Setting the VTOP cookies…", { deadline: undefined });
  await installCookies(chrome.cookies, cookies, (text) => report(tabId, text));
}

/** Whether VTOP still accepts the browser's cookies. */
async function browserSignedIn() {
  const cookies = await chrome.cookies.getAll({ domain: VTOP_DOMAIN });
  if (cookies.length === 0) return false;
  try {
    const check = await fetchWithTimeout(
      VTOP_CONTENT_URL,
      { credentials: "include", redirect: "follow" },
      STATUS_TIMEOUT_MS,
    );
    return check.ok && !isLoginRedirect(check.url);
  } catch {
    return false;
  }
}

async function sessionBounced() {
  try {
    const check = await fetch(VTOP_CONTENT_URL, {
      credentials: "include",
      redirect: "follow",
    });
    return isLoginRedirect(check.url);
  } catch {
    return false;
  }
}

async function expireSession(base: string, key: string) {
  try {
    await fetchWithTimeout(
      `${base}/v1/session/expire`,
      { method: "POST", headers: { Authorization: `Bearer ${key}` } },
      STATUS_TIMEOUT_MS,
    );
  } catch (err) {
    console.warn("VITAP Mate: session expire failed", err);
  }
}

async function performLogin(signal: AbortSignal) {
  const tabId = await getActiveTabId();

  const stored = await chrome.storage.local.get(["token", "fmcToken", "bridgeUrl"]);
  const key = String(stored.token ?? stored.fmcToken ?? "").trim();
  // A server set in the popup, else the built-in one.
  const base = normalizeBridgeUrl(String(stored.bridgeUrl ?? "")) ?? DEFAULT_BRIDGE_URL;

  if (!key) {
    throw new Error("Add the access key from VITAP Mate → Connected apps first.");
  }

  if (!isAccessKey(key)) {
    throw new Error(
      "This looks like an old phone token. Create an access key in VITAP Mate → Connected apps and paste it here.",
    );
  }

  // Still signed in from last time: no need to ask the phone or the server.
  await report(tabId, "Checking Chrome's VTOP session…", { phase: "check" });
  if (await browserSignedIn()) {
    await report(tabId, "Chrome is still signed in. Opening VTOP…", { phase: "open", via: "browser" });
    await goToVtopContent(tabId);
    return;
  }
  throwIfCancelled(signal);

  let cookies = await getSession(base, key, tabId, signal);
  await install(cookies, tabId);

  if (await sessionBounced()) {
    await expireSession(base, key);
    cookies = await getSession(base, key, tabId, signal);
    await install(cookies, tabId);

    if (await sessionBounced()) {
      throw new Error(
        "VTOP rejected the session. Open VITAP Mate, refresh your login, and try again.",
      );
    }
  }

  await report(tabId, "Opening VTOP…", { phase: "open" });
  await goToVtopContent(tabId);
}

async function runLogin() {
  if (loginInFlight) return loginInFlight;
  cancel = new AbortController();
  const startedAt = Date.now();
  state = { phase: "check", text: "Starting…", startedAt, log: [] };
  loginInFlight = performLogin(cancel.signal);
  try {
    await loginInFlight;
    const finishedAt = Date.now();
    state = { ...state, phase: "done", finishedAt, deadline: undefined };
    state.log = [...state.log, { at: finishedAt, text: "Signed in" }];
    const last: LastLogin = {
      at: finishedAt,
      via: state.via ?? "phone",
      ms: finishedAt - startedAt,
      source: state.source,
    };
    await chrome.storage.local.set({ lastLogin: last });
  } catch (err) {
    const text = err instanceof Error ? err.message : "Unknown login error";
    const finishedAt = Date.now();
    state = {
      ...state,
      phase: "error",
      failedPhase: state.phase,
      error: text,
      finishedAt,
      deadline: undefined,
    };
    state.log = [...state.log, { at: finishedAt, text: text === CANCELLED ? "Cancelled" : "Failed" }];
    throw err;
  } finally {
    loginInFlight = null;
    cancel = null;
    await broadcast();
  }
}


chrome.runtime.onMessage.addListener(
  (message: BackgroundMessage, sender, sendResponse) => {
    if (message.type === "OPEN_EXTENSION_PANEL") {
      // Opens the toolbar popup (Chrome 127+); older Chrome ignores it.
      chrome.action.openPopup({ windowId: sender.tab?.windowId }).catch((err) => {
        console.warn("Unable to open the popup", err);
      });
      return;
    }

    if (message.type === "CANCEL_LOGIN") {
      cancel?.abort();
      return;
    }

    if (message.type === "GET_LOGIN_STATE") {
      sendResponse(state);
      return;
    }

    if (message.type === "CHECK_VTOP_SESSION") {
      browserSignedIn().then((signedIn) => sendResponse({ signedIn }));
      return true;
    }

    if (message.type !== "LOGIN_WITH_TOKEN") return;

    runLogin()
      .then(() => {
        sendResponse({ ok: true });
      })
      .catch(async (err) => {
        const tabId = await getActiveTabId();
        const text = err instanceof Error ? err.message : "Unknown login error";

        await sendToTab(tabId, { type: "VTOP_MATE_ERROR", text });

        if (text !== CANCELLED) console.error(err);

        sendResponse({ ok: false, error: text });
      });

    return true;
  },
);
