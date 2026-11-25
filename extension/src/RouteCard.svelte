<script lang="ts">
  import { BROWSER_NAME, VIA, ago, sourceText } from "./format";
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";
  import { cookiePath, wavePath } from "./shapes";
  import { handedOverOf, sourceOf, viaOf, type LastLogin, type LoginPhase, type LoginState, type LoginVia } from "./state";

  let {
    login,
    running,
    vtopSignedIn,
    lastLogin,
    now,
  }: {
    login: LoginState;
    running: boolean;
    vtopSignedIn: boolean | null;
    lastLogin: LastLogin | undefined;
    now: number;
  } = $props();

  const COOKIE = cookiePath(7);
  const BLOB = cookiePath(9);
  const WAVE = wavePath(300);

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
    if (s.phase === "idle") return "idle";
    const failed = s.phase === "error";
    const phase = failed ? s.failedPhase : s.phase;
    const rank = RANK[phase];
    const via = viaOf(s);
    // The browser was still signed in: the session never left it.
    if (via === "browser" && node !== "chrome" && node !== "vtop") return "skip";
    if (node === "phone" || node === "server") {
      const used = via === node || phase === node;
      // The phone was asked first and the server took over.
      const handedOver = node === "phone" && (via === "server" || phase === "server") && Boolean(handedOverOf(s));
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
      ? {
          phase: "done",
          text: "",
          via: lastLogin.source?.from === "server" ? "server" : (lastLogin.via as LoginVia | undefined),
          startedAt: lastLogin.at - lastLogin.ms,
          finishedAt: lastLogin.at,
          log: [],
        }
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

  const deadline = $derived(
    login.phase === "request" || login.phase === "phone" || login.phase === "server"
      ? login.deadline
      : undefined,
  );
  const secondsLeft = $derived(
    deadline && deadline > now ? Math.ceil((deadline - now) / 1000) : 0,
  );
  const serverIn = $derived(
    login.phase === "phone" && login.serverDueAt && login.serverDueAt > now
      ? Math.ceil((login.serverDueAt - now) / 1000)
      : 0,
  );
  const elapsed = $derived(
    login.phase === "idle"
      ? 0
      : ((login.phase === "done" || login.phase === "error" ? login.finishedAt : now) - login.startedAt) / 1000,
  );

  /** The hero: its tone, icon, headline and line under it. */
  const hero = $derived.by(() => {
    if (login.phase === "error") {
      const text = login.error;
      const cut = text.indexOf(". ");
      return {
        tone: "bad",
        icon: "triangle-alert" as IconName,
        title: cut > 0 ? text.slice(0, cut) : text.replace(/\.$/, ""),
        sub: cut > 0 ? text.slice(cut + 2) : "",
      };
    }
    if (login.phase === "done") {
      return {
        tone: "ok",
        icon: "check" as IconName,
        title: `Signed in via ${VIA[login.via ?? "phone"]}`,
        sub: "VTOP is opening in this tab",
      };
    }
    if (running) {
      const sub = login.text.replace(/…$/, "");
      switch (login.phase) {
        case "phone":
          return {
            tone: "active",
            icon: "smartphone" as IconName,
            title: "Waiting for your phone",
            sub: "Open VITAP Mate on your phone to approve",
          };
        case "server":
          if (login.loginStep === "waiting_otp") {
            return {
              tone: "active",
              icon: "mail-check" as IconName,
              title: "Waiting for the OTP",
              sub: "The server is signed in and waits for VTOP's email",
            };
          }
          if (login.loginStep === "checking_otp") {
            return {
              tone: "active",
              icon: "mail-check" as IconName,
              title: "Got the OTP",
              sub: "Finishing the sign-in",
            };
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
    if (vtopSignedIn === null) {
      return {
        tone: "",
        icon: "loader-circle" as IconName,
        title: "Checking VTOP",
        sub: "Is this browser still signed in?",
      };
    }
    if (vtopSignedIn) {
      const sub = lastLogin
        ? `Last sign-in ${ago(lastLogin.at, now)}, ${lastLogin.source?.cached ? "reused from" : "via"} ${VIA[lastLogin.source?.from ?? lastLogin.via] ?? lastLogin.via} in ${(lastLogin.ms / 1000).toFixed(1)} s`
        : "This browser keeps the session until VTOP ends it";
      return { tone: "ok", icon: "check" as IconName, title: "Signed in to VTOP", sub };
    }
    return {
      tone: "",
      icon: "log-in" as IconName,
      title: "Not signed in yet",
      sub: "Your phone or the server makes a session for this browser",
    };
  });

  const loginSource = $derived(sourceOf(login));
  const cookieSource = $derived(
    login.phase !== "idle" ? sourceText(viaOf(login), loginSource, now) : "",
  );
</script>

<div class="hero {hero.tone}">
  <svg class="blob" viewBox="0 0 100 100" aria-hidden="true"><path d={BLOB} /></svg>
  <div class="hero-top">
    <div class="cookie">
      <svg
        class="shape"
        class:spin={running || vtopSignedIn === null}
        viewBox="0 0 100 100"
        aria-hidden="true"><path d={COOKIE} /></svg
      >
      <Icon name={hero.icon} size={24} />
    </div>
    <div class="grow" role="status">
      <h1>{hero.title}</h1>
      {#if hero.sub}<div class="sub">{hero.sub}</div>{/if}
    </div>
    {#if login.phase !== "idle"}<div class="clock">{elapsed.toFixed(1)} s</div>{/if}
  </div>

  <div class="route" aria-label="Sign-in route">
    <div class="node {looks.chrome}">
      <div class="dot"><Icon name="chrome" size={18} /></div>
      <span>{BROWSER_NAME}</span>
    </div>
    <div class="link {linkClass(looks.chrome, looks.edge)}"></div>
    <div class="node {looks.edge}">
      <div class="dot"><Icon name="sparkles" size={18} /></div>
      <span>Mate</span>
    </div>
    <div class="link {linkClass(looks.edge, forkLook)}"></div>
    <div class="fork">
      <div class="node {looks.phone}">
        <div class="dot"><Icon name="smartphone" size={15} /></div>
        <span>Phone</span>
      </div>
      <div class="node {looks.server}">
        <div class="dot"><Icon name="server" size={15} /></div>
        <span>Server</span>
      </div>
    </div>
    <div class="link {linkClass(forkLook, looks.vtop)}"></div>
    <div class="node {looks.vtop}">
      <div class="dot"><Icon name="school" size={18} /></div>
      <span>VTOP</span>
    </div>
  </div>

  {#if running && deadline}
    <div class="wave">
      <svg viewBox="0 0 300 14" preserveAspectRatio="none" aria-hidden="true">
        <defs>
          <clipPath id="wave-clip">
            <rect
              x="0"
              y="-4"
              width={Math.max(0, Math.min(300, (secondsLeft / 120) * 300))}
              height="22"
            />
          </clipPath>
        </defs>
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

  {#if cookieSource}
    <div class="source" class:reused={loginSource?.cached}>
      <Icon name="cookie" size={14} />{cookieSource}
    </div>
  {/if}
</div>

<style>
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
    background: repeating-linear-gradient(
      90deg,
      color-mix(in srgb, currentColor 22%, transparent) 0 5px,
      transparent 5px 9px
    );
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
</style>
