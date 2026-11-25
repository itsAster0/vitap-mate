import type { SessionSource } from "./bridge";

/** The last finished sign-in, kept for the popup. */
export type LastLogin = { at: number; via: string; ms: number; source?: SessionSource };

/** What the popup and the page script keep in `chrome.storage.local`. */
export type Settings = {
  token?: string;
  bridgeUrl?: string;
  /** Sign in when vtop.vitap.ac.in is opened. */
  autoLogin?: boolean;
  /** Sign in again when VTOP sends the browser back to its login page. */
  reSignIn?: boolean;
  /** Notify when the phone has to approve, and when a sign-in fails. */
  notify?: boolean;
  /** When the student signed out of VTOP themselves (ms): no signing in again after that. */
  signedOutAt?: number;
};

export type LoginVia = "browser" | "phone" | "server";
export type LoginStep = "signing_in" | "waiting_otp" | "checking_otp";
export type LoginLogEntry = { at: number; text: string; failed?: boolean };

/** In-progress sign-in phases. */
export type InProgressPhase = "check" | "request" | "phone" | "server" | "cookies" | "open";
/** Where a sign-in is. `phone` and `server` are the two ways a session is made. */
export type LoginPhase = "idle" | InProgressPhase | "done" | "error";

type ActiveBase = {
  /** The step's text, as shown on the VTOP page too. */
  text: string;
  startedAt: number;
  /** What happened, with times, for the popup's activity list. */
  log: LoginLogEntry[];
};

export type IdleLoginState = {
  phase: "idle";
  text: "";
  log: [];
};

export type CheckLoginState = ActiveBase & {
  phase: "check";
};

export type RequestLoginState = ActiveBase & {
  phase: "request";
  via?: LoginVia;
  /** When the wait for a session gives up (ms). */
  deadline?: number;
};

export type PhoneLoginState = ActiveBase & {
  phase: "phone";
  via: "phone";
  /** When the wait for a session gives up (ms). */
  deadline?: number;
  /** When the server takes over from a phone that hasn't answered (ms). */
  serverDueAt?: number;
};

export type ServerLoginState = ActiveBase & {
  phase: "server";
  via: "server";
  /** When the wait for a session gives up (ms). */
  deadline?: number;
  /** What the server's sign-in is doing, while it signs in. */
  loginStep?: LoginStep;
  /** True when the phone was asked first and the server took over. */
  handedOver?: boolean;
};

export type CookiesLoginState = ActiveBase & {
  phase: "cookies";
  via?: LoginVia;
  source?: SessionSource;
  handedOver?: boolean;
};

export type OpenLoginState = ActiveBase & {
  phase: "open";
  via?: LoginVia;
  source?: SessionSource;
  handedOver?: boolean;
};

export type DoneLoginState = ActiveBase & {
  phase: "done";
  finishedAt: number;
  via?: LoginVia;
  source?: SessionSource;
  handedOver?: boolean;
};

export type ErrorLoginState = ActiveBase & {
  phase: "error";
  finishedAt: number;
  error: string;
  /** The step that failed, so the popup can mark it. */
  failedPhase: LoginPhase;
  cancelled?: boolean;
  via?: LoginVia;
  source?: SessionSource;
  handedOver?: boolean;
};

export type InProgressLoginState =
  | CheckLoginState
  | RequestLoginState
  | PhoneLoginState
  | ServerLoginState
  | CookiesLoginState
  | OpenLoginState;

export type LoginState = IdleLoginState | InProgressLoginState | DoneLoginState | ErrorLoginState;

/** Step transition passed to `background.ts`'s `report()`. */
export type ReportStep =
  | { phase?: undefined }
  | { phase: "check" }
  | { phase: "request"; via?: LoginVia; deadline?: number }
  | { phase: "phone"; via?: "phone"; deadline?: number; serverDueAt?: number }
  | { phase: "server"; via?: "server"; deadline?: number; loginStep?: LoginStep; handedOver?: boolean }
  | { phase: "cookies"; via?: LoginVia; source?: SessionSource }
  | { phase: "open"; via?: LoginVia };

/** The parts of `chrome.storage.session` used here, so tests can pass a fake. */
export type SessionStorageApi = {
  get(keys: string): Promise<Record<string, unknown>>;
  set(items: Record<string, unknown>): Promise<void>;
};

export const LOGIN_STATE_KEY = "loginState";

export const INTERRUPTED_ERROR =
  "Sign-in was interrupted when the browser suspended the extension. Try again.";

const TERMINAL_PHASES: ReadonlySet<LoginPhase> = new Set<LoginPhase>(["idle", "done", "error"]);
const VALID_VIA: ReadonlySet<string> = new Set<LoginVia>(["browser", "phone", "server"]);
const VALID_STEPS: ReadonlySet<string> = new Set<LoginStep>(["signing_in", "waiting_otp", "checking_otp"]);
const VALID_PHASES: ReadonlySet<string> = new Set<LoginPhase>([
  "idle",
  "check",
  "request",
  "phone",
  "server",
  "cookies",
  "open",
  "done",
  "error",
]);

export function createIdleState(): IdleLoginState {
  return { phase: "idle", text: "", log: [] };
}

export function isInProgressPhase(phase: LoginPhase): phase is InProgressPhase {
  return !TERMINAL_PHASES.has(phase);
}

export function isInProgressState(state: LoginState): state is InProgressLoginState {
  return isInProgressPhase(state.phase);
}

export function viaOf(state: LoginState): LoginVia | undefined {
  return "via" in state ? state.via : undefined;
}

export function sourceOf(state: LoginState): SessionSource | undefined {
  return "source" in state ? state.source : undefined;
}

export function handedOverOf(state: LoginState): boolean | undefined {
  return "handedOver" in state ? state.handedOver : undefined;
}

/** Pure transition for `report()`: builds the next phase-valid `LoginState` from `current`. */
export function applyReportStep(
  current: LoginState,
  step: ReportStep,
  text: string,
  at: number,
  extraLog?: string,
): LoginState {
  const targetPhase = step.phase ?? (current.phase === "idle" ? "check" : current.phase);
  const currentStep = current.phase === "server" ? current.loginStep : undefined;
  const nextStep = step.phase === "server" ? step.loginStep : undefined;
  const newStep =
    (step.phase !== undefined && step.phase !== current.phase) ||
    (nextStep !== undefined && nextStep !== currentStep);

  const log: LoginLogEntry[] = extraLog ? [...current.log, { at, text: extraLog }] : [...current.log];
  if (newStep || log.length === 0) {
    log.push({ at, text });
  }

  const startedAt = current.phase === "idle" ? at : current.startedAt;
  const prevVia = viaOf(current);
  const prevSource = sourceOf(current);
  const prevHandedOver = handedOverOf(current);

  if (step.phase === undefined) {
    if (current.phase === "idle") {
      return { phase: "check", text, startedAt, log };
    }
    return { ...current, text, log };
  }

  switch (targetPhase) {
    case "check":
      return { phase: "check", text, startedAt, log };
    case "request": {
      const s = step as Extract<ReportStep, { phase: "request" }>;
      return {
        phase: "request",
        text,
        startedAt,
        via: s.via ?? prevVia,
        deadline: s.deadline,
        log,
      };
    }
    case "phone": {
      const s = step as Extract<ReportStep, { phase: "phone" }>;
      return {
        phase: "phone",
        text,
        startedAt,
        via: "phone",
        deadline: s.deadline,
        serverDueAt: s.serverDueAt,
        log,
      };
    }
    case "server": {
      const s = step as Extract<ReportStep, { phase: "server" }>;
      return {
        phase: "server",
        text,
        startedAt,
        via: "server",
        deadline: s.deadline,
        loginStep: s.loginStep,
        handedOver: s.handedOver ?? prevHandedOver,
        log,
      };
    }
    case "cookies": {
      const s = step as Extract<ReportStep, { phase: "cookies" }>;
      return {
        phase: "cookies",
        text,
        startedAt,
        via: s.via ?? prevVia,
        source: s.source ?? prevSource,
        handedOver: prevHandedOver,
        log,
      };
    }
    case "open": {
      const s = step as Extract<ReportStep, { phase: "open" }>;
      return {
        phase: "open",
        text,
        startedAt,
        via: s.via ?? prevVia,
        source: prevSource,
        handedOver: prevHandedOver,
        log,
      };
    }
    default:
      return { phase: "check", text, startedAt, log };
  }
}

function isOptNumber(value: unknown): value is number | undefined {
  return value === undefined || typeof value === "number";
}

function isOptVia(value: unknown): value is LoginVia | undefined {
  return value === undefined || (typeof value === "string" && VALID_VIA.has(value));
}

export function isLoginState(value: unknown): value is LoginState {
  if (typeof value !== "object" || value === null) return false;
  const c = value as Record<string, unknown>;
  if (typeof c.phase !== "string" || !VALID_PHASES.has(c.phase)) return false;
  if (typeof c.text !== "string") return false;
  if (!Array.isArray(c.log)) return false;
  const logOk = c.log.every(
    (entry) =>
      typeof entry === "object" &&
      entry !== null &&
      typeof (entry as Record<string, unknown>).at === "number" &&
      typeof (entry as Record<string, unknown>).text === "string" &&
      ((entry as Record<string, unknown>).failed === undefined ||
        typeof (entry as Record<string, unknown>).failed === "boolean"),
  );
  if (!logOk) return false;

  if (c.phase === "idle") return true;
  if (typeof c.startedAt !== "number") return false;

  switch (c.phase) {
    case "check":
      return true;
    case "request":
      return isOptVia(c.via) && isOptNumber(c.deadline);
    case "phone":
      return c.via === "phone" && isOptNumber(c.deadline) && isOptNumber(c.serverDueAt);
    case "server":
      return (
        c.via === "server" &&
        isOptNumber(c.deadline) &&
        (c.loginStep === undefined || (typeof c.loginStep === "string" && VALID_STEPS.has(c.loginStep)))
      );
    case "cookies":
    case "open":
      return isOptVia(c.via);
    case "done":
      return typeof c.finishedAt === "number" && isOptVia(c.via);
    case "error":
      return (
        typeof c.finishedAt === "number" &&
        typeof c.error === "string" &&
        typeof c.failedPhase === "string" &&
        VALID_PHASES.has(c.failedPhase) &&
        isOptVia(c.via)
      );
    default:
      return false;
  }
}

/**
 * Returns `chrome.storage.session` when available, or `undefined` in older
 * browsers or unit tests where it is absent.
 */
export function getSessionStorage(): SessionStorageApi | undefined {
  try {
    const session = globalThis.chrome?.storage?.session;
    if (session && typeof session.get === "function" && typeof session.set === "function") {
      return session;
    }
  } catch {
    // Unavailable in this environment.
  }
  return undefined;
}

/**
 * Normalizes a persisted state restored after a service-worker restart.
 * Terminal states ("idle", "done", "error") are returned intact.
 * In-progress states when no login is actually running are normalized to an
 * error state so the popup never gets stuck on a spinner with no active work.
 */
export function normalizeRestoredLoginState(
  restored: LoginState,
  inFlight: boolean,
  now: number = Date.now(),
): { state: LoginState; normalized: boolean } {
  if (inFlight || !isInProgressState(restored)) {
    return { state: restored, normalized: false };
  }

  return {
    normalized: true,
    state: {
      phase: "error",
      text: INTERRUPTED_ERROR,
      via: viaOf(restored),
      source: sourceOf(restored),
      handedOver: handedOverOf(restored),
      startedAt: restored.startedAt,
      finishedAt: now,
      error: INTERRUPTED_ERROR,
      failedPhase: restored.phase,
      log: [...restored.log, { at: now, text: "Interrupted", failed: true }],
    },
  };
}

/** Persists `LoginState` to session storage; no-ops gracefully if unavailable. */
export async function saveLoginState(
  state: LoginState,
  storage: SessionStorageApi | undefined = getSessionStorage(),
): Promise<void> {
  if (!storage) return;
  try {
    await storage.set({ [LOGIN_STATE_KEY]: state });
  } catch {
    // Best-effort persistence; never break the login flow.
  }
}

/**
 * Loads and normalizes persisted `LoginState` from session storage.
 * Returns `null` when unavailable, empty, or malformed.
 */
export async function loadLoginState(
  inFlight: boolean,
  storage: SessionStorageApi | undefined = getSessionStorage(),
  now: number = Date.now(),
): Promise<LoginState | null> {
  if (!storage) return null;
  let stored: Record<string, unknown>;
  try {
    stored = await storage.get(LOGIN_STATE_KEY);
  } catch {
    return null;
  }

  const raw = stored?.[LOGIN_STATE_KEY];
  if (!isLoginState(raw)) return null;

  const { state, normalized } = normalizeRestoredLoginState(raw, inFlight, now);
  if (normalized) {
    await saveLoginState(state, storage);
  }
  return state;
}
