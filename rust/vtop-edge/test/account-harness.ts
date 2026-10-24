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
  const env = { ENGINE_URL: "http://engine", BRIDGE_SECRET: "s", ...vars } as unknown as Env;
  const ctx = { storage: { sql: db } } as unknown as DurableObjectState;
  const account = new Account(ctx, env);
  return {
    account,
    /** Private methods, for seeding. */
    inner: account as unknown as Record<string, (...args: unknown[]) => unknown>,
    seed: (doc: AccountRecord) => saveAccount(db, doc),
    load: () => loadAccount(db),
  };
}
