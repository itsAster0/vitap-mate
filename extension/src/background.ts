import {
  DEFAULT_BRIDGE_URL,
  LoginError,
  fetchWithTimeout,
  friendlyLoginError,
  interpretSessionResponse,
  isAccessKey,
  isLoginError,
  isLoginRedirect,
  normalizeBridgeUrl,
  progressText,
  type CookieEditorCookie,
} from "./bridge";
import { VTOP_DOMAIN, installCookies } from "./cookies";
import { BROWSER_NAME } from "./format";
import {
  CANCELLED,
  UNREACHABLE_MESSAGE,
  openLive,
  releaseFromGitHub,
  updateAvailable,
  waitForRequest,
  type LiveClient,
  type Release,
} from "./live";
import {
  isBackgroundMessage,
  type CheckVtopSessionReply,
  type LoginReply,
  type PageMessage,
  type StateMessage,
} from "./messages";
import {
  applyReportStep,
  createIdleState,
  handedOverOf,
  loadLoginState,
  saveLoginState,
  sourceOf,
  viaOf,
  type LastLogin,
  type LoginPhase,
  type LoginState,
  type ReportStep,
  type Settings,
} from "./state";

export type { LastLogin, LoginPhase, LoginState, PageMessage, StateMessage };

const VTOP_CONTENT_URL = "https://vtop.vitap.ac.in/vtop/content";
const START_TIMEOUT_MS = 15000;
const STATUS_TIMEOUT_MS = 10000;
/** Chrome stops a quiet service worker after 30 s, socket or not; a call every 20 s keeps a
 * sign-in that waits on the phone or the OTP alive (Firefox's event page likewise). */
const KEEPALIVE_MS = 20_000;
const UPDATE_ALARM = "check-update";

/** Result of probing VTOP's content page with the browser's current cookies. */
export type VtopProbe = "signed_in" | "bounced" | "unreachable";

/** Runs async tasks strictly one after another; a failure in one never stalls later tasks. */
export function createSerialQueue() {
  let tail: Promise<void> = Promise.resolve();
  return function enqueue<T>(task: () => Promise<T>): Promise<T> {
    const run = tail.then(task, task);
    tail = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  };
}

export type BackgroundDeps = {
  openLive?: (
    base: string,
    key?: string,
    onTrouble?: (why: "key_unknown" | "unreachable") => void,
  ) => LiveClient;
  releaseFromGitHub?: typeof releaseFromGitHub;
  browserName?: string;
  now?: () => number;
  keepaliveMs?: number;
};

export function createBackground(deps: BackgroundDeps = {}) {
  const openSocket = deps.openLive ?? openLive;
  const fetchRelease = deps.releaseFromGitHub ?? releaseFromGitHub;
  const browser = deps.browserName ?? BROWSER_NAME;
  const now = deps.now ?? (() => Date.now());
  const keepaliveMs = deps.keepaliveMs ?? KEEPALIVE_MS;

  let loginInFlight: Promise<void> | null = null;
  let cancel: AbortController | null = null;
  let state: LoginState = createIdleState();
  const enqueueReport = createSerialQueue();

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

  async function broadcast(snapshot: LoginState) {
    const message: StateMessage = { type: "VTOP_MATE_STATE", state: snapshot };
    try {
      await chrome.runtime.sendMessage(message);
    } catch {
      // The popup is closed. It asks for the state when it opens.
    }
  }

  /** Serializes every state mutation, persistence write, tab update, and popup broadcast. */
  function commitState(
    update: (current: LoginState) => { next: LoginState; pageMessage?: PageMessage },
    tabId?: number,
  ): Promise<LoginState> {
    return enqueueReport(async () => {
      const { next, pageMessage } = update(state);
      state = next;
      await saveLoginState(next);
      if (pageMessage) await sendToTab(tabId, pageMessage);
      await broadcast(next);
      return next;
    });
  }

  /**
   * Moves the sign-in on: updates the state the popup draws, logs a new
   * step, and shows `text` on the VTOP page. Serialized so concurrent updates
   * can never interleave and persist stale state.
   */
  function report(
    tabId: number | undefined,
    text: string,
    change: ReportStep | ((current: LoginState) => ReportStep) = {},
    extraLog?: (current: LoginState) => string | undefined,
  ): Promise<LoginState> {
    return commitState((current) => {
      const step = typeof change === "function" ? change(current) : change;
      const next = applyReportStep(current, step, text, now(), extraLog?.(current));
      return {
        next,
        pageMessage: {
          type: "VTOP_MATE_LOADING",
          text,
          serverDueAt: next.phase === "phone" ? next.serverDueAt : undefined,
          tone: "active",
        },
      };
    }, tabId);
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
    if (signal.aborted) throw new LoginError("cancelled", CANCELLED);
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
      chrome.notifications.create(id, {
        type: "basic",
        iconUrl: "icons/icon-128.png",
        title,
        message,
      });
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
    throwIfCancelled(signal);
    let response: Response;
    try {
      response = await fetchWithTimeout(
        `${base}/v1/session`,
        { method: "POST", headers, signal },
        START_TIMEOUT_MS,
      );
    } catch (err) {
      throwIfCancelled(signal);
      throw new Error(friendlyLoginError(0, "", err));
    }
    throwIfCancelled(signal);

    let outcome = await readSessionOutcome(response);
    throwIfCancelled(signal);
    if (outcome.kind === "pending") {
      const requestId = outcome.requestId;
      if (!requestId) throw new Error("The sign-in service sent an unexpected reply.");
      // A failed token fetch leaves the socket signed out, where the request reads as an unknown key.
      let unreachable = false;
      const client = openSocket(base, key, (why) => (unreachable ||= why === "unreachable"));
      let askedPhone = false;
      try {
        await waitForRequest(client, requestId, signal, (view) => {
          const phase = view.stage === "starting" ? "request" : view.stage;
          const serverDueAt = view.serverDueAt ?? undefined;
          if (phase === "phone" && !askedPhone) {
            askedPhone = true;
            void notify(
              "phone",
              "Approve on your phone",
              `${browser} is asking for a VTOP session. Open VITAP Mate on your phone.`,
            );
          }
          // No countdown in the text: the popup and the page count down from `serverDueAt` themselves.
          void report(
            tabId,
            progressText(view.stage, undefined, now(), view.login ?? undefined),
            (current): ReportStep => {
              if (phase === "phone") {
                return { phase: "phone", via: "phone", deadline: view.expiresAt, serverDueAt };
              }
              if (phase === "server") {
                return {
                  phase: "server",
                  via: "server",
                  deadline: view.expiresAt,
                  loginStep: view.login ?? undefined,
                  handedOver: askedPhone || handedOverOf(current),
                };
              }
              return { phase: "request", via: viaOf(current), deadline: view.expiresAt };
            },
            (current) =>
              phase === "server" && askedPhone && current.phase === "phone"
                ? "Your phone didn't answer; the server takes over"
                : undefined,
          );
        });
      } catch (err) {
        if (unreachable && isLoginError(err, "key_unknown")) {
          throw new LoginError("unreachable", UNREACHABLE_MESSAGE);
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
          { method: "GET", headers, signal },
          STATUS_TIMEOUT_MS,
        );
      } catch (err) {
        throwIfCancelled(signal);
        throw new Error(friendlyLoginError(0, "", err));
      }
      throwIfCancelled(signal);
      outcome = await readSessionOutcome(ready);
    }

    if (outcome.kind === "ready") {
      const { from, cached } = outcome.source;
      const text = cached
        ? `Reusing a session your ${from === "unknown" ? "account" : from} made earlier`
        : `Got a new session from your ${from === "server" ? "server sign-in" : "phone"}`;
      await report(tabId, text, (current): ReportStep => ({
        phase: "cookies",
        via: from === "unknown" ? viaOf(current) : from,
        source: outcome.source,
      }));
      return outcome.cookies;
    }
    if (outcome.kind === "error") throw new LoginError("failed", outcome.message);

    throw new LoginError("expired", friendlyLoginError(200, JSON.stringify({ code: "phone_timeout" })));
  }

  async function install(cookies: CookieEditorCookie[], tabId: number | undefined) {
    await report(tabId, "Setting the VTOP cookies…");
    await installCookies(chrome.cookies, cookies, async (text) => {
      await report(tabId, text);
    });
  }

  /** Probes VTOP's content page with the browser's current cookies. */
  async function probeVtopSession(): Promise<VtopProbe> {
    try {
      const check = await fetchWithTimeout(
        VTOP_CONTENT_URL,
        { credentials: "include", redirect: "follow" },
        STATUS_TIMEOUT_MS,
      );
      if (isLoginRedirect(check.url)) return "bounced";
      return check.ok ? "signed_in" : "unreachable";
    } catch {
      return "unreachable";
    }
  }

  /** Whether VTOP still accepts the browser's cookies. */
  async function browserSignedIn(): Promise<boolean> {
    const cookies = await chrome.cookies.getAll({ domain: VTOP_DOMAIN });
    if (cookies.length === 0) return false;
    return (await probeVtopSession()) === "signed_in";
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
    await report(tabId, `Checking ${browser}'s VTOP session…`, { phase: "check" });
    if (await browserSignedIn()) {
      await report(tabId, `${browser} is still signed in. Opening VTOP…`, {
        phase: "open",
        via: "browser",
      });
      await goToVtopContent(tabId);
      return;
    }
    throwIfCancelled(signal);

    let cookies = await getSession(base, key, tabId, signal);
    throwIfCancelled(signal);
    await install(cookies, tabId);

    if ((await probeVtopSession()) === "bounced") {
      throwIfCancelled(signal);
      await expireSession(base, key);
      cookies = await getSession(base, key, tabId, signal);
      throwIfCancelled(signal);
      await install(cookies, tabId);

      if ((await probeVtopSession()) === "bounced") {
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
    const startedAt = now();
    const keepalive = setInterval(
      () => chrome.runtime.getPlatformInfo().catch(() => {}),
      keepaliveMs,
    );

    const task = (async () => {
      await commitState(() => ({
        next: { phase: "check", text: "Starting…", startedAt, log: [] },
      }));
      try {
        await performLogin(signal, preferredTabId);
        const finishedAt = now();
        const done = await commitState((current) => ({
          next: {
            phase: "done",
            text: current.text,
            startedAt: current.phase === "idle" ? startedAt : current.startedAt,
            finishedAt,
            via: viaOf(current),
            source: sourceOf(current),
            handedOver: handedOverOf(current),
            log: [...current.log, { at: finishedAt, text: "Signed in" }],
          },
        }));
        const last: LastLogin = {
          at: finishedAt,
          via: viaOf(done) ?? "phone",
          ms: finishedAt - startedAt,
          source: sourceOf(done),
        };
        await chrome.storage.local.set({ lastLogin: last }).catch(() => {});
      } catch (err) {
        const text = err instanceof Error ? err.message : "Unknown login error";
        const cancelled = isLoginError(err, "cancelled");
        const finishedAt = now();
        await commitState((current) => ({
          next: {
            phase: "error",
            text: current.text,
            startedAt: current.phase === "idle" ? startedAt : current.startedAt,
            finishedAt,
            failedPhase: current.phase,
            error: text,
            cancelled: cancelled || undefined,
            via: viaOf(current),
            source: sourceOf(current),
            handedOver: handedOverOf(current),
            log: [...current.log, { at: finishedAt, text: cancelled ? "Cancelled" : "Failed", failed: true }],
          },
        }));
        if (!cancelled) void notify("failed", "Sign-in failed", `${text} Click to try again.`);
        throw err;
      } finally {
        clearInterval(keepalive);
        loginInFlight = null;
        cancel = null;
      }
    })();

    loginInFlight = task;
    return task;
  }

  // ---- updates ------------------------------------------------------------------------------

  /** Marks the toolbar icon when a newer build is out. */
  function showUpdate(release: Release | null) {
    const behind = updateAvailable(release);
    chrome.action.setBadgeText({ text: behind ? "1" : "" }).catch(() => {});
    if (behind) chrome.action.setBadgeBackgroundColor({ color: "#7A5368" }).catch(() => {});
  }

  /** Asks GitHub for the newest build. Says nothing when GitHub is blocked or down: an update notice
   * is never worth an error. The answer is kept for the popup. */
  async function checkForUpdate() {
    const release = await fetchRelease();
    if (!release) return;
    await chrome.storage.local.set({ release }).catch(() => {});
    showUpdate(release);
  }

  function openActionPopup(windowId?: number) {
    const openPopup = chrome.action?.openPopup;
    if (typeof openPopup !== "function") return;
    const call = (opts?: chrome.action.OpenPopupOptions) =>
      Promise.resolve().then(() => (opts ? openPopup(opts) : openPopup()));
    const attempt = windowId !== undefined ? call({ windowId }).catch(() => call()) : call();
    attempt.catch((err) => {
      console.warn("Unable to open the popup", err);
    });
  }

  function handleMessage(
    message: unknown,
    sender: chrome.runtime.MessageSender,
    sendResponse: (response?: LoginState | CheckVtopSessionReply | LoginReply) => void,
  ): boolean | undefined {
    if (!isBackgroundMessage(message)) return;

    if (message.type === "OPEN_EXTENSION_PANEL") {
      // Opens the toolbar popup when supported (Chrome 127+ takes { windowId }; Firefox takes no
      // arguments and older Chrome lacks openPopup altogether).
      openActionPopup(sender.tab?.windowId);
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

    runLogin(sender.tab?.id)
      .then(() => {
        sendResponse({ ok: true });
      })
      .catch(async (err) => {
        const text = err instanceof Error ? err.message : "Unknown login error";
        const cancelled = isLoginError(err, "cancelled");
        try {
          const tabId = sender.tab?.id ?? (await getActiveTabId());
          await sendToTab(
            tabId,
            cancelled ? { type: "VTOP_MATE_HIDE_LOADING" } : { type: "VTOP_MATE_ERROR", text },
          );
        } catch {
          // Best-effort badge update on the active tab.
        }

        if (!cancelled) console.error(err);

        sendResponse({ ok: false, error: text });
      });

    return true;
  }

  let notificationsHooked = false;
  function hookNotifications() {
    if (notificationsHooked || !chrome.notifications) return;
    notificationsHooked = true;
    chrome.notifications.onClicked.addListener((id) => {
      chrome.notifications.clear(id).catch(() => {});
      if (id === "failed") runLogin().catch(() => {});
    });
  }

  function register() {
    chrome.runtime.onInstalled.addListener(() => {
      chrome.alarms.create(UPDATE_ALARM, { periodInMinutes: 360 });
      void checkForUpdate();
    });
    chrome.runtime.onStartup.addListener(() => void checkForUpdate());
    chrome.alarms.onAlarm.addListener((alarm) => {
      if (alarm.name === UPDATE_ALARM) void checkForUpdate();
    });
    hookNotifications();
    chrome.permissions.onAdded.addListener(hookNotifications);
    chrome.runtime.onMessage.addListener(handleMessage);
  }

  return {
    register,
    handleMessage,
    report,
    runLogin,
    browserSignedIn,
    probeVtopSession,
    getState: () => state,
  };
}

if (typeof chrome !== "undefined" && chrome.runtime?.onMessage) {
  createBackground().register();
}
