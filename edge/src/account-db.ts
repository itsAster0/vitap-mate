// The `Account` object's SQL tables, and its rows to and from records;
// with the checks of what a phone sends to keep there.

import {
  AccountRecord,
  type KeyRecord,
  type KeyUse,
  MAX_KEY_USES,
  MAX_SIGN_INS,
  type PhoneRecord,
  RequestRecord,
  type RequestStatus,
  type SignInRecord,
} from "./account-state";
import { str } from "./common";
import type { GmailGrant, OtpMethod } from "./shared/engine-api";
import * as sql from "./sql";

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
  // Account tokens (account-token.ts), on every install the student showed
  // a VTOP cookie from, linked or not: the student's mailbox owner hash, for
  // calls that bring a token instead of the session, and the `iat` of each
  // install's one live token. Made on first use, so objects of schema 2
  // keep their data.
  db.exec("CREATE TABLE IF NOT EXISTS owner (hash TEXT NOT NULL)");
  db.exec("CREATE TABLE IF NOT EXISTS tokens (device_id TEXT PRIMARY KEY, issued_at INTEGER NOT NULL)");
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

export function requestFrom(row: sql.Row): RequestRecord | null {
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

export function usableText(raw: unknown): string | null {
  return typeof raw === "string" ? raw.trim() || null : null;
}

export interface Credentials {
  username: string;
  password: string;
  gmail: GmailGrant | null;
  /** The OTP auto-fill picked on the phone; null from older phones. */
  otp: OtpMethod | null;
}

/** `username`, `password`, any Gmail access and the OTP method of a body's
 * `credentials`, if it has them. */
export function credentialsOf(raw: unknown): Credentials | null {
  if (raw === null || typeof raw !== "object") return null;
  const { username, password, gmail, otp } = raw as Record<string, unknown>;
  return {
    username: str(username) ?? "",
    password: str(password) ?? "",
    gmail: gmailGrantOf(gmail),
    otp: otp === "gmail" || otp === "forwarding" ? otp : null,
  };
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
