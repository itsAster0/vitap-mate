import {
  DEFAULT_BRIDGE_URL,
  UNKNOWN_KEY_MESSAGE,
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
import {
  CANCELLED,
  UNREACHABLE_MESSAGE,
  openLive,
  queries,
  releaseFromGitHub,
  updateAvailable,
  waitForRequest,
  type Release,
} from "./live";
import {
  createIdleState,
  loadLoginState,
  saveLoginState,
  type LoginPhase,
  type LoginState,
  type Settings,
} from "./state";

export type { LoginPhase, LoginState };

const VTOP_CONTENT_URL = "https://vtop.vitap.ac.in/vtop/content";

type LoginMessage = {
  type: "LOGIN_WITH_TOKEN";
  /** `resign`: the page script, after VTOP signed the browser out. */
  source?: "popup" | "content" | "resign";
};

type BackgroundMessage =
  | LoginMessage
  | { type: "OPEN_EXTENSION_PANEL" }
  | { type: "CANCEL_LOGIN" }
  | { type: "GET_LOGIN_STATE" }
  | { type: "CHECK_VTOP_SESSION" }
  | { type: "RELEASE_SEEN"; release: Release | null };

/** Sent to the popup on every change. */
export type StateMessage = { type: "VTOP_MATE_STATE"; state: LoginState };

/** Sent to the VTOP tab: the step to show on the page. */
export type PageMessage =
  | { type: "VTOP_MATE_LOADING"; text: string; serverDueAt?: number; tone?: "active" | "ok" }
  | { type: "VTOP_MATE_ERROR"; text: string }
  | { type: "VTOP_MATE_HIDE_LOADING" };

/** The last finished sign-in, kept for the popup. */
export type LastLogin = { at: number; via: string; ms: number; source?: SessionSource };

const START_TIMEOUT_MS = 15000;
const STATUS_TIMEOUT_MS = 10000;
/** Chrome stops a quiet service worker after 30 s, socket or not; a call every 20 s keeps a
 * sign-in that waits on the phone or the OTP alive (Firefox's event page likewise). */
const KEEPALIVE_MS = 20_000;
let loginInFlight: Promise<void> | null = null;
let cancel: AbortController | null = null;
let state: LoginState = createIdleState();

async function getActiveTabId() {
  try {
    const tabs = await chrome.tabs.query({
      active: true,
      currentWindow: true,
    });

    return tabs[0]?.id;
  } catch {
    return undefined;
  }
}

async function sendToTab(tabId: number | undefined, message: PageMessage) {
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
  const newStep =
    (change.phase !== undefined && change.phase !== state.phase) ||
    (change.loginStep !== undefined && change.loginStep !== state.loginStep);
  state = { ...state, ...change, text };
  if (newStep || state.log.length === 0) {
    state.log = [...state.log, { at: Date.now(), text }];
  }
  await saveLoginState(state);
  await sendToTab(tabId, { type: "VTOP_MATE_LOADING", text, serverDueAt: state.serverDueAt, tone: "active" });
  await broadcast();
}

async function goToVtopContent(tabId?: number) {
  if (tabId) {
    try {
      await chrome.tabs.update(tabId, {
        url: VTOP_CONTENT_URL,
      });
      return;
    } catch {
      // Tab may have been closed while signing in; open a new one instead.
    }
  }
  await chrome.tabs.create({
    url: VTOP_CONTENT_URL,
  });
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

function throwIfCancelled(signal: AbortSignal) {
  if (signal.aborted) throw new Error(CANCELLED);
}

/** The settings' stored values. */
async function settings(): Promise<Settings> {
  return (await chrome.storage.local.get(null)) as Settings;
}

/** Shows a notification when the student turned them on and the browser allows it. */
async function notify(id: string, title: string, message: string) {
  try {
    if (!(await settings()).notify) return;
    if (!(await chrome.permissions.contains({ permissions: ["notifications"] }))) return;
    chrome.notifications.create(id, { type: "basic", iconUrl: "icons/icon-128.png", title, message });
  } catch {
    // Notifications are a nicety; the popup and the page still show it.
  }
}

/**
 * A session for the key: `POST /v1/session` answers at once when the server
 * has one to reuse; otherwise the request is followed live over the socket,
 * and the cookies are fetched once when it is ready.
 */
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
  if (outcome.kind === "pending") {
    const requestId = outcome.requestId;
    if (!requestId) throw new Error("The sign-in service sent an unexpected reply.");
    // A failed token fetch leaves the socket signed out, where the request reads as an unknown key.
    let unreachable = false;
    const client = openLive(base, key, (why) => (unreachable ||= why === "unreachable"));
    let askedPhone = false;
    try {
      await waitForRequest(client, requestId, signal, (view) => {
        const phase = view.stage === "starting" ? "request" : view.stage;
        const serverDueAt = view.serverDueAt ?? undefined;
        if (phase === "phone" && !askedPhone) {
          askedPhone = true;
          void notify("phone", "Approve on your phone", "Chrome is asking for a VTOP session. Open VITAP Mate on your phone.");
        }
        if (phase === "server" && askedPhone && state.phase === "phone") {
          state.log = [...state.log, { at: Date.now(), text: "Your phone didn't answer; the server takes over" }];
        }
        // No countdown in the text: the popup and the page count down from `serverDueAt` themselves.
        void report(tabId, progressText(view.stage, undefined, Date.now(), view.login ?? undefined), {
          phase,
          via: phase === "request" ? state.via : phase,
          deadline: view.expiresAt,
          serverDueAt,
          loginStep: view.login ?? undefined,
        });
      });
    } catch (err) {
      if (unreachable && err instanceof Error && err.message === UNKNOWN_KEY_MESSAGE) {
        throw new Error(UNREACHABLE_MESSAGE);
      }
      throw err;
    } finally {
      chrome.notifications?.clear("phone").catch(() => {});
      void client.close();
    }
    throwIfCancelled(signal);
    // Ready: the cookies come once, over HTTPS, never over the socket.
    let ready: Response;
    try {
      ready = await fetchWithTimeout(
        `${base}/v1/session/requests/${encodeURIComponent(requestId)}`,
        { method: "GET", headers },
        STATUS_TIMEOUT_MS,
      );
    } catch (err) {
      throw new Error(friendlyLoginError(0, "", err));
    }
    outcome = await readSessionOutcome(ready);
  }

  if (outcome.kind === "ready") {
    const { from, cached } = outcome.source;
    const via = from === "unknown" ? state.via : from;
    const text = cached
      ? `Reusing a session your ${from === "unknown" ? "account" : from} made earlier`
      : `Got a new session from your ${from === "server" ? "server sign-in" : "phone"}`;
    await report(tabId, text, { phase: "cookies", via, source: outcome.source, loginStep: undefined });
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
    const check = await fetchWithTimeout(
      VTOP_CONTENT_URL,
      { credentials: "include", redirect: "follow" },
      STATUS_TIMEOUT_MS,
    );
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

async function performLogin(signal: AbortSignal, preferredTabId?: number) {
  const tabId = preferredTabId ?? (await getActiveTabId());

  const stored = await settings();
  const key = String(stored.token ?? "").trim();
  // A server set in the popup, else the built-in one.
  const base = normalizeBridgeUrl(String(stored.bridgeUrl ?? "")) ?? DEFAULT_BRIDGE_URL;

  if (!isAccessKey(key)) {
    throw new Error("Pair the extension with VITAP Mate first.");
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
  throwIfCancelled(signal);
  await install(cookies, tabId);

  if (await sessionBounced()) {
    throwIfCancelled(signal);
    await expireSession(base, key);
    cookies = await getSession(base, key, tabId, signal);
    throwIfCancelled(signal);
    await install(cookies, tabId);

    if (await sessionBounced()) {
      throw new Error(
        "VTOP rejected the session. Open VITAP Mate, refresh your login, and try again.",
      );
    }
  }

  await report(tabId, "Opening VTOP…", { phase: "open" });
  await sendToTab(tabId, { type: "VTOP_MATE_LOADING", text: "Signed in", tone: "ok" });
  await goToVtopContent(tabId);
}

function runLogin(preferredTabId?: number) {
  if (loginInFlight) return loginInFlight;
  cancel = new AbortController();
  const signal = cancel.signal;
  const startedAt = Date.now();
  state = { phase: "check", text: "Starting…", startedAt, log: [] };
  const keepalive = setInterval(() => chrome.runtime.getPlatformInfo().catch(() => {}), KEEPALIVE_MS);

  const task = (async () => {
    await saveLoginState(state);
    await broadcast();
    try {
      await performLogin(signal, preferredTabId);
      const finishedAt = Date.now();
      state = {
        ...state,
        phase: "done",
        finishedAt,
        deadline: undefined,
        serverDueAt: undefined,
        loginStep: undefined,
      };
      state.log = [...state.log, { at: finishedAt, text: "Signed in" }];
      const last: LastLogin = {
        at: finishedAt,
        via: state.via ?? "phone",
        ms: finishedAt - startedAt,
        source: state.source,
      };
      await chrome.storage.local.set({ lastLogin: last }).catch(() => {});
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
        serverDueAt: undefined,
        loginStep: undefined,
      };
      state.log = [...state.log, { at: finishedAt, text: text === CANCELLED ? "Cancelled" : "Failed" }];
      if (text !== CANCELLED) void notify("failed", "Sign-in failed", `${text} Click to try again.`);
      throw err;
    } finally {
      clearInterval(keepalive);
      loginInFlight = null;
      cancel = null;
      await saveLoginState(state);
      await broadcast();
    }
  })();

  loginInFlight = task;
  return task;
}

// ---- updates --------------------------------------------------------------------------------

/** Marks the toolbar icon when a newer build is out. */
function showUpdate(release: Release | null) {
  const behind = updateAvailable(release);
  chrome.action.setBadgeText({ text: behind ? "1" : "" }).catch(() => {});
  if (behind) chrome.action.setBadgeBackgroundColor({ color: "#7A5368" }).catch(() => {});
}

/** Asks GitHub for the newest build. Says nothing when GitHub is blocked or down: an update notice
 * is never worth an error. The answer is kept for the popup. */
async function checkForUpdate() {
  const release = await releaseFromGitHub();
  if (!release) return;
  await chrome.storage.local.set({ release }).catch(() => {});
  showUpdate(release);
}

const UPDATE_ALARM = "check-update";
chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(UPDATE_ALARM, { periodInMinutes: 360 });
  void checkForUpdate();
});
chrome.runtime.onStartup.addListener(() => void checkForUpdate());
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === UPDATE_ALARM) void checkForUpdate();
});

/** `chrome.notifications` exists only once the optional permission is granted, which can be
 * while this worker runs. */
let notificationsHooked = false;
function hookNotifications() {
  if (notificationsHooked || !chrome.notifications) return;
  notificationsHooked = true;
  chrome.notifications.onClicked.addListener((id) => {
    chrome.notifications.clear(id).catch(() => {});
    if (id === "failed") runLogin().catch(() => {});
  });
}
hookNotifications();
chrome.permissions.onAdded.addListener(hookNotifications);

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

    if (message.type === "RELEASE_SEEN") {
      if (message.release) chrome.storage.local.set({ release: message.release }).catch(() => {});
      showUpdate(message.release);
      return;
    }

    if (message.type === "GET_LOGIN_STATE") {
      if (state.phase === "idle") {
        loadLoginState(loginInFlight !== null)
          .then((restored) => {
            if (state.phase === "idle" && restored) {
              state = restored;
            }
            sendResponse(state);
          })
          .catch(() => sendResponse(state));
        return true;
      }
      sendResponse(state);
      return;
    }

    if (message.type === "CHECK_VTOP_SESSION") {
      browserSignedIn()
        .then((signedIn) => sendResponse({ signedIn }))
        .catch(() => sendResponse({ signedIn: false }));
      return true;
    }

    if (message.type !== "LOGIN_WITH_TOKEN") return;

    runLogin(sender.tab?.id)
      .then(() => {
        sendResponse({ ok: true });
      })
      .catch(async (err) => {
        const text = err instanceof Error ? err.message : "Unknown login error";
        try {
          const tabId = sender.tab?.id ?? (await getActiveTabId());
          await sendToTab(tabId, { type: "VTOP_MATE_ERROR", text });
        } catch {
          // Best-effort error overlay on the active tab.
        }

        if (text !== CANCELLED) console.error(err);

        sendResponse({ ok: false, error: text });
      });

    return true;
  },
);
