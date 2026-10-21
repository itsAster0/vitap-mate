// Durable Objects that hold one record each.
//
// `Mailbox`: a forwarding address, named by its id, and the installs it
// forwards to. A delivered box is cleared by an alarm once it can no
// longer be collected. While the engine signs its student in, it waits
// here for the next OTP, sealed to a key made for that sign-in (a
// "waiter").
// `Index`: one lookup value, named by what it indexes (`owner:<hash>`,
// `key:<hash>`).

import { DurableObject } from "cloudflare:workers";

import { type Env, now } from "./common";
import type { OtpAwaited } from "./shared/engine-api";
import * as sql from "./sql";
import { type MailboxDoc, type Waiter, WAITER_TTL_SECS, clearExpired, nextExpiry } from "./state";

/** The longest one wait lasts. */
const MAX_WAIT_SECS = 30;

const MAILBOX_SCHEMA = `
CREATE TABLE mailbox (
    id TEXT PRIMARY KEY,
    owner_hash TEXT,
    members TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE waiters (
    public_key TEXT PRIMARY KEY,
    owner_hash TEXT NOT NULL,
    since INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    results TEXT NOT NULL
)`;

function waiterFrom(row: sql.Row): Waiter | null {
  const publicKey = sql.text(row, "public_key");
  const ownerHash = sql.text(row, "owner_hash");
  const since = sql.int(row, "since");
  const expiresAt = sql.int(row, "expires_at");
  const results = sql.text(row, "results");
  if (publicKey === null || ownerHash === null || since === null || expiresAt === null || results === null) {
    return null;
  }
  return { publicKey, ownerHash, since, expiresAt, results: JSON.parse(results) as string[] };
}

export class Mailbox extends DurableObject<Env> {
  private ready = false;
  /** Engine waits parked until a code is handed to their waiter, by waiter
   * key. Calls to one object share this instance. */
  private wake = new Map<string, Array<() => void>>();

  private get db(): SqlStorage {
    const db = this.ctx.storage.sql;
    if (!this.ready) {
      sql.prepare(db, MAILBOX_SCHEMA);
      this.ready = true;
    }
    return db;
  }

  /** The address; null when there is none. */
  async doc(): Promise<MailboxDoc | null> {
    const row = sql.first(this.db, "SELECT * FROM mailbox LIMIT 1");
    if (!row) return null;
    const id = sql.text(row, "id");
    const members = sql.text(row, "members");
    const createdAt = sql.int(row, "created_at");
    const updatedAt = sql.int(row, "updated_at");
    if (id === null || members === null || createdAt === null || updatedAt === null) return null;
    return { id, ownerHash: sql.text(row, "owner_hash"), members: JSON.parse(members), createdAt, updatedAt };
  }

  private write(doc: MailboxDoc): void {
    this.db.exec("DELETE FROM mailbox");
    this.db.exec(
      "INSERT INTO mailbox (id, owner_hash, members, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
      doc.id,
      doc.ownerHash,
      JSON.stringify(doc.members),
      doc.createdAt,
      doc.updatedAt,
    );
  }

  async save(doc: MailboxDoc): Promise<void> {
    this.write(doc);
    const at = nextExpiry(doc);
    if (at !== null) {
      await this.ctx.storage.setAlarm(Date.now() + Math.max(1, at - now()) * 1000);
    }
  }

  /** Saves only if new, so a clash of random ids never overwrites; false
   * when the address exists. */
  async create(doc: MailboxDoc): Promise<boolean> {
    if (await this.doc()) return false;
    await this.save(doc);
    return true;
  }

  /** One box per install, each sealed to its own key; false when there is
   * no address. */
  async deliver(boxes: Array<{ deviceId: string; sealed: string }>): Promise<boolean> {
    const doc = await this.doc();
    if (!doc) return false;
    const at = now();
    for (const delivered of boxes) {
      const member = doc.members.find((m) => m.deviceId === delivered.deviceId);
      if (member) {
        member.sealed = delivered.sealed;
        member.sealedAt = at;
      }
    }
    doc.updatedAt = at;
    await this.save(doc);
    return true;
  }

  /** Wipes the address, tables and all. */
  async remove(): Promise<void> {
    await this.ctx.storage.deleteAll();
    this.ready = false;
  }

  override async alarm(): Promise<void> {
    await this.waiters();
    const doc = await this.doc();
    if (doc && clearExpired(doc, now())) this.write(doc);
  }

  // ---- waiters ------------------------------------------------------------------

  private waiter(publicKey: string): Waiter | null {
    const row = sql.first(this.db, "SELECT * FROM waiters WHERE public_key = ?", publicKey);
    return row && waiterFrom(row);
  }

  private putWaiter(waiter: Waiter): void {
    this.db.exec(
      "INSERT OR REPLACE INTO waiters (public_key, owner_hash, since, expires_at, results) VALUES (?, ?, ?, ?, ?)",
      waiter.publicKey,
      waiter.ownerHash,
      waiter.since,
      waiter.expiresAt,
      JSON.stringify(waiter.results),
    );
  }

  /** Live waiters; expired ones are dropped on the way. */
  async waiters(): Promise<Waiter[]> {
    this.db.exec("DELETE FROM waiters WHERE expires_at <= ?", now());
    return sql.rows(this.db, "SELECT * FROM waiters").flatMap((row) => waiterFrom(row) ?? []);
  }

  /** Hands a code sealed for a waiter to it, waking its waits. */
  async waiterResult(publicKey: string, sealed: string): Promise<void> {
    const waiter = this.waiter(publicKey);
    if (!waiter) return;
    waiter.results.push(sealed);
    this.putWaiter(waiter);
    const parked = this.wake.get(publicKey) ?? [];
    this.wake.delete(publicKey);
    for (const wakeUp of parked) wakeUp();
  }

  /** Registers the waiter if new, then hands out the oldest code sealed for
   * it, waiting up to `waitSecs` for one. A wait of 0 only registers, so
   * mail that comes before the first wait is not missed. Null when there
   * is no address. */
  async awaitCode(body: { publicKey: string; ownerHash: string; since: number; waitSecs: number }): Promise<OtpAwaited | null> {
    if (!(await this.doc())) return null;
    const key = body.publicKey;
    if (!this.waiter(key)) {
      this.putWaiter({
        publicKey: key,
        ownerHash: body.ownerHash,
        since: body.since,
        expiresAt: now() + WAITER_TTL_SECS,
        results: [],
      });
      if ((await this.ctx.storage.getAlarm()) === null) {
        await this.ctx.storage.setAlarm(Date.now() + (WAITER_TTL_SECS + 1) * 1000);
      }
    }
    const deadline = now() + Math.min(body.waitSecs, MAX_WAIT_SECS);
    for (;;) {
      const waiter = this.waiter(key);
      if (waiter && waiter.results.length > 0) {
        const sealed = waiter.results.shift()!;
        this.putWaiter(waiter);
        return { sealed };
      }
      const left = deadline - now();
      if (left <= 0) return { sealed: null };
      // Sleeps until `waiterResult` wakes it or time runs out, then looks
      // again.
      let wakeUp!: () => void;
      const woken = new Promise<void>((resolve) => (wakeUp = resolve));
      this.wake.set(key, [...(this.wake.get(key) ?? []), wakeUp]);
      let timer = 0;
      await Promise.race([woken, new Promise<void>((resolve) => (timer = setTimeout(resolve, left * 1000)))]);
      clearTimeout(timer);
      const parked = this.wake.get(key)?.filter((w) => w !== wakeUp);
      if (parked?.length) this.wake.set(key, parked);
      else this.wake.delete(key);
    }
  }
}

const INDEX_SCHEMA = "CREATE TABLE entry (value TEXT NOT NULL)";

export class Index extends DurableObject<Env> {
  private ready = false;

  private get db(): SqlStorage {
    const db = this.ctx.storage.sql;
    if (!this.ready) {
      sql.prepare(db, INDEX_SCHEMA);
      this.ready = true;
    }
    return db;
  }

  async get(): Promise<string | null> {
    const row = sql.first(this.db, "SELECT value FROM entry LIMIT 1");
    return row && sql.text(row, "value");
  }

  async put(value: string): Promise<void> {
    this.db.exec("DELETE FROM entry");
    this.db.exec("INSERT INTO entry (value) VALUES (?)", value);
  }

  /** Deletes only while it still holds the value given. */
  async remove(expected: string): Promise<void> {
    this.db.exec("DELETE FROM entry WHERE value = ?", expected);
  }
}
