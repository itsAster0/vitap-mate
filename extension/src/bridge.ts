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

const INVALID_TOKEN_MESSAGE =
  "This Token is invalid or expired. Get a new Token from the VITAP Mate app and try again.";

export function validateCookies(cookies: CookieEditorCookie[]) {
  if (cookies.length === 0) throw new Error(INVALID_TOKEN_MESSAGE);

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
  if (code === "key_unknown" || (code === "" && status === 401)) return UNKNOWN_KEY_MESSAGE;
  if (code === "phone_unreachable") {
    return parsed.error?.replace(/^phone_unreachable:\s*/, "Your phone is not linked. ") ??
      "Your phone is not linked. Open VITAP Mate → Connected apps → Reconnect this phone.";
  }
  if (code === "phone_timeout" || code === "request_expired") return PHONE_TIMEOUT_MESSAGE;
  if (code === "rate_limited") return "Too many attempts. Wait a minute and try again.";
  if ((code === "phone_session_invalid" || code === "account_mismatch") && parsed.error) {
    return parsed.error.replace(/^[a-z_]+:\s*/, "");
  }
  const normalized = `${parsed.error ?? ""} ${responseText}`.toLowerCase();
  if (
    code === "TOKEN_INVALID" ||
    code === "TOKEN_REQUIRED" ||
    ((normalized.includes("token") || normalized.includes("registration")) &&
      ["invalid", "expired", "not found", "unregistered"].some((word) =>
        normalized.includes(word),
      ))
  ) {
    return INVALID_TOKEN_MESSAGE;
  }
  if (code === "REQUEST_EXPIRED" || status === 410) {
    return "The connection expired. Keep your mobile online and try again.";
  }
  if (code === "RATE_LIMITED" || status === 429) {
    return "Too many attempts. Wait a minute and try again.";
  }
  if (
    normalized.includes("vtop session is not authenticated") ||
    normalized.includes("no vtop account is configured")
  ) {
    return "Open VITAP Mate on your mobile, refresh your VTOP login, then try again.";
  }
  if (["STORE_UNAVAILABLE", "FCM_UNAVAILABLE", "FCM_MISCONFIGURED"].includes(code)) {
    return "The sign-in service is temporarily unavailable. Please try again shortly.";
  }
  if ([502, 503, 504].includes(status)) {
    return "The sign-in service is temporarily unavailable. Please try again shortly.";
  }
  return parsed.error || "Could not connect right now. Please try again.";
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
  "This access key is unknown or was revoked. Create one in VITAP Mate → Connected apps and paste it here.";
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

export type SessionOutcome =
  | { kind: "ready"; cookies: CookieEditorCookie[] }
  | { kind: "pending"; requestId: string | undefined; pollAfterMs: number }
  | { kind: "error"; message: string };

type SessionReply = {
  status?: string;
  code?: string;
  error?: string;
  requestId?: string;
  pollAfterMs?: number;
  cookies?: CookieEditorCookie[];
};

/** Reads a reply from `POST /v1/session` or `GET /v1/session/requests/{id}`. */
export function interpretSessionResponse(httpStatus: number, body: unknown): SessionOutcome {
  const reply = (typeof body === "object" && body !== null ? body : {}) as SessionReply;
  if (reply.status === "ready") return { kind: "ready", cookies: reply.cookies ?? [] };
  if (httpStatus === 202 || reply.status === "pending") {
    return {
      kind: "pending",
      requestId: reply.requestId,
      pollAfterMs: clampPollAfter(reply.pollAfterMs ?? 1500),
    };
  }
  return { kind: "error", message: friendlyLoginError(httpStatus, JSON.stringify(reply)) };
}

/** The bridge the extension uses unless another is set; from .env. */
export const DEFAULT_BRIDGE_URL = (
  import.meta.env.VITE_BRIDGE_URL ?? "https://vtop-bridge.aster0.dev"
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
