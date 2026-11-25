<script lang="ts">
  import { onDestroy } from "svelte";
  import { DEFAULT_BRIDGE_URL, isAccessKey, normalizeBridgeUrl } from "./bridge";
  import { FIREFOX } from "./format";
  import { releaseFromGitHub, updateAvailable, type Release } from "./live";
  import {
    isCheckVtopSessionReply,
    isStateMessage,
    type BackgroundMessage,
  } from "./messages";
  import { watchAccount, type AccountLiveState } from "./popupLive";
  import {
    createIdleState,
    isInProgressPhase,
    isLoginState,
    type LastLogin,
    type LoginState,
    type Settings,
  } from "./state";
  import AccountCard from "./AccountCard.svelte";
  import Icon from "./Icon.svelte";
  import PairingView from "./PairingView.svelte";
  import RouteCard from "./RouteCard.svelte";
  import SettingsView from "./SettingsView.svelte";
  import Switch from "./Switch.svelte";
  import UpdateBanner from "./UpdateBanner.svelte";

  const VERSION = chrome.runtime.getManifest().version;

  let view = $state<"main" | "settings">("main");
  let token = $state("");
  let editingKey = $state(false);
  let autoStartPairing = $state(false);
  let pairingActive = $state(false);
  let autoLogin = $state(false);
  let reSignIn = $state(true);
  let notify = $state(false);
  let savedBridgeUrl = $state(DEFAULT_BRIDGE_URL);
  let notice = $state("");
  let release = $state<Release | null>(null);
  // null while checking whether the browser is still signed in to VTOP.
  let vtopSignedIn = $state<boolean | null>(null);
  let lastLogin = $state<LastLogin | undefined>(undefined);
  let login = $state<LoginState>(createIdleState());
  let now = $state(Date.now());

  const running = $derived(isInProgressPhase(login.phase));
  const behind = $derived(updateAvailable(release));

  // Only tick every 250 ms while a sign-in or pairing countdown is active.
  $effect(() => {
    if (!running && !pairingActive) return;
    now = Date.now();
    const ticker = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(ticker);
  });

  function sendBackground(message: BackgroundMessage): Promise<unknown> {
    return chrome.runtime.sendMessage(message);
  }

  function onMessage(message: unknown) {
    if (!isStateMessage(message)) return;
    login = message.state;
    now = Date.now();
    if (login.phase === "done") {
      vtopSignedIn = true;
      void loadLastLogin();
    }
  }
  chrome.runtime.onMessage.addListener(onMessage);
  onDestroy(() => chrome.runtime.onMessage.removeListener(onMessage));

  async function loadLastLogin() {
    try {
      const stored = await chrome.storage.local.get("lastLogin");
      lastLogin = stored.lastLogin as LastLogin | undefined;
    } catch {
      // Best-effort metadata refresh.
    }
  }

  async function loadSettings() {
    const stored = (await chrome.storage.local.get(null)) as Settings & { lastLogin?: LastLogin };
    token = String(stored.token ?? "");
    autoLogin = Boolean(stored.autoLogin);
    reSignIn = stored.reSignIn !== false;
    notify =
      Boolean(stored.notify) &&
      (await chrome.permissions.contains({ permissions: ["notifications"] }).catch(() => false));
    savedBridgeUrl = normalizeBridgeUrl(String(stored.bridgeUrl ?? "")) ?? DEFAULT_BRIDGE_URL;
    lastLogin = stored.lastLogin;
    editingKey = !isAccessKey(token);
    // A sign-in started before the popup was opened keeps going in the background.
    const current = await sendBackground({ type: "GET_LOGIN_STATE" }).catch(() => null);
    if (isLoginState(current)) login = current;
    const check = await sendBackground({ type: "CHECK_VTOP_SESSION" }).catch(() => null);
    vtopSignedIn = isCheckVtopSessionReply(check) ? check.signedIn : false;
    now = Date.now();
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
      notify = await chrome.permissions
        .request({ permissions: ["notifications"] })
        .catch(() => false);
    }
    await saveSettings();
  }

  // ---- the live link: opened lazily only when a valid key exists -----------------------------

  const liveKey = $derived(isAccessKey(token) ? token.trim() : undefined);
  let liveState = $state<AccountLiveState>({ kind: "idle" });
  let liveTry = $state(0);

  const account = $derived(
    liveState.kind === "ready" || liveState.kind === "connecting" || liveState.kind === "unreachable"
      ? liveState.account
      : undefined,
  );
  const keyRejected = $derived(liveState.kind === "rejected");
  const unreachable = $derived(liveState.kind === "unreachable");

  $effect(() => {
    const base = savedBridgeUrl;
    const key = liveKey;
    void liveTry;
    if (!key) {
      liveState = { kind: "idle" };
      return;
    }
    let confirmed = false;
    return watchAccount(base, key, (next) => {
      liveState = next;
      if (next.kind === "ready" && !confirmed) {
        confirmed = true;
        editingKey = false;
      }
    });
  });

  /** Shows the newest build GitHub has and tells the background, which keeps it and marks the
   * toolbar icon. GitHub blocked or down: the last answer stays, with no error. */
  function seeRelease(r: Release | null) {
    if (!r) return;
    release = r;
    now = Date.now();
    sendBackground({ type: "RELEASE_SEEN", release: r }).catch(() => {});
  }

  chrome.storage.local
    .get("release")
    .then(({ release: kept }) => (release ??= (kept as Release) ?? null), () => {});
  void releaseFromGitHub().then(seeRelease);

  function pairAgain() {
    view = "main";
    editingKey = true;
    autoStartPairing = true;
  }

  async function onPaired(key: string) {
    token = key;
    await saveSettings();
  }

  async function signIn() {
    if (!token.trim()) return;
    notice = "";
    await saveSettings();
    // The popup may close while this runs; the background keeps going and the next popup picks it up.
    await sendBackground({ type: "LOGIN_WITH_TOKEN", source: "popup" });
  }

  async function cancelLogin() {
    try {
      await sendBackground({ type: "CANCEL_LOGIN" });
    } catch {
      // Nothing running.
    }
  }

  async function resetState() {
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
    savedBridgeUrl = DEFAULT_BRIDGE_URL;
    token = "";
    autoLogin = false;
    reSignIn = true;
    notify = false;
    lastLogin = undefined;
    editingKey = true;
    autoStartPairing = false;
    pairingActive = false;
    view = "main";
    notice = "Settings cleared.";
  }

  function openUpdate() {
    if (!release) return;
    const url = FIREFOX ? (release.xpiUrl ?? release.url) : release.url;
    void chrome.tabs.create({ url });
  }

  void loadSettings().catch((err) => {
    console.warn("VITAP Mate: failed to load settings", err);
    vtopSignedIn = false;
  });
</script>

<main class="popup">
  {#if view === "settings"}
    <SettingsView
      {token}
      {autoLogin}
      {reSignIn}
      {notify}
      {savedBridgeUrl}
      version={VERSION}
      {release}
      {behind}
      {running}
      {now}
      onBack={() => (view = "main")}
      onToggleAutoLogin={() => {
        autoLogin = !autoLogin;
        void saveSettings();
      }}
      onToggleReSignIn={() => {
        reSignIn = !reSignIn;
        void saveSettings();
      }}
      onToggleNotify={toggleNotify}
      onServerChange={(origin) => (savedBridgeUrl = origin)}
      onReset={resetState}
      onOpenUpdate={openUpdate}
    />
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
      {#if unreachable}
        <div class="banner alert">
          <div class="ic"><Icon name="wifi-off" size={17} /></div>
          <div class="grow">
            <div class="bt">Can't reach VITAP Mate's server</div>
            <div class="bs">Check your connection. Signing in needs the server.</div>
            <div class="acts">
              <button type="button" class="text-btn strong" onclick={() => liveTry++}>Retry</button>
            </div>
          </div>
        </div>
      {/if}

      <UpdateBanner {release} version={VERSION} {behind} onOpenUpdate={openUpdate} />

      {#if keyRejected && !editingKey}
        <div class="banner alert">
          <div class="ic"><Icon name="key-round" size={17} /></div>
          <div class="grow">
            <div class="bt">This key was revoked</div>
            <div class="bs">
              It was removed in VITAP Mate → Connected apps, or belongs to another server. Pair again to keep signing in.
            </div>
            <div class="acts">
              <button type="button" class="text-btn strong" onclick={pairAgain}>Pair again</button>
            </div>
          </div>
        </div>
      {/if}

      {#if editingKey}
        <PairingView
          bind:token
          {savedBridgeUrl}
          {keyRejected}
          {now}
          autoStart={autoStartPairing}
          onAutoStartHandled={() => (autoStartPairing = false)}
          onTokenInput={() => void saveSettings()}
          {onPaired}
          onUseKey={() => (editingKey = false)}
          onPairingChange={(active) => (pairingActive = active)}
        />
      {:else}
        <RouteCard {login} {running} {vtopSignedIn} {lastLogin} {now} />

        {#if running}
          <button type="button" class="btn tonal" onclick={cancelLogin}>
            <Icon name="x" size={19} />Cancel
          </button>
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
              {@const failed = Boolean(entry.failed)}
              {@const startedAt = login.phase === "idle" ? entry.at : login.startedAt}
              <div
                class="step"
                class:now={last && running}
                class:fail={failed}
                class:done={!failed && !(last && running)}
              >
                <span class="at">{((entry.at - startedAt) / 1000).toFixed(1)}s</span>
                <span class="pip"></span>
                <span>{entry.text}</span>
              </div>
            {/each}
          </div>
        {/if}

        <AccountCard
          {token}
          {account}
          {keyRejected}
          {now}
          onEditKey={() => (editingKey = true)}
        />

        <div class="card">
          <div class="item">
            <div class="txt">
              <div class="t">Auto sign-in</div>
              <div class="s">When you open vtop.vitap.ac.in</div>
            </div>
            <Switch
              on={autoLogin}
              disabled={!isAccessKey(token)}
              label="Auto sign-in"
              onchange={() => {
                autoLogin = !autoLogin;
                void saveSettings();
              }}
            />
          </div>
          <div class="item">
            <div class="txt">
              <div class="t">Sign in again</div>
              <div class="s">When VTOP signs you out mid-use</div>
            </div>
            <Switch
              on={reSignIn}
              disabled={!isAccessKey(token)}
              label="Sign in again"
              onchange={() => {
                reSignIn = !reSignIn;
                void saveSettings();
              }}
            />
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
</style>
