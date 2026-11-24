<script lang="ts">
  import { DEFAULT_BRIDGE_URL, isAccessKey, normalizeBridgeUrl } from "./bridge";
  import { ago } from "./format";
  import Icon from "./Icon.svelte";
  import type { Release } from "./live";
  import Switch from "./Switch.svelte";
  import UpdateBanner from "./UpdateBanner.svelte";

  let {
    token,
    autoLogin,
    reSignIn,
    notify,
    savedBridgeUrl,
    version,
    release,
    behind,
    running,
    now,
    onBack,
    onToggleAutoLogin,
    onToggleReSignIn,
    onToggleNotify,
    onServerChange,
    onReset,
    onOpenUpdate,
  }: {
    token: string;
    autoLogin: boolean;
    reSignIn: boolean;
    notify: boolean;
    savedBridgeUrl: string;
    version: string;
    release: Release | null;
    behind: boolean;
    running: boolean;
    now: number;
    onBack: () => void;
    onToggleAutoLogin: () => void;
    onToggleReSignIn: () => void;
    onToggleNotify: () => void;
    onServerChange: (origin: string) => void;
    onReset: () => void;
    onOpenUpdate: () => void;
  } = $props();

  let bridgeUrl = $state("");
  let serverMessage = $state("");
  let serverError = $state("");

  $effect(() => {
    bridgeUrl = savedBridgeUrl;
  });

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
    bridgeUrl = origin;
    onServerChange(origin);
    serverMessage = "Saved.";
  }

  async function useDefaultServer() {
    await chrome.storage.local.remove("bridgeUrl");
    bridgeUrl = DEFAULT_BRIDGE_URL;
    onServerChange(DEFAULT_BRIDGE_URL);
    serverError = "";
    serverMessage = "Using the default server.";
  }
</script>

<div class="sub-bar">
  <button type="button" class="icon-btn" aria-label="Back" onclick={onBack}>
    <Icon name="arrow-left" />
  </button>
  <div class="title">Settings</div>
</div>
<div class="body">
  <div class="section-label">Signing in</div>
  <div class="card">
    <div class="item">
      <div class="lead"><Icon name="log-in" size={18} /></div>
      <div class="txt">
        <div class="t">Auto sign-in</div>
        <div class="s">When you open vtop.vitap.ac.in</div>
      </div>
      <Switch
        on={autoLogin}
        disabled={!isAccessKey(token)}
        label="Auto sign-in"
        onchange={onToggleAutoLogin}
      />
    </div>
    <div class="item">
      <div class="lead"><Icon name="refresh-cw" size={18} /></div>
      <div class="txt">
        <div class="t">Sign in again</div>
        <div class="s">When VTOP signs you out mid-use</div>
      </div>
      <Switch
        on={reSignIn}
        disabled={!isAccessKey(token)}
        label="Sign in again"
        onchange={onToggleReSignIn}
      />
    </div>
    <div class="item">
      <div class="lead"><Icon name={notify ? "bell" : "bell-off"} size={18} /></div>
      <div class="txt">
        <div class="t">Notifications</div>
        <div class="s">When your phone needs to approve, or a sign-in fails</div>
      </div>
      <Switch on={notify} label="Notifications" onchange={onToggleNotify} />
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
      <div class="help">
        {savedBridgeUrl === DEFAULT_BRIDGE_URL
          ? "Using the default server."
          : `Using ${new URL(savedBridgeUrl).host}.`}
      </div>
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
  <UpdateBanner {release} {version} {behind} {onOpenUpdate} />
  <div class="card">
    <div class="item">
      <div class="lead"><Icon name="package" size={18} /></div>
      <div class="txt">
        <div class="t">Version {version}</div>
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
      <div class="txt">
        <div class="t">Reset all</div>
        <div class="s">Forget the key, the server and the last sign-in</div>
      </div>
      <button type="button" class="chip danger" disabled={running} onclick={onReset}>Reset</button>
    </div>
  </div>
</div>

<style>
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
  .section-label {
    font-size: 13px;
    font-weight: 650;
    color: var(--primary);
    padding: 6px 6px 0;
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
</style>
