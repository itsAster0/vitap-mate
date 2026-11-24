// The one-time move to backend v3 (`backend/scripts/backfill.ts`):
// `POST /internal/export` reads one Durable Object's stored state as it is,
// and `FREEZE` stops the writes while the last pass runs.
//
// The export reads raw rows, never through an object's `db` getter: that
// makes the tables when they are missing and drops them when their schema
// version is old, and an export must not change what it reads.

import { type Env, badRequest, error, isDeviceId, isInternal, jsonBody, notInternal, str } from "./common";
import { isAccountId } from "./account-state";
import { isId as isMailboxId } from "./state";
import { PHONE_ROUTES } from "./vtop-phone";
import * as sql from "./sql";

export const KINDS = ["device", "account", "mailbox", "oauth", "fcm"] as const;
export type Kind = (typeof KINDS)[number];

/** Per kind, the tables exported and the ones only counted. The counted ones
 * last minutes and some hold live secrets in the clear (answered cookies,
 * OAuth grants' access keys, engine OTP waiters), so they are not moved;
 * a device's event log is the v2 event channel, which v3 has no use for. */
export const TABLES: Record<Exclude<Kind, "fcm">, { keep: string[]; count: string[] }> = {
  device: { keep: ["device"], count: ["events"] },
  account: { keep: ["account", "keys", "key_uses", "phones", "sign_ins", "owner", "tokens"], count: ["requests", "pending"] },
  mailbox: { keep: ["mailbox"], count: ["waiters"] },
  oauth: { keep: ["clients"], count: ["requests", "grants"] },
};

/** The mailbox's key-value entries beside its tables (store.ts). */
export const MAILBOX_KV = ["gmail_confirm", "gmail_confirm_at", "last_otp_at"];

export type Exported = {
  /** Whether the object stores anything. */
  found: boolean;
  tables: Record<string, sql.Row[]>;
  kv: Record<string, unknown>;
  /** Rows left out, by table. */
  omitted: Record<string, number>;
};

/** The object's tables as stored; a table it never made is left out. */
export function dump(db: SqlStorage, keep: string[], count: string[]): Exported {
  const present = new Set(
    sql.rows(db, "SELECT name FROM sqlite_master WHERE type = 'table'").map((row) => sql.text(row, "name")),
  );
  const tables: Record<string, sql.Row[]> = {};
  const omitted: Record<string, number> = {};
  for (const name of keep) if (present.has(name)) tables[name] = sql.rows(db, `SELECT * FROM "${name}"`);
  for (const name of count) {
    if (!present.has(name)) continue;
    const row = sql.first(db, `SELECT COUNT(*) AS n FROM "${name}"`);
    omitted[name] = (row && sql.int(row, "n")) ?? 0;
  }
  return { found: Object.values(tables).some((rows) => rows.length > 0), tables, kv: {}, omitted };
}

/** Objects the export reads, by kind. */
type Exportable = { exportState(): Promise<Exported> };

function namespace(env: Env, kind: Exclude<Kind, "fcm">): DurableObjectNamespace {
  const map = { device: env.DEVICE, account: env.ACCOUNT, mailbox: env.MAILBOX, oauth: env.OAUTH };
  return map[kind] as unknown as DurableObjectNamespace;
}

/** Whether `id` names an object of this kind, as the routes name them. */
function validName(kind: Exclude<Kind, "fcm">, id: string): boolean {
  switch (kind) {
    case "device":
      return isDeviceId(id);
    case "account":
      return isAccountId(id);
    case "mailbox":
      return isMailboxId(id);
    case "oauth":
      return id === "oauth";
  }
}

/** `POST /internal/export { kind, id? , objectId? }`: one object's stored
 * state. `id` is its name (a device id, a registration number, a mailbox
 * id, `oauth`); `objectId` the 64-hex id the Cloudflare API lists. */
export async function exportObject(req: Request, env: Env): Promise<Response> {
  if (!isInternal(req, env)) return notInternal();
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
  const kind = str(body.kind) as Kind | undefined;
  if (kind === undefined || !KINDS.includes(kind)) return badRequest(`kind must be one of ${KINDS.join(", ")}.`);
  // The FCM object only caches Google's access token in memory.
  if (kind === "fcm") return Response.json({ kind, found: false, tables: {}, kv: {}, omitted: {} });
  const id = str(body.id) ?? (kind === "oauth" && body.objectId === undefined ? "oauth" : undefined);
  const objectId = str(body.objectId);
  if ((id === undefined) === (objectId === undefined)) return badRequest("Send exactly one of id or objectId.");
  if (id !== undefined && !validName(kind, id)) return badRequest(`id is not a ${kind} name.`);
  if (objectId !== undefined && !/^[0-9a-f]{64}$/.test(objectId)) return badRequest("objectId must be 64 lowercase hex.");
  const ns = namespace(env, kind);
  let stub: Exportable;
  try {
    stub = ns.get(id !== undefined ? ns.idFromName(id) : ns.idFromString(objectId!)) as unknown as Exportable;
  } catch {
    return error(404, "not_found", "No such object.");
  }
  const state = await stub.exportState();
  return Response.json({ kind, ...(id !== undefined ? { id } : { objectId }), ...state });
}

// ---- the freeze -------------------------------------------------------------------

/** POST routes that only read (and the engine's VTOP calls, which keep
 * nothing here), still answered while frozen. */
const READS = new Set([
  "/v1/devices/info",
  "/v1/app/status",
  "/v1/mail-otp/inbox",
  "/v1/mail-otp/devices",
  "/v1/pair/info",
  ...Object.keys(PHONE_ROUTES),
]);

/** With `FREEZE` set, every route that writes the objects answers `503
 * frozen`, so nothing changes between the last export and the switch. Mail
 * still comes in and is read (students keep signing in), and `/internal/*`
 * stays open for the engine and the export. GET routes only read, but the
 * account proof's socket issues a token. */
export function frozen(env: Env, method: string, path: string): Response | null {
  if (!env.FREEZE) return null;
  if (path.startsWith("/internal/") || method === "OPTIONS" || method === "HEAD") return null;
  const writes = method === "GET" ? path === "/v1/account/prove" : !READS.has(path);
  return writes ? error(503, "frozen", "The server is moving; try again in a few minutes.", { headers: { "retry-after": "60" } }) : null;
}
