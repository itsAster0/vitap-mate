export type CookieEditorCookie = {
  domain: string;
  hostOnly?: boolean;
  httpOnly?: boolean;
  name: string;
  path?: string;
  sameSite?: "unspecified" | "lax" | "strict" | "no_restriction";
  secure?: boolean;
  session?: boolean;
  storeId?: string;
  value: string;
  expirationDate?: number;
  id?: number;
};

export type APIErrorResponse = {
  code?: string;
  error?: string;
};

export function validateCookies(cookies: CookieEditorCookie[]) {
  if (cookies.length === 0) throw new Error("The server sent no VTOP cookies. Try again.");

  for (const cookie of cookies) {
    if (!cookie.name?.trim()) throw new Error("The mobile app returned a cookie without a name.");
    if (!cookie.domain?.trim()) throw new Error(`Cookie ${cookie.name} is missing its domain.`);
    if (cookie.value === undefined || cookie.value === null || cookie.value === "") {
      throw new Error(`Cookie ${cookie.name} is missing its value.`);
    }
    if (cookie.domain.replace(/^\./, "") !== "vtop.vitap.ac.in") {
      throw new Error(`Cookie ${cookie.name} has an unexpected domain.`);
    }
  }
}

export function parseAPIError(responseText: string): APIErrorResponse {
  try {
    const parsed = JSON.parse(responseText) as APIErrorResponse;
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

export function friendlyLoginError(
  status: number,
  responseText: string,
  networkError?: unknown,
) {
  if (networkError instanceof DOMException && networkError.name === "AbortError") {
    return "The backend took too long to respond. Please try again.";
  }
  if (networkError) {
    return "Could not reach the backend. Check your internet connection and try again.";
  }

  const parsed = parseAPIError(responseText);
  const code = parsed.code ?? "";
  if (code === "key_unknown" || status === 401) return UNKNOWN_KEY_MESSAGE;
  if (code === "phone_unreachable") return "Your phone is not linked. Open VITAP Mate → Connected apps → Reconnect this phone.";
  if (code === "phone_timeout" || code === "request_expired" || status === 410) return PHONE_TIMEOUT_MESSAGE;
  if (code === "rate_limited" || status === 429) return "Too many attempts. Wait a minute and try again.";
  if ([502, 503, 504].includes(status)) {
    return "The sign-in service is temporarily unavailable. Please try again shortly.";
  }
  // The edge's request errors read `code: what to do`.
  return parsed.error?.replace(/^[a-z_]+:\s*/, "") || "Could not connect right now. Please try again.";
}

export async function fetchWithTimeout(
  input: RequestInfo | URL,
  init: RequestInit,
  timeoutMs: number,
) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timeout);
  }
}

export function clampPollAfter(value: unknown) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 1500;
  return Math.min(5000, Math.max(500, parsed));
}

const ACCESS_KEY_PATTERN = /^vtm_[0-9a-f]{64}$/;

const UNKNOWN_KEY_MESSAGE =
  "This access key is unknown or was revoked. Pair the extension again from VITAP Mate → Connected apps.";
const PHONE_TIMEOUT_MESSAGE =
  "Your phone did not answer in time. Open VITAP Mate once (so it can sign in) and try again.";

export function isAccessKey(value: string) {
  return ACCESS_KEY_PATTERN.test(value.trim());
}

/** VTOP sends dead sessions to its login (or open) page. */
export function isLoginRedirect(url: string) {
  try {
    const path = new URL(url).pathname;
    return path.startsWith("/vtop/login") || path.startsWith("/vtop/initialProcess");
  } catch {
    return url.toLowerCase().includes("/vtop/login");
  }
}

/** Who works on a pending request: the phone, the server, or neither yet. */
export type SessionStage = "phone" | "server" | "starting";

/** Where a ready session came from. */
export type SessionSource = {
  /** Who signed in to VTOP. */
  from: "phone" | "server" | "unknown";
  /** Handed out again from the server's cache rather than made now. */
  cached: boolean;
  /** When it was made (ms), if known. */
  madeAt?: number;
};

export type SessionOutcome =
  | { kind: "ready"; cookies: CookieEditorCookie[]; source: SessionSource }
  | {
      kind: "pending";
      requestId: string | undefined;
      pollAfterMs: number;
      stage: SessionStage;
      /** When the server takes over from a phone that hasn't answered (ms). */
      serverDueAt: number | undefined;
    }
  | { kind: "error"; message: string };

type SessionReply = {
  status?: string;
  code?: string;
  error?: string;
  requestId?: string;
  pollAfterMs?: number;
  stage?: string;
  serverDueAt?: string | null;
  cookies?: CookieEditorCookie[];
  source?: { from?: string; cached?: boolean; madeAt?: string | null };
};

function sourceOf(value: SessionReply["source"]): SessionSource {
  const from = value?.from === "phone" || value?.from === "server" ? value.from : "unknown";
  return { from, cached: Boolean(value?.cached), madeAt: timeOf(value?.madeAt) };
}

function stageOf(value: unknown): SessionStage {
  return value === "phone" || value === "server" ? value : "starting";
}

function timeOf(value: unknown): number | undefined {
  const time = typeof value === "string" ? Date.parse(value) : NaN;
  return Number.isFinite(time) ? time : undefined;
}

/** What to tell the student while a request is pending. */
export function progressText(
  stage: SessionStage,
  serverDueAt: number | undefined,
  now: number = Date.now(),
) {
  if (stage === "server") {
    return "The server is signing in to VTOP. It may wait for the OTP email…";
  }
  if (stage === "phone") {
    const left = serverDueAt === undefined ? 0 : Math.ceil((serverDueAt - now) / 1000);
    return left > 0
      ? `Waiting for your phone… (the server takes over in ${left} s)`
      : "Waiting for your phone…";
  }
  return "Starting sign-in…";
}

/** Reads a reply from `POST /v1/session` or `GET /v1/session/requests/{id}`. */
export function interpretSessionResponse(httpStatus: number, body: unknown): SessionOutcome {
  const reply = (typeof body === "object" && body !== null ? body : {}) as SessionReply;
  if (reply.status === "ready") {
    return { kind: "ready", cookies: reply.cookies ?? [], source: sourceOf(reply.source) };
  }
  if (httpStatus === 202 || reply.status === "pending") {
    return {
      kind: "pending",
      requestId: reply.requestId,
      pollAfterMs: clampPollAfter(reply.pollAfterMs ?? 1500),
      stage: stageOf(reply.stage),
      serverDueAt: timeOf(reply.serverDueAt),
    };
  }
  return { kind: "error", message: friendlyLoginError(httpStatus, JSON.stringify(reply)) };
}

/** The server (vtop-edge) the extension uses unless another is set; from .env. */
export const DEFAULT_BRIDGE_URL = (
  import.meta.env.VITE_BRIDGE_URL ?? "https://vmate.aster0.dev"
).replace(/\/+$/, "");

/** The https origin of a bridge URL the user typed, or null. */
export function normalizeBridgeUrl(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  try {
    const url = new URL(withScheme);
    if (url.protocol !== "https:" || !url.hostname.includes(".")) return null;
    return url.origin;
  } catch {
    return null;
  }
}

// ---- pairing with the app (`POST /v1/pair`, see vtop-edge's pair.ts) ---------

/** Crockford base32: no I, L, O or U, so a code reads back as typed. */
const PAIR_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/** How long the edge holds a key made for a code. */
export const PAIR_TTL_MS = 300_000;

/** A fresh pairing code, 10 characters (50 bits), as `XXXXX-XXXXX`. */
export function newPairCode(
  random: (bytes: Uint8Array<ArrayBuffer>) => Uint8Array = (b) => crypto.getRandomValues(b),
) {
  const chars = [...random(new Uint8Array(10))].map((byte) => PAIR_ALPHABET[byte & 31]);
  return `${chars.slice(0, 5).join("")}-${chars.slice(5).join("")}`;
}

export type PairOutcome =
  | { kind: "ready"; key: string }
  | { kind: "pending"; pollAfterMs: number }
  | { kind: "error"; message: string };

/** Reads a reply from `POST /v1/pair`. */
export function interpretPairResponse(httpStatus: number, body: unknown): PairOutcome {
  const reply = (typeof body === "object" && body !== null ? body : {}) as {
    status?: string;
    key?: string;
    pollAfterMs?: number;
  };
  if (reply.status === "ready" && isAccessKey(reply.key ?? "")) return { kind: "ready", key: reply.key!.trim() };
  if (httpStatus === 202) return { kind: "pending", pollAfterMs: clampPollAfter(reply.pollAfterMs ?? 2000) };
  return { kind: "error", message: friendlyLoginError(httpStatus, JSON.stringify(reply)) };
}
