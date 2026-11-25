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

export type PendingStage =
  /** Request opened, neither phone nor server started yet. */
  | { kind: "starting"; phoneError: RequestError | null }
  /** Waiting on the phone; `vaultDueAt` is set when the server will take over. */
  | { kind: "phone"; vaultDueAt: number | null }
  /** Server is signing in with saved credentials; `phoneAsked` is true when the phone was also asked. */
  | { kind: "server"; phoneAsked: boolean; phoneError: RequestError | null };

export type RequestPhase =
  | { kind: "pending"; stage: PendingStage }
  | { kind: "expired" }
  | { kind: "success"; cookies: string; csrfToken: string | null; source: string }
  | { kind: "error"; error: RequestError };

/** Flat stored `sessionRequests` fields, kept compatible with existing rows without a migration. */
export type StoredRequestFields = {
  status: "pending" | "success" | "error";
  cookies: string | null;
  csrfToken: string | null;
  errorCode?: string | null;
  error: string | null;
  createdAt: number;
  expiresAt: number;
  vaultDueAt: number | null;
  vaultStarted: boolean;
  phoneAsked: boolean;
  phoneErrorCode?: string | null;
  phoneError: string | null;
  source: string | null;
  phones: string[];
  expired?: boolean;
};

/** One session request of an account. Its phase is one discriminated union so callers cannot
 * observe impossible combinations of stored fields. */
export class RequestRecord {
  phase: RequestPhase;
  createdAt: number;
  expiresAt: number;
  /** The installs this request may ask: the key's phone. */
  phones: string[];

  constructor(
    readonly id: string,
    now: number,
    expiresAt: number = now + REQUEST_TIMEOUT_SECS,
    phones: string[] = [],
    phase: RequestPhase = { kind: "pending", stage: { kind: "starting", phoneError: null } },
  ) {
    this.createdAt = now;
    this.expiresAt = expiresAt;
    this.phones = phones;
    this.phase = phase;
  }

  /** Effective phase at `now`: pending past `expiresAt` reads as `expired`. */
  phaseAt(now: number): RequestPhase {
    if (this.phase.kind === "pending" && now >= this.expiresAt) return { kind: "expired" };
    return this.phase;
  }

  pending(now: number): boolean {
    return this.phaseAt(now).kind === "pending";
  }

  /** Whether a pending request has asked its phone (including after server takeover). */
  phoneAsked(): boolean {
    if (this.phase.kind !== "pending") return false;
    const { stage } = this.phase;
    return stage.kind === "phone" || (stage.kind === "server" && stage.phoneAsked);
  }

  /** Whether `deviceId` should see this request in `sessions:pending` at `now`. */
  waitingOnPhone(now: number, deviceId: string): boolean {
    const phase = this.phaseAt(now);
    if (phase.kind !== "pending") return false;
    const asked = phase.stage.kind === "phone" || (phase.stage.kind === "server" && phase.stage.phoneAsked);
    return asked && this.phones.includes(deviceId);
  }

  /** Marks that at least one phone was asked for this request. */
  markPhoneAsked(): void {
    if (this.phase.kind !== "pending") return;
    switch (this.phase.stage.kind) {
      case "starting":
        this.phase = { kind: "pending", stage: { kind: "phone", vaultDueAt: null } };
        break;
      case "server":
        this.phase = { kind: "pending", stage: { kind: "server", phoneAsked: true, phoneError: null } };
        break;
      case "phone":
        break;
    }
  }

  /** Schedules server takeover at `dueAt` while waiting on the phone. */
  scheduleServer(dueAt: number): void {
    if (this.phase.kind === "pending" && this.phase.stage.kind === "phone") {
      this.phase = { kind: "pending", stage: { kind: "phone", vaultDueAt: dueAt } };
    }
  }

  /** Clears a scheduled server takeover when saved credentials are no longer usable. */
  clearVaultDue(): void {
    if (this.phase.kind === "pending" && this.phase.stage.kind === "phone") {
      this.phase = { kind: "pending", stage: { kind: "phone", vaultDueAt: null } };
    }
  }

  /** Starts the server sign-in if the request is still pending and not already on the server. */
  startServer(now: number): boolean {
    const phase = this.phaseAt(now);
    if (phase.kind !== "pending" || phase.stage.kind === "server") return false;
    this.phase = {
      kind: "pending",
      stage: {
        kind: "server",
        phoneAsked: phase.stage.kind === "phone",
        phoneError: phase.stage.kind === "starting" ? phase.stage.phoneError : null,
      },
    };
    return true;
  }

  /** Records why the phone could not be asked, kept to end the request if the server fails too. */
  setPhoneError(error: RequestError): void {
    if (this.phase.kind !== "pending" || this.phase.stage.kind === "phone") return;
    this.phase = { kind: "pending", stage: { ...this.phase.stage, phoneError: error } };
  }

  /** Stored phone-ask failure while pending without a phone. */
  phoneFailure(): RequestError | null {
    if (this.phase.kind !== "pending" || this.phase.stage.kind === "phone") return null;
    return this.phase.stage.phoneError;
  }

  /** Failure code and message when the request failed, else null. */
  failure(): RequestError | null {
    return this.phase.kind === "error" ? this.phase.error : null;
  }

  /** Marks a pending request expired at or past `expiresAt`. */
  expire(now: number): boolean {
    if (this.phase.kind !== "pending" || now < this.expiresAt) return false;
    this.phase = { kind: "expired" };
    return true;
  }

  /** Answers the request; `source` is who made the session. */
  succeed(cookies: string, source: string, csrfToken: string | null = null): void {
    this.phase = { kind: "success", cookies, csrfToken, source };
  }

  fail(error: RequestError): void {
    this.phase = { kind: "error", error };
  }

  /** Who is working on a pending request, for clients to say what they
   * wait for: the server signing in, the phone, or neither yet. */
  stage(): "server" | "phone" | "starting" {
    return this.phase.kind === "pending" ? this.phase.stage.kind : "starting";
  }

  /** The pending reply's progress fields. `serverDueAt` is when the server
   * takes over from a phone that has not answered. */
  private progress(stage: PendingStage) {
    const due = stage.kind === "phone" && stage.vaultDueAt !== null ? rfc3339(stage.vaultDueAt) : null;
    return { status: "pending", stage: stage.kind, serverDueAt: due };
  }

  /** The pending reply of `POST /v1/session`. */
  startedView() {
    const progress =
      this.phase.kind === "pending"
        ? this.progress(this.phase.stage)
        : { status: "pending", stage: "starting" as const, serverDueAt: null };
    return {
      ...progress,
      requestId: this.id,
      pollAfterMs: POLL_AFTER_MS,
      expiresAt: rfc3339(this.expiresAt),
    };
  }

  /** The reply of a poll, and its status code. */
  pollView(now: number): { status: number; body: unknown } {
    const phase = this.phaseAt(now);
    switch (phase.kind) {
      case "expired":
        return { status: 410, body: errorBody("request_expired", "The session request expired.") };
      case "pending":
        return { status: 202, body: this.progress(phase.stage) };
      case "success":
        return { status: 200, body: ready(phase.cookies, sourceView(phase.source, null, false)) };
      case "error":
        // A poll that worked, about a request that failed: 200, with the
        // error fields as every error reply has them.
        return { status: 200, body: { status: "error", ...errorBody(phase.error.code, phase.error.message) } };
    }
  }
}

function parseStoredError(code: string | null | undefined, message: string | null): RequestError {
  if (code != null) return { code, message: message ?? "" };
  return parseRequestError(message ?? "");
}

function parseStoredPhoneError(code: string | null | undefined, message: string | null): RequestError | null {
  if (message === null) return null;
  if (code != null) return { code, message };
  return parseRequestError(message);
}

/** Parses a stored `sessionRequests` row into a `RequestRecord` with one discriminated `phase`. */
export function fromRequestFields(id: string, f: StoredRequestFields): RequestRecord {
  let phase: RequestPhase;
  switch (f.status) {
    case "success":
      phase = {
        kind: "success",
        cookies: f.cookies ?? "",
        csrfToken: f.csrfToken,
        source: f.source ?? "unknown",
      };
      break;
    case "error":
      phase = { kind: "error", error: parseStoredError(f.errorCode, f.error) };
      break;
    case "pending":
      if (f.expired) {
        phase = { kind: "expired" };
      } else if (f.vaultStarted) {
        phase = {
          kind: "pending",
          stage: {
            kind: "server",
            phoneAsked: f.phoneAsked,
            phoneError: parseStoredPhoneError(f.phoneErrorCode, f.phoneError),
          },
        };
      } else if (f.phoneAsked) {
        phase = {
          kind: "pending",
          stage: { kind: "phone", vaultDueAt: f.vaultDueAt },
        };
      } else {
        phase = {
          kind: "pending",
          stage: {
            kind: "starting",
            phoneError: parseStoredPhoneError(f.phoneErrorCode, f.phoneError),
          },
        };
      }
      break;
  }
  return new RequestRecord(id, f.createdAt, f.expiresAt, f.phones, phase);
}

/** Projects `RequestRecord` back onto the flat `sessionRequests` storage columns. */
export function toRequestFields(r: RequestRecord): StoredRequestFields {
  const base = {
    createdAt: r.createdAt,
    expiresAt: r.expiresAt,
    phones: r.phones,
  };
  switch (r.phase.kind) {
    case "pending": {
      const { stage } = r.phase;
      return {
        ...base,
        status: "pending",
        cookies: null,
        csrfToken: null,
        errorCode: null,
        error: null,
        vaultDueAt: stage.kind === "phone" ? stage.vaultDueAt : null,
        vaultStarted: stage.kind === "server",
        phoneAsked: stage.kind === "phone" || (stage.kind === "server" && stage.phoneAsked),
        phoneErrorCode: stage.kind === "phone" ? null : (stage.phoneError?.code ?? null),
        phoneError: stage.kind === "phone" ? null : (stage.phoneError?.message ?? null),
        source: null,
        expired: undefined,
      };
    }
    case "expired":
      return {
        ...base,
        status: "pending",
        cookies: null,
        csrfToken: null,
        errorCode: null,
        error: null,
        vaultDueAt: null,
        vaultStarted: false,
        phoneAsked: false,
        phoneErrorCode: null,
        phoneError: null,
        source: null,
        expired: true,
      };
    case "success":
      return {
        ...base,
        status: "success",
        cookies: r.phase.cookies,
        csrfToken: r.phase.csrfToken,
        errorCode: null,
        error: null,
        vaultDueAt: null,
        vaultStarted: false,
        phoneAsked: false,
        phoneErrorCode: null,
        phoneError: null,
        source: r.phase.source,
        expired: undefined,
      };
    case "error":
      return {
        ...base,
        status: "error",
        cookies: null,
        csrfToken: null,
        errorCode: r.phase.error.code,
        error: r.phase.error.message,
        vaultDueAt: null,
        vaultStarted: false,
        phoneAsked: false,
        phoneErrorCode: null,
        phoneError: null,
        source: null,
        expired: undefined,
      };
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
