// SQL storage for the Durable Objects. Each object keeps its own tables,
// which the dashboard's data viewer can read.

export type Row = Record<string, SqlStorageValue>;

/** Bumped when a table changes: an object made by an older version is
 * wiped and its tables made again. */
const SCHEMA_VERSION = 2;

export function rows(db: SqlStorage, query: string, ...binds: unknown[]): Row[] {
  return db.exec<Row>(query, ...binds).toArray();
}

export function first(db: SqlStorage, query: string, ...binds: unknown[]): Row | null {
  return rows(db, query, ...binds)[0] ?? null;
}

/** Makes the tables of `schema` (statements separated by `;`) unless this
 * version made them already. Tables of an older version are dropped. */
export function prepare(db: SqlStorage, schema: string): void {
  const hasMeta = first(db, "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'meta'");
  if (hasMeta && first(db, "SELECT schema_version FROM meta")?.schema_version === SCHEMA_VERSION) {
    return;
  }
  const tables = rows(
    db,
    // Not SQLite's own tables, nor the runtime's (`_cf_…`).
    "SELECT name FROM sqlite_master WHERE type = 'table' AND substr(name, 1, 7) != 'sqlite_' AND substr(name, 1, 4) != '_cf_'",
  );
  for (const { name } of tables) db.exec(`DROP TABLE IF EXISTS "${name}"`);
  for (const statement of schema.split(";").map((s) => s.trim()).filter(Boolean)) {
    db.exec(statement);
  }
  db.exec("CREATE TABLE meta (schema_version INTEGER NOT NULL)");
  db.exec("INSERT INTO meta (schema_version) VALUES (?)", SCHEMA_VERSION);
}

export function text(row: Row, column: string): string | null {
  const value = row[column];
  return typeof value === "string" ? value : null;
}

export function int(row: Row, column: string): number | null {
  const value = row[column];
  return typeof value === "number" && value >= 0 ? value : null;
}

export function flag(row: Row, column: string): boolean {
  return (int(row, column) ?? 0) !== 0;
}
