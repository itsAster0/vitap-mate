import { describe, expect, it } from "vitest";

import { memorySql } from "../test/memory-sql";
import { loadAccount, prepareAccountDb, saveAccount } from "./account";
import { AccountRecord } from "./account-state";

const NOW = 1_800_000_000;

function linked(): AccountRecord {
  const a = new AccountRecord("22BCE0001", NOW);
  a.addPhone({ deviceId: "d1", label: null, addedAt: NOW, seenAt: NOW });
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

  it("keeps a key's outing permission and Gmail access", () => {
    const db = memorySql();
    prepareAccountDb(db);
    const first = linked();
    first.saveVault("sealed", NOW, true);
    saveAccount(db, first);
    const a = loadAccount(db)!;
    expect(a.vaultGmail).toBe(true);
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
});
