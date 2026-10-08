<script lang="ts">
  import { isAccessKey } from "./bridge";

  let token = "";
  let showToken = false;
  let autoLogin = false;
  let loading = false;
  let status = "";
  let error = "";

  async function loadSettings() {
    const stored = await chrome.storage.local.get(["token", "fmcToken", "autoLogin"]);
    token = String(stored.token ?? stored.fmcToken ?? "");
    autoLogin = Boolean(stored.autoLogin);

    // Migrate older installations without making the implementation name visible.
    if (!stored.token && stored.fmcToken) {
      await chrome.storage.local.set({ token });
      await chrome.storage.local.remove("fmcToken");
    }
  }

  async function saveSettings() {
    if (!token.trim()) autoLogin = false;
    await chrome.storage.local.set({ token: token.trim(), autoLogin });
  }

  async function login() {
    if (!token.trim()) {
      error = "Paste the token copied from the VITAP Mate app.";
      return;
    }

    loading = true;
    status = "Keep your mobile online…";
    error = "";

    try {
      await saveSettings();
      const response = await chrome.runtime.sendMessage({
        type: "LOGIN_WITH_TOKEN",
        source: "popup",
      });

      if (!response?.ok) {
        throw new Error(response?.error || "Could not sign in. Please try again.");
      }

      status = "Signed in. Opening VTOP…";
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not sign in. Please try again.";
      status = "";
    } finally {
      loading = false;
    }
  }

  async function resetState() {
    await chrome.storage.local.remove([
      "token",
      "fmcToken",
      "autoLogin",
      "lastAutoLoginAt",
    ]);
    token = "";
    showToken = false;
    autoLogin = false;
    loading = false;
    status = "Settings cleared.";
    error = "";
  }

  loadSettings();

  $: showKeyHint = token.trim().length > 0 && !isAccessKey(token);
</script>

<main class="min-h-screen w-full min-w-[320px] bg-slate-950 text-slate-100">
  <header class="border-b border-slate-800 bg-gradient-to-br from-indigo-950 via-slate-950 to-slate-950 px-5 py-5">
    <div class="flex items-center gap-3">
      <img src="icons/icon-48.png" alt="" class="h-11 w-11 rounded-xl shadow-lg" />
      <div class="min-w-0 flex-1">
        <h1 class="text-lg font-semibold tracking-tight">VITAP Mate</h1>
        <p class="text-xs text-slate-400">One-click VTOP sign in</p>
      </div>
      <button
        type="button"
        disabled={loading}
        on:click={resetState}
        class="rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-400 hover:bg-slate-800 hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-indigo-400 disabled:opacity-50"
        aria-label="Clear extension settings"
      >
        Reset
      </button>
    </div>
  </header>

  <section class="space-y-4 p-5">
    <div class="rounded-xl border border-indigo-900/70 bg-indigo-950/40 p-3 text-xs leading-5 text-indigo-100">
      In the VITAP Mate app, open <strong>More → Chrome Extension</strong>, copy your Token, then paste it below.
    </div>

    <div class="space-y-2">
      <label for="token" class="block text-sm font-medium">Access key</label>
      <div class="flex gap-2">
        <input
          id="token"
          type={showToken ? "text" : "password"}
          bind:value={token}
          on:input={saveSettings}
          placeholder="vtm_…"
          autocomplete="off"
          spellcheck="false"
          class="min-w-0 flex-1 rounded-xl border border-slate-700 bg-slate-900 px-3 py-2.5 text-sm outline-none placeholder:text-slate-500 focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/20"
        />
        <button
          type="button"
          on:click={() => (showToken = !showToken)}
          class="rounded-xl border border-slate-700 px-3 text-xs font-medium hover:bg-slate-800 focus-visible:outline focus-visible:outline-2 focus-visible:outline-indigo-400"
          aria-label={showToken ? "Hide access key" : "Show access key"}
        >
          {showToken ? "Hide" : "Show"}
        </button>
      </div>
      {#if showKeyHint}
        <p class="text-xs text-amber-400">This doesn't look like an access key.</p>
      {/if}
      <p class="text-xs text-slate-500">Create one in VITAP Mate → Connected apps.</p>
    </div>

    <div class="flex items-start gap-3 rounded-xl border border-sky-900/70 bg-sky-950/40 p-3">
      <span class="mt-1 h-2.5 w-2.5 shrink-0 rounded-full bg-sky-400 shadow-[0_0_10px_rgba(56,189,248,0.8)]"></span>
      <div>
        <p class="text-sm font-medium text-sky-100">Connects through your mobile</p>
        <p class="mt-1 text-xs leading-5 text-sky-200/70">
          Keep the VITAP Mate app online while signing in. Your mobile securely sends the VTOP session to Chrome.
        </p>
      </div>
    </div>

    <button
      type="button"
      disabled={loading || !token.trim()}
      on:click={login}
      class="w-full rounded-xl bg-indigo-500 px-4 py-2.5 text-sm font-semibold text-white shadow-lg shadow-indigo-950 hover:bg-indigo-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-400 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {loading ? "Signing in…" : "Sign in to VTOP"}
    </button>

    {#if status}
      <p role="status" class="rounded-xl border border-emerald-900 bg-emerald-950/60 px-3 py-2.5 text-xs text-emerald-200">
        {status}
      </p>
    {/if}

    {#if error}
      <p role="alert" class="rounded-xl border border-rose-900 bg-rose-950/60 px-3 py-2.5 text-xs leading-5 text-rose-200">
        {error}
      </p>
    {/if}
  </section>
</main>
