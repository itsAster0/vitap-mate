// The one-time backfill: every Durable Object of the old edge, read through
// its `POST /internal/export`, imported into Convex by `migrate:importBatch`
// under the old ids. Safe to run again: the import inserts what's missing and
// only brings rows it made up to a newer edge state (convex/migrate.ts).
//
//   bun scripts/backfill.ts --dry-run          # read the edge, print counts per kind
//   bun scripts/backfill.ts                    # read the edge and import
//   bun scripts/backfill.ts --save out.json    # also keep the exports (0600) to replay
//   bun scripts/backfill.ts --from out.json    # import saved (or synthetic) exports
//   bun scripts/backfill.ts --ids ids.json     # names to read besides the listing:
//                                              #   { "account": [regno], "device": [id], "mailbox": [id] }
//
// Env:
//   EDGE_URL, INTERNAL_TOKEN        the old edge and its internal token
//   CF_API_TOKEN, CF_ACCOUNT_ID     optional: list every object through the Cloudflare
//                                   API (Workers Scripts read); without them only --ids and
//                                   what the accounts name (phones, keys' phones, mailbox)
//   EDGE_SCRIPT                     the Worker whose namespaces to list (default vmate-edge)
//   CONVEX_SELF_HOSTED_URL, CONVEX_SELF_HOSTED_ADMIN_KEY
//                                   where to import; read from .env.local when unset
// No secret, sealed value or regno is printed: only counts.

import { ConvexHttpClient } from "convex/browser";
import { makeFunctionReference } from "convex/server";

declare const Bun: {
  argv: string[];
  file(path: string): { text(): Promise<string>; exists(): Promise<boolean> };
  write(path: string, data: string, options?: { mode?: number }): Promise<number>;
};
declare const process: { env: Record<string, string | undefined>; exit(code: number): never };

type Kind = "device" | "account" | "mailbox" | "oauth";
type Row = Record<string, string | number | null>;
type Exported = {
  kind: Kind;
  id?: string;
  objectId?: string;
  found: boolean;
  tables: Record<string, Row[]>;
  kv: Record<string, string | number | null>;
  omitted: Record<string, number>;
  regno?: string;
};

const CLASSES: Record<string, Kind> = { Device: "device", Account: "account", Mailbox: "mailbox", OAuth: "oauth" };
const ORDER: Kind[] = ["device", "account", "mailbox", "oauth"];
const BATCH = 25;
const PARALLEL = 8;

// ---- arguments and env ---------------------------------------------------------------

const args = Bun.argv.slice(2);
const flag = (name: string) => args.includes(name);
const option = (name: string) => {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
};
const dryRun = flag("--dry-run");

function fail(message: string): never {
  console.error(`backfill: ${message}`);
  process.exit(1);
}

async function localEnv(): Promise<Record<string, string>> {
  const file = Bun.file(new URL("../.env.local", import.meta.url).pathname);
  if (!(await file.exists())) return {};
  const out: Record<string, string> = {};
  for (const line of (await file.text()).split("\n")) {
    const at = line.indexOf("=");
    if (at > 0 && !line.trimStart().startsWith("#")) out[line.slice(0, at).trim()] = line.slice(at + 1).trim().replace(/^['"]|['"]$/g, "");
  }
  return out;
}

// ---- reading the edge -------------------------------------------------------------------

async function retrying<T>(what: string, run: () => Promise<T>): Promise<T> {
  for (let attempt = 1; ; attempt++) {
    try {
      return await run();
    } catch (e) {
      if (attempt >= 3) throw new Error(`${what}: ${e}`);
      await new Promise((resolve) => setTimeout(resolve, 500 * 2 ** attempt));
    }
  }
}

async function exportOne(edge: string, token: string, body: { kind: Kind; id?: string; objectId?: string }): Promise<Exported> {
  return retrying(`export ${body.kind}`, async () => {
    const res = await fetch(`${edge}/internal/export`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (res.status !== 200) throw new Error(`edge answered ${res.status}`);
    return (await res.json()) as Exported;
  });
}

/** Every object of the edge's namespaces that stores anything, as the Cloudflare API lists them. */
async function listed(apiToken: string, accountId: string, script: string): Promise<Array<{ kind: Kind; objectId: string }>> {
  const cf = async <T>(path: string): Promise<{ result: T; result_info?: { cursor?: string } }> =>
    retrying("Cloudflare API", async () => {
      const res = await fetch(`https://api.cloudflare.com/client/v4/accounts/${accountId}${path}`, { headers: { authorization: `Bearer ${apiToken}` } });
      if (res.status !== 200) throw new Error(`answered ${res.status}`);
      return res.json();
    });
  const namespaces = (await cf<Array<{ id: string; class: string; script: string }>>("/workers/durable_objects/namespaces")).result.filter(
    (ns) => ns.script === script && ns.class in CLASSES,
  );
  const out: Array<{ kind: Kind; objectId: string }> = [];
  for (const ns of namespaces) {
    let cursor: string | undefined;
    do {
      const page = await cf<Array<{ id: string; hasStoredData: boolean }>>(
        `/workers/durable_objects/namespaces/${ns.id}/objects?limit=1000${cursor ? `&cursor=${encodeURIComponent(cursor)}` : ""}`,
      );
      for (const object of page.result) if (object.hasStoredData) out.push({ kind: CLASSES[ns.class], objectId: object.id });
      cursor = page.result_info?.cursor || undefined;
    } while (cursor);
  }
  return out;
}

/** The object's own name, read from its rows, so one read by its object id and one read by
 * name are counted once. */
function nameOf(item: Exported): string | null {
  const first = (table: string) => item.tables[table]?.[0]?.id;
  const name = item.kind === "device" ? first("device") : item.kind === "account" ? first("account") : item.kind === "mailbox" ? first("mailbox") : "oauth";
  return typeof name === "string" ? name : (item.id ?? null);
}

async function pool<T>(items: T[], run: (item: T) => Promise<void>): Promise<void> {
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(PARALLEL, items.length) }, async () => {
      while (next < items.length) await run(items[next++]);
    }),
  );
}

async function readEdge(): Promise<Exported[]> {
  const edge = (process.env.EDGE_URL ?? "").replace(/\/+$/, "");
  const token = process.env.INTERNAL_TOKEN ?? "";
  if (!edge || !token) fail("EDGE_URL and INTERNAL_TOKEN are required (or --from).");
  const got = new Map<string, Exported>();
  const keep = (item: Exported) => {
    const name = nameOf(item);
    if (item.found && name !== null) got.set(`${item.kind}:${name}`, item);
  };
  const apiToken = process.env.CF_API_TOKEN;
  const cfAccount = process.env.CF_ACCOUNT_ID;
  if (apiToken && cfAccount) {
    const objects = await listed(apiToken, cfAccount, process.env.EDGE_SCRIPT ?? "vmate-edge");
    console.log(`listed ${objects.length} objects with data`);
    await pool(objects, async (o) => keep(await exportOne(edge, token, o)));
  } else {
    console.log("no CF_API_TOKEN/CF_ACCOUNT_ID: reading --ids, the OAuth object and what accounts name");
  }
  const named: Array<{ kind: Kind; id: string }> = [{ kind: "oauth", id: "oauth" }];
  const idsFile = option("--ids");
  if (idsFile) {
    const ids = JSON.parse(await Bun.file(idsFile).text()) as Partial<Record<Kind, string[]>>;
    for (const kind of ORDER) for (const id of ids[kind] ?? []) named.push({ kind, id });
  }
  const unread = (list: Array<{ kind: Kind; id: string }>) => list.filter((n) => !got.has(`${n.kind}:${n.id}`));
  await pool(unread(named), async (n) => keep(await exportOne(edge, token, n)));
  // The accounts name their phones, their keys' phones and their mailbox.
  const more: Array<{ kind: Kind; id: string }> = [];
  for (const item of got.values()) {
    if (item.kind !== "account") continue;
    for (const p of item.tables.phones ?? []) if (typeof p.device_id === "string") more.push({ kind: "device", id: p.device_id });
    for (const k of item.tables.keys ?? []) if (typeof k.device_id === "string") more.push({ kind: "device", id: k.device_id });
    const mailbox = item.tables.account?.[0]?.mailbox_id;
    if (typeof mailbox === "string") more.push({ kind: "mailbox", id: mailbox });
  }
  const seen = new Set<string>();
  await pool(
    unread(more).filter((n) => !seen.has(`${n.kind}:${n.id}`) && seen.add(`${n.kind}:${n.id}`)),
    async (n) => keep(await exportOne(edge, token, n)),
  );
  return [...got.values()];
}

// ---- counting ---------------------------------------------------------------------------

/** What the edge holds, by v3 table, named as the import's tally names them. */
function oldCounts(items: Exported[]): Record<string, number> {
  const out: Record<string, number> = {};
  const add = (key: string, n: number) => (out[key] = (out[key] ?? 0) + n);
  for (const item of items) {
    const t = item.tables;
    if (item.kind === "device") add("devices", t.device?.length ?? 0);
    if (item.kind === "account") {
      add("accounts", t.account?.length ?? 0);
      add("accessKeys", t.keys?.length ?? 0);
      add("signIns", t.sign_ins?.length ?? 0);
      add("phones", t.phones?.length ?? 0);
      add("v2Tokens", t.tokens?.length ?? 0);
    }
    if (item.kind === "mailbox") {
      add("mailboxes", t.mailbox?.length ?? 0);
      for (const row of t.mailbox ?? []) add("mailboxDevices", (JSON.parse(String(row.members ?? "[]")) as unknown[]).length);
    }
    if (item.kind === "oauth") add("oauthClients", t.clients?.length ?? 0);
    for (const [table, n] of Object.entries(item.omitted)) add(`not moved: ${item.kind}.${table}`, n);
  }
  return out;
}

const IMPORTED = new Set(["inserted", "updated", "merged", "linked", "deleted", "unlinked"]);

function summary(old: Record<string, number>, tally: Record<string, number>): void {
  const tables = new Set([...Object.keys(old).filter((k) => !k.startsWith("not moved")), ...Object.keys(tally).map((k) => k.split(".")[0])]);
  console.log("\ntable            old  imported  skipped   (outcomes)");
  for (const table of [...tables].sort()) {
    const outcomes = Object.entries(tally).filter(([k]) => k.split(".")[0] === table);
    const imported = outcomes.filter(([k]) => IMPORTED.has(k.split(".")[1])).reduce((n, [, c]) => n + c, 0);
    const skipped = outcomes.filter(([k]) => !IMPORTED.has(k.split(".")[1])).reduce((n, [, c]) => n + c, 0);
    const detail = outcomes.map(([k, c]) => `${k.split(".").slice(1).join(".")} ${c}`).join(", ");
    console.log(`${table.padEnd(16)} ${String(old[table] ?? "-").padStart(4)} ${String(imported).padStart(9)} ${String(skipped).padStart(8)}   ${detail}`);
  }
  for (const [k, n] of Object.entries(old).filter(([k]) => k.startsWith("not moved"))) console.log(`${k}: ${n}`);
}

// ---- main -------------------------------------------------------------------------------

const from = option("--from");
const items: Exported[] = from ? (JSON.parse(await Bun.file(from).text()) as Exported[]) : await readEdge();
const save = option("--save");
if (save) await Bun.write(save, JSON.stringify(items), { mode: 0o600 });

const old = oldCounts(items);
const perKind = Object.fromEntries(ORDER.map((kind) => [kind, items.filter((i) => i.kind === kind && i.found).length]));
console.log(`objects with data: ${ORDER.map((k) => `${k} ${perKind[k]}`).join(", ")}`);
if (dryRun) {
  for (const [k, n] of Object.entries(old)) console.log(`  ${k}: ${n}`);
  process.exit(0);
}

const local = await localEnv();
const url = process.env.CONVEX_SELF_HOSTED_URL ?? local.CONVEX_SELF_HOSTED_URL;
const adminKey = process.env.CONVEX_SELF_HOSTED_ADMIN_KEY ?? local.CONVEX_SELF_HOSTED_ADMIN_KEY;
if (!url || !adminKey) fail("CONVEX_SELF_HOSTED_URL and CONVEX_SELF_HOSTED_ADMIN_KEY are required (env or .env.local).");
const client = new ConvexHttpClient(url);
// Admin auth is what lets a client call internal functions, as `convex run` does; the typings
// leave the method out.
(client as unknown as { setAdminAuth(key: string): void }).setAdminAuth(adminKey);
const importBatch = makeFunctionReference<"mutation", { items: Exported[] }, Record<string, number>>("migrate:importBatch");

// A mailbox belongs to the account whose `mailbox_id` names it.
const regnoOf = new Map<string, string>();
for (const item of items) {
  const row = item.tables.account?.[0];
  if (item.kind === "account" && typeof row?.mailbox_id === "string" && typeof row.id === "string") regnoOf.set(row.mailbox_id, row.id);
}

const tally: Record<string, number> = {};
console.log(`importing into ${new URL(url).host}`);
for (const kind of ORDER) {
  const ofKind = items
    .filter((i) => i.kind === kind)
    .map((i) => {
      const mailbox = kind === "mailbox" ? i.tables.mailbox?.[0]?.id : undefined;
      const regno = typeof mailbox === "string" ? regnoOf.get(mailbox) : undefined;
      return regno !== undefined ? { ...i, regno } : i;
    });
  for (let at = 0; at < ofKind.length; at += BATCH) {
    const got = await retrying(`import ${kind}`, () => client.mutation(importBatch, { items: ofKind.slice(at, at + BATCH) }));
    for (const [k, n] of Object.entries(got)) tally[k] = (tally[k] ?? 0) + n;
  }
  console.log(`  ${kind}: ${ofKind.length} objects`);
}
summary(old, tally);
