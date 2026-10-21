// An `Account` Durable Object over in-memory SQLite, with the engine faked:
// enough to drive its methods in unit tests.

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
  const env = {
    BRIDGE_URL: "container",
    BRIDGE_SECRET: "s",
    ...vars,
    ENGINE: {
      getByName: () => ({
        fetch: async (req: Request) => engine(new URL(req.url).pathname, await req.json()),
      }),
    },
  } as unknown as Env;
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
