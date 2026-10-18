const VTOP_ROOT_PATHS = ["/vtop", "/vtop/"];
const VTOP_CONTENT_PATH = "/vtop/content";

function createOverlay() {
  let overlay = document.getElementById("vtop-mate-loading");

  if (overlay) return overlay;

  overlay = document.createElement("div");
  overlay.id = "vtop-mate-loading";

  overlay.style.position = "fixed";
  overlay.style.top = "12px";
  overlay.style.right = "12px";
  overlay.style.zIndex = "2147483647";
  overlay.style.background = "#09090b";
  overlay.style.color = "#fafafa";
  overlay.style.border = "1px solid #3f3f46";
  overlay.style.borderRadius = "8px";
  overlay.style.padding = "10px 14px";
  overlay.style.fontSize = "13px";
  overlay.style.fontFamily = "system-ui, sans-serif";
  overlay.style.boxShadow = "0 10px 25px rgba(0,0,0,0.35)";
  overlay.style.display = "none";

  document.documentElement.appendChild(overlay);

  return overlay;
}

function showOverlay(text: string) {
  const overlay = createOverlay();

  overlay.textContent = text;
  overlay.style.display = "block";
}

function hideOverlay(delay = 1200) {
  const overlay = document.getElementById("vtop-mate-loading");

  if (!overlay) return;

  window.setTimeout(() => {
    overlay.style.display = "none";
  }, delay);
}

/**
 * After the extension is reloaded or updated, the copy of this script
 * already in open tabs loses its link to the extension, and every
 * `chrome.*` call throws "Extension context invalidated".
 */
function extensionAlive() {
  try {
    return Boolean(chrome.runtime?.id);
  } catch {
    return false;
  }
}

/** Messages the background, or asks for a reload if this tab is stale. */
function sendToBackground(message: unknown, onReply?: (reply: any) => void) {
  if (!extensionAlive()) {
    document.getElementById("vtop-mate-panel-launcher")?.remove();
    showOverlay("VITAP Mate was updated. Refresh this page to use it.");
    hideOverlay(4000);
    return;
  }
  try {
    chrome.runtime.sendMessage(message, (reply) => {
      // Reading lastError stops Chrome logging it when nobody answers.
      void chrome.runtime.lastError;
      onReply?.(reply);
    });
  } catch {
    showOverlay("VITAP Mate was updated. Refresh this page to use it.");
    hideOverlay(4000);
  }
}

function isVtopRootPage() {
  return (
    window.location.hostname === "vtop.vitap.ac.in" &&
    window.location.pathname !== VTOP_CONTENT_PATH &&
    VTOP_ROOT_PATHS.includes(window.location.pathname)
  );
}

function isVtopPageOutsideContent() {
  return (
    window.location.hostname === "vtop.vitap.ac.in" &&
    window.location.pathname !== VTOP_CONTENT_PATH
  );
}

async function maybeAutoLogin() {
  if (!isVtopRootPage() || !extensionAlive()) return;

  const result = await chrome.storage.local.get([
    "token",
    "fmcToken",
    "autoLogin",
    "lastAutoLoginAt",
  ]);

  const token = String(result.token ?? result.fmcToken ?? "").trim();
  const autoLogin = Boolean(result.autoLogin);
  const lastAutoLoginAt = Number(result.lastAutoLoginAt ?? 0);

  if (!token || !autoLogin) return;

  const now = Date.now();

  if (now - lastAutoLoginAt < 15_000) {
    console.log("VITAP Mate: skipping auto-login cooldown");
    return;
  }

  await chrome.storage.local.set({ lastAutoLoginAt: now });
  showOverlay("Auto login running...");

  sendToBackground({ type: "LOGIN_WITH_TOKEN", source: "content" }, (response) => {
    if (!response?.ok) {
      showOverlay(`Auto login failed: ${response?.error ?? "Unknown error"}`);
      hideOverlay(3500);
    }
  });
}

const INLINE_ID = "vtop-mate-signin";

/**
 * Puts a "VITAP Mate" button next to the sign-in button on VTOP's Student
 * card. VTOP loads that card after the page, so this runs again whenever
 * the page changes.
 */
function addStudentCardButton() {
  if (!extensionAlive() || document.getElementById(INLINE_ID)) return;
  const studentButton = document.querySelector<HTMLButtonElement>(
    "#stdForm button[type=submit]",
  );
  if (!studentButton) return;

  const button = document.createElement("button");
  button.id = INLINE_ID;
  button.type = "button";
  button.title = "Sign in with VITAP Mate";
  button.setAttribute("aria-label", "Sign in with VITAP Mate");
  // Same size as VTOP's button so both fit side by side on the card.
  button.className = "btn fw-bold ms-1";
  button.style.background = "#4f46e5";
  button.style.borderColor = "#6366f1";
  button.style.color = "#ffffff";
  button.style.display = "inline-flex";
  button.style.alignItems = "center";
  button.style.justifyContent = "center";
  button.style.verticalAlign = "top";
  button.style.width = `${studentButton.offsetWidth || 40}px`;
  button.style.height = `${studentButton.offsetHeight || 38}px`;
  button.style.padding = "0";

  const icon = document.createElement("img");
  icon.src = chrome.runtime.getURL("icons/icon-32.png");
  icon.alt = "";
  icon.style.width = "22px";
  icon.style.height = "22px";
  icon.style.borderRadius = "4px";
  button.append(icon);

  // The card is a link that submits VTOP's own form; keep the click here.
  button.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (!extensionAlive()) {
      sendToBackground({});
      return;
    }
    const stored = await chrome.storage.local.get(["token", "fmcToken"]);
    if (!String(stored.token ?? stored.fmcToken ?? "").trim()) {
      sendToBackground({ type: "OPEN_EXTENSION_PANEL" });
      return;
    }
    button.disabled = true;
    button.style.opacity = "0.6";
    showOverlay("Signing in with VITAP Mate…");
    sendToBackground({ type: "LOGIN_WITH_TOKEN", source: "content" }, (response) => {
      button.disabled = false;
      button.style.opacity = "";
      if (!response?.ok) hideOverlay(3500);
    });
  });

  studentButton.insertAdjacentElement("afterend", button);
  document.getElementById("vtop-mate-panel-launcher")?.remove();
}

function createPanelLauncher() {
  if (!isVtopPageOutsideContent()) return;

  if (
    document.getElementById("vtop-mate-panel-launcher") ||
    document.getElementById(INLINE_ID)
  ) {
    return;
  }

  const button = document.createElement("button");
  button.id = "vtop-mate-panel-launcher";
  button.type = "button";
  button.textContent = "Open VITAP Mate";
  button.style.position = "fixed";
  button.style.right = "12px";
  button.style.bottom = "12px";
  button.style.zIndex = "2147483647";
  button.style.border = "1px solid #6366f1";
  button.style.borderRadius = "10px";
  button.style.background = "#312e81";
  button.style.color = "#ffffff";
  button.style.padding = "10px 14px";
  button.style.font = "600 13px system-ui, sans-serif";
  button.style.boxShadow = "0 10px 25px rgba(0,0,0,0.35)";
  button.style.cursor = "pointer";
  button.addEventListener("click", () => {
    sendToBackground({ type: "OPEN_EXTENSION_PANEL" });
  });

  document.documentElement.appendChild(button);
}

chrome.runtime.onMessage.addListener((message) => {
  if (message.type === "VTOP_MATE_LOADING") {
    showOverlay(message.text ?? "Loading...");
  }

  if (message.type === "VTOP_MATE_ERROR") {
    showOverlay(message.text ?? "Error");
    hideOverlay(3500);
  }

  if (message.type === "VTOP_MATE_HIDE_LOADING") {
    hideOverlay();
  }
});

addStudentCardButton();
createPanelLauncher();
maybeAutoLogin();

if (isVtopPageOutsideContent()) {
  new MutationObserver(() => addStudentCardButton()).observe(document.documentElement, {
    childList: true,
    subtree: true,
  });
}
