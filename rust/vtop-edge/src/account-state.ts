// What an `Account` object keeps, and the decisions it makes: when a
// cached session or saved credentials may be used, who is asked for a
// session first, how a request looks to its client. No Worker APIs here,
// so it is tested on its own.

import { APP_SECRET_PREFIX, KEY_PREFIX, hash, hashesEqual, randomToken } from "./shared/tokens";

/** How long a cached session is handed out before a new one is fetched. */
export const SESSION_CACHE_SECS = 7 * 24 * 3600;
/** How long a client may wait on one session request. */
export const REQUEST_TIMEOUT_SECS = 120;
/** How long a finished request is kept for late polls and answers. */
export const REQUEST_KEEP_SECS = 600;
export const POLL_AFTER_MS = 1500;
/** Session requests one account may make per minute. */
export const ACCOUNT_RATE_PER_MINUTE = 60;
/** Phones one account may link. Linking one more drops the one seen least
 * recently. */
export const MAX_PHONES = 5;
/** A phone's `seenAt` is written at most this often. */
const PHONE_SEEN_EVERY_SECS = 3600;
export const MAX_KEYS_PER_ACCOUNT = 10;
/** A key's `lastUsedAt` is written at most this often. */
export const TOUCH_EVERY_SECS = 60;
export const MAX_LABEL_CHARS = 60;

export const PHONE_UNREACHABLE = "phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone.";

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

export function newKey(): string {
  return randomToken(KEY_PREFIX);
}

export function newAppSecret(): string {
  return randomToken(APP_SECRET_PREFIX);
}

/** The secret a linking phone gets. One already linked (`me`, found by the
 * secret it presented) keeps it: a new one would turn away its requests
 * still in flight, and the phone would read that as being unlinked. */
export function linkSecret(presented: string | undefined, me: number | null): string {
  const mine = presented?.trim();
  return me !== null && mine ? mine : newAppSecret();
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

/** The code before the colon of a request error (`code: message`). */
export function errorCode(error: string): string {
  const colon = error.indexOf(":");
  return (colon < 0 ? "login_failed" : error.slice(0, colon)).trim();
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

export interface KeyRecord {
  hash: string;
  label: string;
  createdAt: number;
  lastUsedAt: number | null;
  /** The install of the phone that created it: the only phone its session
   * requests go to. */
  deviceId: string;
  /** Its latest uses, newest first, at most `MAX_KEY_USES`. */
  uses: KeyUse[];
}

/** Which app used a key: the browser extension or an AI agent (MCP). */
export type KeyVia = "extension" | "mcp";
export interface KeyUse {
  at: number;
  via: KeyVia;
}
export const MAX_KEY_USES = 10;

export type SignInBy = "server" | "phone";
/** A VTOP sign-in for the account: by the server (saved password) or by a
 * phone (its answer brought a session). */
export interface SignInRecord {
  at: number;
  by: SignInBy;
  ok: boolean;
  reason: string | null;
}
export const MAX_SIGN_INS = 10;

/** One linked app install: its own app secret, and the install session
 * requests are pushed to. */
export interface PhoneRecord {
  deviceId: string;
  secretHash: string;
  /** What the app calls itself ("OnePlus CPH2573"), for the phone list. */
  label: string | null;
  addedAt: number;
  seenAt: number;
}

/** The short id a phone is shown and removed by; the device id itself
 * stays private. */
export function phoneId(deviceId: string): string {
  return keyId(hash(deviceId));
}

/** One student's account. Secrets are sealed by the engine with
 * `VAULT_KEY`, the engine's secret: storage alone does not open them, but
 * whoever controls the engine can. */
export class AccountRecord {
  settings: AccountSettings = defaultSettings();
  semesterId: string | null = null;
  /** The linked phones; each is asked for sessions. */
  phones: PhoneRecord[] = [];
  /** The student's forwarding address, so the dashboard leads from the
   * registration number to it (its Index is named by a keyed hash). */
  mailboxId: string | null = null;
  sealedVault: string | null = null;
  vaultSavedAt: number | null = null;
  /** The saved credentials carry Gmail access, so the server can read the
   * OTP without a forwarding address. */
  vaultGmail = false;
  sealedSession: string | null = null;
  sessionSavedAt: number | null = null;
  /** Who made the cached session: `phone` or `server`. */
  sessionSource: string | null = null;
  /** When VTOP last rejected the saved password (on the server or on the
   * phone). Saved credentials are dropped then and not used again until
   * the phone sends new ones. */
  credentialsRejectedAt: number | null = null;
  keys: KeyRecord[] = [];
  /** The latest sign-ins, newest first, at most `MAX_SIGN_INS`. */
  signIns: SignInRecord[] = [];
  createdAt: number;
  updatedAt: number;
  /** Rate limit window: its start and the hits in it. */
  windowStart = 0;
  windowHits = 0;
  /** The record as read from storage, so a save writes only what this
   * request changed and leaves changes other requests made meanwhile. */
  loaded: AccountRecord | null = null;

  constructor(
    readonly id: string,
    now: number,
  ) {
    this.createdAt = now;
    this.updatedAt = now;
  }

  /** A deep copy, without `loaded`. */
  clone(): AccountRecord {
    const copy = Object.assign(new AccountRecord(this.id, this.createdAt), this);
    copy.settings = { ...this.settings };
    copy.phones = this.phones.map((p) => ({ ...p }));
    copy.keys = this.keys.map((k) => ({ ...k, uses: k.uses.map((u) => ({ ...u })) }));
    copy.signIns = this.signIns.map((s) => ({ ...s }));
    copy.loaded = null;
    return copy;
  }

  /** What `/v1/app/status` shows a linked phone: what the server keeps
   * and its recent activity. [forwarding] is the student's address. */
  statusView(forwarding: string | null) {
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
      signIns: this.signIns.map((s) => ({ ...s })),
      phones: this.phones.length,
      keys: this.keys.map((key) => ({
        id: keyId(key.hash),
        label: key.label,
        lastUsedAt: key.lastUsedAt,
        uses: key.uses.map((u) => ({ ...u })),
      })),
    };
  }

  /** Remembers the current state as what storage holds. */
  markLoaded(): void {
    this.loaded = this.clone();
  }

  /** Saved credentials that may still be used: not past the student's
   * chosen lifetime. */
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

  /** Google refused the saved Gmail access: the same credentials without
   * it take their place, keeping their age. */
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

  /** The phone should send credentials with its answer: none are saved,
   * saved ones expire (each answer restarts their clock), or the server
   * has no way to read the OTP with them (Gmail access may come along). */
  wantCredentials(now: number): boolean {
    return (
      this.usableVault(now) === null ||
      this.settings.vaultTtlSecs !== null ||
      (!this.vaultGmail && this.mailboxId === null)
    );
  }

  linked(): boolean {
    return this.phones.length > 0;
  }

  /** The index of the linked phone holding this app secret. */
  phoneBySecret(presented: string | undefined): number | null {
    const secret = presented?.trim();
    if (!secret) return null;
    const presentedHash = hash(secret);
    const index = this.phones.findIndex((p) => hashesEqual(p.secretHash, presentedHash));
    return index < 0 ? null : index;
  }

  /** Adds `phone`, replacing an earlier entry for the same install; the
   * least recently seen go while there are too many. The new one stays. */
  addPhone(phone: PhoneRecord): void {
    this.phones = this.phones.filter((p) => p.deviceId !== phone.deviceId);
    while (this.phones.length >= MAX_PHONES) {
      let oldest = 0;
      this.phones.forEach((p, i) => {
        const old = this.phones[oldest];
        if (Math.max(p.seenAt, p.addedAt) < Math.max(old.seenAt, old.addedAt)) oldest = i;
      });
      this.phones.splice(oldest, 1);
    }
    this.phones.push(phone);
  }

  /** Unlinks the phone with this short id. */
  removePhone(id: string): PhoneRecord | null {
    const wanted = id.trim();
    if (!wanted) return null;
    const at = this.phones.findIndex((p) => phoneId(p.deviceId) === wanted);
    return at < 0 ? null : this.phones.splice(at, 1)[0];
  }

  /** The installs a key's session requests go to: the phone that created
   * it, none once that phone is unlinked. */
  phonesForKey(keyHash: string): string[] {
    const owner = this.keys.find((key) => key.hash === keyHash)?.deviceId;
    return this.phones.map((p) => p.deviceId).filter((device) => device === owner);
  }

  /** A phone moved to another install: its keys follow it. */
  phoneMoved(from: string, to: string): void {
    for (const key of this.keys) {
      if (key.deviceId === from) key.deviceId = to;
    }
  }

  /** Records a visit of a phone; true when that changed what is stored. */
  touchPhone(index: number, now: number): boolean {
    const phone = this.phones[index];
    if (now - phone.seenAt < PHONE_SEEN_EVERY_SECS) return false;
    phone.seenAt = now;
    return true;
  }

  hasKey(keyHash: string): boolean {
    return this.keys.some((key) => hashesEqual(key.hash, keyHash));
  }

  /** Records a use of a key, by [via]; true when it was due to be
   * written (then it also joins the key's latest uses). */
  touchKey(keyHash: string, now: number, via: KeyVia = "extension"): boolean {
    const key = this.keys.find((k) => k.hash === keyHash);
    if (!key) return false;
    const due = key.lastUsedAt === null || now >= key.lastUsedAt + TOUCH_EVERY_SECS;
    if (due) {
      key.lastUsedAt = now;
      key.uses = [{ at: now, via }, ...key.uses].slice(0, MAX_KEY_USES);
    }
    return due;
  }

  recordSignIn(at: number, by: SignInBy, ok: boolean, reason: string | null = null): void {
    this.signIns = [{ at, by, ok, reason }, ...this.signIns].slice(0, MAX_SIGN_INS);
  }

  /** False when the account has the most keys allowed. */
  addKey(keyHash: string, label: string, deviceId: string, now: number): boolean {
    if (this.keys.length >= MAX_KEYS_PER_ACCOUNT) return false;
    this.keys.push({ hash: keyHash, label, createdAt: now, lastUsedAt: null, deviceId, uses: [] });
    return true;
  }

  /** Removes the key with this short id; its hash, for the index. */
  revokeKey(id: string): string | null {
    const wanted = id.trim();
    if (!wanted) return null;
    const at = this.keys.findIndex((key) => keyId(key.hash) === wanted);
    return at < 0 ? null : this.keys.splice(at, 1)[0].hash;
  }

  /** Counts a session call; false once the minute's allowance is used. */
  allow(now: number): boolean {
    if (now >= this.windowStart + 60) {
      this.windowStart = now;
      this.windowHits = 1;
      return true;
    }
    this.windowHits += 1;
    return this.windowHits - 1 < ACCOUNT_RATE_PER_MINUTE;
  }

  /** What `/v1/account` shows. Keys and the phone list only to a linked
   * phone (`thisPhone`, its index). */
  view(thisPhone: number | null, forwarding: boolean, now: number) {
    const mine = thisPhone !== null;
    const vault = this.usableVault(now);
    return {
      registrationNumber: this.id,
      settings: this.settings,
      semesterId: this.semesterId,
      linked: this.linked(),
      savedCredentials: vault !== null,
      vaultSavedAt: vault !== null ? this.vaultSavedAt : null,
      vaultGmail: vault !== null && this.vaultGmail,
      credentialsRejectedAt: this.credentialsRejectedAt,
      forwarding,
      phoneLinked: this.linked(),
      phoneCount: this.phones.length,
      thisPhone: mine,
      phones: mine
        ? this.phones.map((phone, i) => ({
            id: phoneId(phone.deviceId),
            label: phone.label,
            addedAt: phone.addedAt,
            seenAt: Math.max(phone.seenAt, phone.addedAt),
            thisPhone: i === thisPhone,
          }))
        : [],
      keys: mine
        ? this.keys.map((key) => ({
            id: keyId(key.hash),
            label: key.label,
            createdAt: key.createdAt,
            lastUsedAt: key.lastUsedAt,
            phoneId: phoneId(key.deviceId),
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
  error: string | null = null;
  createdAt: number;
  expiresAt: number;
  /** When a sign-in with saved credentials is due (phone first). */
  vaultDueAt: number | null = null;
  vaultStarted = false;
  phoneAsked = false;
  /** Why the phone could not be asked, kept to end the request with if the
   * sign-in fails too. */
  phoneError: string | null = null;
  /** Who made the session, once answered: `phone` or `server`. */
  source: string | null = null;
  /** The installs this request may ask: the key's phone. */
  phones: string[] = [];

  constructor(
    readonly id: string,
    readonly responseToken: string,
    now: number,
  ) {
    this.createdAt = now;
    this.expiresAt = now + REQUEST_TIMEOUT_SECS;
  }

  pending(now: number): boolean {
    return this.status === "pending" && now < this.expiresAt;
  }

  /** Answers the request; `source` is who made the session. */
  succeed(cookies: string, source: string): void {
    this.status = "success";
    this.cookies = cookies;
    this.error = null;
    this.source = source;
  }

  fail(error: string): void {
    this.status = "error";
    this.error = error;
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
      return { status: 410, body: { code: "request_expired", error: "The session request expired." } };
    }
    switch (this.status) {
      case "pending":
        return { status: 202, body: this.progress() };
      case "success":
        return { status: 200, body: ready(this.cookies ?? "", sourceView(this.source, null, false)) };
      case "error": {
        const error = this.error ?? "";
        return { status: 200, body: { status: "error", code: errorCode(error), error } };
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
