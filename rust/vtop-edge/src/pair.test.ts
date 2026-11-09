import { describe, expect, it } from "vitest";

import { accountObject } from "../test/account-harness";
import { memorySql } from "../test/memory-sql";
import { AccountRecord } from "./account-state";
import type { Env } from "./common";
import { claim, pairCode } from "./pair";
import { Index } from "./store";

/** `Index` objects over in-memory SQLite, by name, with their alarms. */
function indexes() {
  const objects = new Map<string, Index>();
  const alarms = new Map<string, number>();
  const INDEX = {
    getByName(name: string) {
      let object = objects.get(name);
      if (!object) {
        const storage = { sql: memorySql(), setAlarm: async (at: number) => void alarms.set(name, at) };
        object = new Index({ storage } as unknown as DurableObjectState, {} as Env);
        objects.set(name, object);
      }
      return object;
    },
  };
  // No MCP sign-in waits on these codes (oauth.ts).
  const OAUTH = { getByName: () => ({ pending: async () => null }) };
  return { INDEX, OAUTH, alarms, env: { INDEX } as unknown as Env };
}

describe("pairing codes", () => {
  it("reads codes as typed", () => {
    expect(pairCode("abcde-12345")).toBe("ABCDE12345");
    expect(pairCode(" ABCDE 12345 ")).toBe("ABCDE12345");
    expect(pairCode("oil0011111")).toBe("0110011111");
    expect(pairCode("ABCDE1234")).toBeNull();
    expect(pairCode("ABCDE1234U")).toBeNull();
    expect(pairCode(undefined)).toBeNull();
  });
});

describe("POST /v1/pair", () => {
  it("is pending until the app makes the key, then hands it over once", async () => {
    const { INDEX, OAUTH, alarms, env } = indexes();
    const rig = accountObject(undefined, { INDEX, OAUTH, PUBLIC_URL: "https://vmate.example" });
    const at = Math.floor(Date.now() / 1000);
    const doc = new AccountRecord("22BCE0001", at);
    doc.addPhone({ deviceId: "d1", label: null, addedAt: at, seenAt: at });

    const pending = await claim({ code: "ABCDE-12345" }, env);
    expect(pending.status).toBe(202);
    expect(await pending.json()).toMatchObject({ status: "pending" });

    const made = (await rig.inner.linkedOp("create-key", doc, 0, { label: "Chrome", pairCode: "abcde12345" }, at)) as Response;
    expect(made.status).toBe(201);
    const { key } = (await made.json()) as { key: string };
    expect([...alarms.values()]).toContain((at + 300) * 1000);

    const ready = await claim({ code: "ABCDE12345" }, env);
    expect(ready.status).toBe(200);
    expect(await ready.json()).toEqual({ status: "ready", key });
    expect((await claim({ code: "ABCDE12345" }, env)).status).toBe(202);
  });

  it("refuses bad codes", async () => {
    const { INDEX, env } = indexes();
    expect((await claim({ code: "nope" }, env)).status).toBe(400);
    expect((await claim(null, env)).status).toBe(400);
    const rig = accountObject(undefined, { INDEX });
    const at = Math.floor(Date.now() / 1000);
    const doc = new AccountRecord("22BCE0001", at);
    doc.addPhone({ deviceId: "d1", label: null, addedAt: at, seenAt: at });
    const made = (await rig.inner.linkedOp("create-key", doc, 0, { pairCode: "x" }, at)) as Response;
    expect(made.status).toBe(400);
    expect(doc.keys).toHaveLength(0);
  });

  it("forgets the key when the alarm fires", async () => {
    const { INDEX } = indexes();
    const index = INDEX.getByName("pair:x");
    await index.putUntil("vtm_k", Date.now() + 1000);
    await index.alarm();
    expect(await index.take()).toBeNull();
  });
});
