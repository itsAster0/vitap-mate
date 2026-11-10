// Accounts on the edge: one `Account` object per student, named by the
// registration number. It holds the linked phones (device ids), access keys,
// settings, the sealed session cache and saved credentials, and the
// session requests of agents and the extension.
//
// A session request is answered from the cache (checked with VTOP by the
// engine), or left pending while the phone that created the key is asked
// (an event on its install) and/or the engine signs in with saved
// credentials, the OTP read from the student's forwarded mail or, without
// a forwarding address, from their Gmail. Server sign-in therefore needs
// one of the two: credentials are only kept for students with forwarding
// set up or Gmail access sent along.
//
// Routes:
// - Key holders (`Authorization: Bearer vtm_…`): `GET /v1/whoami`,
//   `POST /v1/session`, `GET /v1/session/requests/{id}`,
//   `POST /v1/session/expire`.
// - The extension while it pairs, with no key yet: `POST /v1/pair` (pair.ts).
//   The app asks `POST /v1/pair/info` whose code it is (oauth.ts).
// - Phones sign every call (auth.ts); a phone is its device id.
// - The phone's answer to a request: `POST /v1/session/answer?account=`.
// - The app, with a live VTOP session: `/v1/link`, `/v1/keys`,
//   `/v1/keys/revoke`, `/v1/keys/outings`, `/v1/keys/move`, `/v1/account`, `/v1/account/{
//   forget-credentials,delete,settings,semester,remove-phone,clear-sign-ins}`. And without
//   one: `/v1/account/credentials-rejected` (VTOP turned the password down
//   on the phone, so the server must stop trying it too) and `/v1/app/*`.
// - The engine: `/internal/accounts/login-done`.

import { DurableObject } from "cloudflare:workers";

import {
  AccountRecord,
  type KeyRecord,
  type KeyUse,
  type KeyVia,
  MAX_KEY_USES,
  MAX_SIGN_INS,
  type SignInRecord,
  type PhoneRecord,
  PHONE_UNREACHABLE,
  REQUEST_KEEP_SECS,
  RequestRecord,
  type RequestStatus,
  errorCode,
  isAccountId,
  keyId,
  keyLabel,
  newKey,
  parseSettings,
  plan,
  ready,
  semesterId,
  toHeader,
  validCookies,
} from "./account-state";
import { type Signed, signed } from "./auth";
import { oauthStub, pairInfo } from "./oauth";
import { PAIR_TTL_SECS, claim, pairCode, pairStub } from "./pair";
import {
  type Env,
  badRequest,
  bearer,
  deviceStub,
  empty,
  engineJson,
  enginePost,
  mcpUrl,
  error,
  isInternal,
  jsonBody,
  notInternal,
  now,
  ownerStub,
  str,
  variable,
} from "./common";
import type { DeviceInfo, EventBody } from "./device";
import {
  LOGIN,
  type LoginDone,
  type LoginStart,
  SEAL,
  SESSION,
  type GmailGrant,
  type SealVault,
  type SealedVault,
  type SessionCheck,
  type SessionChecked,
  accountId,
} from "./shared/engine-api";
import { KEY_PREFIX, hash } from "./shared/tokens";
import * as sql from "./sql";
import { address as mailAddress, memberLabel } from "./state";

/** How long `/v1/app/login` waits for the server's sign-in: over the
 * engine's VTOP sign-in and its wait for the OTP mail (45 s). */
const APP_LOGIN_WAIT_MS = 90_000;

const PHONE_UNAVAILABLE = "phone_unavailable: Could not reach the phone right now.";
const PHONE_SESSION_INVALID =
  "phone_session_invalid: VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.";
const ACCOUNT_MISMATCH = "account_mismatch: Your phone is signed in to VTOP as a different student.";

function accountStub(env: Env, id: string) {
  return env.ACCOUNT.getByName(id);
}

function keyIndex(env: Env, keyHash: string) {
  return env.INDEX.getByName(`key:${keyHash}`);
}

/** The student's forwarding address (needed to sign in on the server), if
 * they have one. */
async function forwardingMailbox(env: Env, ownerHash: string): Promise<string | null> {
  if (!ownerHash) return null;
  return (await ownerStub(env, ownerHash).get()) || null;
}

/** The routes this module answers at the edge. */
export function handles(path: string): boolean {
  return (
    ["/v1/whoami", "/v1/session", "/v1/pair", "/v1/pair/info", "/v1/link", "/v1/keys", "/v1/keys/revoke", "/v1/keys/outings", "/v1/keys/move", "/v1/account"].includes(
      path,
    ) ||
    path.startsWith("/v1/session/") ||
    path.startsWith("/v1/account/")
  );
}

const APP_OPS: Record<string, AppOp> = {
  "/v1/link": "link",
  "/v1/keys": "create-key",
  "/v1/keys/revoke": "revoke-key",
  "/v1/keys/outings": "key-outings",
  "/v1/keys/move": "move-keys",
  "/v1/account": "account",
  "/v1/account/forget-credentials": "forget-credentials",
  "/v1/account/delete": "delete",
  "/v1/account/settings": "settings",
  "/v1/account/semester": "semester",
  "/v1/account/remove-phone": "remove-phone",
  "/v1/account/clear-sign-ins": "clear-sign-ins",
};

/** Which app a key call comes from: vtop-mcp says so in `x-vtop-client`;
 * anything else is the browser extension. */
export function keyVia(req: Request): KeyVia {
  return req.headers.get("x-vtop-client")?.trim().toLowerCase() === "mcp" ? "mcp" : "extension";
}

export async function handle(req: Request, env: Env, path: string): Promise<Response> {
  if (req.method === "GET" && path === "/v1/whoami") return withKey(req, env, (a, c) => a.whoami(c));
  if (req.method === "GET" && path.startsWith("/v1/session/requests/")) {
    const requestId = path.slice("/v1/session/requests/".length);
    return withKey(req, env, (a, c) => a.poll(c, requestId));
  }
  if (req.method !== "POST") return error(405, "method_not_allowed", "Method not allowed.");
  if (path === "/v1/session") return withKey(req, env, (a, c) => a.startSession(c, keyVia(req)));
  if (path === "/v1/session/expire") return withKey(req, env, (a, c) => a.expire(c));
  if (path === "/v1/pair") return claim(await jsonBody(req), env);
  if (path === "/v1/pair/info") return pairInfo(await jsonBody(req), env);
  const op = APP_OPS[path];
  if (!op && path !== "/v1/session/answer" && path !== "/v1/account/credentials-rejected") {
    return error(404, "not_found", "No such route.");
  }
  // The rest come from phones.
  const auth = await signed(req, env);
  if (auth instanceof Response) return auth;
  const body = auth.json;
  if (!body) return badRequest("Invalid JSON body.");
  if (path === "/v1/session/answer") {
    const account = accountId(new URL(req.url).searchParams.get("account") ?? "");
    if (!isAccountId(account)) return error(404, "request_not_found", "Unknown session request.");
    return accountStub(env, account).answer(auth.device.id, body);
  }
  if (path === "/v1/account/credentials-rejected") {
    const account = accountId(str(body.registrationNumber) ?? "");
    if (!isAccountId(account)) return badRequest("registrationNumber is required.");
    return accountStub(env, account).app("credentials-rejected", { checked: null, mailboxId: null, body, device: auth.device });
  }
  return withSession(env, op, body, auth.device);
}

type AccountStub = ReturnType<typeof accountStub>;

/** Finds the account of the access key on the request. */
async function withKey(
  req: Request,
  env: Env,
  then: (account: AccountStub, keyHash: string) => Promise<Response>,
): Promise<Response> {
  const presented = bearer(req);
  const unknown = () => error(401, "key_unknown", "Unknown or revoked access key.");
  if (!presented.startsWith(KEY_PREFIX)) return unknown();
  const keyHash = hash(presented);
  const account = await keyIndex(env, keyHash).get();
  if (account === null) return unknown();
  return then(accountStub(env, account), keyHash);
}

/** Asks the engine whose live VTOP session the app sent, then hands the
 * call to that student's account. */
async function withSession(env: Env, op: AppOp, body: Record<string, unknown>, device: DeviceInfo): Promise<Response> {
  const session = body.session as Record<string, unknown> | undefined;
  const cookies = str(session?.cookies)?.trim();
  if (!cookies) return badRequest("session.cookies is required.");
  let checked: SessionChecked;
  try {
    checked = await engineJson<SessionChecked>(env, SESSION, { cookies } satisfies SessionCheck);
  } catch (e) {
    console.log(`account: session check failed: ${e}`);
    return error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
  }
  const { registrationNumber, ownerHash } = checked;
  if (!registrationNumber || !ownerHash) {
    return checked.status === "expired"
      ? error(401, "session_invalid", "VTOP does not accept this session; sign in again.")
      : error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
  }
  const mailboxId = await forwardingMailbox(env, ownerHash);
  return accountStub(env, registrationNumber).app(op, { checked, mailboxId, body, device });
}

/** `POST /v1/app/status`, `/v1/app/session/clear` and `/v1/app/login`: a
 * linked phone's view of its account, and a sign-in by the server. The
 * phone names its account by registration number and signs the call. */
export async function appCall(auth: Signed, env: Env, op: "status" | "session-clear" | "login"): Promise<Response> {
  const body = auth.json;
  if (!body) return badRequest("Invalid JSON body.");
  const registrationNumber = str(body.registrationNumber)?.trim();
  if (!registrationNumber) return badRequest("registrationNumber is required.");
  const stub = accountStub(env, accountId(registrationNumber));
  const device = auth.device.id;
  switch (op) {
    case "status":
      return stub.appStatus(device);
    case "session-clear":
      return stub.appSessionClear(device);
    case "login":
      return stub.appLogin(device, body);
  }
}

/** Takes an install off a student's phones, as it deletes itself
 * (`/v1/devices/delete`); nothing when it is not one of them. */
export async function forgetPhone(env: Env, registrationNumber: string, deviceId: string): Promise<void> {
  const account = accountId(registrationNumber);
  if (isAccountId(account)) await accountStub(env, account).forgetPhone(deviceId);
}

/** `POST /internal/accounts/login-done`, from the engine. */
export async function internal(req: Request, env: Env, path: string): Promise<Response> {
  if (!isInternal(req, env)) return notInternal();
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
  if (path !== "/internal/accounts/login-done") return error(404, "not_found", "No such route.");
  const account = accountId(str(body.account) ?? "");
  if (!isAccountId(account)) return badRequest("account is required.");
  const done = body as unknown as LoginDone;
  if (typeof done.requestId !== "string" || !isLoginOutcome(done.outcome)) return badRequest("Invalid JSON body.");
  return accountStub(env, account).loginDone(done);
}

function isLoginOutcome(raw: unknown): raw is LoginDone["outcome"] {
  if (raw === null || typeof raw !== "object") return false;
  const outcome = raw as Record<string, unknown>;
  switch (outcome.status) {
    case "ok":
      return typeof outcome.cookies === "string" && typeof outcome.sealedSession === "string";
    case "failed":
      return typeof outcome.reason === "string";
    case "gmail_revoked":
      return typeof outcome.sealedVault === "string";
    case "invalid_credentials":
    case "vault_unusable":
    case "forwarding_required":
      return true;
    default:
      return false;
  }
}

// ---- the object -------------------------------------------------------------

type AppOp =
  | "credentials-rejected"
  | "link"
  | "create-key"
  | "revoke-key"
  | "key-outings"
  | "move-keys"
  | "account"
  | "forget-credentials"
  | "delete"
  | "settings"
  | "semester"
  | "remove-phone"
  | "clear-sign-ins";

/** An app call: whose session came with it (none for
 * `credentials-rejected`), the student's forwarding address, the body and
 * the install that signed it. */
interface AppCall {
  checked: SessionChecked | null;
  mailboxId: string | null;
  body: Record<string, unknown>;
  device: DeviceInfo;
}

export const SCHEMA = `
CREATE TABLE account (
    id TEXT PRIMARY KEY,
    always_use_vault INTEGER NOT NULL,
    phone_wait_secs INTEGER NOT NULL,
    vault_ttl_secs INTEGER,
    semester_id TEXT,
    mailbox_id TEXT,
    sealed_vault TEXT,
    vault_saved_at INTEGER,
    sealed_session TEXT,
    session_saved_at INTEGER,
    session_source TEXT,
    credentials_rejected_at INTEGER,
    vault_gmail INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    window_start INTEGER NOT NULL,
    window_hits INTEGER NOT NULL
);
CREATE TABLE keys (
    hash TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    device_id TEXT NOT NULL,
    outings INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE phones (
    device_id TEXT PRIMARY KEY,
    label TEXT,
    added_at INTEGER NOT NULL,
    seen_at INTEGER NOT NULL
);
CREATE TABLE requests (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    cookies TEXT,
    csrf_token TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    vault_due_at INTEGER,
    vault_started INTEGER NOT NULL,
    phone_asked INTEGER NOT NULL,
    phone_error TEXT,
    source TEXT,
    phones TEXT NOT NULL
);
CREATE TABLE pending (request_id TEXT NOT NULL);
CREATE TABLE sign_ins (at INTEGER NOT NULL, by TEXT NOT NULL, ok INTEGER NOT NULL, reason TEXT);
CREATE TABLE key_uses (key_hash TEXT NOT NULL, at INTEGER NOT NULL, via TEXT NOT NULL)`;

export function prepareAccountDb(db: SqlStorage): void {
  sql.prepare(db, SCHEMA);
}

type Value = string | number | null;

/** The `account` row's columns besides `id`. */
function accountColumns(doc: AccountRecord): Array<[string, Value]> {
  return [
    ["always_use_vault", doc.settings.alwaysUseVault ? 1 : 0],
    ["phone_wait_secs", doc.settings.phoneWaitSecs],
    ["vault_ttl_secs", doc.settings.vaultTtlSecs],
    ["semester_id", doc.semesterId],
    ["mailbox_id", doc.mailboxId],
    ["sealed_vault", doc.sealedVault],
    ["vault_saved_at", doc.vaultSavedAt],
    ["vault_gmail", doc.vaultGmail ? 1 : 0],
    ["sealed_session", doc.sealedSession],
    ["session_saved_at", doc.sessionSavedAt],
    ["session_source", doc.sessionSource],
    ["credentials_rejected_at", doc.credentialsRejectedAt],
    ["created_at", doc.createdAt],
    ["updated_at", doc.updatedAt],
    ["window_start", doc.windowStart],
    ["window_hits", doc.windowHits],
  ];
}

function accountFrom(row: sql.Row, keys: KeyRecord[], phones: PhoneRecord[]): AccountRecord | null {
  const id = sql.text(row, "id");
  const phoneWaitSecs = sql.int(row, "phone_wait_secs");
  const createdAt = sql.int(row, "created_at");
  const updatedAt = sql.int(row, "updated_at");
  if (id === null || phoneWaitSecs === null || createdAt === null || updatedAt === null) return null;
  const doc = new AccountRecord(id, createdAt);
  doc.settings = {
    alwaysUseVault: sql.flag(row, "always_use_vault"),
    phoneWaitSecs,
    vaultTtlSecs: sql.int(row, "vault_ttl_secs"),
  };
  doc.semesterId = sql.text(row, "semester_id");
  doc.mailboxId = sql.text(row, "mailbox_id");
  doc.sealedVault = sql.text(row, "sealed_vault");
  doc.vaultSavedAt = sql.int(row, "vault_saved_at");
  doc.vaultGmail = sql.flag(row, "vault_gmail");
  doc.sealedSession = sql.text(row, "sealed_session");
  doc.sessionSavedAt = sql.int(row, "session_saved_at");
  doc.sessionSource = sql.text(row, "session_source");
  doc.credentialsRejectedAt = sql.int(row, "credentials_rejected_at");
  doc.updatedAt = updatedAt;
  doc.windowStart = sql.int(row, "window_start") ?? 0;
  doc.windowHits = sql.int(row, "window_hits") ?? 0;
  doc.keys = keys;
  doc.phones = phones;
  return doc;
}

function keyFrom(row: sql.Row): KeyRecord | null {
  const keyHash = sql.text(row, "hash");
  const label = sql.text(row, "label");
  const createdAt = sql.int(row, "created_at");
  const deviceId = sql.text(row, "device_id");
  if (keyHash === null || label === null || createdAt === null || deviceId === null) return null;
  return {
    hash: keyHash,
    label,
    createdAt,
    lastUsedAt: sql.int(row, "last_used_at"),
    deviceId,
    outings: sql.flag(row, "outings"),
    uses: [],
  };
}

function signInFrom(row: sql.Row): SignInRecord | null {
  const at = sql.int(row, "at");
  const by = sql.text(row, "by");
  if (at === null || (by !== "server" && by !== "phone")) return null;
  return { at, by, ok: sql.flag(row, "ok"), reason: sql.text(row, "reason") };
}

function putSignIn(db: SqlStorage, signIn: SignInRecord): void {
  db.exec(
    "INSERT INTO sign_ins (at, by, ok, reason) VALUES (?, ?, ?, ?)",
    signIn.at,
    signIn.by,
    signIn.ok ? 1 : 0,
    signIn.reason,
  );
}

function putKeyUse(db: SqlStorage, keyHash: string, use: KeyUse): void {
  db.exec("INSERT INTO key_uses (key_hash, at, via) VALUES (?, ?, ?)", keyHash, use.at, use.via);
}

/** Drops all but the newest sign-ins and key uses. */
function pruneHistory(db: SqlStorage, doc: AccountRecord): void {
  db.exec(
    "DELETE FROM sign_ins WHERE rowid NOT IN (SELECT rowid FROM sign_ins ORDER BY at DESC, rowid DESC LIMIT ?)",
    MAX_SIGN_INS,
  );
  for (const key of doc.keys) {
    db.exec(
      "DELETE FROM key_uses WHERE key_hash = ? AND rowid NOT IN (SELECT rowid FROM key_uses WHERE key_hash = ? ORDER BY at DESC, rowid DESC LIMIT ?)",
      key.hash,
      key.hash,
      MAX_KEY_USES,
    );
  }
}

/** The stored account, or null. */
export function loadAccount(db: SqlStorage): AccountRecord | null {
  const row = sql.first(db, "SELECT * FROM account LIMIT 1");
  if (!row) return null;
  const keys = sql.rows(db, "SELECT * FROM keys ORDER BY created_at").flatMap((r) => keyFrom(r) ?? []);
  for (const use of sql.rows(db, "SELECT * FROM key_uses ORDER BY at DESC, rowid DESC")) {
    const key = keys.find((k) => k.hash === sql.text(use, "key_hash"));
    const at = sql.int(use, "at");
    const via = sql.text(use, "via");
    if (!key || at === null || (via !== "extension" && via !== "mcp") || key.uses.length >= MAX_KEY_USES) continue;
    key.uses.push({ at, via });
  }
  const phones = sql.rows(db, "SELECT * FROM phones ORDER BY added_at").flatMap((r) => phoneFrom(r) ?? []);
  const doc = accountFrom(row, keys, phones);
  if (doc) {
    doc.signIns = sql
      .rows(db, "SELECT * FROM sign_ins ORDER BY at DESC, rowid DESC LIMIT ?", MAX_SIGN_INS)
      .flatMap((r) => signInFrom(r) ?? []);
    doc.markLoaded();
  }
  return doc;
}

/** Writes [doc]: only what changed since it was loaded, when it was. */
export function saveAccount(db: SqlStorage, doc: AccountRecord): void {
  if (doc.loaded) writeChanges(db, doc.loaded, doc);
  else writeAccount(db, doc);
  pruneHistory(db, doc);
  doc.markLoaded();
}

function phoneFrom(row: sql.Row): PhoneRecord | null {
  const deviceId = sql.text(row, "device_id");
  const addedAt = sql.int(row, "added_at");
  if (deviceId === null || addedAt === null) return null;
  return { deviceId, label: sql.text(row, "label"), addedAt, seenAt: sql.int(row, "seen_at") ?? 0 };
}

function requestFrom(row: sql.Row): RequestRecord | null {
  const id = sql.text(row, "id");
  const status = sql.text(row, "status");
  const createdAt = sql.int(row, "created_at");
  const expiresAt = sql.int(row, "expires_at");
  if (id === null || status === null || createdAt === null || expiresAt === null) return null;
  const request = new RequestRecord(id, createdAt);
  request.status = status as RequestStatus;
  request.cookies = sql.text(row, "cookies");
  request.csrfToken = sql.text(row, "csrf_token");
  request.error = sql.text(row, "error");
  request.expiresAt = expiresAt;
  request.vaultDueAt = sql.int(row, "vault_due_at");
  request.vaultStarted = sql.flag(row, "vault_started");
  request.phoneAsked = sql.flag(row, "phone_asked");
  request.phoneError = sql.text(row, "phone_error");
  request.source = sql.text(row, "source");
  request.phones = (sql.text(row, "phones") ?? "").split(",").filter(Boolean);
  return request;
}

function putKey(db: SqlStorage, key: KeyRecord): void {
  db.exec(
    "INSERT OR REPLACE INTO keys (hash, label, created_at, last_used_at, device_id, outings) VALUES (?, ?, ?, ?, ?, ?)",
    key.hash,
    key.label,
    key.createdAt,
    key.lastUsedAt,
    key.deviceId,
    key.outings ? 1 : 0,
  );
}

function putPhone(db: SqlStorage, phone: PhoneRecord): void {
  db.exec(
    "INSERT OR REPLACE INTO phones (device_id, label, added_at, seen_at) VALUES (?, ?, ?, ?)",
    phone.deviceId,
    phone.label,
    phone.addedAt,
    phone.seenAt,
  );
}

/** Replaces everything stored with `doc`. */
function writeAccount(db: SqlStorage, doc: AccountRecord): void {
  const columns = accountColumns(doc);
  db.exec("DELETE FROM account");
  db.exec(
    `INSERT INTO account (id, ${columns.map(([name]) => name).join(", ")}) VALUES (?${", ?".repeat(columns.length)})`,
    doc.id,
    ...columns.map(([, value]) => value),
  );
  db.exec("DELETE FROM keys");
  for (const key of doc.keys) putKey(db, key);
  db.exec("DELETE FROM key_uses");
  for (const key of doc.keys) for (const use of key.uses) putKeyUse(db, key.hash, use);
  db.exec("DELETE FROM sign_ins");
  for (const signIn of doc.signIns) putSignIn(db, signIn);
  db.exec("DELETE FROM phones");
  for (const phone of doc.phones) putPhone(db, phone);
}

function sameRecord<T extends object>(a: T, b: T): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** Writes only what changed since `base` was read, so requests running at
 * the same time (each waits on the network between reading and saving) do
 * not undo each other's changes. */
function writeChanges(db: SqlStorage, base: AccountRecord, doc: AccountRecord): void {
  const before = accountColumns(base);
  const changed = accountColumns(doc).filter(([, value], i) => value !== before[i][1]);
  if (changed.length) {
    db.exec(
      `UPDATE account SET ${changed.map(([name]) => `${name} = ?`).join(", ")} WHERE id = ?`,
      ...changed.map(([, value]) => value),
      doc.id,
    );
  }
  for (const key of base.keys) {
    if (!doc.keys.some((k) => k.hash === key.hash)) {
      db.exec("DELETE FROM keys WHERE hash = ?", key.hash);
      db.exec("DELETE FROM key_uses WHERE key_hash = ?", key.hash);
    }
  }
  // History only grows: add what this request recorded.
  for (const signIn of doc.signIns) {
    if (!base.signIns.some((s) => sameRecord(s, signIn))) putSignIn(db, signIn);
  }
  for (const key of doc.keys) {
    const was = base.keys.find((k) => k.hash === key.hash)?.uses ?? [];
    for (const use of key.uses) if (!was.some((u) => sameRecord(u, use))) putKeyUse(db, key.hash, use);
  }
  for (const key of doc.keys) {
    const was = base.keys.find((k) => k.hash === key.hash);
    if (!was) {
      putKey(db, key);
    } else if (
      was.label !== key.label ||
      was.lastUsedAt !== key.lastUsedAt ||
      was.deviceId !== key.deviceId ||
      was.outings !== key.outings
    ) {
      // A key revoked meanwhile stays revoked: only update a row that is
      // still there.
      db.exec(
        "UPDATE keys SET label = ?, last_used_at = ?, device_id = ?, outings = ? WHERE hash = ?",
        key.label,
        key.lastUsedAt,
        key.deviceId,
        key.outings ? 1 : 0,
        key.hash,
      );
    }
  }
  for (const phone of base.phones) {
    if (!doc.phones.some((p) => p.deviceId === phone.deviceId)) {
      db.exec("DELETE FROM phones WHERE device_id = ?", phone.deviceId);
    }
  }
  for (const phone of doc.phones) {
    if (!base.phones.some((p) => samePhone(p, phone))) putPhone(db, phone);
  }
}

function samePhone(a: PhoneRecord, b: PhoneRecord): boolean {
  return a.deviceId === b.deviceId && a.label === b.label && a.addedAt === b.addedAt && a.seenAt === b.seenAt;
}

function usableText(raw: unknown): string | null {
  return typeof raw === "string" ? raw.trim() || null : null;
}

interface Credentials {
  username: string;
  password: string;
  gmail: GmailGrant | null;
}

/** `username`, `password` and any Gmail access of a body's `credentials`,
 * if it has them. */
function credentialsOf(raw: unknown): Credentials | null {
  if (raw === null || typeof raw !== "object") return null;
  const { username, password, gmail } = raw as Record<string, unknown>;
  return { username: str(username) ?? "", password: str(password) ?? "", gmail: gmailGrantOf(gmail) };
}

/** Gmail access the phone sent with its credentials, if usable. */
export function gmailGrantOf(raw: unknown): GmailGrant | null {
  if (raw === null || typeof raw !== "object") return null;
  const grant = raw as Record<string, unknown>;
  const refreshToken = usableText(grant.refreshToken);
  const clientId = usableText(grant.clientId);
  if (refreshToken === null || clientId === null) return null;
  const clientSecret = usableText(grant.clientSecret);
  return {
    refreshToken,
    clientId,
    ...(clientSecret !== null ? { clientSecret } : {}),
    deleteAfterReading: grant.deleteAfterReading === true,
  };
}

function pending(request: RequestRecord): Response {
  return Response.json(request.startedView(), { status: 202, headers: { "retry-after": "2" } });
}

function notLinked(): Response {
  return error(403, "not_linked", "This phone is not linked to the account; link it from the app.");
}

export class Account extends DurableObject<Env> {
  private ready = false;

  private get db(): SqlStorage {
    const db = this.ctx.storage.sql;
    if (!this.ready) {
      prepareAccountDb(db);
      this.ready = true;
    }
    return db;
  }

  private doc(): AccountRecord | null {
    return loadAccount(this.db);
  }

  private save(doc: AccountRecord): void {
    doc.updatedAt = now();
    saveAccount(this.db, doc);
  }

  private request(id: string): RequestRecord | null {
    if (!id || id.length > 64) return null;
    const row = sql.first(this.db, "SELECT * FROM requests WHERE id = ?", id);
    return row && requestFrom(row);
  }

  private putRequest(request: RequestRecord): void {
    this.db.exec(
      `INSERT OR REPLACE INTO requests (id, status, cookies, error, created_at, expires_at,
           vault_due_at, vault_started, phone_asked, phone_error, source, phones, csrf_token)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      request.id,
      request.status,
      request.cookies,
      request.error,
      request.createdAt,
      request.expiresAt,
      request.vaultDueAt,
      request.vaultStarted ? 1 : 0,
      request.phoneAsked ? 1 : 0,
      request.phoneError,
      request.source,
      request.phones.join(","),
      request.csrfToken,
    );
  }

  private requests(): RequestRecord[] {
    return sql.rows(this.db, "SELECT * FROM requests").flatMap((r) => requestFrom(r) ?? []);
  }

  private pendingId(): string | null {
    const row = sql.first(this.db, "SELECT request_id FROM pending LIMIT 1");
    return row && sql.text(row, "request_id");
  }

  private setPending(id: string): void {
    this.db.exec("DELETE FROM pending");
    this.db.exec("INSERT INTO pending (request_id) VALUES (?)", id);
  }

  // ---- key holders --------------------------------------------------------

  private keyDoc(keyHash: string): AccountRecord | Response {
    const doc = this.doc();
    return doc?.hasKey(keyHash) ? doc : error(401, "key_unknown", "Unknown or revoked access key.");
  }

  async whoami(keyHash: string): Promise<Response> {
    const doc = this.keyDoc(keyHash);
    if (doc instanceof Response) return doc;
    const key = doc.keys.find((k) => k.hash === keyHash);
    const vault = doc.usableVault(now());
    // Read-only: how a session for this key will be made, for clients to
    // show. Changing it stays in the app.
    return Response.json({
      registrationNumber: doc.id,
      keyLabel: key?.label ?? null,
      keyCreatedAt: key?.createdAt ?? null,
      keyLastUsedAt: key?.lastUsedAt ?? null,
      keyOutings: key?.outings ?? false,
      semesterId: doc.semesterId,
      settings: doc.settings,
      phoneLinked: doc.linked(),
      savedCredentials: vault !== null,
      vaultSavedAt: vault !== null ? doc.vaultSavedAt : null,
      credentialsRejectedAt: doc.credentialsRejectedAt,
    });
  }

  async startSession(keyHash: string, via: KeyVia = "extension"): Promise<Response> {
    const doc = this.keyDoc(keyHash);
    if (doc instanceof Response) return doc;
    const at = now();
    if (!doc.allow(at)) {
      this.save(doc);
      const response = error(429, "rate_limited", "Too many requests for this account.");
      response.headers.set("retry-after", "60");
      return response;
    }
    doc.touchKey(keyHash, at, via);

    const cached = await this.cachedSession(doc, at);
    if (cached !== null) {
      this.save(doc);
      return Response.json(ready(cached.cookies, doc.cachedSource()));
    }

    const pendingId = this.pendingId();
    const joined = pendingId === null ? null : this.request(pendingId);
    if (joined?.pending(at)) {
      this.save(doc);
      return pending(joined);
    }

    // Recorded before any network call, so a session call that comes in
    // meanwhile joins this request instead of starting another.
    const request = new RequestRecord(crypto.randomUUID(), at);
    request.phones = doc.phonesForKey(keyHash);
    this.putRequest(request);
    this.setPending(request.id);
    await this.begin(doc, request, at);
    this.save(doc);
    // The phone or the engine may have finished it already.
    if (this.request(request.id)?.pending(at)) this.putRequest(request);
    await this.reschedule(doc);
    return pending(request);
  }

  /** The cached session's cookies and CSRF token when the engine finds it
   * still works; one that expired is dropped. */
  private async cachedSession(
    doc: AccountRecord,
    at: number,
  ): Promise<{ cookies: string; csrfToken: string | null } | null> {
    const cached = doc.cachedSession(at);
    if (cached === null) return null;
    const check: SessionCheck = { sealedSession: cached, account: doc.id };
    try {
      const checked = await engineJson<SessionChecked>(this.env, SESSION, check);
      if (checked.status === "expired") {
        console.log("account: cached session no longer works");
        doc.clearSession();
        return null;
      }
      return checked.cookies ? { cookies: checked.cookies, csrfToken: checked.csrfToken ?? null } : null;
    } catch (e) {
      console.log(`account: cannot check the cache: ${e}`);
      return null;
    }
  }

  /** Asks the key's phones and/or starts a sign-in, in the order the
   * settings choose. */
  private async begin(doc: AccountRecord, request: RequestRecord, at: number): Promise<void> {
    const chosen = plan(doc.settings, doc.usableVault(at) !== null);
    switch (chosen.kind) {
      case "vault_first":
        if (!(await this.startLogin(doc, request, at))) await this.askPhoneOrFail(doc, request, at);
        return;
      case "phone_first": {
        const failure = await this.askPhone(doc, request, at);
        if (failure === null) {
          request.vaultDueAt = at + chosen.waitSecs;
        } else {
          request.phoneError = failure;
          if (!(await this.startLogin(doc, request, at))) request.fail(failure);
        }
        return;
      }
      case "phone_only":
        await this.askPhoneOrFail(doc, request, at);
    }
  }

  private async askPhoneOrFail(doc: AccountRecord, request: RequestRecord, at: number): Promise<void> {
    const failure = await this.askPhone(doc, request, at);
    if (failure !== null) request.fail(failure);
  }

  /** An event on each of the request's phones (installs); the first to
   * answer at `/v1/session/answer` wins. Null once one was asked, else why
   * none could be. */
  private async askPhone(doc: AccountRecord, request: RequestRecord, at: number): Promise<string | null> {
    if (request.phones.length === 0) return PHONE_UNREACHABLE;
    let base: string;
    try {
      base = variable(this.env, "PUBLIC_URL");
    } catch {
      return PHONE_UNAVAILABLE;
    }
    const event: EventBody = {
      kind: "vtop_cookie_request",
      requestId: request.id,
      callbackUrl: `${base}/v1/session/answer?account=${doc.id}`,
      wantCredentials: doc.wantCredentials(at) ? "1" : "0",
    };
    let failure = PHONE_UNREACHABLE;
    for (const device of request.phones) {
      try {
        if ((await deviceStub(this.env, device).append(event)) !== null) request.phoneAsked = true;
      } catch (e) {
        console.log(`account: asking a phone failed: ${e}`);
        failure = PHONE_UNAVAILABLE;
      }
    }
    return request.phoneAsked ? null : failure;
  }

  /** Starts a sign-in with saved credentials; false when there are none or
   * the engine cannot take it. */
  private async startLogin(doc: AccountRecord, request: RequestRecord, at: number): Promise<boolean> {
    const sealedVault = doc.usableVault(at);
    if (sealedVault === null) return false;
    const start: LoginStart = { account: doc.id, sealedVault, requestId: request.id };
    try {
      const response = await enginePost(this.env, LOGIN, start);
      if (response.status === 202) {
        request.vaultStarted = true;
        return true;
      }
      console.log(`account: engine refused a sign-in: ${response.status}`);
    } catch (e) {
      console.log(`account: engine unreachable for a sign-in: ${e}`);
    }
    return false;
  }

  async poll(keyHash: string, requestId: string): Promise<Response> {
    const doc = this.keyDoc(keyHash);
    if (doc instanceof Response) return doc;
    const request = this.request(requestId.trim());
    if (!request) return error(404, "request_not_found", "Unknown session request.");
    const { status, body } = request.pollView(now());
    return Response.json(body, { status });
  }

  async expire(keyHash: string): Promise<Response> {
    const doc = this.keyDoc(keyHash);
    if (doc instanceof Response) return doc;
    doc.clearSession();
    this.save(doc);
    return empty();
  }

  // ---- answers ------------------------------------------------------------

  /** The engine finished a sign-in with saved credentials. */
  async loginDone(done: LoginDone): Promise<Response> {
    const doc = this.doc();
    if (!doc) return error(404, "not_linked", "No such account.");
    const at = now();
    const request = this.request(done.requestId);
    // Set when the phone should be asked instead; with the message to end
    // the request with if it cannot be (null: why the phone could not).
    let fallback: { reason: string | null } | null = null;
    const outcome = done.outcome;
    switch (outcome.status) {
      case "ok":
        doc.recordSignIn(at, "server", true);
        doc.saveSession(outcome.sealedSession, at, "server");
        if (request?.pending(at)) request.succeed(outcome.cookies, "server", outcome.csrfToken ?? null);
        break;
      case "invalid_credentials":
        console.log("account: VTOP rejected saved credentials; dropping them");
        doc.recordSignIn(at, "server", false, "VTOP rejected the saved password");
        doc.rejectCredentials(at);
        fallback = { reason: "credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate." };
        break;
      case "forwarding_required":
        // The engine did not ask VTOP (it would have mailed an OTP nobody
        // reads). The credentials stay: the phone sends them again, with
        // Gmail access if it has it, until the server can read the OTP.
        doc.recordSignIn(at, "server", false, "Neither OTP forwarding nor Gmail is set up");
        fallback = {
          reason:
            "forwarding_required: Set up OTP email forwarding or connect Gmail in VITAP Mate to sign in from the server.",
        };
        break;
      case "gmail_revoked":
        console.log("account: Google refused the saved Gmail access; dropping it");
        doc.recordSignIn(at, "server", false, "Google refused the saved Gmail access");
        doc.dropGmail(outcome.sealedVault);
        fallback = {
          reason: "gmail_revoked: Gmail access was revoked. Connect Gmail again in VITAP Mate to sign in from the server.",
        };
        break;
      case "vault_unusable":
        doc.recordSignIn(at, "server", false, "The saved password can no longer be used");
        doc.clearVault();
        fallback = { reason: null };
        break;
      case "failed":
        console.log(`account: server sign-in failed for now: ${outcome.reason}`);
        doc.recordSignIn(at, "server", false, outcome.reason);
        fallback = { reason: null };
        break;
    }
    if (fallback && request?.pending(at)) {
      if (!request.phoneAsked) {
        const failure = await this.askPhone(doc, request, at);
        if (failure !== null) request.fail(fallback.reason ?? failure);
      } else if (request.phoneError !== null) {
        request.fail(fallback.reason ?? request.phoneError);
      }
    }
    this.save(doc);
    if (request) this.putRequest(request);
    if (outcome.status === "invalid_credentials") await this.tellPhones(doc, { kind: "credentials_rejected" });
    return empty();
  }

  /** An event on every linked phone; one that cannot be reached is skipped. */
  private async tellPhones(doc: AccountRecord, event: EventBody): Promise<void> {
    for (const phone of doc.phones) {
      try {
        await deviceStub(this.env, phone.deviceId).append(event);
      } catch (e) {
        console.log(`account: telling a phone failed: ${e}`);
      }
    }
  }

  // ---- app status -----------------------------------------------------------

  /** The account, when [deviceId] is one of its phones. */
  private phoneDoc(deviceId: string): AccountRecord | Response {
    const doc = this.doc();
    return doc && doc.phoneIndex(deviceId) !== null ? doc : notLinked();
  }

  async appStatus(deviceId: string): Promise<Response> {
    const doc = this.phoneDoc(deviceId);
    if (doc instanceof Response) return doc;
    let forwarding: string | null = null;
    if (doc.mailboxId !== null) {
      try {
        forwarding = mailAddress(doc.mailboxId, variable(this.env, "MAIL_DOMAIN"));
      } catch {
        // No mail domain configured: the address cannot be shown.
      }
    }
    return Response.json(doc.statusView(forwarding));
  }

  /** A linked phone in UDP mode asks the server to sign it in with the
   * saved password, the engine reading the OTP from forwarded mail or
   * Gmail, so the phone sends nothing secret. Answers
   * once done, like `/v1/vtop/login`; a rejected password is
   * `invalid_credentials`, and the saved one is dropped (see [loginDone]). */
  async appLogin(deviceId: string, body: Record<string, unknown>): Promise<Response> {
    const doc = this.phoneDoc(deviceId);
    if (doc instanceof Response) return doc;
    const at = now();
    if (doc.usableVault(at) === null) {
      return error(409, "no_saved_credentials", "The server keeps no password for this account.");
    }
    const registrationNumber = str(body.registrationNumber)?.trim().toUpperCase() ?? null;
    // The app's `SessionState`: without the CSRF token it would not count
    // the session as signed in.
    const signedIn = (cookies: string, csrfToken: string | null) =>
      Response.json({
        status: "ok",
        session: { cookies, csrf_token: csrfToken, registration_number: registrationNumber },
      });
    const cached = await this.cachedSession(doc, at);
    if (cached !== null) {
      this.save(doc);
      return signedIn(cached.cookies, cached.csrfToken);
    }

    const pendingId = this.pendingId();
    let request = pendingId === null ? null : this.request(pendingId);
    if (!request?.pending(at)) {
      // No phones to ask: this phone is the one asking.
      request = new RequestRecord(crypto.randomUUID(), at);
      this.putRequest(request);
      this.setPending(request.id);
      const started = await this.startLogin(doc, request, at);
      this.save(doc);
      if (!started) {
        request.fail("server_login_failed: The server could not start a sign-in.");
        this.putRequest(request);
        return error(503, "server_login_failed", "The server could not start a sign-in.");
      }
      // The engine may have finished it already.
      if (this.request(request.id)?.pending(at)) this.putRequest(request);
    }

    const id = request.id;
    const until = Date.now() + APP_LOGIN_WAIT_MS;
    let current = this.request(id);
    while (current?.pending(now()) && Date.now() < until) {
      await new Promise((resolve) => setTimeout(resolve, 500));
      current = this.request(id);
    }
    switch (current?.status) {
      case "success":
        return signedIn(current.cookies ?? "", current.csrfToken);
      case "error":
        if (errorCode(current.error ?? "") === "credentials_rejected") {
          return error(401, "invalid_credentials", "VTOP rejected the saved password.");
        }
        return error(503, "server_login_failed", current.error ?? "The server could not sign in.");
      default:
        return error(504, "vtop_timeout", "The server is still signing in.");
    }
  }

  /** Unlinks the install, which is deleting itself; its keys stay. */
  async forgetPhone(deviceId: string): Promise<void> {
    const doc = this.doc();
    const index = doc?.phoneIndex(deviceId) ?? null;
    if (doc === null || index === null) return;
    doc.phones.splice(index, 1);
    this.save(doc);
    console.log("account: a phone deleted itself");
  }

  async appSessionClear(deviceId: string): Promise<Response> {
    const doc = this.phoneDoc(deviceId);
    if (doc instanceof Response) return doc;
    doc.clearSession();
    this.save(doc);
    return Response.json({ ok: true });
  }

  /** The answer of [deviceId], one of the phones asked, to a session
   * request. */
  async answer(deviceId: string, answer: Record<string, unknown>): Promise<Response> {
    const notFound = () => error(404, "request_not_found", "Unknown session request.");
    const doc = this.doc();
    if (!doc) return notFound();
    const request = this.request((str(answer.requestId) ?? "").trim());
    if (!request) return notFound();
    if (!request.phones.includes(deviceId) || doc.phoneIndex(deviceId) === null) {
      return error(403, "not_asked", "This phone was not asked for this session.");
    }
    const at = now();
    const failed = (str(answer.error) ?? "").trim();
    if (failed) {
      if (request.pending(at)) {
        request.fail(failed);
        this.putRequest(request);
      }
      return Response.json({ ok: true });
    }
    const cookies = validCookies(answer.cookies ?? []);
    if (!cookies) return error(400, "COOKIES_INVALID", "The callback contained invalid cookies.");
    const header = toHeader(cookies);
    let checked: SessionChecked;
    try {
      checked = await engineJson<SessionChecked>(this.env, SESSION, { cookies: header } satisfies SessionCheck);
    } catch (e) {
      console.log(`account: checking a phone answer failed: ${e}`);
      return error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
    }
    switch (checked.status) {
      case "valid":
        if (checked.registrationNumber === doc.id) break;
        if (request.pending(at)) {
          request.fail(ACCOUNT_MISMATCH);
          this.putRequest(request);
        }
        return error(403, "account_mismatch", "These cookies belong to a different student than the one who asked.");
      case "expired":
        if (request.pending(at)) {
          request.fail(PHONE_SESSION_INVALID);
          this.putRequest(request);
        }
        return error(401, "session_invalid", "VTOP does not accept these cookies.");
      default:
        return error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
    }

    // The answer is genuine: cache it even if the client gave up.
    const late = !request.pending(at);
    if (!late) {
      request.succeed(header, "phone");
      this.putRequest(request);
    }
    if (checked.sealedSession) doc.saveSession(checked.sealedSession, at, "phone");
    doc.recordSignIn(at, "phone", true);
    const credentials = credentialsOf(answer.credentials);
    if (credentials) {
      const username = usableText(credentials.username) ?? usableText(answer.username);
      if (username !== null) {
        await this.keepCredentials(doc, checked.ownerHash ?? "", username, credentials, at);
      }
    }
    this.save(doc);
    return Response.json({ ok: true, late });
  }

  /** Seals and keeps credentials, only for students with forwarding set
   * up or Gmail access sent along; true when kept. */
  private async keepCredentials(
    doc: AccountRecord,
    ownerHash: string,
    username: string,
    credentials: Credentials,
    at: number,
  ): Promise<boolean> {
    const { password, gmail } = credentials;
    if (!password || !ownerHash) return false;
    if (gmail === null && (await forwardingMailbox(this.env, ownerHash)) === null) {
      console.log("account: credentials not kept: no forwarding address or Gmail access");
      return false;
    }
    try {
      const sealed = await engineJson<SealedVault>(this.env, SEAL, {
        account: doc.id,
        username,
        password,
        ...(gmail !== null ? { gmail } : {}),
      } satisfies SealVault);
      doc.saveVault(sealed.sealedVault, at, gmail !== null);
      return true;
    } catch (e) {
      console.log(`account: sealing credentials failed: ${e}`);
      return false;
    }
  }

  // ---- the app ------------------------------------------------------------

  async app(op: AppOp, call: AppCall): Promise<Response> {
    const at = now();
    const existing = this.doc();
    const body = call.body;
    const thisPhone = existing?.phoneIndex(call.device.id) ?? null;

    if (op === "credentials-rejected") {
      if (!existing || thisPhone === null) return notLinked();
      existing.rejectCredentials(at);
      this.save(existing);
      console.log("account: the phone reported rejected credentials");
      return empty();
    }

    const checked = call.checked;
    if (!checked) return badRequest("session.cookies is required.");
    if (existing && existing.mailboxId !== call.mailboxId) {
      existing.mailboxId = call.mailboxId;
      this.save(existing);
    }
    const id = accountId(checked.registrationNumber ?? "");
    const linked = existing?.linked() ?? false;
    const forwarding = call.mailboxId !== null;

    switch (op) {
      case "link":
        return this.link(existing, id, call.device, checked, forwarding, body, at);
      case "account": {
        const doc = existing ?? new AccountRecord(id, at);
        if (thisPhone !== null && doc.touchPhone(thisPhone, at)) this.save(doc);
        return Response.json(doc.view(thisPhone, forwarding, at));
      }
      case "forget-credentials":
        if (existing) {
          existing.clearVault();
          this.save(existing);
        }
        return empty();
    }
    // Revoking and deleting work before any phone is linked; the rest need
    // a linked phone.
    if (!linked && op !== "revoke-key" && op !== "delete") {
      return error(409, "not_linked", "Link this account from the app first.");
    }
    if (linked && thisPhone === null) return notLinked();
    return this.linkedOp(op, existing ?? new AccountRecord(id, at), thisPhone, body, at);
  }

  /** Links this install, next to any phones linked already; linking it
   * again updates its entry. */
  private async link(
    existing: AccountRecord | null,
    id: string,
    device: DeviceInfo,
    checked: SessionChecked,
    forwarding: boolean,
    body: Record<string, unknown>,
    at: number,
  ): Promise<Response> {
    const rawSemester = str(body.semesterId);
    const semester = rawSemester === undefined ? null : semesterId(rawSemester);
    if (rawSemester !== undefined && semester === null) return badRequest("semesterId is not a VTOP semester id.");
    const doc = existing ?? new AccountRecord(id, at);
    const me = doc.phoneIndex(device.id);
    const previous = me === null ? null : doc.phones[me];
    doc.addPhone({
      deviceId: device.id,
      label: memberLabel(str(body.label)) ?? device.label ?? previous?.label ?? null,
      addedAt: previous?.addedAt ?? at,
      seenAt: at,
    });
    if (checked.sealedSession) doc.saveSession(checked.sealedSession, at, "phone");
    if (semester !== null) doc.semesterId = semester;
    let saved = false;
    const credentials = credentialsOf(body.credentials);
    const username = usableText(credentials?.username);
    if (credentials && (forwarding || credentials.gmail !== null) && username !== null) {
      saved = await this.keepCredentials(doc, checked.ownerHash ?? "", username, credentials, at);
    }
    this.save(doc);
    return Response.json({ savedCredentials: saved || doc.usableVault(at) !== null, forwarding });
  }

  /** `me`: the calling phone, or null while the account has no linked phone
   * (only revoking and deleting get here then). */
  private async linkedOp(
    op: AppOp,
    doc: AccountRecord,
    me: number | null,
    body: Record<string, unknown>,
    at: number,
  ): Promise<Response> {
    if (me === null && op !== "revoke-key" && op !== "delete") return notLinked();
    switch (op) {
      case "create-key": {
        // A code from the extension, which then collects the key (pair.ts),
        // or from an MCP client's authorize page (oauth.ts), whose key is
        // named after the client.
        const pair = body.pairCode === undefined ? undefined : pairCode(str(body.pairCode));
        if (pair === null) return badRequest("pairCode must be a pairing code.");
        const app = pair === undefined ? null : await oauthStub(this.env).pending(pair, at);
        const key = newKey();
        const keyHash = hash(key);
        // Only this phone is asked for the key's sessions.
        const outings = body.outings === true;
        const label = keyLabel(app?.name ?? str(body.label));
        if (!doc.addKey(keyHash, label, doc.phones[me!].deviceId, at, outings)) {
          return error(409, "key_limit", "This account already has the most keys allowed; revoke one first.");
        }
        if (app !== null && !(await oauthStub(this.env).approve(pair!, key, at))) {
          return error(410, "pair_expired", "That code has expired; start connecting again.");
        }
        await keyIndex(this.env, keyHash).put(doc.id);
        if (pair !== undefined && app === null) await pairStub(this.env, pair).putUntil(key, (at + PAIR_TTL_SECS) * 1000);
        this.save(doc);
        return Response.json({ id: keyId(keyHash), key, mcpUrl: mcpUrl(this.env), app }, { status: 201 });
      }
      case "key-outings": {
        if (typeof body.outings !== "boolean") return badRequest("outings must be true or false.");
        if (!doc.setKeyOutings(str(body.id) ?? "", body.outings)) {
          return error(404, "key_not_found", "No such key on this account.");
        }
        this.save(doc);
        return empty();
      }
      case "move-keys": {
        const moved = doc.moveKeys(body.from === undefined ? null : (str(body.from) ?? ""), doc.phones[me!].deviceId);
        if (moved === null) return error(404, "phone_not_found", "No such phone on this account.");
        this.save(doc);
        return Response.json({ moved });
      }
      case "revoke-key": {
        const revoked = doc.revokeKey(str(body.id) ?? "");
        if (revoked === null) return error(404, "key_not_found", "No such key on this account.");
        await keyIndex(this.env, revoked).remove(doc.id);
        this.save(doc);
        return empty();
      }
      case "remove-phone":
        if (!doc.removePhone(str(body.id) ?? "")) {
          return error(404, "phone_not_found", "No such phone on this account.");
        }
        this.save(doc);
        return empty();
      case "clear-sign-ins":
        doc.signIns = [];
        this.save(doc);
        this.db.exec("DELETE FROM sign_ins");
        return empty();
      case "settings": {
        const settings = parseSettings(body.settings);
        if (!settings) return badRequest("settings must use the offered choices.");
        doc.settings = settings;
        this.save(doc);
        return empty();
      }
      case "semester": {
        const semester = semesterId(str(body.semesterId) ?? "");
        if (semester === null) return badRequest("semesterId is required.");
        doc.semesterId = semester;
        this.save(doc);
        return empty();
      }
      case "delete":
        for (const key of doc.keys) await keyIndex(this.env, key.hash).remove(doc.id);
        await this.ctx.storage.deleteAll();
        await this.ctx.storage.deleteAlarm();
        this.ready = false;
        console.log("account: deleted at the user's request");
        return empty();
      default:
        return error(404, "not_found", "No such route.");
    }
  }

  // ---- timers -------------------------------------------------------------

  /** The next thing due: a phone-first wait running out, a finished request
   * to drop, saved credentials expiring. */
  private async reschedule(doc: AccountRecord): Promise<void> {
    const due: number[] = [];
    for (const request of this.requests()) {
      if (request.vaultDueAt !== null && !request.vaultStarted && request.status === "pending") {
        due.push(request.vaultDueAt);
      }
      due.push(request.createdAt + REQUEST_KEEP_SECS);
    }
    const ttl = doc.settings.vaultTtlSecs;
    if (ttl !== null && doc.vaultSavedAt !== null && doc.sealedVault !== null) due.push(doc.vaultSavedAt + ttl);
    if (due.length === 0) {
      await this.ctx.storage.deleteAlarm();
      return;
    }
    const wait = Math.max(1, Math.min(...due) - now());
    await this.ctx.storage.setAlarm(Date.now() + wait * 1000);
  }

  override async alarm(): Promise<void> {
    const at = now();
    const doc = this.doc();
    if (!doc) return;
    for (const request of this.requests()) {
      if (at >= request.createdAt + REQUEST_KEEP_SECS) {
        this.db.exec("DELETE FROM requests WHERE id = ?", request.id);
        continue;
      }
      const due = request.vaultDueAt !== null && at >= request.vaultDueAt;
      if (due && !request.vaultStarted && request.pending(at)) {
        // The phone stayed silent: sign in with saved credentials.
        if (!(await this.startLogin(doc, request, at))) request.vaultDueAt = null;
        this.putRequest(request);
      }
    }
    if (doc.sealedVault !== null && doc.usableVault(at) === null) {
      doc.clearVault();
      this.save(doc);
    }
    await this.reschedule(doc);
  }
}
