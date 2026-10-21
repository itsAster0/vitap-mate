// @ts-nocheck — Node's SQLite has no types here (no @types/node).
// Cloudflare's `SqlStorage` over Node's in-memory SQLite: enough for tests
// to run the Durable Objects' load and save code.

import { DatabaseSync } from "node:sqlite";

export function memorySql(): SqlStorage {
  const db = new DatabaseSync(":memory:");
  return {
    exec(query: string, ...binds: unknown[]) {
      const statement = db.prepare(query);
      const rows = statement.columns().length > 0 ? statement.all(...binds) : (statement.run(...binds), []);
      const plain = rows.map((row) => ({ ...row }));
      return { toArray: () => plain, one: () => plain[0] };
    },
  } as unknown as SqlStorage;
}
