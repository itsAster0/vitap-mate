import { describe, expect, it } from "vitest";

import { memorySql } from "../test/memory-sql";
import { loadAccount, prepareAccountDb, saveAccount, SCHEMA } from "./account";
import { AccountRecord } from "./account-state";
import * as sql from "./sql";
import { hash } from "./shared/tokens";

const NOW = 1_800_000_000;

function linked(): AccountRecord {
  const a = new AccountRecord("22BCE0001", NOW);
  a.addPhone({ deviceId: "d1", secretHash: hash("vta_x"), label: null, addedAt: NOW, seenAt: NOW });
  a.addKey("00abcdefgh", "laptop", "d1", NOW);
  a.saveVault("sealed", NOW);
  return a;
}

describe("account storage", () => {
  it("keeps sign-ins and key uses across a reload", () => {
    const db = memorySql();
    prepareAccountDb(db);
    const a = linked();
    a.recordSignIn(NOW, "server", true);
    a.touchKey("00abcdefgh", NOW, "mcp");
    saveAccount(db, a);

    const b = loadAccount(db)!;
    expect(b.signIns).toEqual([{ at: NOW, by: "server", ok: true, reason: null }]);
    expect(b.keys[0].uses).toEqual([{ at: NOW, via: "mcp" }]);

    b.recordSignIn(NOW + 5, "phone", false, "no");
    b.touchKey("00abcdefgh", NOW + 60, "extension");
    saveAccount(db, b);
    const c = loadAccount(db)!;
    expect(c.signIns.map((s) => s.at)).toEqual([NOW + 5, NOW]);
    expect(c.keys[0].uses.map((u) => u.via)).toEqual(["extension", "mcp"]);
  });

  it("adds the Gmail column to accounts stored before it", () => {
    const db = memorySql();
    prepareAccountDb(db);
    saveAccount(db, linked());
    // As an account stored before the column was added.
    db.exec("ALTER TABLE account DROP COLUMN vault_gmail");
    prepareAccountDb(db);
    const a = loadAccount(db)!;
    expect(a.sealedVault).toBe("sealed");
    expect(a.vaultGmail).toBe(false);
    a.saveVault("sealed", NOW, true);
    saveAccount(db, a);
    expect(loadAccount(db)!.vaultGmail).toBe(true);
    // Preparing again leaves it be.
    prepareAccountDb(db);
    expect(loadAccount(db)!.vaultGmail).toBe(true);
  });

  it("keeps a key's outing permission, and adds the column to keys stored before it", () => {
    const db = memorySql();
    prepareAccountDb(db);
    saveAccount(db, linked());
    db.exec("ALTER TABLE keys DROP COLUMN outings");
    prepareAccountDb(db);
    const a = loadAccount(db)!;
    expect(a.keys[0].outings).toBe(false);
    a.setKeyOutings("00abcdef", true);
    saveAccount(db, a);
    const b = loadAccount(db)!;
    expect(b.keys[0].outings).toBe(true);
    b.setKeyOutings("00abcdef", false);
    saveAccount(db, b);
    expect(loadAccount(db)!.keys[0].outings).toBe(false);
  });

  it("a forgotten password stays forgotten after a reload", () => {
    const db = memorySql();
    prepareAccountDb(db);
    saveAccount(db, linked());

    const b = loadAccount(db)!;
    b.clearVault();
    saveAccount(db, b);

    const c = loadAccount(db)!;
    expect(c.usableVault(NOW)).toBeNull();
    expect(c.statusView(null).vaultSavedAt).toBeNull();
  });

  it("an account stored before these tables existed loads and saves", () => {
    const db = memorySql();
    sql.prepare(db, SCHEMA);
    db.exec(
      "INSERT INTO account (id, always_use_vault, phone_wait_secs, sealed_vault, vault_saved_at, created_at, updated_at, window_start, window_hits) VALUES ('22BCE0001', 0, 10, 'sealed', ?, ?, ?, 0, 0)",
      NOW,
      NOW,
      NOW,
    );
    db.exec("INSERT INTO keys (hash, label, created_at, device_id) VALUES ('00abcdefgh', 'laptop', ?, 'd1')", NOW);
    db.exec("INSERT INTO phones (device_id, secret_hash, added_at, seen_at) VALUES ('d1', 'h', ?, ?)", NOW, NOW);

    prepareAccountDb(db);
    const a = loadAccount(db)!;
    expect(a.keys.map((k) => k.label)).toEqual(["laptop"]);
    expect(a.phones).toHaveLength(1);
    expect(a.sealedVault).toBe("sealed");
    expect(a.signIns).toEqual([]);

    a.recordSignIn(NOW, "server", true);
    saveAccount(db, a);
    expect(loadAccount(db)!.signIns).toHaveLength(1);
    expect(loadAccount(db)!.keys).toHaveLength(1);
  });
});
