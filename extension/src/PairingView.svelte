<script lang="ts">
  import { encode } from "uqr";
  import { onDestroy } from "svelte";
  import {
    PAIR_TTL_MS,
    fetchWithTimeout,
    interpretPairResponse,
    isAccessKey,
    newPairCode,
  } from "./bridge";
  import Icon from "./Icon.svelte";
  import { pairCodeHash } from "./live";
  import { watchPairing } from "./popupLive";
  import { cookiePath } from "./shapes";

  const CLAIM_TIMEOUT_MS = 10_000;
  const COOKIE = cookiePath(7);
  const BLOB = cookiePath(9);
  const RING = cookiePath(12, 0.06);

  let {
    token = $bindable(""),
    savedBridgeUrl,
    keyRejected,
    now,
    autoStart = false,
    onAutoStartHandled,
    onTokenInput,
    onPaired,
    onUseKey,
    onPairingChange,
  }: {
    token: string;
    savedBridgeUrl: string;
    keyRejected: boolean;
    now: number;
    autoStart?: boolean;
    onAutoStartHandled: () => void;
    onTokenInput: () => void;
    onPaired: (key: string) => void;
    onUseKey: () => void;
    onPairingChange: (active: boolean) => void;
  } = $props();

  let pairing = $state("");
  let pairUntil = $state(0);
  let pairError = $state("");
  const pairQr = $derived(pairing ? encode(pairing, { border: 2 }) : null);
  const pairQrPath = $derived(
    pairQr
      ? pairQr.data.flatMap((row, y) => row.map((on, x) => (on ? `M${x} ${y}h1v1h-1z` : ""))).join("")
      : "",
  );
  const pairLeft = $derived(Math.max(0, Math.ceil((pairUntil - now) / 1000)));
  const showKeyHint = $derived(token.trim().length > 0 && !isAccessKey(token));

  let pairTimers: ReturnType<typeof setTimeout>[] = [];
  let stopPairWatch: (() => void) | undefined;

  function stopPairing() {
    for (const t of pairTimers) clearTimeout(t);
    pairTimers = [];
    stopPairWatch?.();
    stopPairWatch = undefined;
    if (pairing) {
      pairing = "";
      onPairingChange(false);
    }
  }
  onDestroy(stopPairing);

  /** Shows a new code and watches for the app to approve it, then takes the key once over HTTP. */
  async function startPairing() {
    stopPairing();
    pairError = "";
    const code = newPairCode();
    const until = Date.now() + PAIR_TTL_MS;
    const base = savedBridgeUrl;
    pairing = code;
    pairUntil = until;
    onPairingChange(true);
    const fail = (message: string) => {
      if (pairing !== code) return;
      stopPairing();
      pairError = message;
    };
    pairTimers.push(setTimeout(() => fail("The code ran out. Get a new one."), PAIR_TTL_MS));
    const codeHash = await pairCodeHash(code);
    if (pairing !== code) return;
    stopPairWatch = watchPairing(base, codeHash, {
      onReady: () => {
        if (pairing !== code) return;
        stopPairWatch?.();
        stopPairWatch = undefined;
        void claimKey(base, code, until, fail);
      },
      onError: (message) => fail(message),
    });
  }

  async function claimKey(
    base: string,
    code: string,
    until: number,
    fail: (message: string) => void,
  ) {
    while (pairing === code && Date.now() < until) {
      let wait = 1000;
      try {
        const response = await fetchWithTimeout(
          `${base}/v1/pair`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ code }),
          },
          CLAIM_TIMEOUT_MS,
        );
        const outcome = interpretPairResponse(
          response.status,
          await response.json().catch(() => null),
        );
        if (pairing !== code) return;
        if (outcome.kind === "ready") {
          stopPairing();
          onPaired(outcome.key);
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

  $effect(() => {
    if (autoStart) {
      onAutoStartHandled();
      void startPairing();
    }
  });
</script>

{#if pairing}
  <div class="pair-hero">
    <div class="qr-wrap">
      <svg class="shape spin" viewBox="0 0 100 100" aria-hidden="true"><path d={RING} /></svg>
      {#if pairQr}
        <div class="qr">
          <svg
            viewBox="0 0 {pairQr.size} {pairQr.size}"
            shape-rendering="crispEdges"
            role="img"
            aria-label="Pairing QR code"><path d={pairQrPath} fill="#000" /></svg
          >
        </div>
      {/if}
    </div>
    <div class="code" aria-label="Pairing code">
      {pairing.slice(0, 5)}<span class="dash">-</span>{pairing.slice(6)}
    </div>
    <div class="ring-time">
      Code works for {Math.floor(pairLeft / 60)}:{String(pairLeft % 60).padStart(2, "0")} more
    </div>
  </div>
  <ol class="steps">
    <li>Open the VITAP Mate app</li>
    <li><span><b>Connected apps</b> → <b>Pair a browser</b></span></li>
    <li>Scan the code or type it in</li>
  </ol>
  <button type="button" class="btn outline" onclick={stopPairing}>Cancel</button>
{:else}
  <div class="hero">
    <svg class="blob" viewBox="0 0 100 100" aria-hidden="true"><path d={BLOB} /></svg>
    <div class="hero-top">
      <div class="cookie">
        <svg class="shape" viewBox="0 0 100 100" aria-hidden="true"><path d={COOKIE} /></svg>
        <Icon name="link" size={24} />
      </div>
      <div>
        <h1>Pair with the app</h1>
        <div class="sub">The app makes a key for this browser. No password is shared.</div>
      </div>
    </div>
  </div>
  {#if !token.trim()}
    <ol class="steps">
      <li>Open the VITAP Mate app</li>
      <li><span><b>Connected apps</b> → <b>Pair a browser</b></span></li>
      <li>Scan the code or type it in</li>
    </ol>
  {/if}
  <button type="button" class="btn filled" onclick={startPairing}>
    <Icon name="qr-code" size={19} />Pair with the app
  </button>
  {#if pairError}<div class="help bad">{pairError}</div>{/if}
  <div class="field" class:bad={showKeyHint || keyRejected}>
    <label for="token">Or paste an access key</label>
    <input
      id="token"
      type="password"
      bind:value={token}
      oninput={onTokenInput}
      placeholder="vtm_…"
      autocomplete="off"
      spellcheck="false"
    />
  </div>
  {#if showKeyHint}
    <div class="help warn">This doesn't look like an access key.</div>
  {:else if keyRejected}
    <div class="help bad">
      Unknown or revoked key. Check the server in Settings, or create a new key.
    </div>
  {:else if isAccessKey(token)}
    <button type="button" class="text-btn center" onclick={onUseKey}>Use this key</button>
  {/if}
{/if}

<style>
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
</style>
