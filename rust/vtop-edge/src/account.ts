// Accounts on the edge: one `Account` object per student, named by the
// registration number. It holds the linked phones' secrets, access keys,
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
// - The phone's answer to a request: `POST /v1/session/answer?account=`.
// - The app, with a live VTOP session: `/v1/link`, `/v1/keys`,
//   `/v1/keys/revoke`, `/v1/keys/outings`, `/v1/account`, `/v1/account/{fcm-token,
//   forget-credentials,delete,settings,semester,remove-phone}`. And with
//   only its app secret: `/v1/account/credentials-rejected` (VTOP turned
//   the password down on the phone, so the server must stop trying it too).
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
  linkSecret,
  newKey,
  parseSettings,
  plan,
  ready,
  semesterId,
  toHeader,
  validCookies,
} from "./account-state";
import {
  type Env,
  badRequest,
  bearer,
  createDevice,
  deviceMatches,
  deviceStub,
  empty,
  engineJson,
  enginePost,
  mcpUrl,
  error,
  isEngine,
  jsonBody,
  now,
  ownerStub,
  setDeviceFcm,
  unknownDevice,
  str,
  variable,
} from "./common";
import type { EventBody } from "./device";
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
import { KEY_PREFIX, hash, hashesEqual, randomToken } from "./shared/tokens";
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
    ["/v1/whoami", "/v1/session", "/v1/link", "/v1/keys", "/v1/keys/revoke", "/v1/keys/outings", "/v1/account"].includes(
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
  "/v1/account": "account",
  "/v1/account/fcm-token": "fcm-token",
  "/v1/account/forget-credentials": "forget-credentials",
  "/v1/account/delete": "delete",
  "/v1/account/settings": "settings",
  "/v1/account/semester": "semester",
  "/v1/account/remove-phone": "remove-phone",
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
  switch (path) {
    case "/v1/session":
      return withKey(req, env, (a, c) => a.startSession(c, keyVia(req)));
    case "/v1/session/expire":
      return withKey(req, env, (a, c) => a.expire(c));
    case "/v1/session/answer": {
      const account = accountId(new URL(req.url).searchParams.get("account") ?? "");
      if (!isAccountId(account)) return error(404, "request_not_found", "Unknown session request.");
      const body = await jsonBody(req);
      if (!body) return badRequest("Invalid JSON body.");
      return accountStub(env, account).answer(body);
    }
    case "/v1/account/credentials-rejected": {
      const body = await jsonBody(req);
      if (!body) return badRequest("Invalid JSON body.");
      const account = accountId(str(body.registrationNumber) ?? "");
      if (!isAccountId(account)) return badRequest("registrationNumber is required.");
      return accountStub(env, account).app("credentials-rejected", { checked: null, mailboxId: null, body });
    }
  }
  const op = APP_OPS[path];
  if (!op) return error(404, "not_found", "No such route.");
  return withSession(req, env, op);
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
async function withSession(req: Request, env: Env, op: AppOp): Promise<Response> {
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
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
  return accountStub(env, registrationNumber).app(op, { checked, mailboxId, body });
}

/** `POST /v1/data/refresh`: VTOP data through the engine, streamed (see
 * data-refresh.ts). The phone names its account by registration number and
 * proves it is one of its phones with its app secret. */
/** `POST /v1/app/status`, `/v1/app/session/clear` and `/v1/app/login`: a
 * linked phone's view of its account, and a sign-in by the server,
 * authenticated like `/v1/data/refresh`. */
export async function appCall(req: Request, env: Env, op: "status" | "session-clear" | "login"): Promise<Response> {
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
  const registrationNumber = str(body.registrationNumber)?.trim();
  if (!registrationNumber) return badRequest("registrationNumber is required.");
  const stub = accountStub(env, accountId(registrationNumber));
  switch (op) {
    case "status":
      return stub.appStatus(body);
    case "session-clear":
      return stub.appSessionClear(body);
    case "login":
      return stub.appLogin(body);
  }
}

/** `POST /internal/accounts/login-done`, from the engine. */
export async function internal(req: Request, env: Env, path: string): Promise<Response> {
  if (!isEngine(req, env)) return error(401, "unauthorized", "The engine secret is required.");
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
  | "account"
  | "fcm-token"
  | "forget-credentials"
  | "delete"
  | "settings"
  | "semester"
  | "remove-phone";

/** An app call: whose session came with it (none for
 * `credentials-rejected`), the student's forwarding address and the body. */
interface AppCall {
  checked: SessionChecked | null;
  mailboxId: string | null;
  body: Record<string, unknown>;
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
    device_id TEXT NOT NULL
);
CREATE TABLE phones (
    device_id TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    label TEXT,
    added_at INTEGER NOT NULL,
    seen_at INTEGER NOT NULL
);
CREATE TABLE requests (
    id TEXT PRIMARY KEY,
    response_token TEXT NOT NULL,
    status TEXT NOT NULL,
    cookies TEXT,
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
CREATE TABLE pending (request_id TEXT NOT NULL)`;

/** Tables added after `SCHEMA`: made if missing, so accounts stored before
 * them keep everything (a new `SCHEMA_VERSION` would wipe them). */
const EXTRA_SCHEMA = [
  "CREATE TABLE IF NOT EXISTS sign_ins (at INTEGER NOT NULL, by TEXT NOT NULL, ok INTEGER NOT NULL, reason TEXT)",
  "CREATE TABLE IF NOT EXISTS key_uses (key_hash TEXT NOT NULL, at INTEGER NOT NULL, via TEXT NOT NULL)",
];

/** `account` columns added after `SCHEMA`, the same way. */
const EXTRA_ACCOUNT_COLUMNS = [["vault_gmail", "INTEGER NOT NULL DEFAULT 0"]];
/** `keys` columns added after `SCHEMA`. */
const EXTRA_KEY_COLUMNS = [["outings", "INTEGER NOT NULL DEFAULT 0"]];

function addColumns(db: SqlStorage, table: string, wanted: string[][]): void {
  const columns = new Set(sql.rows(db, `PRAGMA table_info(${table})`).map((row) => sql.text(row, "name")));
  for (const [name, type] of wanted) {
    if (!columns.has(name)) db.exec(`ALTER TABLE ${table} ADD COLUMN ${name} ${type}`);
  }
}

export function prepareAccountDb(db: SqlStorage): void {
  sql.prepare(db, SCHEMA);
  for (const statement of EXTRA_SCHEMA) db.exec(statement);
  addColumns(db, "account", EXTRA_ACCOUNT_COLUMNS);
  addColumns(db, "keys", EXTRA_KEY_COLUMNS);
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
  const secretHash = sql.text(row, "secret_hash");
  const addedAt = sql.int(row, "added_at");
  if (deviceId === null || secretHash === null || addedAt === null) return null;
  return { deviceId, secretHash, label: sql.text(row, "label"), addedAt, seenAt: sql.int(row, "seen_at") ?? 0 };
}

function requestFrom(row: sql.Row): RequestRecord | null {
  const id = sql.text(row, "id");
  const responseToken = sql.text(row, "response_token");
  const status = sql.text(row, "status");
  const createdAt = sql.int(row, "created_at");
  const expiresAt = sql.int(row, "expires_at");
  if (id === null || responseToken === null || status === null || createdAt === null || expiresAt === null) {
    return null;
  }
  const request = new RequestRecord(id, responseToken, createdAt);
  request.status = status as RequestStatus;
  request.cookies = sql.text(row, "cookies");
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
    "INSERT OR REPLACE INTO phones (device_id, secret_hash, label, added_at, seen_at) VALUES (?, ?, ?, ?, ?)",
    phone.deviceId,
    phone.secretHash,
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

function samePhone(a: PhoneRecord, b: PhoneRecord): boolean {
  return (
    a.deviceId === b.deviceId &&
    a.secretHash === b.secretHash &&
    a.label === b.label &&
    a.addedAt === b.addedAt &&
    a.seenAt === b.seenAt
  );
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

function appSecretInvalid(): Response {
  return error(403, "app_secret_invalid", "This phone is not linked to the account; link it from the app.");
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
      `INSERT OR REPLACE INTO requests (id, response_token, status, cookies, error, created_at, expires_at,
           vault_due_at, vault_started, phone_asked, phone_error, source, phones)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      request.id,
      request.responseToken,
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

    const cookies = await this.cachedCookies(doc, at);
    if (cookies !== null) {
      this.save(doc);
      return Response.json(ready(cookies, doc.cachedSource()));
    }

    const pendingId = this.pendingId();
    const joined = pendingId === null ? null : this.request(pendingId);
    if (joined?.pending(at)) {
      this.save(doc);
      return pending(joined);
    }

    // Recorded before any network call, so a session call that comes in
    // meanwhile joins this request instead of starting another.
    const request = new RequestRecord(crypto.randomUUID(), randomToken(""), at);
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

  /** The cached session's cookies when the engine finds it still works;
   * one that expired is dropped. */
  private async cachedCookies(doc: AccountRecord, at: number): Promise<string | null> {
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
      return checked.cookies ?? null;
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
      responseToken: request.responseToken,
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
        if (request?.pending(at)) request.succeed(outcome.cookies, "server");
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

  /** The account, when [body]'s app secret is one of its phones'. */
  private phoneDoc(body: Record<string, unknown>): AccountRecord | Response {
    const doc = this.doc();
    if (!doc || doc.phoneBySecret(str(body.appSecret)) === null) {
      return error(403, "not_linked", "This phone is not linked to the account.");
    }
    return doc;
  }

  async appStatus(body: Record<string, unknown>): Promise<Response> {
    const doc = this.phoneDoc(body);
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
   * Gmail, so the phone sends nothing secret but its app secret. Answers
   * once done, like `/v1/vtop/login`; a rejected password is
   * `invalid_credentials`, and the saved one is dropped (see [loginDone]). */
  async appLogin(body: Record<string, unknown>): Promise<Response> {
    const doc = this.phoneDoc(body);
    if (doc instanceof Response) return doc;
    const at = now();
    if (doc.usableVault(at) === null) {
      return error(409, "no_saved_credentials", "The server keeps no password for this account.");
    }
    const registrationNumber = str(body.registrationNumber)?.trim().toUpperCase() ?? null;
    const signedIn = (cookies: string) =>
      Response.json({ status: "ok", session: { cookies, registration_number: registrationNumber } });
    const cookies = await this.cachedCookies(doc, at);
    if (cookies !== null) {
      this.save(doc);
      return signedIn(cookies);
    }

    const pendingId = this.pendingId();
    let request = pendingId === null ? null : this.request(pendingId);
    if (!request?.pending(at)) {
      // No phones to ask: this phone is the one asking.
      request = new RequestRecord(crypto.randomUUID(), randomToken(""), at);
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
        return signedIn(current.cookies ?? "");
      case "error":
        if (errorCode(current.error ?? "") === "credentials_rejected") {
          return error(401, "invalid_credentials", "VTOP rejected the saved password.");
        }
        return error(503, "server_login_failed", current.error ?? "The server could not sign in.");
      default:
        return error(504, "vtop_timeout", "The server is still signing in.");
    }
  }

  async appSessionClear(body: Record<string, unknown>): Promise<Response> {
    const doc = this.phoneDoc(body);
    if (doc instanceof Response) return doc;
    doc.clearSession();
    this.save(doc);
    return Response.json({ ok: true });
  }

  /** The phone's answer to a session request. */
  async answer(answer: Record<string, unknown>): Promise<Response> {
    const notFound = () => error(404, "request_not_found", "Unknown session request.");
    const doc = this.doc();
    if (!doc) return notFound();
    const request = this.request((str(answer.requestId) ?? "").trim());
    if (!request) return notFound();
    if (!hashesEqual(hash(request.responseToken), hash((str(answer.responseToken) ?? "").trim()))) {
      return error(403, "token_invalid", "The response token does not match.");
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
    // Which phone answered is known only when it says so, or when there is
    // just one.
    const device = usableText(answer.deviceId);
    const answeredBy =
      device !== null ? doc.phones.find((p) => p.deviceId === device) : doc.phones.length === 1 ? doc.phones[0] : undefined;
    const token = usableText(answer.fcmToken);
    if (token !== null && answeredBy) await setDeviceFcm(this.env, answeredBy.deviceId, token);
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
    const appSecret = str(body.appSecret);

    if (op === "credentials-rejected") {
      if (!existing || existing.phoneBySecret(appSecret) === null) return appSecretInvalid();
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
    const thisPhone = existing?.phoneBySecret(appSecret) ?? null;
    const linked = existing?.linked() ?? false;
    const forwarding = call.mailboxId !== null;

    switch (op) {
      case "link":
        return this.link(existing, id, checked, forwarding, body, at);
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
    // a linked phone's secret.
    if (!linked && op !== "revoke-key" && op !== "delete") {
      return error(409, "not_linked", "Link this account from the app first.");
    }
    if (linked && thisPhone === null) return appSecretInvalid();
    return this.linkedOp(op, existing ?? new AccountRecord(id, at), thisPhone, body, at);
  }

  /** Links this install, next to any phones linked already; linking it
   * again hands it a new secret. */
  private async link(
    existing: AccountRecord | null,
    id: string,
    checked: SessionChecked,
    forwarding: boolean,
    body: Record<string, unknown>,
    at: number,
  ): Promise<Response> {
    const me = existing?.phoneBySecret(str(body.appSecret)) ?? null;
    const rawSemester = str(body.semesterId);
    const semester = rawSemester === undefined ? null : semesterId(rawSemester);
    if (rawSemester !== undefined && semester === null) return badRequest("semesterId is not a VTOP semester id.");
    const fcm = usableText(body.fcmToken);
    const doc = existing ?? new AccountRecord(id, at);
    const previous = me === null ? null : { ...doc.phones[me] };

    let device: string;
    const deviceId = str(body.deviceId);
    const deviceSecret = str(body.deviceSecret);
    if (deviceId !== undefined && deviceSecret !== undefined) {
      if (!(await deviceMatches(this.env, deviceId, deviceSecret))) return unknownDevice();
      if (fcm !== null) await setDeviceFcm(this.env, deviceId, fcm);
      device = deviceId;
    } else if (fcm === null) {
      return badRequest("fcmToken, or deviceId and deviceSecret, are required.");
    } else if (previous) {
      await setDeviceFcm(this.env, previous.deviceId, fcm);
      device = previous.deviceId;
    } else {
      device = (await createDevice(this.env, fcm)).id;
    }

    // The install may have moved (a new one after a reinstall of the push
    // token): its old entry goes.
    if (previous) {
      doc.phones = doc.phones.filter((p) => p.deviceId !== previous.deviceId);
      if (previous.deviceId !== device) doc.phoneMoved(previous.deviceId, device);
    }
    const secret = linkSecret(str(body.appSecret), me);
    doc.addPhone({
      deviceId: device,
      secretHash: hash(secret),
      label: memberLabel(str(body.label)) ?? previous?.label ?? null,
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
    return Response.json({
      appSecret: secret,
      savedCredentials: saved || doc.usableVault(at) !== null,
      forwarding,
    });
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
    if (me === null && op !== "revoke-key" && op !== "delete") return appSecretInvalid();
    switch (op) {
      case "create-key": {
        const key = newKey();
        const keyHash = hash(key);
        // Only this phone is asked for the key's sessions.
        const outings = body.outings === true;
        if (!doc.addKey(keyHash, keyLabel(str(body.label)), doc.phones[me!].deviceId, at, outings)) {
          return error(409, "key_limit", "This account already has the most keys allowed; revoke one first.");
        }
        await keyIndex(this.env, keyHash).put(doc.id);
        this.save(doc);
        return Response.json({ id: keyId(keyHash), key, mcpUrl: mcpUrl(this.env) }, { status: 201 });
      }
      case "key-outings": {
        if (typeof body.outings !== "boolean") return badRequest("outings must be true or false.");
        if (!doc.setKeyOutings(str(body.id) ?? "", body.outings)) {
          return error(404, "key_not_found", "No such key on this account.");
        }
        this.save(doc);
        return empty();
      }
      case "revoke-key": {
        const revoked = doc.revokeKey(str(body.id) ?? "");
        if (revoked === null) return error(404, "key_not_found", "No such key on this account.");
        await keyIndex(this.env, revoked).remove(doc.id);
        this.save(doc);
        return empty();
      }
      case "fcm-token": {
        const token = usableText(body.fcmToken);
        if (token === null) return badRequest("fcmToken is required.");
        const phone = doc.phones[me!];
        const before = phone.deviceId;
        const deviceId = str(body.deviceId);
        const deviceSecret = str(body.deviceSecret);
        if (deviceId !== undefined && deviceSecret !== undefined) {
          if (!(await deviceMatches(this.env, deviceId, deviceSecret))) return unknownDevice();
          await setDeviceFcm(this.env, deviceId, token);
          phone.deviceId = deviceId;
        } else if (!(await setDeviceFcm(this.env, phone.deviceId, token))) {
          // The install is gone: make one for this token, so the phone can
          // still be asked.
          phone.deviceId = (await createDevice(this.env, token)).id;
        }
        const moved = phone.deviceId;
        if (before !== moved) doc.phoneMoved(before, moved);
        // Another entry for the install it moved to is this phone.
        let seen = false;
        doc.phones = doc.phones.filter((p) => {
          const keep = p.deviceId !== moved || !seen;
          if (p.deviceId === moved) seen = true;
          return keep;
        });
        this.save(doc);
        return empty();
      }
      case "remove-phone":
        if (!doc.removePhone(str(body.id) ?? "")) {
          return error(404, "phone_not_found", "No such phone on this account.");
        }
        this.save(doc);
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
