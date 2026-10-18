<script lang="ts">
  import { onDestroy } from "svelte";
  import { DEFAULT_BRIDGE_URL, isAccessKey, normalizeBridgeUrl } from "./bridge";
  import type { LastLogin, LoginPhase, LoginState, StateMessage } from "./background";

  let token = "";
  let editingKey = false;
  let autoLogin = false;
  let bridgeUrl = DEFAULT_BRIDGE_URL;
  let savedBridgeUrl = DEFAULT_BRIDGE_URL;
  let serverMessage = "";
  let serverError = "";
  let notice = "";
  /** What `/v1/whoami` says about the key's account (times in seconds). */
  type Account = {
    registrationNumber?: string;
    keyLabel?: string | null;
    keyLastUsedAt?: number | null;
    settings?: { alwaysUseVault: boolean; phoneWaitSecs: number; vaultTtlSecs: number | null };
    phoneLinked?: boolean;
    savedCredentials?: boolean;
    vaultSavedAt?: number | null;
    credentialsRejectedAt?: number | null;
  };
  let account: Account | undefined;
  // Who the key belongs to, checked when a key is entered.
  let keyOwner = "";
  let keyError = "";
  let checkedKey = "";
  // null while checking whether Chrome is still signed in to VTOP.
  let vtopSignedIn: boolean | null = null;
  let lastLogin: LastLogin | undefined;
  let login: LoginState = { phase: "idle", text: "", log: [] };
  let now = Date.now();
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
    const stored = await chrome.storage.local.get(["token", "fmcToken", "autoLogin", "bridgeUrl", "lastLogin"]);
    token = String(stored.token ?? stored.fmcToken ?? "");
    autoLogin = Boolean(stored.autoLogin);
    savedBridgeUrl = normalizeBridgeUrl(String(stored.bridgeUrl ?? "")) ?? DEFAULT_BRIDGE_URL;
    bridgeUrl = savedBridgeUrl;
    lastLogin = stored.lastLogin as LastLogin | undefined;
    editingKey = !isAccessKey(token);
    // Migrate older installations without making the implementation name visible.
    if (!stored.token && stored.fmcToken) {
      await chrome.storage.local.set({ token });
      await chrome.storage.local.remove("fmcToken");
    }
    // A sign-in started before the popup was opened keeps going in the background.
    login = (await chrome.runtime.sendMessage({ type: "GET_LOGIN_STATE" })) ?? login;
    const check = await chrome.runtime.sendMessage({ type: "CHECK_VTOP_SESSION" });
    vtopSignedIn = Boolean(check?.signedIn);
  }

  async function saveSettings() {
    if (!token.trim()) autoLogin = false;
    await chrome.storage.local.set({ token: token.trim(), autoLogin });
  }

  /** Asks the server whose key this is, so a wrong key shows up at once. */
  async function checkKey(key: string) {
    checkedKey = key;
    keyOwner = "";
    keyError = "";
    account = undefined;
    try {
      const response = await fetch(`${savedBridgeUrl}/v1/whoami`, {
        headers: { Authorization: `Bearer ${key}` },
      });
      if (key !== checkedKey) return;
      if (response.status === 401) {
        keyError = "Unknown or revoked key. Check the server in Settings, or create a new key.";
        return;
      }
      if (!response.ok) return;
      const who = (await response.json()) as Account;
      account = who;
      keyOwner = [who.registrationNumber, who.keyLabel ? `"${who.keyLabel}"` : ""]
        .filter(Boolean)
        .join(" · ");
      editingKey = false;
    } catch {
      // Offline or the server is unreachable; sign-in will say so.
    }
  }

  // Called straight from a click: Chrome only shows its permission prompt
  // during a user gesture.
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
        serverError = "Chrome needs your permission to reach that server.";
        return;
      }
      await chrome.storage.local.set({ bridgeUrl: origin });
    }
    bridgeUrl = savedBridgeUrl = origin;
    serverMessage = "Saved.";
    checkedKey = "";
  }

  async function useDefaultServer() {
    await chrome.storage.local.remove("bridgeUrl");
    bridgeUrl = savedBridgeUrl = DEFAULT_BRIDGE_URL;
    serverError = "";
    serverMessage = "Using the default server.";
    checkedKey = "";
  }

  async function signIn() {
    if (!token.trim()) return;
    notice = "";
    await saveSettings();
    // The popup may close while this runs; the background keeps going and
    // the next popup picks the state up again.
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
    await chrome.storage.local.remove([
      "token",
      "fmcToken",
      "autoLogin",
      "lastAutoLoginAt",
      "bridgeUrl",
      "lastLogin",
    ]);
    bridgeUrl = savedBridgeUrl = DEFAULT_BRIDGE_URL;
    token = "";
    autoLogin = false;
    lastLogin = undefined;
    editingKey = true;
    notice = "Settings cleared.";
  }

  // ---- the flow diagram ----------------------------------------------------

  type Node = "chrome" | "edge" | "phone" | "server" | "vtop";
  type NodeLook = "idle" | "active" | "done" | "skipped" | "error";

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
    // Chrome was still signed in: the session never left the browser.
    if (s.via === "browser" && node !== "chrome" && node !== "vtop") return "skipped";
    if (node === "phone" || node === "server") {
      const used = s.via === node || (phase === node);
      // The phone was asked first and the server took over.
      const handedOver = node === "phone" && s.via === "server" && s.log.some((e) => e.text.startsWith("Waiting for your phone"));
      if (rank < 2) return "idle";
      if (handedOver) return "skipped";
      if (!used) return "skipped";
    }
    const nodeRank = node === "vtop" && s.via === "browser" ? 3 : NODE_RANK[node];
    const active =
      (node === "vtop" && (phase === "cookies" || phase === "open")) ||
      (node !== "vtop" && nodeRank === Math.min(rank, 3) && phase !== "done");
    if (active) return failed ? "error" : "active";
    if (rank > nodeRank || s.phase === "done") return "done";
    return "idle";
  }

  $: looks = {
    chrome: look("chrome", login),
    edge: look("edge", login),
    phone: look("phone", login),
    server: look("server", login),
    vtop: look("vtop", login),
  };

  const NODE_CLASS: Record<NodeLook, string> = {
    idle: "border-slate-800 bg-slate-900 text-slate-400",
    active: "border-indigo-400 bg-indigo-500/15 text-indigo-100 shadow-[0_0_0_3px_rgba(99,102,241,0.18)]",
    done: "border-emerald-700/70 bg-emerald-500/10 text-emerald-200",
    skipped: "border-slate-800/70 bg-transparent text-slate-600",
    error: "border-rose-500 bg-rose-500/15 text-rose-100",
  };

  function lineClass(from: NodeLook, to: NodeLook) {
    if (to === "skipped" || from === "skipped") return "bg-slate-800";
    if (to === "done" || to === "active" || to === "error") return "bg-emerald-600/70";
    return "bg-slate-800";
  }

  // ---- derived -------------------------------------------------------------

  $: running = !["idle", "done", "error"].includes(login.phase);
  $: showKeyHint = token.trim().length > 0 && !isAccessKey(token);
  $: if (isAccessKey(token) && token.trim() !== checkedKey) checkKey(token.trim());
  $: if (!isAccessKey(token)) {
    keyOwner = "";
    keyError = "";
  }
  $: secondsLeft = login.deadline && login.deadline > now ? Math.ceil((login.deadline - now) / 1000) : 0;
  $: serverIn =
    login.phase === "phone" && login.serverDueAt && login.serverDueAt > now
      ? Math.ceil((login.serverDueAt - now) / 1000)
      : 0;
  $: elapsed = login.startedAt ? ((login.finishedAt ?? now) - login.startedAt) / 1000 : 0;

  function ago(at: number, at_now: number) {
    const s = Math.max(0, Math.round((at_now - at) / 1000));
    if (s < 60) return "just now";
    if (s < 3600) return `${Math.floor(s / 60)} min ago`;
    if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
    return `${Math.floor(s / 86400)} d ago`;
  }

  /** Each row: label, value, and a colour for the value. */
  function accountRows(a: Account, at_now: number): [string, string, string][] {
    const rows: [string, string, string][] = [];
    const s = a.settings;
    const server = a.savedCredentials && s;
    let order: string;
    if (!server) order = a.phoneLinked ? "Phone only" : "No way to sign in";
    else if (s.alwaysUseVault) order = "Server first";
    else order = a.phoneLinked ? `Phone, then server after ${s.phoneWaitSecs} s` : "Server";
    rows.push(["Who signs in", order, server || a.phoneLinked ? "text-slate-200" : "text-rose-300"]);
    if (a.credentialsRejectedAt) {
      rows.push(["Server password", `Rejected ${ago(a.credentialsRejectedAt * 1000, at_now)}`, "text-rose-300"]);
    } else if (a.savedCredentials) {
      const saved = a.vaultSavedAt ? `Saved ${ago(a.vaultSavedAt * 1000, at_now)}` : "Saved";
      const ttl = s?.vaultTtlSecs ? ` · kept ${s.vaultTtlSecs / 86400} d` : "";
      rows.push(["Server password", saved + ttl, "text-emerald-300"]);
    } else {
      rows.push(["Server password", "Off", "text-slate-400"]);
    }
    rows.push(["Phone", a.phoneLinked ? "Linked" : "Not linked", a.phoneLinked ? "text-emerald-300" : "text-amber-300"]);
    if (a.keyLastUsedAt) rows.push(["This key last used", ago(a.keyLastUsedAt * 1000, at_now), "text-slate-400"]);
    return rows;
  }

  const VIA: Record<string, string> = { browser: "Chrome's session", phone: "phone", server: "server" };

  loadSettings();
</script>

<main class="w-[360px] bg-slate-950 text-slate-100">
  <!-- header -->
  <header class="flex items-center gap-2.5 border-b border-slate-800/80 px-4 py-3">
    <img src="icons/icon-48.png" alt="" class="h-8 w-8 rounded-lg" />
    <div class="min-w-0 flex-1">
      <h1 class="text-sm font-semibold leading-4">VITAP Mate</h1>
      <p class="mt-0.5 flex items-center gap-1.5 text-[11px] text-slate-400">
        {#if vtopSignedIn === null}
          <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-slate-500"></span> Checking VTOP…
        {:else if vtopSignedIn}
          <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span> Signed in to VTOP
        {:else}
          <span class="h-1.5 w-1.5 rounded-full bg-slate-500"></span> Not signed in to VTOP
        {/if}
      </p>
    </div>
    {#if lastLogin}
      <span class="text-right text-[10px] leading-3 text-slate-500" title="Last sign-in">
        last {ago(lastLogin.at, now)}<br />via {VIA[lastLogin.via] ?? lastLogin.via} · {(lastLogin.ms / 1000).toFixed(1)} s
      </span>
    {/if}
  </header>

  <section class="space-y-3 p-4">
    <!-- flow diagram -->
    <div class="rounded-xl border border-slate-800 bg-slate-900/40 p-3" aria-label="Sign-in flow">
      <div class="flex items-center">
        <div class={`flow-node ${NODE_CLASS[looks.chrome]}`}>
          <span class="flow-mark">{looks.chrome === "done" ? "✓" : "◐"}</span>Chrome
        </div>
        <div class={`h-px flex-1 ${lineClass(looks.chrome, looks.edge)}`}></div>
        <div class={`flow-node ${NODE_CLASS[looks.edge]}`}>
          <span class="flow-mark">{looks.edge === "done" ? "✓" : "◆"}</span>Mate
        </div>
        <div class={`h-px w-2 ${lineClass(looks.edge, looks.phone === "skipped" ? looks.server : looks.phone)}`}></div>
        <div class="flex flex-col gap-1.5 border-l border-slate-700 pl-2">
          <div class={`flow-node ${NODE_CLASS[looks.phone]}`}>
            <span class="flow-mark">{looks.phone === "done" ? "✓" : "▯"}</span>Phone
          </div>
          <div class={`flow-node ${NODE_CLASS[looks.server]}`}>
            <span class="flow-mark">{looks.server === "done" ? "✓" : "☁"}</span>Server
          </div>
        </div>
        <div class={`h-px flex-1 ${lineClass(looks.server === "skipped" ? looks.phone : looks.server, looks.vtop)}`}></div>
        <div class={`flow-node ${NODE_CLASS[looks.vtop]}`}>
          <span class="flow-mark">{looks.vtop === "done" ? "✓" : "▣"}</span>VTOP
        </div>
      </div>

      <!-- what is happening now -->
      <div class="mt-3 flex min-h-[2.25rem] items-start justify-between gap-2 text-xs">
        <p
          class={login.phase === "error"
            ? "text-rose-300"
            : login.phase === "done"
              ? "text-emerald-300"
              : "text-slate-300"}
          role="status"
        >
          {#if login.phase === "idle"}
            {vtopSignedIn ? "Chrome is signed in. Open VTOP right away." : "Ready. Your phone or the server makes a session for Chrome."}
          {:else if login.phase === "done"}
            Signed in via {VIA[login.via ?? "phone"]} in {elapsed.toFixed(1)} s.
          {:else if login.phase === "error"}
            {login.error}
          {:else}
            {login.text}
          {/if}
        </p>
        {#if running || login.phase === "done" || login.phase === "error"}
          <span class="shrink-0 font-mono text-[11px] tabular-nums text-slate-500">{elapsed.toFixed(1)} s</span>
        {/if}
      </div>

      {#if running && login.deadline}
        <div class="mt-2 h-1 overflow-hidden rounded-full bg-slate-800">
          <div
            class="h-full rounded-full bg-indigo-400 transition-[width] duration-300"
            style={`width: ${Math.max(0, Math.min(100, (secondsLeft / 120) * 100))}%`}
          ></div>
        </div>
        <p class="mt-1 flex justify-between text-[10px] text-slate-500">
          <span>{serverIn ? `server takes over in ${serverIn} s` : ""}</span>
          <span>{secondsLeft} s left</span>
        </p>
      {/if}

      <!-- step log -->
      {#if login.log.length > 0}
        <ol class="mt-2 space-y-0.5 border-t border-slate-800 pt-2">
          {#each login.log as entry, i}
            <li class="flex items-baseline gap-2 text-[11px]">
              <span class="w-10 shrink-0 text-right font-mono tabular-nums text-slate-500">
                {(((entry.at - (login.startedAt ?? entry.at)) / 1000)).toFixed(1)}s
              </span>
              <span
                class={i === login.log.length - 1 && running
                  ? "text-indigo-200"
                  : entry.text === "Failed" || entry.text === "Cancelled"
                    ? "text-rose-300"
                    : "text-slate-400"}
              >
                {entry.text}
              </span>
            </li>
          {/each}
        </ol>
      {/if}
    </div>

    <!-- access key -->
    {#if editingKey}
      <div class="space-y-1.5">
        {#if !token.trim()}
          <ol class="space-y-1 text-xs text-slate-400">
            <li><span class="step-dot">1</span>Open the VITAP Mate app</li>
            <li><span class="step-dot">2</span>More → Connected apps → Create key</li>
            <li><span class="step-dot">3</span>Paste the key here</li>
          </ol>
        {/if}
        <input
          id="token"
          type="password"
          bind:value={token}
          on:input={saveSettings}
          placeholder="Access key  vtm_…"
          autocomplete="off"
          spellcheck="false"
          aria-label="Access key"
          class="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 font-mono text-xs outline-none placeholder:font-sans placeholder:text-slate-500 focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/20"
        />
        {#if showKeyHint}
          <p class="text-[11px] text-amber-400">This doesn't look like an access key.</p>
        {:else if keyError}
          <p class="text-[11px] text-rose-300">{keyError}</p>
        {/if}
      </div>
    {:else}
      <div class="flex items-center gap-2 rounded-lg border border-slate-800 px-3 py-2">
        <span class={`text-sm ${keyError ? "text-rose-400" : "text-emerald-400"}`}>{keyError ? "!" : "✓"}</span>
        <div class="min-w-0 flex-1">
          <p class="truncate text-xs font-medium">{keyOwner || (keyError ? "Key not accepted" : "Access key saved")}</p>
          <p class="truncate font-mono text-[10px] text-slate-500">
            {token.trim().slice(0, 8)}…{token.trim().slice(-4)}
          </p>
        </div>
        <button type="button" class="link" on:click={() => (editingKey = true)}>Change</button>
      </div>
      {#if keyError}<p class="text-[11px] text-rose-300">{keyError}</p>{/if}
      {#if account?.settings}
        <details class="group rounded-lg border border-slate-800 px-3 py-2" open>
          <summary class="flex cursor-pointer list-none items-center justify-between text-[11px] font-medium text-slate-300">
            Sign-in settings
            <span class="text-slate-500 transition-transform group-open:rotate-90">›</span>
          </summary>
          <dl class="mt-1.5 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[11px]">
            {#each accountRows(account, now) as [label, value, tone]}
              <dt class="text-slate-500">{label}</dt>
              <dd class={`text-right ${tone}`}>{value}</dd>
            {/each}
          </dl>
          <p class="mt-1.5 text-[10px] text-slate-600">Change these in VITAP Mate → Connected apps.</p>
        </details>
      {/if}
    {/if}

    <!-- actions -->
    {#if running}
      <button type="button" on:click={cancelLogin} class="btn border border-slate-700 text-slate-200 hover:bg-slate-800">
        Cancel
      </button>
    {:else}
      <button
        type="button"
        disabled={!isAccessKey(token)}
        on:click={signIn}
        class="btn bg-indigo-500 text-white shadow-lg shadow-indigo-950 hover:bg-indigo-400 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {vtopSignedIn ? "Open VTOP" : login.phase === "error" ? "Try again" : "Sign in to VTOP"}
      </button>
    {/if}

    <label class="flex cursor-pointer items-center justify-between gap-3 text-xs text-slate-300">
      <span>
        Auto sign-in
        <span class="block text-[10px] text-slate-500">When you open vtop.vitap.ac.in</span>
      </span>
      <input
        type="checkbox"
        bind:checked={autoLogin}
        on:change={saveSettings}
        disabled={!isAccessKey(token)}
        class="peer sr-only"
      />
      <span
        class="relative h-5 w-9 shrink-0 rounded-full bg-slate-700 transition-colors after:absolute after:left-0.5 after:top-0.5 after:h-4 after:w-4 after:rounded-full after:bg-white after:transition-transform peer-checked:bg-indigo-500 peer-checked:after:translate-x-4 peer-disabled:opacity-40"
      ></span>
    </label>

    {#if notice}<p class="text-[11px] text-emerald-300">{notice}</p>{/if}

    <!-- settings -->
    <details class="group border-t border-slate-800 pt-2">
      <summary class="flex cursor-pointer list-none items-center justify-between text-[11px] text-slate-500 hover:text-slate-300">
        <span>Settings</span>
        <span class="truncate">
          Server: {savedBridgeUrl === DEFAULT_BRIDGE_URL ? "default" : new URL(savedBridgeUrl).host}
          <span class="inline-block transition-transform group-open:rotate-90">›</span>
        </span>
      </summary>
      <div class="mt-2 space-y-2">
        <input
          id="bridge-url"
          type="url"
          bind:value={bridgeUrl}
          placeholder={DEFAULT_BRIDGE_URL}
          autocomplete="off"
          spellcheck="false"
          aria-label="Server address"
          class="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-1.5 text-xs outline-none placeholder:text-slate-500 focus:border-indigo-400"
        />
        <div class="flex items-center gap-1">
          <button type="button" on:click={saveServer} class="chip">Save</button>
          <button
            type="button"
            disabled={savedBridgeUrl === DEFAULT_BRIDGE_URL && bridgeUrl === DEFAULT_BRIDGE_URL}
            on:click={useDefaultServer}
            class="chip disabled:opacity-40"
          >
            Use default
          </button>
          <span class="flex-1"></span>
          <button type="button" on:click={resetState} disabled={running} class="chip text-rose-300 disabled:opacity-40">
            Reset all
          </button>
        </div>
        {#if serverMessage}<p class="text-[11px] text-emerald-300">{serverMessage}</p>{/if}
        {#if serverError}<p class="text-[11px] text-rose-300">{serverError}</p>{/if}
      </div>
    </details>
  </section>
</main>

<style>
  .flow-node {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    border-width: 1px;
    border-radius: 0.5rem;
    padding: 0.25rem 0.4rem;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color 0.3s,
      border-color 0.3s,
      color 0.3s,
      box-shadow 0.3s;
  }
  .flow-mark {
    font-size: 10px;
    opacity: 0.8;
  }
  .step-dot {
    display: inline-grid;
    place-items: center;
    width: 1rem;
    height: 1rem;
    margin-right: 0.4rem;
    border-radius: 9999px;
    background: rgb(30 41 59);
    font-size: 10px;
    color: rgb(165 180 252);
  }
  .btn {
    width: 100%;
    border-radius: 0.6rem;
    padding: 0.55rem 1rem;
    font-size: 0.8rem;
    font-weight: 600;
  }
  .link {
    font-size: 11px;
    color: rgb(165 180 252);
  }
  .link:hover {
    color: white;
  }
  .chip {
    border-radius: 0.4rem;
    padding: 0.25rem 0.55rem;
    font-size: 11px;
    background: rgb(30 41 59);
  }
  .chip:hover:not(:disabled) {
    background: rgb(51 65 85);
  }
</style>
