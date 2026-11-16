// An `Account` Durable Object over in-memory SQLite, with the engine faked:
// enough to drive its methods in unit tests.

import { vi } from "vitest";

import { Account, loadAccount, prepareAccountDb, saveAccount } from "../src/account";
import type { AccountRecord } from "../src/account-state";
import type { Env } from "../src/common";
import { memorySql } from "./memory-sql";

export type FakeEngine = (path: string, body: unknown) => Response | Promise<Response>;

export function accountObject(
  engine: FakeEngine = () => new Response(null, { status: 503 }),
  vars: Record<string, unknown> = {},
) {
  const db = memorySql();
  prepareAccountDb(db);
  vi.stubGlobal("fetch", async (url: string, init: RequestInit) =>
    engine(new URL(url).pathname, JSON.parse(init.body as string)),
  );
  const env = { ENGINE_URL: "http://engine", INTERNAL_TOKEN: "s", ...vars } as unknown as Env;
  const deleteAll = async () => {
    const tables = db.exec<{ name: string }>("SELECT name FROM sqlite_master WHERE type = 'table'").toArray();
    for (const { name } of tables) db.exec(`DROP TABLE "${name}"`);
  };
  const ctx = { storage: { sql: db, deleteAll, deleteAlarm: async () => {} } } as unknown as DurableObjectState;
  const account = new Account(ctx, env);
  return {
    account,
    /** Private methods, for seeding. */
    inner: account as unknown as Record<string, (...args: unknown[]) => unknown>,
    seed: (doc: AccountRecord) => saveAccount(db, doc),
    load: () => loadAccount(db),
  };
}
