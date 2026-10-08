import {
  fetchWithTimeout,
  friendlyLoginError,
  interpretSessionResponse,
  isAccessKey,
  isLoginRedirect,
  type CookieEditorCookie,
  validateCookies,
} from "./bridge";

const API_BASE_URL = "https://vtop-bridge-production.up.railway.app";
const VTOP_DOMAIN = "vtop.vitap.ac.in";
const VTOP_CONTENT_URL = "https://vtop.vitap.ac.in/vtop/content";

type LoginMessage = {
  type: "LOGIN_WITH_TOKEN";
  source?: "popup" | "content";
};

type OpenPanelMessage = {
  type: "OPEN_EXTENSION_PANEL";
};

type BackgroundMessage = LoginMessage | OpenPanelMessage;

const POLL_TIMEOUT_MS = 120000;
const START_TIMEOUT_MS = 15000;
const STATUS_TIMEOUT_MS = 10000;
let loginInFlight: Promise<void> | null = null;

function getCookieUrl(
  cookie: Pick<CookieEditorCookie, "domain" | "path" | "secure">,
) {
  const cleanDomain = cookie.domain.replace(/^\./, "");
  const path = cookie.path ?? "/";
  const protocol = cookie.secure === false ? "http" : "https";

  return `${protocol}://${cleanDomain}${path}`;
}

function normalizeSameSite(value: CookieEditorCookie["sameSite"]) {
  if (!value) return "unspecified";

  switch (value) {
    case "lax":
      return "lax";
    case "strict":
      return "strict";
    case "no_restriction":
      return "no_restriction";
    case "unspecified":
    default:
      return "unspecified";
  }
}

function sleep(milliseconds: number) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
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

async function clearVtopCookies(cookies: chrome.cookies.Cookie[]) {
  for (const cookie of cookies) {
    const cleanDomain = cookie.domain.replace(/^\./, "");
    const protocol = cookie.secure ? "https" : "http";

    const urls = [
      `${protocol}://${cleanDomain}${cookie.path}`,
      `${protocol}://${cleanDomain}/`,
      `https://${cleanDomain}${cookie.path}`,
      `https://${cleanDomain}/vtop`,
      `https://${cleanDomain}/`,
    ];

    for (const url of urls) {
      try {
        const removed = await chrome.cookies.remove({
          url,
          name: cookie.name,
          storeId: cookie.storeId,
        });

        if (removed) {
          break;
        }
      } catch (err) {
        console.warn("VITAP Mate: cookie removal failed", err);
      }
    }
  }
}

async function setCookieFromCookieEditorFormat(cookie: CookieEditorCookie) {
  if (!cookie.name) throw new Error("Cookie missing name");
  if (!cookie.domain) throw new Error(`Cookie ${cookie.name} missing domain`);
  if (cookie.value === undefined || cookie.value === null) {
    throw new Error(`Cookie ${cookie.name} missing value`);
  }

  const url = getCookieUrl(cookie);

  const details: chrome.cookies.SetDetails = {
    url,
    name: cookie.name,
    value: cookie.value,
    path: cookie.path ?? "/vtop",
    secure: cookie.secure ?? true,
    httpOnly: cookie.httpOnly ?? true,
    sameSite: normalizeSameSite(cookie.sameSite),
    storeId: cookie.storeId,
  };

  // Cookie-Editor hostOnly:true means do NOT pass domain.
  if (!cookie.hostOnly) {
    details.domain = cookie.domain;
  }

  // Session cookie means no expirationDate.
  if (!cookie.session && cookie.expirationDate) {
    details.expirationDate = cookie.expirationDate;
  }

  const result = await chrome.cookies.set(details);

  if (!result) {
    throw new Error(`Chrome failed to set cookie: ${cookie.name}`);
  }
}

function chromeCookieToEditorCookie(cookie: chrome.cookies.Cookie): CookieEditorCookie {
  return {
    domain: cookie.domain,
    hostOnly: cookie.hostOnly,
    httpOnly: cookie.httpOnly,
    name: cookie.name,
    path: cookie.path,
    sameSite: cookie.sameSite,
    secure: cookie.secure,
    session: cookie.session,
    storeId: cookie.storeId,
    value: cookie.value,
    expirationDate: cookie.expirationDate,
  };
}

async function restoreCookies(cookies: chrome.cookies.Cookie[]) {
  const current = await chrome.cookies.getAll({ domain: VTOP_DOMAIN });
  await clearVtopCookies(current);
  for (const cookie of cookies) {
    await setCookieFromCookieEditorFormat(chromeCookieToEditorCookie(cookie));
  }
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

async function getSession(key: string, tabId: number | undefined) {
  const headers = { Authorization: `Bearer ${key}` };

  let response: Response;
  try {
    response = await fetchWithTimeout(
      `${API_BASE_URL}/v1/session`,
      { method: "POST", headers },
      START_TIMEOUT_MS,
    );
  } catch (err) {
    throw new Error(friendlyLoginError(0, "", err));
  }

  let outcome = await readSessionOutcome(response);
  const deadline = Date.now() + POLL_TIMEOUT_MS;

  while (outcome.kind === "pending" && Date.now() < deadline) {
    const requestId = outcome.requestId;
    if (!requestId) {
      throw new Error("The sign-in service sent an unexpected reply.");
    }

    await sendToTab(tabId, {
      type: "VTOP_MATE_LOADING",
      text: "Waiting for your phone...",
    });
    await sleep(outcome.pollAfterMs);

    let statusResponse: Response;
    try {
      statusResponse = await fetchWithTimeout(
        `${API_BASE_URL}/v1/session/requests/${encodeURIComponent(requestId)}`,
        { method: "GET", headers },
        STATUS_TIMEOUT_MS,
      );
    } catch (err) {
      throw new Error(friendlyLoginError(0, "", err));
    }

    outcome = await readSessionOutcome(statusResponse);
  }

  if (outcome.kind === "ready") return outcome.cookies;
  if (outcome.kind === "error") throw new Error(outcome.message);

  throw new Error(friendlyLoginError(200, JSON.stringify({ code: "phone_timeout" })));
}

async function installCookies(cookies: CookieEditorCookie[], tabId: number | undefined) {
  validateCookies(cookies);

  await sendToTab(tabId, {
    type: "VTOP_MATE_LOADING",
    text: "Clearing old cookies...",
  });

  const previousCookies = await chrome.cookies.getAll({ domain: VTOP_DOMAIN });
  await clearVtopCookies(previousCookies);

  await sendToTab(tabId, {
    type: "VTOP_MATE_LOADING",
    text: "Setting cookies...",
  });

  try {
    for (const cookie of cookies) {
      await setCookieFromCookieEditorFormat(cookie);
    }
  } catch (err) {
    try {
      await restoreCookies(previousCookies);
    } catch (restoreError) {
      console.error("VITAP Mate: cookie rollback failed", restoreError);
    }
    throw err;
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

async function expireSession(key: string) {
  try {
    await fetchWithTimeout(
      `${API_BASE_URL}/v1/session/expire`,
      { method: "POST", headers: { Authorization: `Bearer ${key}` } },
      STATUS_TIMEOUT_MS,
    );
  } catch (err) {
    console.warn("VITAP Mate: session expire failed", err);
  }
}

async function performLogin(source: "popup" | "content" = "popup") {
  const tabId = await getActiveTabId();

  await sendToTab(tabId, {
    type: "VTOP_MATE_LOADING",
    text: "Logging in...",
  });

  const stored = await chrome.storage.local.get(["token", "fmcToken", "autoLogin"]);
  const key = String(stored.token ?? stored.fmcToken ?? "").trim();

  if (!key) {
    throw new Error("Add the access key from VITAP Mate → Connected apps first.");
  }

  if (!isAccessKey(key)) {
    throw new Error(
      "This looks like an old phone token. Create an access key in VITAP Mate → Connected apps and paste it here.",
    );
  }

  console.log("Login source:", source);

  let cookies = await getSession(key, tabId);
  await installCookies(cookies, tabId);

  if (await sessionBounced()) {
    await expireSession(key);
    cookies = await getSession(key, tabId);
    await installCookies(cookies, tabId);

    if (await sessionBounced()) {
      throw new Error(
        "VTOP rejected the session. Open VITAP Mate, refresh your login, and try again.",
      );
    }
  }

  await sendToTab(tabId, {
    type: "VTOP_MATE_LOADING",
    text: "Redirecting...",
  });

  await goToVtopContent(tabId);
}

async function runLogin(source: "popup" | "content" = "popup") {
  if (loginInFlight) return loginInFlight;
  loginInFlight = performLogin(source);
  try {
    await loginInFlight;
  } finally {
    loginInFlight = null;
  }
}

void chrome.sidePanel
  .setPanelBehavior({ openPanelOnActionClick: true })
  .catch((err) => console.warn("Unable to configure the extension side panel", err));

chrome.runtime.onMessage.addListener(
  (message: BackgroundMessage, sender, sendResponse) => {
    if (message.type === "OPEN_EXTENSION_PANEL") {
      const tabId = sender.tab?.id;
      if (!tabId) return;

      chrome.sidePanel.open({ tabId }).catch((err) => {
        console.warn("Unable to open the extension side panel", err);
      });

      return;
    }

    if (message.type !== "LOGIN_WITH_TOKEN") return;

    runLogin(message.source ?? "popup")
      .then(() => {
        sendResponse({ ok: true });
      })
      .catch(async (err) => {
        const tabId = await getActiveTabId();

        await sendToTab(tabId, {
          type: "VTOP_MATE_ERROR",
          text: err instanceof Error ? err.message : "Unknown login error",
        });

        console.error(err);

        sendResponse({
          ok: false,
          error: err instanceof Error ? err.message : "Unknown login error",
        });
      });

    return true;
  },
);
