// Pure account state and decision rules: cached session and saved credential
// lifetimes, phone-vs-server sign-in plans, and session request views. The
// `accounts` row holds an `AccountRecord` field for field (`toFields` /
// `fromFields`), and phones, keys and sign-ins live in their own tables. No
// Convex APIs here, so it is tested on its own.

import { sha256HexSync } from "./sha256";

/** Personal access key for clients (extension, MCP). */
export const KEY_PREFIX = "vtm_";

function errorBody(code: string, message: string): { code: string; error: string } {
  return { code, error: message };
}

/** How long a cached session is handed out before a new one is fetched. */
export const SESSION_CACHE_SECS = 7 * 24 * 3600;
/** How long a client may wait on one session request. */
export const REQUEST_TIMEOUT_SECS = 120;
/** How long a finished request is kept for late polls and answers. */
export const REQUEST_KEEP_SECS = 600;
export const POLL_AFTER_MS = 1500;
/** Session requests one account may make per minute. */
/** Phones one account may link. Linking one more drops the one seen least
 * recently. */
export const MAX_PHONES = 5;
export const MAX_KEYS_PER_ACCOUNT = 10;
/** A key's `lastUsedAt` is written at most this often. */
export const TOUCH_EVERY_SECS = 60;
export const MAX_LABEL_CHARS = 60;

export type RequestError = { code: string; message: string };

export const PHONE_UNREACHABLE: RequestError = {
  code: "phone_unreachable",
  message: "Open VITAP Mate → Connected apps → Reconnect this phone.",
};

// ---- settings, keys and ids ---------------------------------------------------------

/** How sessions of an account are served; the student picks these in the
 * app. */
export interface AccountSettings {
  /** Sign in with saved credentials as soon as a session is needed,
   * without asking the phone first. */
  alwaysUseVault: boolean;
  /** When not `alwaysUseVault`: how long the phone gets to answer before
   * the server signs in with saved credentials. */
  phoneWaitSecs: number;
  /** How long saved credentials are kept after they were last sent; null
   * keeps them until VTOP rejects them. */
  vaultTtlSecs: number | null;
}

export const PHONE_WAIT_CHOICES = [10, 20, 45];
export const VAULT_TTL_CHOICES = [86_400, 172_800, null];

export function defaultSettings(): AccountSettings {
  return { alwaysUseVault: true, phoneWaitSecs: 20, vaultTtlSecs: null };
}

/** Settings as the app sends them, if they use the offered choices. */
export function parseSettings(raw: unknown): AccountSettings | null {
  if (raw === null || typeof raw !== "object") return null;
  const { alwaysUseVault, phoneWaitSecs, vaultTtlSecs = null } = raw as Record<string, unknown>;
  if (typeof alwaysUseVault !== "boolean" || typeof phoneWaitSecs !== "number") return null;
  if (!PHONE_WAIT_CHOICES.includes(phoneWaitSecs)) return null;
  if (!VAULT_TTL_CHOICES.includes(vaultTtlSecs as number | null)) return null;
  return { alwaysUseVault, phoneWaitSecs, vaultTtlSecs: vaultTtlSecs as number | null };
}

/** What an account id looks like: VTOP registration numbers are short and
 * alphanumeric. */
export function isAccountId(id: string): boolean {
  return /^[0-9A-Z]{1,20}$/.test(id);
}

/** The short id users see and revoke a key by. */
export function keyId(keyHash: string): string {
  return keyHash.slice(0, 8);
}

export function keyLabel(raw: string | undefined): string {
  const label = raw?.trim() || "Access key";
  return [...label].slice(0, MAX_LABEL_CHARS).join("");
}

/** A VTOP semester id as the app sends it: plain alphanumerics. */
export function semesterId(raw: string): string | null {
  const id = raw.trim();
  return /^[0-9A-Za-z]{1,40}$/.test(id) ? id : null;
}

const PACKED_ERROR = /^([a-z][a-z0-9_]*):(?:\s+(.*)|$)/s;

/** The code before the colon of a legacy request error (`code: message`). */
export function errorCode(error: string): string {
  return parseRequestError(error).code;
}

/** Splits a `"code: message"` string (from an external caller or legacy row) into `{ code, message }`.
 * Only accepts lowercase `snake_case` codes; anything else falls back to `login_failed`. */
export function parseRequestError(raw: string): RequestError {
  const match = PACKED_ERROR.exec(raw.trim());
  if (!match) return { code: "login_failed", message: raw };
  const [, code, rest] = match;
  const message = rest?.trim() ?? "";
  return { code, message: message || raw };
}

// ---- cookies --------------------------------------------------------------------------

export const VTOP_DOMAIN = "vtop.vitap.ac.in";

/** A cookie as the Cookie-Editor extension installs it. */
export interface CookieEditorCookie {
  domain: string;
  hostOnly: boolean;
  httpOnly: boolean;
  name: string;
  path: string;
  sameSite: string;
  secure: boolean;
  session: boolean;
  storeId?: string;
  value: string;
  expirationDate?: number;
  id?: number;
}

/** The cookies of a phone's answer, if every one has a name, a value and a
 * domain. */
export function validCookies(raw: unknown): Array<{ name: string; value: string }> | null {
  if (!Array.isArray(raw) || raw.length === 0) return null;
  const cookies = [];
  for (const cookie of raw) {
    const { name, value, domain } = (cookie ?? {}) as Record<string, unknown>;
    if (typeof name !== "string" || !name.trim()) return null;
    if (typeof value !== "string" || !value) return null;
    if (typeof domain !== "string" || !domain.trim()) return null;
    cookies.push({ name, value });
  }
  return cookies;
}

/** `name=value; name=value`. */
export function toHeader(cookies: Array<{ name: string; value: string }>): string {
  return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
}

/** The same cookies the app builds in `cookieEditorCookiesFromHeader`. */
export function fromHeader(header: string): CookieEditorCookie[] {
  return header.split(";").flatMap((part, index) => {
    const pair = part.trim();
    const eq = pair.indexOf("=");
    if (eq < 0) return [];
    const name = pair.slice(0, eq).trim();
    const value = pair.slice(eq + 1).trim();
    if (!name || !value) return [];
    return [
      {
        domain: VTOP_DOMAIN,
        hostOnly: true,
        httpOnly: false,
        name,
        path: "/",
        sameSite: "unspecified",
        secure: true,
        session: true,
        storeId: "0",
        value,
        id: index + 1,
      },
    ];
  });
}

// ---- the account --------------------------------------------------------------------
// Phones are the `devices` rows linked to the account, keys its `accessKeys` rows and
// sign-ins its `signIns` rows; the rules below take them as lists.

export type KeyVia = "extension" | "mcp";
export type KeyUse = { at: number; via: KeyVia };
export const MAX_KEY_USES = 10;

/** An access key as the account sees it. */
export type KeyRecord = {
  hash: string;
  label: string;
  createdAt: number;
  lastUsedAt: number | null;
  /** The install of the phone that created it: the only phone its session requests go to. */
  deviceId: string;
  /** Whether agents may apply for and cancel outings with it. Off until the student turns it
   * on in the app. */
  outings: boolean;
  /** Its latest uses, newest first, at most `MAX_KEY_USES`. */
  uses: KeyUse[];
};

export type SignInBy = "server" | "phone";
/** A VTOP sign-in for the account: by the server (saved password) or by a phone (its answer
 * brought a session). */
export type SignInRecord = { at: number; by: SignInBy; ok: boolean; reason: string | null };
export const MAX_SIGN_INS = 10;

/** One linked phone, by its device id. */
export type PhoneRecord = { deviceId: string; label: string | null; addedAt: number; seenAt: number };

/** The short id a phone is shown and removed by; the device id itself stays private. */
export function phoneId(deviceId: string): string {
  return keyId(sha256HexSync(deviceId));
}

/** The entry seen least recently, which makes room when one too many links or joins. */
export function leastRecent(entries: ReadonlyArray<{ addedAt: number; seenAt: number }>): number {
  let oldest = 0;
  entries.forEach((p, i) => {
    const old = entries[oldest];
    if (Math.max(p.seenAt, p.addedAt) < Math.max(old.seenAt, old.addedAt)) oldest = i;
  });
  return oldest;
}

/** A key's use by [via]: its new `lastUsedAt` and `uses` when due to be written (at most once a
 * `TOUCH_EVERY_SECS`), else null. */
export function keyUse(key: KeyRecord, now: number, via: KeyVia): { lastUsedAt: number; uses: KeyUse[] } | null {
  if (key.lastUsedAt !== null && now < key.lastUsedAt + TOUCH_EVERY_SECS) return null;
  return { lastUsedAt: now, uses: [{ at: now, via }, ...key.uses].slice(0, MAX_KEY_USES) };
}

/** The installs a key's session requests go to: the phone that created it, none once that phone
 * is unlinked. */
export function phonesForKey(key: KeyRecord | undefined, phones: PhoneRecord[]): string[] {
  return phones.map((p) => p.deviceId).filter((device) => device === key?.deviceId);
}

/** Which keys move to `to` when phone `from` (a short id, or null) hands over: those of `from`
 * and of phones no longer linked. Null when `from` is no other phone of the account. */
export function keysToMove(keys: KeyRecord[], phones: PhoneRecord[], from: string | null, to: string): { source: PhoneRecord | null; moving: KeyRecord[] } | null {
  const source = from === null ? null : (phones.find((p) => phoneId(p.deviceId) === from.trim()) ?? null);
  if (from !== null && (!source || source.deviceId === to)) return null;
  const moving = keys.filter((key) => key.deviceId === source?.deviceId || !phones.some((p) => p.deviceId === key.deviceId));
  return { source, moving };
}

/** One student's account: settings, the cached session and saved credentials. Secrets are
 * sealed by the engine with `VAULT_KEY`: storage alone does not open them, but whoever controls
 * the engine can. */
export class AccountRecord {
  settings: AccountSettings = defaultSettings();
  semesterId: string | null = null;
  /** The student's forwarding address, read from `mailboxes` (not stored here). */
  mailboxId: string | null = null;
  sealedVault: string | null = null;
  vaultSavedAt: number | null = null;
  /** The saved credentials carry Gmail access, so the server can read the OTP without a
   * forwarding address. */
  vaultGmail = false;
  sealedSession: string | null = null;
  sessionSavedAt: number | null = null;
  /** Who made the cached session: `phone` or `server`. */
  sessionSource: string | null = null;
  /** When VTOP last rejected the saved password (on the server or on the phone). Saved
   * credentials are dropped then and not used again until the phone sends new ones. */
  credentialsRejectedAt: number | null = null;
  createdAt: number;
  updatedAt: number;

  constructor(
    readonly id: string,
    now: number,
  ) {
    this.createdAt = now;
    this.updatedAt = now;
  }

  /** What `app:status` shows a linked phone: what the server keeps and its recent activity.
   * [forwarding] is the student's address. */
  statusView(forwarding: string | null, phones: number, keys: KeyRecord[], signIns: SignInRecord[]) {
    const ttl = this.settings.vaultTtlSecs;
    const vault = this.sealedVault !== null;
    return {
      sessionSavedAt: this.sealedSession !== null ? this.sessionSavedAt : null,
      sessionSource: this.sealedSession !== null ? this.sessionSource : null,
      vaultSavedAt: vault ? this.vaultSavedAt : null,
      vaultExpiresAt: vault && ttl !== null && this.vaultSavedAt !== null ? this.vaultSavedAt + ttl : null,
      vaultGmail: vault && this.vaultGmail,
      credentialsRejectedAt: this.credentialsRejectedAt,
      forwarding,
      signIns: signIns.map((s) => ({ ...s })),
      phones,
      keys: keys.map((key) => ({ id: keyId(key.hash), label: key.label, lastUsedAt: key.lastUsedAt, uses: key.uses.map((u) => ({ ...u })) })),
    };
  }

  /** Saved credentials that may still be used: not past the student's chosen lifetime. */
  usableVault(now: number): string | null {
    const ttl = this.settings.vaultTtlSecs;
    if (ttl !== null && this.vaultSavedAt !== null && now >= this.vaultSavedAt + ttl) return null;
    return this.sealedVault;
  }

  cachedSession(now: number): string | null {
    return now < (this.sessionSavedAt ?? 0) + SESSION_CACHE_SECS ? this.sealedSession : null;
  }

  /** Caches a session; `source` is who made it (`phone` or `server`). */
  saveSession(sealed: string, now: number, source: string): void {
    this.sealedSession = sealed;
    this.sessionSavedAt = now;
    this.sessionSource = source;
  }

  clearSession(): void {
    this.sealedSession = null;
    this.sessionSavedAt = null;
    this.sessionSource = null;
  }

  /** Where the cached session came from, for clients to show. */
  cachedSource() {
    return sourceView(this.sessionSource, this.sessionSavedAt, true);
  }

  /** Keeps sealed credentials; `gmail` when they carry Gmail access. */
  saveVault(sealed: string, now: number, gmail = false): void {
    this.sealedVault = sealed;
    this.vaultSavedAt = now;
    this.vaultGmail = gmail;
    this.credentialsRejectedAt = null;
  }

  /** Google refused the saved Gmail access: the same credentials without it take their place,
   * keeping their age. */
  dropGmail(sealed: string): void {
    if (this.sealedVault === null) return;
    this.sealedVault = sealed;
    this.vaultGmail = false;
  }

  clearVault(): void {
    this.sealedVault = null;
    this.vaultSavedAt = null;
    this.vaultGmail = false;
  }

  rejectCredentials(now: number): void {
    this.clearVault();
    this.credentialsRejectedAt = now;
  }

  /** The phone should send credentials with its answer: none are saved, saved ones expire (each
   * answer restarts their clock), or the server has no way to read the OTP with them (Gmail
   * access may come along). */
  wantCredentials(now: number): boolean {
    return this.usableVault(now) === null || this.settings.vaultTtlSecs !== null || (!this.vaultGmail && this.mailboxId === null);
  }

  /** What `account:view` shows. Keys and the phone list only to a linked phone (`me`). */
  view(phones: PhoneRecord[], keys: KeyRecord[], me: string | null, forwarding: boolean, now: number) {
    const mine = me !== null && phones.some((p) => p.deviceId === me);
    const vault = this.usableVault(now);
    return {
      registrationNumber: this.id,
      createdAt: this.createdAt,
      settings: this.settings,
      semesterId: this.semesterId,
      linked: phones.length > 0,
      savedCredentials: vault !== null,
      vaultSavedAt: vault !== null ? this.vaultSavedAt : null,
      vaultGmail: vault !== null && this.vaultGmail,
      credentialsRejectedAt: this.credentialsRejectedAt,
      forwarding,
      phoneLinked: phones.length > 0,
      phoneCount: phones.length,
      thisPhone: mine,
      phones: mine
        ? phones.map((phone) => ({
            id: phoneId(phone.deviceId),
            label: phone.label,
            addedAt: phone.addedAt,
            seenAt: Math.max(phone.seenAt, phone.addedAt),
            thisPhone: phone.deviceId === me,
          }))
        : [],
      keys: mine
        ? keys.map((key) => ({
            id: keyId(key.hash),
            label: key.label,
            createdAt: key.createdAt,
            lastUsedAt: key.lastUsedAt,
            phoneId: phoneId(key.deviceId),
            outings: key.outings,
            uses: key.uses.map((u) => ({ ...u })),
          }))
        : [],
    };
  }
}

// ---- session requests ---------------------------------------------------------------

/** Who is asked for a session, in what order. `vault_first`: sign in with
 * saved credentials, ask the phone if that fails. `phone_first`: ask the
 * phone, sign in if it is silent for `waitSecs`. `phone_only`: only the
 * phone can answer. */
export type Plan = { kind: "vault_first" } | { kind: "phone_first"; waitSecs: number } | { kind: "phone_only" };

export function plan(settings: AccountSettings, hasVault: boolean): Plan {
  if (!hasVault) return { kind: "phone_only" };
  return settings.alwaysUseVault ? { kind: "vault_first" } : { kind: "phone_first", waitSecs: settings.phoneWaitSecs };
}

export type RequestStatus = "pending" | "success" | "error";

/** One session request of an account. */
export class RequestRecord {
  status: RequestStatus = "pending";
  /** The `Cookie` header, once answered. */
  cookies: string | null = null;
  /** Its CSRF token, when the server signed in (the app needs it). */
  csrfToken: string | null = null;
  errorCode: string | null = null;
  error: string | null = null;
  createdAt: number;
  expiresAt: number;
  /** When a sign-in with saved credentials is due (phone first). */
  vaultDueAt: number | null = null;
  vaultStarted = false;
  phoneAsked = false;
  /** Why the phone could not be asked, kept to end the request with if the
   * sign-in fails too. */
  phoneErrorCode: string | null = null;
  phoneError: string | null = null;
  /** Who made the session, once answered: `phone` or `server`. */
  source: string | null = null;
  /** The installs this request may ask: the key's phone. */
  phones: string[] = [];

  constructor(
    readonly id: string,
    now: number,
  ) {
    this.createdAt = now;
    this.expiresAt = now + REQUEST_TIMEOUT_SECS;
  }

  pending(now: number): boolean {
    return this.status === "pending" && now < this.expiresAt;
  }

  /** Answers the request; `source` is who made the session. */
  succeed(cookies: string, source: string, csrfToken: string | null = null): void {
    this.status = "success";
    this.cookies = cookies;
    this.csrfToken = csrfToken;
    this.errorCode = null;
    this.error = null;
    this.source = source;
  }

  fail(error: RequestError): void {
    this.status = "error";
    this.errorCode = error.code;
    this.error = error.message;
  }

  setPhoneError(error: RequestError): void {
    this.phoneErrorCode = error.code;
    this.phoneError = error.message;
  }

  /** Failure code and message, reading both structured rows and legacy `"code: message"` rows. */
  failure(): RequestError {
    const raw = this.error ?? "";
    if (this.errorCode !== null) return { code: this.errorCode, message: raw };
    return { code: errorCode(raw), message: raw };
  }

  /** Stored phone-ask failure, reading both structured and legacy rows. */
  phoneFailure(): RequestError | null {
    if (this.phoneError === null) return null;
    if (this.phoneErrorCode !== null) return { code: this.phoneErrorCode, message: this.phoneError };
    return parseRequestError(this.phoneError);
  }

  /** Who is working on a pending request, for clients to say what they
   * wait for: the server signing in, the phone, or neither yet. */
  stage(): "server" | "phone" | "starting" {
    if (this.vaultStarted) return "server";
    return this.phoneAsked ? "phone" : "starting";
  }

  /** The pending reply's progress fields. `serverDueAt` is when the server
   * takes over from a phone that has not answered. */
  private progress() {
    const due = this.vaultDueAt !== null && !this.vaultStarted ? rfc3339(this.vaultDueAt) : null;
    return { status: "pending", stage: this.stage(), serverDueAt: due };
  }

  /** The pending reply of `POST /v1/session`. */
  startedView() {
    return {
      ...this.progress(),
      requestId: this.id,
      pollAfterMs: POLL_AFTER_MS,
      expiresAt: rfc3339(this.expiresAt),
    };
  }

  /** The reply of a poll, and its status code. */
  pollView(now: number): { status: number; body: unknown } {
    if (now >= this.expiresAt && this.status === "pending") {
      return { status: 410, body: errorBody("request_expired", "The session request expired.") };
    }
    switch (this.status) {
      case "pending":
        return { status: 202, body: this.progress() };
      case "success":
        return { status: 200, body: ready(this.cookies ?? "", sourceView(this.source, null, false)) };
      case "error": {
        const { code, message } = this.failure();
        // A poll that worked, about a request that failed: 200, with the
        // error fields as every error reply has them.
        return { status: 200, body: { status: "error", ...errorBody(code, message) } };
      }
    }
  }
}

/** A ready session as clients get it: VTOP's `SessionState` shape, the
 * cookies in Cookie-Editor form, and where the session came from. */
export function ready(cookieHeader: string, source: ReturnType<typeof sourceView>) {
  return {
    status: "ready",
    session: { cookies: cookieHeader },
    cookies: fromHeader(cookieHeader),
    source,
  };
}

/** `from`: who signed in to VTOP (`phone`, `server`, or `unknown`).
 * `cached`: handed out again from the cache. `madeAt`: when it was made, if
 * known. */
export function sourceView(from: string | null, madeAt: number | null, cached: boolean) {
  return { from: from ?? "unknown", cached, madeAt: madeAt === null ? null : rfc3339(madeAt) };
}

/** Unix seconds as RFC 3339 UTC, as the bridge writes `expiresAt`. */
export function rfc3339(secs: number): string {
  return new Date(secs * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");
}

// ---- the `accounts` row -------------------------------------------------------------

/** What the `accounts` row stores of a record, beside `regno` and `ownerHash`. */
export type AccountFields = {
  settings: AccountSettings;
  semesterId: string | null;
  sealedVault: string | null;
  vaultSavedAt: number | null;
  vaultGmail: boolean;
  sealedSession: string | null;
  sessionSavedAt: number | null;
  sessionSource: string | null;
  credentialsRejectedAt: number | null;
  sweepAt?: number;
  createdAt: number;
  updatedAt: number;
};

/** When the sweep must next look at the row: the earlier of the cached session's and the saved
 * credentials' expiry; undefined (unset) when neither is kept. */
export function sweepAtOf(f: Omit<AccountFields, "sweepAt">): number | undefined {
  const due: number[] = [];
  if (f.sealedSession !== null) due.push((f.sessionSavedAt ?? 0) + SESSION_CACHE_SECS);
  const ttl = f.settings.vaultTtlSecs;
  if (f.sealedVault !== null && ttl !== null && f.vaultSavedAt !== null) due.push(f.vaultSavedAt + ttl);
  return due.length > 0 ? Math.min(...due) : undefined;
}

export function toFields(r: AccountRecord): AccountFields {
  const fields = {
    settings: r.settings,
    semesterId: r.semesterId,
    sealedVault: r.sealedVault,
    vaultSavedAt: r.vaultSavedAt,
    vaultGmail: r.vaultGmail,
    sealedSession: r.sealedSession,
    sessionSavedAt: r.sessionSavedAt,
    sessionSource: r.sessionSource,
    credentialsRejectedAt: r.credentialsRejectedAt,
    createdAt: r.createdAt,
    updatedAt: r.updatedAt,
  };
  return { ...fields, sweepAt: sweepAtOf(fields) };
}

/** The record of a stored row; `mailboxId` is the student's forwarding address, kept elsewhere. */
export function fromFields(id: string, f: AccountFields, mailboxId: string | null): AccountRecord {
  const r = Object.assign(new AccountRecord(id, f.createdAt), f);
  r.mailboxId = mailboxId;
  return r;
}
