<script lang="ts">
  import { UNKNOWN_KEY_MESSAGE } from "./bridge";
  import { ago } from "./format";
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";
  import type { KeyView } from "./live";

  let {
    token,
    account,
    keyRejected,
    now,
    onEditKey,
  }: {
    token: string;
    account: KeyView | undefined;
    keyRejected: boolean;
    now: number;
    onEditKey: () => void;
  } = $props();

  type Row = { icon: IconName; label: string; value: string; tone: "" | "good" | "warn" | "bad" };

  function accountRows(a: KeyView, atNow: number): Row[] {
    const rows: Row[] = [];
    const s = a.settings;
    const server = a.savedCredentials;
    let order: string;
    if (!server) order = a.phoneLinked ? "Phone only" : "No way to sign in";
    else if (s.alwaysUseVault) order = "Server first";
    else order = a.phoneLinked ? `Phone, then the server after ${s.phoneWaitSecs} s` : "Server";
    rows.push({
      icon: "route",
      label: "Who signs in",
      value: order,
      tone: server || a.phoneLinked ? "" : "bad",
    });
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
          ? {
              icon: "smartphone",
              label: "This key's phone",
              value: `${name} linked, notifications on`,
              tone: "good",
            }
          : {
              icon: "bell-off",
              label: "This key's phone",
              value: `${name} linked, notifications off`,
              tone: "warn",
            },
      );
    } else {
      rows.push({ icon: "smartphone", label: "This key's phone", value: "Not linked", tone: "warn" });
    }
    if (a.keyLastUsedAt) {
      rows.push({
        icon: "history",
        label: "This key last used",
        value: ago(a.keyLastUsedAt * 1000, atNow),
        tone: "",
      });
    }
    return rows;
  }

  const trimmed = $derived(token.trim());
</script>

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
      <div class="s mono">{trimmed.slice(0, 8)}…{trimmed.slice(-4)}</div>
    </div>
    <button type="button" class="text-btn" onclick={onEditKey}>Change</button>
  </div>
  {#if keyRejected}
    <div class="help bad foot-pad">{UNKNOWN_KEY_MESSAGE}</div>
  {:else if account}
    <div class="divider"></div>
    {#each accountRows(account, now) as row}
      <div class="item">
        <div class="lead {row.tone}"><Icon name={row.icon} size={18} /></div>
        <div class="txt">
          <div class="t">{row.label}</div>
          <div class="s {row.tone}">{row.value}</div>
        </div>
      </div>
    {/each}
    <div class="foot">Change these in VITAP Mate → Connected apps.</div>
  {/if}
</div>

<style>
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
  .foot-pad {
    padding-bottom: 10px;
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
</style>
