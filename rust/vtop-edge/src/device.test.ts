import { describe, expect, it } from "vitest";

import type { Env } from "./common";
import { Device } from "./device";
import { memorySql } from "../test/memory-sql";

/** A `Device` object over in-memory SQLite, with storage that can be wiped. */
function deviceObject() {
  let db = memorySql();
  let alarm: number | null = null;
  const closed: string[] = [];
  const storage = {
    get sql() {
      return db;
    },
    getAlarm: async () => alarm,
    setAlarm: async (at: number) => void (alarm = at),
    deleteAlarm: async () => void (alarm = null),
    deleteAll: async () => void (db = memorySql()),
  };
  const socket = { close: (_code: number, reason: string) => void closed.push(reason), send: () => {} };
  const ctx = { storage, getWebSockets: () => [socket], setWebSocketAutoResponse: () => {} };
  (globalThis as Record<string, unknown>).WebSocketRequestResponsePair ??= class {};
  const device = new Device(ctx as unknown as DurableObjectState, {} as Env);
  return { device, closed, alarm: () => alarm };
}

describe("Device", () => {
  it("sums itself up, then forgets everything when deleted", async () => {
    const rig = deviceObject();
    await rig.device.register({ id: "d1", signKey: "k", sealKey: "s", label: "Pixel", platform: "android", fcmToken: null });
    await rig.device.append({ kind: "credentials_rejected" });
    expect(await rig.device.summary()).toMatchObject({ label: "Pixel", pushToken: false, events: 1, latest: 1 });
    expect(rig.alarm()).not.toBeNull();

    await rig.device.remove();
    expect(rig.closed).toEqual(["deleted"]);
    expect(rig.alarm()).toBeNull();
    expect(await rig.device.info()).toBeNull();
    expect(await rig.device.summary()).toBeNull();
    // Registering again starts afresh.
    expect(await rig.device.register({ id: "d1", signKey: "k", sealKey: "s", label: null, platform: "android", fcmToken: null })).toBe(true);
    expect((await rig.device.summary())?.latest).toBe(0);
  });
});
