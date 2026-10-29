import { describe, expect, it } from "vitest";
import type { Env } from "./common";
import { type UdpKey, newKey, nextMondayIst, rotated } from "./udp-gateway";

const WEEK = 7 * 24 * 3600;
// Wed 29 Oct 2026 16:00 UTC; its week ends Mon 2 Nov 2026 00:00 IST.
const WED = Date.UTC(2026, 9, 29, 16) / 1000;
const MONDAY = Date.UTC(2026, 10, 1, 18, 30) / 1000;

let made = 0;
const make = async (until: number): Promise<UdpKey> => ({ privateKey: `p${++made}`, publicKey: `k${made}`, until });

describe("udp gateway keys", () => {
  it("expire every phone's copy at the same Monday 00:00 IST", () => {
    expect(nextMondayIst(WED)).toBe(MONDAY);
    expect(nextMondayIst(MONDAY)).toBe(MONDAY + WEEK);
  });

  it("keep one key for the week, then rotate and keep the old one a week", async () => {
    const first = await rotated([], WED, make);
    expect(first.map((k) => k.until)).toEqual([MONDAY]);
    expect(await rotated(first, WED + 3600, make)).toBe(first);

    const second = await rotated(first, MONDAY, make);
    expect(second.map((k) => k.until)).toEqual([MONDAY + WEEK, MONDAY]);

    const third = await rotated(second, MONDAY + WEEK, make);
    expect(third.map((k) => k.until)).toEqual([MONDAY + 2 * WEEK, MONDAY + WEEK]);
  });

  it("drop a key that ended over a week ago", async () => {
    const old = await rotated([], WED, make);
    expect((await rotated(old, MONDAY + WEEK + 1, make)).map((k) => k.until)).toEqual([MONDAY + 2 * WEEK]);
  });

  it("make X25519 key pairs as hex", async () => {
    const key = await newKey(MONDAY);
    expect(key.privateKey).toMatch(/^[0-9a-f]{64}$/);
    expect(key.publicKey).toMatch(/^[0-9a-f]{64}$/);
    expect(key.publicKey).not.toBe((await newKey(MONDAY)).publicKey);
  });
});

describe("internal udp keys", () => {
  it("seal the keys to the gateway, for the engine only", async () => {
    const { open } = await import("./shared/mail-seal");
    const { udpKeys, SEALED_FOR } = await import("./udp-gateway");
    const { x25519 } = await import("@noble/curves/ed25519.js");
    const { toBase64Url } = await import("./shared/base64");
    const secretBytes = x25519.utils.randomSecretKey();
    const secret = toBase64Url(secretBytes);
    const publicKey = toBase64Url(x25519.getPublicKey(secretBytes));
    const key = await newKey(MONDAY);
    const env = {
      BRIDGE_SECRET: "relay",
      GATEWAY_IDENTITY_KEY: publicKey,
      UDP_KEYS: { getByName: () => ({ keys: async () => [key] }) },
    } as unknown as Env;
    const ask = (auth: string) =>
      udpKeys(new Request("https://edge/internal/udp-keys", { headers: { authorization: `Bearer ${auth}` } }), env);

    expect((await ask("wrong")).status).toBe(401);
    const { sealed } = (await (await ask("relay")).json()) as { sealed: string };
    const opened = open(secret, SEALED_FOR, sealed);
    expect(JSON.parse(new TextDecoder().decode(opened!))).toEqual({ keys: [key.privateKey] });
    expect(open(secret, "mail", sealed)).toBeNull();
  });
});
