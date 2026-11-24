import type { SessionSource } from "./bridge";

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

/** Where a sign-in is. `phone` and `server` are the two ways a session is made. */
export type LoginPhase =
  | "idle"
  | "check"
  | "request"
  | "phone"
  | "server"
  | "cookies"
  | "open"
  | "done"
  | "error";

export type LoginState = {
  phase: LoginPhase;
  /** The step's text, as shown on the VTOP page too. */
  text: string;
  /** Which way the session came, once known. */
  via?: "browser" | "phone" | "server";
  /** Where the session the server handed out came from. */
  source?: SessionSource;
  startedAt?: number;
  finishedAt?: number;
  /** When the wait for a session gives up (ms). */
  deadline?: number;
  /** When the server takes over from a phone that hasn't answered (ms). */
  serverDueAt?: number;
  /** What the server's sign-in is doing, while it signs in. */
  loginStep?: "signing_in" | "waiting_otp" | "checking_otp";
  error?: string;
  /** The step that failed, so the popup can mark it. */
  failedPhase?: LoginPhase;
  /** What happened, with times, for the popup's activity list. */
  log: { at: number; text: string }[];
};

/** The parts of `chrome.storage.session` used here, so tests can pass a fake. */
export type SessionStorageApi = {
  get(keys: string): Promise<Record<string, unknown>>;
  set(items: Record<string, unknown>): Promise<void>;
};

export const LOGIN_STATE_KEY = "loginState";

export const INTERRUPTED_ERROR =
  "Sign-in was interrupted when the browser suspended the extension. Try again.";

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

const TERMINAL_PHASES: ReadonlySet<LoginPhase> = new Set<LoginPhase>([
  "idle",
  "done",
  "error",
]);

export function createIdleState(): LoginState {
  return { phase: "idle", text: "", log: [] };
}

export function isInProgressPhase(phase: LoginPhase): boolean {
  return !TERMINAL_PHASES.has(phase);
}

export function isLoginState(value: unknown): value is LoginState {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  if (typeof candidate.phase !== "string" || !VALID_PHASES.has(candidate.phase)) {
    return false;
  }
  if (typeof candidate.text !== "string") return false;
  if (!Array.isArray(candidate.log)) return false;
  return candidate.log.every(
    (entry) =>
      typeof entry === "object" &&
      entry !== null &&
      typeof (entry as Record<string, unknown>).at === "number" &&
      typeof (entry as Record<string, unknown>).text === "string",
  );
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
  if (inFlight || !isInProgressPhase(restored.phase)) {
    return { state: restored, normalized: false };
  }

  return {
    normalized: true,
    state: {
      ...restored,
      phase: "error",
      text: INTERRUPTED_ERROR,
      failedPhase: restored.phase,
      error: INTERRUPTED_ERROR,
      finishedAt: now,
      deadline: undefined,
      serverDueAt: undefined,
      log: [...restored.log, { at: now, text: "Interrupted" }],
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
