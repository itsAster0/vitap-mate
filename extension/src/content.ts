// Runs on VTOP's pages: the sign-in card that shows the current step, auto sign-in, signing in again
// after VTOP signs the browser out, and the buttons that start a sign-in. This file is a classic
// script, so it imports only types from shared modules (runtime imports would emit a module chunk).

import type { BackgroundMessage, LoginReply, PageMessage } from "./messages";

const VTOP_ROOT_PATHS = ["/vtop", "/vtop/"];
const VTOP_CONTENT_PATH = "/vtop/content";
const COOLDOWN_MS = 15_000;
const LAUNCHER_ID = "vtop-mate-panel-launcher";
const INLINE_ID = "vtop-mate-signin";

type Tone = "active" | "ok" | "error";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** Local runtime guard matching `messages.ts` so this file stays a standalone classic script. */
function isPageMessage(value: unknown): value is PageMessage {
  if (!isRecord(value) || typeof value.type !== "string") return false;
  switch (value.type) {
    case "VTOP_MATE_LOADING":
      return (
        typeof value.text === "string" &&
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

function isLoginReply(value: unknown): value is LoginReply {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  return value.ok ? true : typeof value.error === "string";
}

// ---- the page badge ---------------------------------------------------------------------------

/** A cookie with `n` scallops, as the popup draws it (shapes.ts). */
function cookiePath(n: number, depth = 0.09) {
  const points: string[] = [];
  for (let i = 0; i <= 180; i++) {
    const t = (i / 180) * Math.PI * 2;
    const r = 46 * (1 - depth + depth * Math.cos(n * t));
    points.push(`${(50 + r * Math.cos(t)).toFixed(2)} ${(50 + r * Math.sin(t)).toFixed(2)}`);
  }
  return `M${points.join(" L")}Z`;
}

// Lucide 0.469.0 (ISC): loader-circle, check, triangle-alert, x.
const GLYPHS: Record<Tone | "x", string> = {
  active: '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
  ok: '<path d="M20 6 9 17l-5-5"/>',
  error:
    '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
};
const svg = (inner: string, size: number) =>
  `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${inner}</svg>`;

const BADGE_CSS = `
:host { all: initial; }
.badge {
  --primary:#4D44E3; --on-primary:#fff; --pc:#E2DFFF; --opc:#3323CC;
  --ok:#2E6B32; --okc:#CDEFC9; --ookc:#0F3D14; --err:#BF0715; --errc:#FFDAD6; --oerrc:#93000B;
  position: fixed; top: 14px; right: 14px; z-index: 2147483647; width: 300px; box-sizing: border-box;
  display: flex; gap: 12px; align-items: center; padding: 12px; border-radius: 22px;
  font: 13px/1.35 Inter, "Google Sans Text", system-ui, sans-serif; -webkit-font-smoothing: antialiased;
  box-shadow: 0 8px 24px -6px rgba(20,16,60,.35); background: var(--pc); color: var(--opc);
  transition: opacity .25s, transform .25s; opacity: 1; transform: none;
}
@media (prefers-color-scheme: dark) {
  .badge { --primary:#C3C0FF; --on-primary:#1D00A5; --pc:#3323CC; --opc:#E2DFFF;
    --ok:#A6D9A2; --okc:#1F4D25; --ookc:#CDEFC9; --err:#FFB4AB; --errc:#93000B; --oerrc:#FFDAD6; }
}
.badge.hidden { opacity: 0; transform: translateY(-8px); pointer-events: none; }
.badge.ok { background: var(--okc); color: var(--ookc); }
.badge.error { background: var(--errc); color: var(--oerrc); }
.cookie { width: 40px; height: 40px; flex: none; position: relative; display: grid; place-items: center; color: var(--on-primary); }
.cookie svg.shape { position: absolute; inset: 0; width: 100%; height: 100%; }
.cookie svg.shape path { fill: var(--primary); }
.ok .cookie svg.shape path { fill: var(--ok); }
.ok .cookie { color: var(--okc); }
.error .cookie svg.shape path { fill: var(--err); }
.error .cookie { color: var(--errc); }
.cookie > svg:not(.shape) { position: relative; }
.active .cookie svg.shape { animation: spin 9s linear infinite; }
.active .cookie > svg:not(.shape) { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .cookie svg { animation: none !important; } }
.txt { flex: 1; min-width: 0; }
.t { font-weight: 700; font-size: 13.5px; }
.s { font-size: 12px; opacity: .85; margin-top: 1px; }
.s:empty { display: none; }
button { all: unset; cursor: pointer; width: 30px; height: 30px; border-radius: 99px; display: grid; place-items: center; opacity: .75; flex: none; }
button:hover { opacity: 1; background: color-mix(in srgb, currentColor 10%, transparent); }
button:focus-visible { outline: 2px solid currentColor; }
`;

let badge: { root: HTMLElement; title: HTMLElement; sub: HTMLElement; glyph: HTMLElement } | undefined;
let tone: Tone = "active";
let serverDueAt: number | undefined;
let hideTimer: number | undefined;
let countdown: number | undefined;
/** The sign-in this tab started is a "sign in again" one, for its last words. */
let resigning = false;

function ensureBadge() {
  if (badge) return badge;
  const host = document.createElement("div");
  host.id = "vtop-mate-loading";
  const shadow = host.attachShadow({ mode: "closed" });
  shadow.innerHTML = `<style>${BADGE_CSS}</style>
<div class="badge hidden" role="status" aria-live="polite">
  <div class="cookie"><svg class="shape" viewBox="0 0 100 100" aria-hidden="true"><path d="${cookiePath(7)}"/></svg><span class="glyph"></span></div>
  <div class="txt"><div class="t"></div><div class="s"></div></div>
  <button type="button" aria-label="Close">${svg(GLYPHS.x, 16)}</button>
</div>`;
  const root = shadow.querySelector<HTMLElement>(".badge")!;
  shadow.querySelector("button")!.addEventListener("click", () => {
    // While signing in, the cross cancels it; after, it only closes the card.
    if (tone === "active") sendToBackground({ type: "CANCEL_LOGIN" });
    hideBadge(0);
  });
  document.documentElement.appendChild(host);
  badge = {
    root,
    title: shadow.querySelector<HTMLElement>(".t")!,
    sub: shadow.querySelector<HTMLElement>(".s")!,
    glyph: shadow.querySelector<HTMLElement>(".glyph")!,
  };
  return badge;
}

function subText() {
  if (tone !== "active" || serverDueAt === undefined) return "";
  const left = Math.ceil((serverDueAt - Date.now()) / 1000);
  return left > 0 ? `Server takes over in ${left} s` : "";
}

function showBadge(text: string, next: Tone, sub = "") {
  window.clearTimeout(hideTimer);
  hideTimer = undefined;
  const b = ensureBadge();
  if (tone !== next || b.glyph.childElementCount === 0) {
    b.glyph.outerHTML = `<span class="glyph" style="display:grid">${svg(GLYPHS[next], 18)}</span>`;
    b.glyph = b.root.querySelector<HTMLElement>(".glyph")!;
  }
  tone = next;
  b.root.className = `badge ${next}`;
  b.title.textContent = text.replace(/…$/, "");
  b.sub.textContent = sub || subText();
  window.clearInterval(countdown);
  if (next === "active" && serverDueAt !== undefined) {
    countdown = window.setInterval(() => (b.sub.textContent = subText()), 1000);
  }
}

function hideBadge(delay = 1200) {
  if (!badge) return;
  window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    hideTimer = undefined;
    window.clearInterval(countdown);
    badge?.root.classList.add("hidden");
  }, delay);
}

// ---- talking to the extension -----------------------------------------------------------------

/**
 * After the extension is reloaded or updated, the copy of this script already in open tabs loses
 * its link to the extension, and every `chrome.*` call throws "Extension context invalidated".
 */
function extensionAlive() {
  try {
    return Boolean(chrome.runtime?.id);
  } catch {
    return false;
  }
}

function staleTab() {
  document.getElementById(LAUNCHER_ID)?.remove();
  showBadge("VITAP Mate was updated", "error", "Refresh this page to use it.");
  hideBadge(4000);
}

/** Messages the background, or asks for a reload if this tab is stale. */
function sendToBackground(message: BackgroundMessage, onReply?: (reply: unknown) => void) {
  if (!extensionAlive()) return staleTab();
  try {
    chrome.runtime.sendMessage(message, (reply: unknown) => {
      // Reading lastError stops Chrome logging it when nobody answers.
      void chrome.runtime.lastError;
      onReply?.(reply);
    });
  } catch {
    staleTab();
  }
}

async function stored(keys: string[]): Promise<Record<string, unknown> | null> {
  try {
    return await chrome.storage.local.get(keys);
  } catch {
    return null;
  }
}

function startSignIn(source: "content" | "resign", onDone?: (ok: boolean) => void) {
  resigning = source === "resign";
  sendToBackground({ type: "LOGIN_WITH_TOKEN", source }, (response) => {
    if (!isLoginReply(response)) {
      showBadge("Sign-in was interrupted", "error", "Try again.");
      hideBadge(3500);
      onDone?.(false);
      return;
    }
    onDone?.(response.ok);
  });
}

// ---- when to sign in by itself ----------------------------------------------------------------

const onVtop = () => window.location.hostname === "vtop.vitap.ac.in";
const isRootPage = () => onVtop() && VTOP_ROOT_PATHS.includes(window.location.pathname);
const isOutsideContent = () => onVtop() && window.location.pathname !== VTOP_CONTENT_PATH;
/** VTOP sends a browser it signed out to its login (or open) page, as bridge.ts's isLoginRedirect. */
const isLoginPage = () =>
  onVtop() &&
  (window.location.pathname.startsWith("/vtop/login") ||
    window.location.pathname.startsWith("/vtop/initialProcess"));

/** Takes the shared 15 s cooldown, so a page that keeps bouncing doesn't loop. */
async function takeCooldown(lastAutoLoginAt: unknown) {
  const now = Date.now();
  if (now - Number(lastAutoLoginAt ?? 0) < COOLDOWN_MS) return false;
  try {
    await chrome.storage.local.set({ lastAutoLoginAt: now });
    return true;
  } catch {
    return false;
  }
}

async function maybeSignIn() {
  if (!extensionAlive() || (!isRootPage() && !isLoginPage())) return;
  const s = await stored([
    "token",
    "autoLogin",
    "reSignIn",
    "lastAutoLoginAt",
    "lastLogin",
    "signedOutAt",
  ]);
  if (!s || !String(s.token ?? "").trim()) return;

  if (isRootPage()) {
    if (!s.autoLogin || !(await takeCooldown(s.lastAutoLoginAt))) return;
    showBadge("Signing in with VITAP Mate", "active");
    startSignIn("content");
    return;
  }

  // Signed out by VTOP, not by the student: only after a sign-in of ours, and not after a logout click.
  const lastAt = Number((s.lastLogin as { at?: number } | undefined)?.at ?? 0);
  if (s.reSignIn === false || !lastAt || Number(s.signedOutAt ?? 0) > lastAt) return;
  if (!(await takeCooldown(s.lastAutoLoginAt))) return;
  showBadge("Signing in again", "active", "VTOP signed you out");
  startSignIn("resign");
}

/** A logout the student clicked stops "Sign in again" until the next sign-in. */
function watchLogout() {
  document.addEventListener(
    "click",
    (event) => {
      const el = (event.target as Element | null)?.closest?.("a, button, input[type=submit], form");
      if (!el) return;
      const said = [
        el.getAttribute("href"),
        el.getAttribute("onclick"),
        el.getAttribute("action"),
        el.id,
        el.textContent?.slice(0, 40),
      ]
        .join(" ")
        .toLowerCase();
      if (!/log\s?out|sign\s?out/.test(said) || !extensionAlive()) return;
      chrome.storage.local.set({ signedOutAt: Date.now() }).catch(() => {});
    },
    true,
  );
}

// ---- buttons on VTOP --------------------------------------------------------------------------

/**
 * Puts a "VITAP Mate" button next to the sign-in button on VTOP's Student card. VTOP loads that
 * card after the page, so this runs again until the button is mounted.
 */
function addStudentCardButton(): boolean {
  if (!extensionAlive()) return false;
  if (document.getElementById(INLINE_ID)) return true;
  const studentButton = document.querySelector<HTMLButtonElement>("#stdForm button[type=submit]");
  if (!studentButton) return false;

  const button = document.createElement("button");
  button.id = INLINE_ID;
  button.type = "button";
  button.title = "Sign in with VITAP Mate";
  button.setAttribute("aria-label", "Sign in with VITAP Mate");
  // Same size as VTOP's button so both fit side by side on the card.
  button.className = "btn fw-bold ms-1";
  Object.assign(button.style, {
    background: "#4D44E3",
    borderColor: "#4D44E3",
    color: "#ffffff",
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    verticalAlign: "top",
    width: `${studentButton.offsetWidth || 40}px`,
    height: `${studentButton.offsetHeight || 38}px`,
    padding: "0",
  });

  const icon = document.createElement("img");
  icon.src = chrome.runtime.getURL("icons/icon-32.png");
  icon.alt = "";
  Object.assign(icon.style, { width: "22px", height: "22px", borderRadius: "6px" });
  button.append(icon);

  // The card is a link that submits VTOP's own form; keep the click here.
  button.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();
    const s = await stored(["token"]);
    if (!s) return staleTab();
    if (!String(s.token ?? "").trim()) {
      sendToBackground({ type: "OPEN_EXTENSION_PANEL" });
      return;
    }
    button.disabled = true;
    button.style.opacity = "0.6";
    showBadge("Signing in with VITAP Mate", "active");
    startSignIn("content", () => {
      button.disabled = false;
      button.style.opacity = "";
    });
  });

  studentButton.insertAdjacentElement("afterend", button);
  document.getElementById(LAUNCHER_ID)?.remove();
  return true;
}

function createPanelLauncher() {
  if (
    !isOutsideContent() ||
    document.getElementById(LAUNCHER_ID) ||
    document.getElementById(INLINE_ID)
  ) {
    return;
  }

  const button = document.createElement("button");
  button.id = LAUNCHER_ID;
  button.type = "button";
  button.textContent = "Open VITAP Mate";
  Object.assign(button.style, {
    position: "fixed",
    right: "14px",
    bottom: "14px",
    zIndex: "2147483647",
    border: "0",
    borderRadius: "16px",
    background: "#4D44E3",
    color: "#ffffff",
    height: "48px",
    padding: "0 20px",
    font: '600 14px Inter, "Google Sans Text", system-ui, sans-serif',
    boxShadow: "0 8px 24px -6px rgba(20,16,60,.45)",
    cursor: "pointer",
  });
  button.addEventListener("click", () => sendToBackground({ type: "OPEN_EXTENSION_PANEL" }));
  document.documentElement.appendChild(button);
}

chrome.runtime.onMessage.addListener((message: unknown) => {
  if (!isPageMessage(message)) return;

  if (message.type === "VTOP_MATE_LOADING") {
    serverDueAt = message.serverDueAt;
    if (message.tone === "ok") {
      serverDueAt = undefined;
      if (resigning) showBadge("Signed in again", "ok", "VTOP had signed you out");
      else showBadge(message.text || "Signed in", "ok");
      resigning = false;
      hideBadge(3000);
    } else {
      showBadge(message.text || "Signing in", "active");
    }
    return;
  }

  if (message.type === "VTOP_MATE_ERROR") {
    serverDueAt = undefined;
    resigning = false;
    const text = message.text || "Sign-in failed";
    const cut = text.indexOf(". ");
    showBadge(
      cut > 0 ? text.slice(0, cut) : text.replace(/\.$/, ""),
      "error",
      cut > 0 ? text.slice(cut + 2) : "",
    );
    hideBadge(6000);
    return;
  }

  hideBadge();
});

const mounted = addStudentCardButton();
createPanelLauncher();
watchLogout();
void maybeSignIn();

if (!mounted && isOutsideContent()) {
  const observer = new MutationObserver(() => {
    if (!extensionAlive() || addStudentCardButton()) {
      observer.disconnect();
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
}
