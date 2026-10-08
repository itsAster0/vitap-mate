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
  if (!isVtopRootPage()) return;

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

  chrome.runtime.sendMessage(
    { type: "LOGIN_WITH_TOKEN", source: "content" },
    (response) => {
      if (!response?.ok) {
        showOverlay(`Auto login failed: ${response?.error ?? "Unknown error"}`);
        hideOverlay(3500);
      }
    },
  );
}

function createPanelLauncher() {
  if (!isVtopPageOutsideContent()) return;

  if (document.getElementById("vtop-mate-panel-launcher")) return;

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
    chrome.runtime.sendMessage({
      type: "OPEN_EXTENSION_PANEL",
    });
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

createPanelLauncher();
maybeAutoLogin();
