<script lang="ts">
  import { encode } from "uqr";
  import { onDestroy } from "svelte";
  import type { ConvexClient } from "convex/browser";
  import {
    DEFAULT_BRIDGE_URL,
    PAIR_TTL_MS,
    UNKNOWN_KEY_MESSAGE,
    interpretPairResponse,
    isAccessKey,
    newPairCode,
    normalizeBridgeUrl,
    type SessionSource,
  } from "./bridge";
  import type { LastLogin, StateMessage } from "./background";
  import type { LoginPhase, LoginState, Settings } from "./state";
  import {
    FIRST_ANSWER_MS,
    UNREACHABLE_MESSAGE,
    openLive,
    pairCodeHash,
    queries,
    releaseFromGitHub,
    updateAvailable,
    type KeyView,
    type Release,
  } from "./live";
  import { cookiePath, wavePath } from "./shapes";
  import type { IconName } from "./icons";
  import Icon from "./Icon.svelte";

  const VERSION = chrome.runtime.getManifest().version;
  const FIREFOX = navigator.userAgent.includes("Firefox");
  const COOKIE = cookiePath(7);
  const BLOB = cookiePath(9);
  const RING = cookiePath(12, 0.06);
  const WAVE = wavePath(300);

  let view = $state<"main" | "settings">("main");
  let token = $state("");
  let editingKey = $state(false);
  let autoLogin = $state(false);
  let reSignIn = $state(true);
  let notify = $state(false);
  let bridgeUrl = $state(DEFAULT_BRIDGE_URL);
  let savedBridgeUrl = $state(DEFAULT_BRIDGE_URL);
  let serverMessage = $state("");
  let serverError = $state("");
  let notice = $state("");
  /** The key's account, live; null once the server says the key is gone. */
  let account = $state<KeyView | null | undefined>(undefined);
  let keyRejected = $state(false);
  let unreachable = $state(false);
  let release = $state<Release | null>(null);
  // null while checking whether the browser is still signed in to VTOP.
  let vtopSignedIn = $state<boolean | null>(null);
  let lastLogin = $state<LastLogin | undefined>(undefined);
  let login = $state<LoginState>({ phase: "idle", text: "", log: [] });
  let now = $state(Date.now());
  const ticker = setInterval(() => (now = Date.now()), 250);
  onDestroy(() => clearInterval(ticker));

  function onMessage(message: StateMessage) {
    if (message?.type !== "VTOP_MATE_STATE") return;
    login = message.state;
    if (login.phase === "done") {
      vtopSignedIn = true;
      loadLastLogin();
    }
  }
  chrome.runtime.onMessage.addListener(onMessage);
  onDestroy(() => chrome.runtime.onMessage.removeListener(onMessage));

  async function loadLastLogin() {
    const stored = await chrome.storage.local.get("lastLogin");
    lastLogin = stored.lastLogin as LastLogin | undefined;
  }

  async function loadSettings() {
    const stored = (await chrome.storage.local.get(null)) as Settings & { lastLogin?: LastLogin };
    token = String(stored.token ?? "");
    autoLogin = Boolean(stored.autoLogin);
    reSignIn = stored.reSignIn !== false;
    notify =
      Boolean(stored.notify) && (await chrome.permissions.contains({ permissions: ["notifications"] }).catch(() => false));
    savedBridgeUrl = normalizeBridgeUrl(String(stored.bridgeUrl ?? "")) ?? DEFAULT_BRIDGE_URL;
    bridgeUrl = savedBridgeUrl;
    lastLogin = stored.lastLogin;
    editingKey = !isAccessKey(token);
    // A sign-in started before the popup was opened keeps going in the background.
    login = (await chrome.runtime.sendMessage({ type: "GET_LOGIN_STATE" })) ?? login;
    const check = await chrome.runtime.sendMessage({ type: "CHECK_VTOP_SESSION" });
    vtopSignedIn = Boolean(check?.signedIn);
  }

  async function saveSettings() {
    if (!token.trim()) autoLogin = false;
    const values: Settings = { token: token.trim(), autoLogin, reSignIn, notify };
    await chrome.storage.local.set(values);
  }

  // Called straight from the click: the browser shows its prompt only during a user gesture.
  async function toggleNotify() {
    if (notify) {
      notify = false;
    } else {
      notify = await chrome.permissions.request({ permissions: ["notifications"] }).catch(() => false);
    }
    await saveSettings();
  }

  // ---- the live link: the key's account and the newest build --------------------------------

  const liveKey = $derived(isAccessKey(token) ? token.trim() : undefined);
  let live = $state.raw<ConvexClient | undefined>(undefined);
  let liveTry = $state(0);

  $effect(() => {
    const base = savedBridgeUrl;
    const key = liveKey;
    void liveTry;
    keyRejected = false;
    unreachable = false;
    account = undefined;
    let signedIn = false;
    const client = openLive(
      base,
      key,
      (why) => {
        if (why === "key_unknown") keyRejected = true;
        else unreachable = true;
      },
      (yes) => (signedIn = yes),
    );
    // Only a key has something to watch; without one the socket waits for a pairing.
    const silent = key ? setTimeout(() => (unreachable = true), FIRST_ANSWER_MS) : undefined;
    const stops: (() => void)[] = [];
    if (key) {
      stops.push(
        client.onUpdate(
          queries.key,
          {},
          (v) => {
            clearTimeout(silent);
            unreachable = false;
            // Signed out (the token fetch failed), every key reads as unknown: only trust a signed-in null.
            if (v === null && !signedIn) return;
            account = v;
            keyRejected = v === null;
            if (v) editingKey = false;
          },
          () => (unreachable = true),
        ),
      );
    }
    live = client;
    return () => {
      clearTimeout(silent);
      for (const stop of stops) stop();
      void client.close();
    };
  });

  /** Shows the newest build GitHub has and tells the background, which keeps it and marks the
   * toolbar icon. GitHub blocked or down: the last answer stays, with no error. */
  function seeRelease(r: Release | null) {
    if (!r) return;
    release = r;
    chrome.runtime.sendMessage({ type: "RELEASE_SEEN", release: r }).catch(() => {});
  }

  chrome.storage.local.get("release").then(({ release: kept }) => (release ??= (kept as Release) ?? null), () => {});
  releaseFromGitHub().then(seeRelease);

  // ---- server -------------------------------------------------------------------------------

  // Called straight from a click: the browser only shows its permission prompt during a user gesture.
  async function saveServer() {
    serverMessage = "";
    serverError = "";
    const origin = normalizeBridgeUrl(bridgeUrl);
    if (!origin) {
      serverError = "Enter an https address, like https://vtop-bridge.example.com";
      return;
    }
    if (origin === DEFAULT_BRIDGE_URL) {
      await chrome.storage.local.remove("bridgeUrl");
    } else {
      const granted = await chrome.permissions.request({ origins: [`${origin}/*`] });
      if (!granted) {
        serverError = "Your browser needs your permission to reach that server.";
        return;
      }
      await chrome.storage.local.set({ bridgeUrl: origin });
    }
    bridgeUrl = savedBridgeUrl = origin;
    serverMessage = "Saved.";
  }

  async function useDefaultServer() {
    await chrome.storage.local.remove("bridgeUrl");
    bridgeUrl = savedBridgeUrl = DEFAULT_BRIDGE_URL;
    serverError = "";
    serverMessage = "Using the default server.";
  }

  // ---- pairing: the app makes a key for the code shown here --------------------------------

  let pairing = $state("");
  let pairUntil = $state(0);
  let pairError = $state("");
  const pairQr = $derived(pairing ? encode(pairing, { border: 2 }) : null);
  const pairQrPath = $derived(
    pairQr ? pairQr.data.flatMap((row, y) => row.map((on, x) => (on ? `M${x} ${y}h1v1h-1z` : ""))).join("") : "",
  );
  let pairTimers: ReturnType<typeof setTimeout>[] = [];
  let stopPairWatch: (() => void) | undefined;

  function stopPairing() {
    for (const t of pairTimers) clearTimeout(t);
    pairTimers = [];
    stopPairWatch?.();
    stopPairWatch = undefined;
    pairing = "";
  }
  onDestroy(stopPairing);

  /** Shows a new code and watches for the app to approve it, then takes the key once over HTTP. */
  async function startPairing() {
    stopPairing();
    pairError = "";
    const code = newPairCode();
    const until = Date.now() + PAIR_TTL_MS;
    pairing = code;
    pairUntil = until;
    const fail = (message: string) => {
      if (pairing !== code) return;
      stopPairing();
      pairError = message;
    };
    pairTimers.push(setTimeout(() => fail("The code ran out. Get a new one."), PAIR_TTL_MS));
    const silent = setTimeout(() => fail(UNREACHABLE_MESSAGE), FIRST_ANSWER_MS);
    pairTimers.push(silent);
    const codeHash = await pairCodeHash(code);
    if (pairing !== code || !live) return;
    stopPairWatch = live.onUpdate(
      queries.pairing,
      { codeHash },
      ({ ready }) => {
        clearTimeout(silent);
        if (ready && pairing === code) {
          stopPairWatch?.();
          stopPairWatch = undefined;
          void claimKey(code, until, fail);
        }
      },
      () => fail(UNREACHABLE_MESSAGE),
    );
  }

  async function claimKey(code: string, until: number, fail: (message: string) => void) {
    while (pairing === code && Date.now() < until) {
      let wait = 1000;
      try {
        const response = await fetch(`${savedBridgeUrl}/v1/pair`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ code }),
        });
        const outcome = interpretPairResponse(response.status, await response.json().catch(() => null));
        if (pairing !== code) return;
        if (outcome.kind === "ready") {
          stopPairing();
          token = outcome.key;
          await saveSettings();
          return;
        }
        if (outcome.kind === "error") return fail(outcome.message);
        wait = outcome.pollAfterMs;
      } catch {
        // Offline for a moment: try again until the code runs out.
      }
      await new Promise((r) => pairTimers.push(setTimeout(r, wait)));
    }
  }

  function pairAgain() {
    view = "main";
    editingKey = true;
    void startPairing();
  }

  // ---- signing in ---------------------------------------------------------------------------

  async function signIn() {
    if (!token.trim()) return;
    notice = "";
    await saveSettings();
    // The popup may close while this runs; the background keeps going and the next popup picks it up.
    await chrome.runtime.sendMessage({ type: "LOGIN_WITH_TOKEN", source: "popup" });
  }

  async function cancelLogin() {
    try {
      await chrome.runtime.sendMessage({ type: "CANCEL_LOGIN" });
    } catch {
      // Nothing running.
    }
  }

  async function resetState() {
    stopPairing();
    await chrome.storage.local.remove([
      "token",
      "autoLogin",
      "reSignIn",
      "notify",
      "lastAutoLoginAt",
      "signedOutAt",
      "bridgeUrl",
      "lastLogin",
    ]);
    bridgeUrl = savedBridgeUrl = DEFAULT_BRIDGE_URL;
    token = "";
    autoLogin = false;
    reSignIn = true;
    notify = false;
    lastLogin = undefined;
    editingKey = true;
    view = "main";
    notice = "Settings cleared.";
  }

  function openUpdate() {
    if (!release) return;
    const url = FIREFOX ? (release.xpiUrl ?? release.url) : release.url;
    void chrome.tabs.create({ url });
  }

  // ---- the route ----------------------------------------------------------------------------

  type Node = "chrome" | "edge" | "phone" | "server" | "vtop";
  type NodeLook = "idle" | "now" | "done" | "skip" | "err";

  const RANK: Record<LoginPhase, number> = {
    idle: -1,
    check: 0,
    request: 1,
    phone: 2,
    server: 2,
    cookies: 3,
    open: 4,
    done: 5,
    error: -1,
  };
  const NODE_RANK: Record<Node, number> = { chrome: 0, edge: 1, phone: 2, server: 2, vtop: 3 };

  function look(node: Node, s: LoginState): NodeLook {
    const failed = s.phase === "error";
    const phase = failed ? (s.failedPhase ?? "idle") : s.phase;
    const rank = RANK[phase];
    if (s.phase === "idle") return "idle";
    // The browser was still signed in: the session never left it.
    if (s.via === "browser" && node !== "chrome" && node !== "vtop") return "skip";
    if (node === "phone" || node === "server") {
      const used = s.via === node || phase === node;
      // The phone was asked first and the server took over.
      const handedOver =
        node === "phone" && (s.via === "server" || phase === "server") && s.log.some((e) => e.text.startsWith("Waiting for your phone"));
      if (rank < 2) return "idle";
      if (handedOver) return "skip";
      if (!used) return "skip";
    }
    const nodeRank = NODE_RANK[node];
    const active =
      (node === "vtop" && (phase === "cookies" || phase === "open")) ||
      (node !== "vtop" && nodeRank === Math.min(rank, 3) && phase !== "done");
    if (active) return failed ? "err" : "now";
    if (rank > nodeRank || s.phase === "done") return "done";
    return "idle";
  }

  /** The route to draw: the sign-in's, or the last one's while the browser is still signed in. */
  const shown = $derived<LoginState>(
    login.phase === "idle" && vtopSignedIn && lastLogin
      ? { phase: "done", text: "", via: lastLogin.source?.from === "server" ? "server" : (lastLogin.via as LoginState["via"]), log: [] }
      : login,
  );
  const looks = $derived({
    chrome: look("chrome", shown),
    edge: look("edge", shown),
    phone: look("phone", shown),
    server: look("server", shown),
    vtop: look("vtop", shown),
  });
  const forkLook = $derived(looks.server === "skip" ? looks.phone : looks.server);

  function linkClass(from: NodeLook, to: NodeLook) {
    if (from === "now") return "dash";
    return to === "done" || to === "now" || to === "err" ? "on" : "";
  }

  // ---- derived ------------------------------------------------------------------------------

  const running = $derived(!["idle", "done", "error"].includes(login.phase));
  const showKeyHint = $derived(token.trim().length > 0 && !isAccessKey(token));
  const secondsLeft = $derived(login.deadline && login.deadline > now ? Math.ceil((login.deadline - now) / 1000) : 0);
  const serverIn = $derived(
    login.phase === "phone" && login.serverDueAt && login.serverDueAt > now ? Math.ceil((login.serverDueAt - now) / 1000) : 0,
  );
  const elapsed = $derived(login.startedAt ? ((login.finishedAt ?? now) - login.startedAt) / 1000 : 0);
  const behind = $derived(updateAvailable(release));
  const pairLeft = $derived(Math.max(0, Math.ceil((pairUntil - now) / 1000)));

  function ago(at: number, atNow: number) {
    const s = Math.max(0, Math.round((atNow - at) / 1000));
    if (s < 60) return "just now";
    if (s < 3600) return `${Math.floor(s / 60)} min ago`;
    if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
    return `${Math.floor(s / 86400)} d ago`;
  }

  type Row = { icon: IconName; label: string; value: string; tone: "" | "good" | "warn" | "bad" };

  function accountRows(a: KeyView, atNow: number): Row[] {
    const rows: Row[] = [];
    const s = a.settings;
    const server = a.savedCredentials;
    let order: string;
    if (!server) order = a.phoneLinked ? "Phone only" : "No way to sign in";
    else if (s.alwaysUseVault) order = "Server first";
    else order = a.phoneLinked ? `Phone, then the server after ${s.phoneWaitSecs} s` : "Server";
    rows.push({ icon: "route", label: "Who signs in", value: order, tone: server || a.phoneLinked ? "" : "bad" });
    if (a.credentialsRejectedAt) {
      rows.push({
        icon: "shield-alert",
        label: "Server password",
        value: `Rejected ${ago(a.credentialsRejectedAt * 1000, atNow)}`,
        tone: "bad",
      });
    } else if (a.savedCredentials) {
      const saved = a.vaultSavedAt ? `Saved ${ago(a.vaultSavedAt * 1000, atNow)}` : "Saved";
      const ttl = s.vaultTtlSecs ? `, kept ${Math.round(s.vaultTtlSecs / 86400)} d` : "";
      rows.push({ icon: "shield-check", label: "Server password", value: saved + ttl, tone: "good" });
    } else {
      rows.push({ icon: "shield-off", label: "Server password", value: "Off", tone: "" });
    }
    if (a.phone) {
      const name = a.phone.label ?? "Phone";
      rows.push(
        a.phone.notifications
          ? { icon: "smartphone", label: "This key's phone", value: `${name} linked, notifications on`, tone: "good" }
          : { icon: "bell-off", label: "This key's phone", value: `${name} linked, notifications off`, tone: "warn" },
      );
    } else {
      rows.push({ icon: "smartphone", label: "This key's phone", value: "Not linked", tone: "warn" });
    }
    if (a.keyLastUsedAt) {
      rows.push({ icon: "history", label: "This key last used", value: ago(a.keyLastUsedAt * 1000, atNow), tone: "" });
    }
    return rows;
  }

  const VIA: Record<string, string> = { browser: "the browser's session", phone: "your phone", server: "the server" };

  /** "Cookies from your phone, made 2 h ago, reused". */
  function sourceText(via: string | undefined, source: SessionSource | undefined, atNow: number) {
    if (via === "browser") return "Already in the browser";
    if (!source) return via === "server" ? "Cookies from the server sign-in" : via === "phone" ? "Cookies from your phone" : "";
    const who = source.from === "server" ? "the server sign-in" : source.from === "phone" ? "your phone" : "a saved session";
    const made = source.madeAt ? `, made ${ago(source.madeAt, atNow)}` : "";
    return `Cookies from ${who}${made}, ${source.cached ? "reused" : "new"}`;
  }

  /** The hero: its tone, icon, headline and line under it. */
  const hero = $derived.by(() => {
    if (login.phase === "error") {
      const text = login.error ?? "The sign-in failed.";
      const cut = text.indexOf(". ");
      return {
        tone: "bad",
        icon: "triangle-alert" as IconName,
        title: cut > 0 ? text.slice(0, cut) : text.replace(/\.$/, ""),
        sub: cut > 0 ? text.slice(cut + 2) : "",
      };
    }
    if (login.phase === "done") {
      return { tone: "ok", icon: "check" as IconName, title: `Signed in via ${VIA[login.via ?? "phone"]}`, sub: "VTOP is opening in this tab" };
    }
    if (running) {
      const sub = login.text.replace(/…$/, "");
      switch (login.phase) {
        case "phone":
          return { tone: "active", icon: "smartphone" as IconName, title: "Waiting for your phone", sub: "Open VITAP Mate on your phone to approve" };
        case "server":
          if (login.loginStep === "waiting_otp") {
            return { tone: "active", icon: "mail-check" as IconName, title: "Waiting for the OTP", sub: "The server is signed in and waits for VTOP's email" };
          }
          if (login.loginStep === "checking_otp") {
            return { tone: "active", icon: "mail-check" as IconName, title: "Got the OTP", sub: "Finishing the sign-in" };
          }
          return { tone: "active", icon: "server" as IconName, title: "The server is signing in", sub };
        case "cookies":
          return { tone: "active", icon: "cookie" as IconName, title: "Setting the cookies", sub };
        case "open":
          return { tone: "active", icon: "external-link" as IconName, title: "Opening VTOP", sub };
        default:
          return { tone: "active", icon: "loader-circle" as IconName, title: "Signing in", sub };
      }
    }
    if (vtopSignedIn === null) return { tone: "", icon: "loader-circle" as IconName, title: "Checking VTOP", sub: "Is this browser still signed in?" };
    if (vtopSignedIn) {
      const sub = lastLogin
        ? `Last sign-in ${ago(lastLogin.at, now)}, ${lastLogin.source?.cached ? "reused from" : "via"} ${VIA[lastLogin.source?.from ?? lastLogin.via] ?? lastLogin.via} in ${(lastLogin.ms / 1000).toFixed(1)} s`
        : "This browser keeps the session until VTOP ends it";
      return { tone: "ok", icon: "check" as IconName, title: "Signed in to VTOP", sub };
    }
    return { tone: "", icon: "log-in" as IconName, title: "Not signed in yet", sub: "Your phone or the server makes a session for this browser" };
  });

  const showPairing = $derived(editingKey);

  loadSettings();
</script>

{#snippet toggle(on: boolean, disabled: boolean, label: string, change: () => void)}
  <button
    type="button"
    class="switch"
    class:on
    role="switch"
    aria-checked={on}
    aria-label={label}
    {disabled}
    onclick={change}
  ></button>
{/snippet}

{#snippet updateBanner()}
  {#if behind && release}
    <div class="banner update">
      <div class="ic"><Icon name="download" size={17} /></div>
      <div class="grow">
        <div class="bt">{release.version !== VERSION ? `Update ready: ${VERSION} → ${release.version}` : "Update ready"}</div>
        {#if release.subject}<div class="bs">{release.subject}</div>{/if}
        <div class="acts"><button type="button" class="text-btn strong" onclick={openUpdate}>Get the update</button></div>
      </div>
    </div>
  {/if}
{/snippet}

{#snippet trouble()}
  {#if unreachable}
    <div class="banner alert">
      <div class="ic"><Icon name="wifi-off" size={17} /></div>
      <div class="grow">
        <div class="bt">Can't reach VITAP Mate's server</div>
        <div class="bs">Check your connection. Signing in needs the server.</div>
        <div class="acts"><button type="button" class="text-btn strong" onclick={() => liveTry++}>Retry</button></div>
      </div>
    </div>
  {/if}
{/snippet}

<main class="popup">
  {#if view === "settings"}
    <div class="sub-bar">
      <button type="button" class="icon-btn" aria-label="Back" onclick={() => (view = "main")}><Icon name="arrow-left" /></button>
      <div class="title">Settings</div>
    </div>
    <div class="body">
      <div class="section-label">Signing in</div>
      <div class="card">
        <div class="item">
          <div class="lead"><Icon name="log-in" size={18} /></div>
          <div class="txt"><div class="t">Auto sign-in</div><div class="s">When you open vtop.vitap.ac.in</div></div>
          {@render toggle(autoLogin, !isAccessKey(token), "Auto sign-in", () => { autoLogin = !autoLogin; void saveSettings(); })}
        </div>
        <div class="item">
          <div class="lead"><Icon name="refresh-cw" size={18} /></div>
          <div class="txt"><div class="t">Sign in again</div><div class="s">When VTOP signs you out mid-use</div></div>
          {@render toggle(reSignIn, !isAccessKey(token), "Sign in again", () => { reSignIn = !reSignIn; void saveSettings(); })}
        </div>
        <div class="item">
          <div class="lead"><Icon name={notify ? "bell" : "bell-off"} size={18} /></div>
          <div class="txt"><div class="t">Notifications</div><div class="s">When your phone needs to approve, or a sign-in fails</div></div>
          {@render toggle(notify, false, "Notifications", toggleNotify)}
        </div>
      </div>

      <div class="section-label">Server</div>
      <div class="card pad">
        <div class="field" class:bad={serverError}>
          <label for="bridge-url">Server address</label>
          <input
            id="bridge-url"
            type="url"
            bind:value={bridgeUrl}
            placeholder={DEFAULT_BRIDGE_URL}
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        {#if serverError}
          <div class="help bad">{serverError}</div>
        {:else if serverMessage}
          <div class="help good">{serverMessage}</div>
        {:else}
          <div class="help">{savedBridgeUrl === DEFAULT_BRIDGE_URL ? "Using the default server." : `Using ${new URL(savedBridgeUrl).host}.`}</div>
        {/if}
        <div class="chips">
          <button type="button" class="chip pri" onclick={saveServer}>Save</button>
          <button
            type="button"
            class="chip"
            disabled={savedBridgeUrl === DEFAULT_BRIDGE_URL && bridgeUrl === DEFAULT_BRIDGE_URL}
            onclick={useDefaultServer}>Use default</button
          >
        </div>
      </div>

      <div class="section-label">This extension</div>
      {@render updateBanner()}
      <div class="card">
        <div class="item">
          <div class="lead"><Icon name="package" size={18} /></div>
          <div class="txt">
            <div class="t">Version {VERSION}</div>
            <div class="s">
              {#if !release}
                Not checked yet
              {:else if behind}
                {release.version} is out
              {:else}
                Up to date, checked {ago(release.checkedAt, now)}
              {/if}
            </div>
          </div>
        </div>
        <div class="item">
          <div class="lead bad"><Icon name="trash-2" size={18} /></div>
          <div class="txt"><div class="t">Reset all</div><div class="s">Forget the key, the server and the last sign-in</div></div>
          <button type="button" class="chip danger" disabled={running} onclick={resetState}>Reset</button>
        </div>
      </div>
    </div>
  {:else}
    <div class="bar">
      <img class="logo" src="logo.svg" alt="" />
      <div class="name">VITAP Mate</div>
      <button type="button" class="icon-btn" aria-label="Settings" onclick={() => (view = "settings")}>
        <Icon name="settings" />
        {#if behind}<span class="dot-badge"></span>{/if}
      </button>
    </div>

    <div class="body">
      {@render trouble()}
      {@render updateBanner()}
      {#if keyRejected && !editingKey}
        <div class="banner alert">
          <div class="ic"><Icon name="key-round" size={17} /></div>
          <div class="grow">
            <div class="bt">This key was revoked</div>
            <div class="bs">It was removed in VITAP Mate → Connected apps, or belongs to another server. Pair again to keep signing in.</div>
            <div class="acts"><button type="button" class="text-btn strong" onclick={pairAgain}>Pair again</button></div>
          </div>
        </div>
      {/if}

      {#if showPairing && pairing}
        <div class="pair-hero">
          <div class="qr-wrap">
            <svg class="shape spin" viewBox="0 0 100 100" aria-hidden="true"><path d={RING} /></svg>
            {#if pairQr}
              <div class="qr">
                <svg viewBox="0 0 {pairQr.size} {pairQr.size}" shape-rendering="crispEdges" role="img" aria-label="Pairing QR code"
                  ><path d={pairQrPath} fill="#000" /></svg
                >
              </div>
            {/if}
          </div>
          <div class="code" aria-label="Pairing code">{pairing.slice(0, 5)}<span class="dash">-</span>{pairing.slice(6)}</div>
          <div class="ring-time">Code works for {Math.floor(pairLeft / 60)}:{String(pairLeft % 60).padStart(2, "0")} more</div>
        </div>
        <ol class="steps">
          <li>Open the VITAP Mate app</li>
          <li><span><b>Connected apps</b> → <b>Pair a browser</b></span></li>
          <li>Scan the code or type it in</li>
        </ol>
        <button type="button" class="btn outline" onclick={stopPairing}>Cancel</button>
      {:else if showPairing}
        <div class="hero">
          <svg class="blob" viewBox="0 0 100 100" aria-hidden="true"><path d={BLOB} /></svg>
          <div class="hero-top">
            <div class="cookie"><svg class="shape" viewBox="0 0 100 100" aria-hidden="true"><path d={COOKIE} /></svg><Icon name="link" size={24} /></div>
            <div><h1>Pair with the app</h1><div class="sub">The app makes a key for this browser. No password is shared.</div></div>
          </div>
        </div>
        {#if !token.trim()}
          <ol class="steps">
            <li>Open the VITAP Mate app</li>
            <li><span><b>Connected apps</b> → <b>Pair a browser</b></span></li>
            <li>Scan the code or type it in</li>
          </ol>
        {/if}
        <button type="button" class="btn filled" onclick={startPairing}><Icon name="qr-code" size={19} />Pair with the app</button>
        {#if pairError}<div class="help bad">{pairError}</div>{/if}
        <div class="field" class:bad={showKeyHint || keyRejected}>
          <label for="token">Or paste an access key</label>
          <input
            id="token"
            type="password"
            bind:value={token}
            oninput={saveSettings}
            placeholder="vtm_…"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        {#if showKeyHint}
          <div class="help warn">This doesn't look like an access key.</div>
        {:else if keyRejected}
          <div class="help bad">Unknown or revoked key. Check the server in Settings, or create a new key.</div>
        {:else if isAccessKey(token)}
          <button type="button" class="text-btn center" onclick={() => (editingKey = false)}>Use this key</button>
        {/if}
      {:else}
        <div class="hero {hero.tone}">
          <svg class="blob" viewBox="0 0 100 100" aria-hidden="true"><path d={BLOB} /></svg>
          <div class="hero-top">
            <div class="cookie">
              <svg class="shape" class:spin={running || vtopSignedIn === null} viewBox="0 0 100 100" aria-hidden="true"><path d={COOKIE} /></svg>
              <Icon name={hero.icon} size={24} />
            </div>
            <div class="grow" role="status">
              <h1>{hero.title}</h1>
              {#if hero.sub}<div class="sub">{hero.sub}</div>{/if}
            </div>
            {#if login.phase !== "idle"}<div class="clock">{elapsed.toFixed(1)} s</div>{/if}
          </div>

          <div class="route" aria-label="Sign-in route">
            <div class="node {looks.chrome}"><div class="dot"><Icon name="chrome" size={18} /></div><span>{FIREFOX ? "Firefox" : "Chrome"}</span></div>
            <div class="link {linkClass(looks.chrome, looks.edge)}"></div>
            <div class="node {looks.edge}"><div class="dot"><Icon name="sparkles" size={18} /></div><span>Mate</span></div>
            <div class="link {linkClass(looks.edge, forkLook)}"></div>
            <div class="fork">
              <div class="node {looks.phone}"><div class="dot"><Icon name="smartphone" size={15} /></div><span>Phone</span></div>
              <div class="node {looks.server}"><div class="dot"><Icon name="server" size={15} /></div><span>Server</span></div>
            </div>
            <div class="link {linkClass(forkLook, looks.vtop)}"></div>
            <div class="node {looks.vtop}"><div class="dot"><Icon name="school" size={18} /></div><span>VTOP</span></div>
          </div>

          {#if running && login.deadline}
            <div class="wave">
              <svg viewBox="0 0 300 14" preserveAspectRatio="none" aria-hidden="true">
                <defs><clipPath id="wave-clip"><rect x="0" y="-4" width={Math.max(0, Math.min(300, (secondsLeft / 120) * 300))} height="22" /></clipPath></defs>
                <path class="track" d="M0 7 H300" />
                <g clip-path="url(#wave-clip)"><path class="ink" d={WAVE} /></g>
              </svg>
              <div class="legend">
                <span>
                  {#if serverIn}Server takes over in <b>{serverIn} s</b>{:else if login.phase === "server"}Server sign-in{/if}
                </span>
                <span><b>{secondsLeft} s</b> left</span>
              </div>
            </div>
          {/if}

          {#if login.phase !== "idle" && sourceText(login.via, login.source, now)}
            <div class="source" class:reused={login.source?.cached}><Icon name="cookie" size={14} />{sourceText(login.via, login.source, now)}</div>
          {/if}
        </div>

        {#if running}
          <button type="button" class="btn tonal" onclick={cancelLogin}><Icon name="x" size={19} />Cancel</button>
        {:else}
          <button type="button" class="btn filled" disabled={!isAccessKey(token)} onclick={signIn}>
            {#if vtopSignedIn}
              <Icon name="external-link" size={19} />Open VTOP
            {:else if login.phase === "error"}
              <Icon name="rotate-cw" size={19} />Try again
            {:else}
              <Icon name="log-in" size={19} />Sign in to VTOP
            {/if}
          </button>
        {/if}

        {#if login.log.length > 0}
          <div class="card log">
            <div class="log-title">Steps</div>
            {#each login.log as entry, i}
              {@const last = i === login.log.length - 1}
              {@const failed = entry.text === "Failed" || entry.text === "Cancelled" || entry.text === "Interrupted"}
              <div class="step" class:now={last && running} class:fail={failed} class:done={!failed && !(last && running)}>
                <span class="at">{((entry.at - (login.startedAt ?? entry.at)) / 1000).toFixed(1)}s</span>
                <span class="pip"></span>
                <span>{entry.text}</span>
              </div>
            {/each}
          </div>
        {/if}

        <div class="card">
          <div class="item">
            <div class="lead {keyRejected ? 'bad' : 'pri'}"><Icon name="key-round" size={18} /></div>
            <div class="txt">
              <div class="t">
                {#if keyRejected}
                  Key not accepted
                {:else if account}
                  <span class="regno">{account.registrationNumber}</span>
                  {account.keyLabel ?? ""}
                {:else}
                  Access key saved
                {/if}
              </div>
              <div class="s mono">{token.trim().slice(0, 8)}…{token.trim().slice(-4)}</div>
            </div>
            <button type="button" class="text-btn" onclick={() => (editingKey = true)}>Change</button>
          </div>
          {#if keyRejected}
            <div class="help bad foot-pad">{UNKNOWN_KEY_MESSAGE}</div>
          {:else if account}
            <div class="divider"></div>
            {#each accountRows(account, now) as row}
              <div class="item">
                <div class="lead {row.tone}"><Icon name={row.icon} size={18} /></div>
                <div class="txt"><div class="t">{row.label}</div><div class="s {row.tone}">{row.value}</div></div>
              </div>
            {/each}
            <div class="foot">Change these in VITAP Mate → Connected apps.</div>
          {/if}
        </div>

        <div class="card">
          <div class="item">
            <div class="txt"><div class="t">Auto sign-in</div><div class="s">When you open vtop.vitap.ac.in</div></div>
            {@render toggle(autoLogin, !isAccessKey(token), "Auto sign-in", () => { autoLogin = !autoLogin; void saveSettings(); })}
          </div>
          <div class="item">
            <div class="txt"><div class="t">Sign in again</div><div class="s">When VTOP signs you out mid-use</div></div>
            {@render toggle(reSignIn, !isAccessKey(token), "Sign in again", () => { reSignIn = !reSignIn; void saveSettings(); })}
          </div>
        </div>
      {/if}

      {#if notice}<div class="help good">{notice}</div>{/if}
    </div>
  {/if}
</main>

<style>
  .popup {
    width: 360px;
    background: var(--surface);
    color: var(--on-surface);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 8px 8px 16px;
  }
  .logo {
    width: 28px;
    height: 28px;
  }
  .name {
    flex: 1;
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.01em;
  }
  .icon-btn {
    width: 40px;
    height: 40px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    color: var(--on-surface-variant);
    position: relative;
    border: 0;
    background: transparent;
  }
  .icon-btn:hover {
    background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  }
  .dot-badge {
    position: absolute;
    top: 8px;
    right: 9px;
    width: 8px;
    height: 8px;
    border-radius: 99px;
    background: var(--error);
    outline: 2px solid var(--surface);
  }
  .body {
    padding: 4px 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .sub-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 10px 8px 4px;
  }
  .sub-bar .title {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  /* The hero: status, the cookie and the route. */
  .hero {
    border-radius: 28px;
    padding: 18px 18px 16px;
    background: var(--c);
    position: relative;
    overflow: hidden;
    transition:
      background-color 0.4s,
      color 0.4s;
  }
  .hero.active {
    background: var(--primary-container);
    color: var(--on-primary-container);
  }
  .hero.ok {
    background: var(--success-container);
    color: var(--on-success-container);
  }
  .hero.bad {
    background: var(--error-container);
    color: var(--on-error-container);
  }
  .blob {
    position: absolute;
    right: -46px;
    top: -54px;
    width: 170px;
    height: 170px;
    opacity: 0.55;
  }
  .blob path {
    fill: color-mix(in srgb, currentColor 9%, transparent);
  }
  .hero-top {
    display: flex;
    gap: 14px;
    align-items: center;
    position: relative;
  }
  .cookie {
    width: 56px;
    height: 56px;
    flex: none;
    position: relative;
    display: grid;
    place-items: center;
    color: var(--on-surface-variant);
  }
  .cookie .shape {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .cookie :global(.icon) {
    position: relative;
  }
  .cookie .shape path {
    fill: var(--c-lowest);
    transition: fill 0.4s;
  }
  .hero.active .cookie .shape path {
    fill: var(--primary);
  }
  .hero.active .cookie {
    color: var(--on-primary);
  }
  .hero.ok .cookie .shape path {
    fill: var(--success);
  }
  .hero.ok .cookie {
    color: var(--success-container);
  }
  .hero.bad .cookie .shape path {
    fill: var(--error);
  }
  .hero.bad .cookie {
    color: var(--error-container);
  }
  .spin {
    animation: spin 9s linear infinite;
    transform-origin: 50% 50%;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .hero h1 {
    margin: 0;
    font-size: 20px;
    line-height: 1.2;
    font-weight: 750;
    letter-spacing: -0.015em;
  }
  .hero .sub {
    margin-top: 3px;
    font-size: 13px;
    opacity: 0.82;
    line-height: 1.4;
  }
  .clock {
    white-space: nowrap;
    flex: none;
    align-self: flex-start;
    font-variant-numeric: tabular-nums;
    font-weight: 650;
    font-size: 13px;
    padding: 4px 9px;
    border-radius: 99px;
    background: color-mix(in srgb, currentColor 10%, transparent);
  }

  /* The route: browser, Mate, phone or server, VTOP. */
  .route {
    display: flex;
    align-items: center;
    margin-top: 16px;
    position: relative;
  }
  .node {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    width: 54px;
    flex: none;
  }
  .node .dot {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, currentColor 8%, transparent);
    opacity: 0.6;
    transition:
      background-color 0.3s,
      border-radius 0.3s,
      opacity 0.3s;
  }
  .node span {
    font-size: 11px;
    font-weight: 600;
    opacity: 0.7;
  }
  .node.done .dot {
    background: var(--success);
    color: var(--success-container);
    opacity: 1;
    border-radius: 999px;
  }
  .node.now .dot {
    background: var(--primary);
    color: var(--on-primary);
    opacity: 1;
    border-radius: 14px;
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 22%, transparent);
  }
  .node.err .dot {
    background: var(--error);
    color: var(--error-container);
    opacity: 1;
  }
  .node.skip .dot {
    opacity: 0.28;
  }
  .node.skip span {
    opacity: 0.35;
    text-decoration: line-through;
  }
  .node.done span,
  .node.now span,
  .node.err span {
    opacity: 1;
  }
  .link {
    flex: 1;
    height: 3px;
    border-radius: 3px;
    background: color-mix(in srgb, currentColor 14%, transparent);
    margin-bottom: 18px;
  }
  .link.on {
    background: var(--success);
  }
  .link.dash {
    background: repeating-linear-gradient(90deg, color-mix(in srgb, currentColor 22%, transparent) 0 5px, transparent 5px 9px);
  }
  .fork {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .fork .node {
    flex-direction: row;
    width: auto;
    gap: 6px;
  }
  .fork .node .dot {
    width: 28px;
    height: 28px;
    border-radius: 9px;
  }
  .fork .node.done .dot {
    border-radius: 999px;
  }

  /* Wavy progress: the 120 s a sign-in may wait. */
  .wave {
    margin-top: 14px;
  }
  .wave svg {
    width: 100%;
    height: 14px;
    display: block;
    overflow: visible;
  }
  .wave .track {
    stroke: color-mix(in srgb, currentColor 16%, transparent);
    stroke-width: 4;
    stroke-linecap: round;
    fill: none;
  }
  .wave .ink {
    stroke: currentColor;
    stroke-width: 4;
    stroke-linecap: round;
    fill: none;
    animation: drift 1.4s linear infinite;
  }
  @keyframes drift {
    to {
      transform: translateX(-16px);
    }
  }
  .wave .legend {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    margin-top: 6px;
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
  }
  .wave .legend b {
    font-weight: 650;
  }
  .source {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 12px;
    font-size: 12px;
    font-weight: 600;
    padding: 5px 10px 5px 8px;
    border-radius: 99px;
    background: color-mix(in srgb, currentColor 10%, transparent);
    position: relative;
  }
  .source.reused {
    background: var(--warn-container);
    color: var(--on-warn-container);
  }

  /* Buttons */
  .btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 52px;
    border-radius: 999px;
    font: inherit;
    font-weight: 650;
    font-size: 15px;
    border: 0;
    width: 100%;
    transition: border-radius 0.2s;
  }
  .btn:active:not(:disabled) {
    border-radius: 16px;
  }
  .btn.filled {
    background: var(--primary);
    color: var(--on-primary);
  }
  .btn.tonal {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
  }
  .btn.outline {
    background: transparent;
    color: var(--on-surface);
    box-shadow: inset 0 0 0 1px var(--outline);
  }
  .btn:disabled {
    background: color-mix(in srgb, var(--on-surface) 12%, transparent);
    color: color-mix(in srgb, var(--on-surface) 38%, transparent);
  }
  .text-btn {
    border: 0;
    background: transparent;
    color: var(--primary);
    font: inherit;
    font-weight: 650;
    font-size: 13px;
    padding: 8px 10px;
    border-radius: 99px;
  }
  .text-btn:hover {
    background: color-mix(in srgb, currentColor 8%, transparent);
  }
  .text-btn.center {
    align-self: center;
  }

  /* Cards and lists */
  .card {
    background: var(--c-low);
    border-radius: 20px;
    padding: 6px 0;
  }
  .card.pad {
    padding: 10px 12px 12px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    min-height: 52px;
  }
  .lead {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    background: var(--c-highest);
    color: var(--on-surface-variant);
    display: grid;
    place-items: center;
    flex: none;
  }
  .lead.good {
    background: var(--success-container);
    color: var(--on-success-container);
  }
  .lead.warn {
    background: var(--warn-container);
    color: var(--on-warn-container);
  }
  .lead.bad {
    background: var(--error-container);
    color: var(--on-error-container);
  }
  .lead.pri {
    background: var(--primary-container);
    color: var(--on-primary-container);
    border-radius: 999px;
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  .t {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }
  .s {
    font-size: 12.5px;
    color: var(--on-surface-variant);
    line-height: 1.35;
    margin-top: 1px;
  }
  .s.bad {
    color: var(--error);
  }
  .s.good {
    color: var(--success);
  }
  .s.warn {
    color: var(--warn);
  }
  .mono {
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .divider {
    height: 1px;
    background: var(--outline-variant);
    margin: 2px 16px;
    opacity: 0.6;
  }
  .foot {
    font-size: 12px;
    color: var(--on-surface-variant);
    padding: 2px 16px 10px;
  }
  .section-label {
    font-size: 13px;
    font-weight: 650;
    color: var(--primary);
    padding: 6px 6px 0;
  }
  .regno {
    display: inline-flex;
    font-weight: 700;
    font-size: 13px;
    padding: 2px 10px;
    border-radius: 99px;
    background: var(--primary-container);
    color: var(--on-primary-container);
    letter-spacing: 0.02em;
    margin-right: 4px;
  }

  /* M3 switch */
  .switch {
    width: 52px;
    height: 32px;
    border-radius: 99px;
    background: var(--c-highest);
    box-shadow: inset 0 0 0 2px var(--outline);
    position: relative;
    flex: none;
    border: 0;
    padding: 0;
  }
  .switch::after {
    content: "";
    position: absolute;
    left: 8px;
    top: 8px;
    width: 16px;
    height: 16px;
    border-radius: 99px;
    background: var(--outline);
    transition: all 0.2s;
  }
  .switch.on {
    background: var(--primary);
    box-shadow: none;
  }
  .switch.on::after {
    left: 24px;
    top: 4px;
    width: 24px;
    height: 24px;
    background: var(--on-primary);
  }
  .switch:disabled {
    opacity: 0.38;
  }

  /* Step log */
  .log {
    padding: 8px 16px 10px;
  }
  .log-title {
    font-size: 13px;
    font-weight: 650;
    padding: 6px 0 4px;
  }
  .step {
    display: grid;
    grid-template-columns: 42px 14px 1fr;
    gap: 8px;
    align-items: start;
    font-size: 12.5px;
    line-height: 1.35;
    padding: 3px 0;
    color: var(--on-surface-variant);
  }
  .step .at {
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
  }
  .step .pip {
    width: 8px;
    height: 8px;
    border-radius: 99px;
    background: var(--outline-variant);
    margin: 4px 3px 0;
  }
  .step.done .pip {
    background: var(--success);
  }
  .step.now {
    color: var(--on-surface);
    font-weight: 600;
  }
  .step.now .pip {
    background: var(--primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 25%, transparent);
  }
  .step.fail {
    color: var(--error);
    font-weight: 600;
  }
  .step.fail .pip {
    background: var(--error);
  }

  /* Banners */
  .banner {
    border-radius: 20px;
    padding: 12px 12px 10px 14px;
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .banner.update {
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
  }
  .banner.alert {
    background: var(--error-container);
    color: var(--on-error-container);
  }
  .banner .ic {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, currentColor 12%, transparent);
    flex: none;
  }
  .bt {
    font-weight: 700;
    font-size: 14px;
  }
  .bs {
    font-size: 12.5px;
    line-height: 1.4;
    opacity: 0.9;
    margin-top: 2px;
  }
  .acts {
    display: flex;
    gap: 4px;
    margin-top: 6px;
    margin-left: -10px;
  }
  .acts .text-btn {
    color: inherit;
  }
  .acts .text-btn.strong {
    background: color-mix(in srgb, currentColor 12%, transparent);
  }

  /* Fields */
  .field {
    position: relative;
  }
  .field input {
    width: 100%;
    height: 56px;
    border-radius: 16px 16px 4px 4px;
    border: 0;
    background: var(--c-highest);
    color: var(--on-surface);
    font: inherit;
    font-size: 14px;
    padding: 22px 16px 6px;
    outline: none;
    box-shadow: inset 0 -1px 0 var(--on-surface-variant);
  }
  .field input::placeholder {
    color: var(--on-surface-variant);
    opacity: 0.6;
  }
  .field label {
    position: absolute;
    left: 16px;
    top: 8px;
    font-size: 12px;
    color: var(--on-surface-variant);
  }
  .field:focus-within input {
    box-shadow: inset 0 -3px 0 var(--primary);
  }
  .field:focus-within label {
    color: var(--primary);
  }
  .field.bad input {
    box-shadow: inset 0 -3px 0 var(--error);
  }
  .field.bad label {
    color: var(--error);
  }
  .help {
    font-size: 12px;
    padding: 0 16px;
    color: var(--on-surface-variant);
  }
  .card .help {
    padding-top: 4px;
  }
  .help.bad {
    color: var(--error);
  }
  .help.warn {
    color: var(--warn);
  }
  .help.good {
    color: var(--success);
  }
  .foot-pad {
    padding-bottom: 10px;
  }
  .chips {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    padding: 8px 0 0;
  }
  .chip {
    height: 32px;
    padding: 0 14px;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    font-size: 13px;
    font-weight: 600;
    background: transparent;
    color: var(--on-surface-variant);
    box-shadow: inset 0 0 0 1px var(--outline-variant);
    border: 0;
    font-family: inherit;
  }
  .chip.pri {
    background: var(--primary);
    color: var(--on-primary);
    box-shadow: none;
  }
  .chip.danger {
    color: var(--error);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--error) 50%, transparent);
  }
  .chip:disabled {
    opacity: 0.38;
  }

  /* Pairing */
  .pair-hero {
    border-radius: 28px;
    background: var(--primary-container);
    color: var(--on-primary-container);
    padding: 18px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .qr-wrap {
    position: relative;
    width: 196px;
    height: 196px;
    display: grid;
    place-items: center;
  }
  .qr-wrap .shape {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .qr-wrap .shape path {
    fill: var(--primary);
  }
  .qr {
    position: relative;
    width: 136px;
    height: 136px;
    background: #fff;
    border-radius: 14px;
    padding: 6px;
  }
  .qr svg {
    width: 100%;
    height: 100%;
    display: block;
  }
  .code {
    font-size: 26px;
    font-weight: 750;
    letter-spacing: 0.08em;
    margin-top: 12px;
    font-variant-numeric: tabular-nums;
    user-select: all;
  }
  .code .dash {
    opacity: 0.45;
    margin: 0 4px;
  }
  .ring-time {
    font-size: 12.5px;
    margin-top: 4px;
    opacity: 0.85;
    font-variant-numeric: tabular-nums;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0 4px;
    counter-reset: s;
  }
  .steps li {
    counter-increment: s;
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 6px 8px;
    font-size: 13.5px;
  }
  .steps li::before {
    content: counter(s);
    width: 26px;
    height: 26px;
    border-radius: 9px;
    background: var(--secondary-container);
    color: var(--on-secondary-container);
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 12.5px;
    flex: none;
  }
  .steps li b {
    font-weight: 650;
  }
</style>
