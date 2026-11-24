import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { memorySql } from "../test/memory-sql";
import { prepareAccountDb, saveAccount } from "./account-db";
import { AccountRecord } from "./account-state";
import type { Env } from "./common";
import { TABLES, dump, exportObject, frozen } from "./export";
import worker from "./index";

const NOW = 1_800_000_000;
const tables = (db: SqlStorage) =>
  db
    .exec<{ name: string }>("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
    .toArray()
    .map((r) => r.name);

function accountDb(): SqlStorage {
  const db = memorySql();
  prepareAccountDb(db);
  const a = new AccountRecord("22BCE0001", NOW);
  a.addPhone({ deviceId: "d".repeat(32), label: "Pixel", addedAt: NOW, seenAt: NOW + 5 });
  a.addKey("00abcdefgh", "laptop", "d".repeat(32), NOW);
  a.saveVault("sealed-vault", NOW);
  saveAccount(db, a);
  db.exec("INSERT INTO owner (hash) VALUES (?)", "owner-hash");
  db.exec(
    "INSERT INTO requests (id, status, cookies, created_at, expires_at, vault_started, phone_asked, phones) VALUES ('r1', 'success', 'JSESSIONID=secret', ?, ?, 0, 0, '')",
    NOW,
    NOW + 600,
  );
  return db;
}

describe("dump", () => {
  it("exports the kept tables as stored and only counts the short-lived ones", () => {
    const db = accountDb();
    const out = dump(db, TABLES.account.keep, TABLES.account.count);
    expect(out.found).toBe(true);
    expect(out.tables.account[0]).toMatchObject({ id: "22BCE0001", sealed_vault: "sealed-vault" });
    expect(out.tables.keys).toEqual([expect.objectContaining({ hash: "00abcdefgh", label: "laptop" })]);
    expect(out.tables.phones).toEqual([expect.objectContaining({ device_id: "d".repeat(32), seen_at: NOW + 5 })]);
    expect(out.tables.owner).toEqual([{ hash: "owner-hash" }]);
    // The answered cookies never leave.
    expect(out.tables.requests).toBeUndefined();
    expect(JSON.stringify(out)).not.toContain("JSESSIONID");
    expect(out.omitted).toEqual({ requests: 1, pending: 0 });
  });

  it("changes nothing in an object that stores nothing", () => {
    const db = memorySql();
    const out = dump(db, TABLES.device.keep, TABLES.device.count);
    expect(out).toEqual({ found: false, tables: {}, kv: {}, omitted: {} });
    expect(tables(db)).toEqual([]);
  });
});

describe("POST /internal/export", () => {
  const stored = { found: true, tables: { device: [{ id: "x" }] }, kv: {}, omitted: {} };
  const exportState = vi.fn(async () => stored);
  const asked: string[] = [];
  const ns = {
    idFromName: (name: string) => `name:${name}`,
    idFromString: (hex: string) => `hex:${hex}`,
    get: (id: string) => (asked.push(id), { exportState }),
  };
  const env = { INTERNAL_TOKEN: "internal", DEVICE: ns, ACCOUNT: ns, MAILBOX: ns, OAUTH: ns } as unknown as Env;
  const call = (body: unknown, token = "internal") =>
    exportObject(
      new Request("https://edge.test/internal/export", {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
        body: JSON.stringify(body),
      }),
      env,
    );

  beforeEach(() => {
    asked.length = 0;
  });

  it("needs the internal token", async () => {
    expect((await call({ kind: "device", id: "a".repeat(32) }, "nope")).status).toBe(401);
  });

  it("reads an object by its name or by its object id", async () => {
    const byName = await call({ kind: "device", id: "a".repeat(32) });
    expect(byName.status).toBe(200);
    expect(await byName.json()).toEqual({ kind: "device", id: "a".repeat(32), ...stored });
    const hex = "f".repeat(64);
    expect(await (await call({ kind: "account", objectId: hex })).json()).toMatchObject({ kind: "account", objectId: hex });
    expect(await (await call({ kind: "oauth" })).json()).toMatchObject({ id: "oauth" });
    expect(asked).toEqual([`name:${"a".repeat(32)}`, `hex:${hex}`, "name:oauth"]);
  });

  it("refuses names and ids that are not of the kind", async () => {
    for (const body of [
      { kind: "index", id: "x" },
      { kind: "device", id: "22BCE0001" },
      { kind: "account", id: "lower" },
      { kind: "mailbox", objectId: "abc" },
      { kind: "device", id: "a".repeat(32), objectId: "f".repeat(64) },
      { kind: "device" },
    ]) {
      expect((await call(body)).status, JSON.stringify(body)).toBe(400);
    }
    expect(asked).toEqual([]);
  });

  it("has nothing stored for the FCM token cache", async () => {
    expect(await (await call({ kind: "fcm" })).json()).toEqual({ kind: "fcm", found: false, tables: {}, kv: {}, omitted: {} });
  });
});

describe("FREEZE", () => {
  const on = { FREEZE: "1" } as unknown as Env;

  it("is off unless set", () => {
    expect(frozen({} as Env, "POST", "/v1/keys")).toBeNull();
  });

  it("stops the writes", () => {
    for (const [method, path] of [
      ["POST", "/v1/keys"],
      ["POST", "/v1/devices/register"],
      ["POST", "/v1/mail-otp/register"],
      ["POST", "/v1/account/settings"],
      ["POST", "/v1/session"],
      ["POST", "/register"],
      ["GET", "/v1/account/prove"],
    ]) {
      expect(frozen(on, method, path)?.status, `${method} ${path}`).toBe(503);
    }
  });

  it("keeps reads, mail, the proxy and the internal routes", () => {
    for (const [method, path] of [
      ["GET", "/healthz"],
      ["GET", "/v1/events"],
      ["GET", "/v1/whoami"],
      ["POST", "/v1/devices/info"],
      ["POST", "/v1/mail-otp/inbox"],
      ["POST", "/v1/vtop/fetch"],
      ["POST", "/internal/export"],
      ["POST", "/internal/mail-otp/await"],
      ["OPTIONS", "/v1/keys"],
    ]) {
      expect(frozen(on, method, path), `${method} ${path}`).toBeNull();
    }
  });

  describe("in the Worker", () => {
    beforeEach(() => {
      vi.stubGlobal("fetch", async () => new Response("{}", { status: 200 }));
    });
    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it("answers 503 frozen before any object is touched", async () => {
      const env = { FREEZE: "1", ALLOWED_ORIGINS: "" } as unknown as Env;
      const res = await worker.fetch(new Request("https://edge.test/v1/keys", { method: "POST", body: "{}" }), env);
      expect(res.status).toBe(503);
      expect(res.headers.get("retry-after")).toBe("60");
      expect(await res.json()).toMatchObject({ code: "frozen" });
    });
  });
});
