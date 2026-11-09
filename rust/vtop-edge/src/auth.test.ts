import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { checkSignature, deviceIdFor, signed, signedMessage } from "./auth";
import type { Env } from "./common";
import type { DeviceInfo } from "./device";
import worker from "./index";
import { toBase64Url } from "./shared/base64";

const EDGE = "https://vmate.aster0.dev";
const T = 1_800_000_000;

// vtop-shared's `device_sig` test vector.
const VECTOR_SEED = "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA";
const VECTOR_PUBLIC = "ebVWLo_mVPlAeLES6KmLp5AfhTrmlb7X4OORC60ElmQ";
const VECTOR_SIG = "t56VLtrjIqH8PQQHcZZIoPNs43L0LSjOwAWk4dYtKYTCuwDHxsIGtfb6LhQGuJ3_uUAmXBSVs7enz4oqtPWRCw";

const seed = Uint8Array.from({ length: 32 }, (_, i) => i + 1);
const signKey = ed25519.getPublicKey(seed);
const sealKey = toBase64Url(x25519.keygen().publicKey);
const deviceId = deviceIdFor(signKey);

/** A request signed as the app signs it. */
function request(
  method: string,
  pathWithQuery: string,
  body: string | null,
  opts: { time?: number; device?: string | null; sig?: (s: string) => string; signedBody?: string } = {},
): Request {
  const time = String(opts.time ?? T);
  const bytes = new TextEncoder().encode(opts.signedBody ?? body ?? "");
  const sig = toBase64Url(ed25519.sign(signedMessage(method, pathWithQuery, time, bytes), seed));
  const headers: Record<string, string> = { "x-vm-time": time, "x-vm-sig": opts.sig ? opts.sig(sig) : sig };
  if (opts.device !== null) headers["x-vm-device"] = opts.device ?? deviceId;
  return new Request(`${EDGE}${pathWithQuery}`, { method, headers, body });
}

/** Devices kept in a map, as the `Device` objects would. */
function devices() {
  const stored = new Map<string, DeviceInfo>();
  const DEVICE = {
    getByName: (id: string) => ({
      info: async () => stored.get(id) ?? null,
      register: async (info: Omit<DeviceInfo, "fcmToken"> & { fcmToken: string | null }) => {
        const before = stored.get(id);
        stored.set(id, { ...info, fcmToken: info.fcmToken ?? before?.fcmToken ?? "" });
        return before === undefined;
      },
      update: async (changes: Partial<DeviceInfo>) => {
        stored.set(id, { ...stored.get(id)!, ...changes });
      },
    }),
  };
  return { stored, env: { DEVICE } as unknown as Env };
}

function registered() {
  const rig = devices();
  rig.stored.set(deviceId, {
    id: deviceId,
    signKey: toBase64Url(signKey),
    sealKey,
    label: null,
    platform: "android",
    fcmToken: "",
  });
  return rig;
}

async function code(response: Response | object): Promise<unknown> {
  return response instanceof Response ? ((await response.json()) as { code: string }).code : "ok";
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T * 1000);
});
afterEach(() => vi.useRealTimers());

describe("device signatures", () => {
  it("match vtop-shared's test vector", async () => {
    expect(toBase64Url(seed)).toBe(VECTOR_SEED);
    expect(toBase64Url(signKey)).toBe(VECTOR_PUBLIC);
    const body = new TextEncoder().encode('{"a":1}');
    const req = (sig: string) =>
      new Request(`${EDGE}/v1/devices/register`, { method: "POST", headers: { "x-vm-time": String(T), "x-vm-sig": sig } });
    expect(await checkSignature(req(VECTOR_SIG), body, signKey)).toBeNull();
    // Padding on input is accepted too.
    expect(await checkSignature(req(`${VECTOR_SIG}==`), body, signKey)).toBeNull();
    expect(await checkSignature(req(VECTOR_SIG), new TextEncoder().encode('{"a":2}'), signKey)).not.toBeNull();
  });

  it("hash an empty body as SHA-256 of nothing", () => {
    const message = new TextDecoder().decode(signedMessage("get", "/v1/events?after=3", String(T), new Uint8Array()));
    expect(message).toBe(
      `vm1\nGET\n/v1/events?after=3\n${T}\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`,
    );
  });

  it("accept a valid request, with its query", async () => {
    const { env } = registered();
    const auth = await signed(request("POST", "/v1/app/status?x=1", '{"registrationNumber":"22BCE0001"}'), env);
    expect(await code(auth)).toBe("ok");
    expect(auth).toMatchObject({ device: { id: deviceId }, json: { registrationNumber: "22BCE0001" } });
    expect(await code(await signed(request("GET", "/v1/events?after=3", null), env))).toBe("ok");
  });

  it("reject a tampered body, path or signature", async () => {
    const { env } = registered();
    const tampered = request("POST", "/v1/app/status", '{"registrationNumber":"22BCE0002"}', {
      signedBody: '{"registrationNumber":"22BCE0001"}',
    });
    const response = (await signed(tampered, env)) as Response;
    expect(response.status).toBe(401);
    expect(await code(response)).toBe("bad_signature");
    const sig = (s: string) => (s[0] === "A" ? "B" : "A") + s.slice(1);
    expect(await code(await signed(request("POST", "/v1/app/status", "{}", { sig }), env))).toBe("bad_signature");
    expect(await code(await signed(request("POST", "/v1/app/status", "{}", { sig: () => "" }), env))).toBe(
      "bad_signature",
    );
  });

  it("reject a stale time, saying the server's", async () => {
    const { env } = registered();
    expect(await code(await signed(request("GET", "/v1/udp", null, { time: T - 120 }), env))).toBe("ok");
    const response = (await signed(request("GET", "/v1/udp", null, { time: T - 121 }), env)) as Response;
    expect(response.status).toBe(401);
    expect(await response.json()).toMatchObject({ code: "stale_time", serverTime: T });
    expect(await code(await signed(request("GET", "/v1/udp", null, { time: T + 121 }), env))).toBe("stale_time");
  });

  it("reject an unknown device", async () => {
    const { env } = devices();
    const response = (await signed(request("GET", "/v1/udp", null), env)) as Response;
    expect(response.status).toBe(404);
    expect(await code(response)).toBe("unknown_device");
    expect(await code(await signed(request("GET", "/v1/udp", null, { device: "nope" }), env))).toBe("unknown_device");
    expect(await code(await signed(request("GET", "/v1/udp", null, { device: null }), env))).toBe("bad_signature");
  });
});

describe("POST /v1/devices/register", () => {
  const body = (label: string, fcmToken?: string) =>
    JSON.stringify({ signKey: toBase64Url(signKey), sealKey, label, platform: "android", fcmToken });

  it("is signed by the key it registers, and idempotent per key", async () => {
    const { env, stored } = devices();
    const first = await worker.fetch(request("POST", "/v1/devices/register", body("Pixel", "fcm1"), { device: null }), env);
    expect(first.status).toBe(201);
    expect(await first.json()).toEqual({ deviceId });

    const again = await worker.fetch(request("POST", "/v1/devices/register", body("Pixel 8"), { device: null }), env);
    expect(again.status).toBe(200);
    expect(await again.json()).toEqual({ deviceId });
    expect(stored.size).toBe(1);
    expect(stored.get(deviceId)).toMatchObject({ label: "Pixel 8", fcmToken: "fcm1", sealKey, platform: "android" });
  });

  it("refuses a body not signed by that key", async () => {
    const { env, stored } = devices();
    const other = ed25519.getPublicKey(new Uint8Array(32).fill(9));
    const forged = JSON.stringify({ signKey: toBase64Url(other), sealKey, label: "x", platform: "android" });
    const response = await worker.fetch(request("POST", "/v1/devices/register", forged, { device: null }), env);
    expect(response.status).toBe(401);
    expect(stored.size).toBe(0);
  });

  it("then lets the device update itself", async () => {
    const { env, stored } = devices();
    await worker.fetch(request("POST", "/v1/devices/register", body("Pixel"), { device: null }), env);
    const response = await worker.fetch(request("POST", "/v1/devices/update", JSON.stringify({ fcmToken: "fcm2" })), env);
    expect(response.status).toBe(204);
    expect(stored.get(deviceId)?.fcmToken).toBe("fcm2");
  });
});

describe("/internal/* and /v1/udp", () => {
  const key = toBase64Url(x25519.keygen().publicKey);
  const report = { host: "140.245.225.49", ports: [3478, 51820], keys: [{ id: "k1", publicKey: key, from: T, until: T + 604800 }] };

  function rig() {
    const { env, stored } = registered();
    let udp: string | null = null;
    const INDEX = { getByName: () => ({ get: async () => udp, put: async (v: string) => void (udp = v) }) };
    return { env: { ...env, INDEX, INTERNAL_TOKEN: "tok" } as unknown as Env, stored };
  }
  const post = (token: string) =>
    new Request(`${EDGE}/internal/udp-keys`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}` },
      body: JSON.stringify(report),
    });

  it("need the internal token", async () => {
    const { env } = rig();
    expect((await worker.fetch(post("wrong"), env)).status).toBe(401);
    expect((await worker.fetch(post(""), { ...env, INTERNAL_TOKEN: "" } as Env)).status).toBe(401);
  });

  it("serve the gateway's latest report to signed phones", async () => {
    const { env } = rig();
    expect((await worker.fetch(request("GET", "/v1/udp", null), env)).status).toBe(503);
    expect((await worker.fetch(post("tok"), env)).status).toBe(204);
    const response = await worker.fetch(request("GET", "/v1/udp", null), env);
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual(report);
    const unsigned = await worker.fetch(new Request(`${EDGE}/v1/udp`), env);
    expect(unsigned.status).toBe(401);
  });
});

describe("POST /v1/devices/delete and /info", () => {
  function rig() {
    const { env, stored } = registered();
    const calls: string[] = [];
    const base = (env as unknown as { DEVICE: { getByName: (id: string) => Record<string, unknown> } }).DEVICE;
    const DEVICE = {
      getByName: (id: string) => ({
        ...base.getByName(id),
        summary: async () => ({ label: "Pixel", platform: "android", pushToken: true, createdAt: 5, events: 2, latest: 7 }),
        remove: async () => void stored.delete(id),
      }),
    };
    let members = [deviceId, "other"].map((d) => ({ deviceId: d, sealed: null, sealedAt: null, addedAt: T, seenAt: T }));
    const MAILBOX = {
      getByName: (id: string) => ({
        doc: async () => ({ id, ownerHash: null, members, createdAt: T, updatedAt: T }),
        save: async (doc: { members: typeof members }) => void (members = doc.members),
      }),
    };
    const ACCOUNT = { getByName: (id: string) => ({ forgetPhone: async (d: string) => void calls.push(`${id}:${d}`) }) };
    const all = { ...env, DEVICE, MAILBOX, ACCOUNT } as unknown as Env;
    return { env: all, stored, calls, members: () => members.map((m) => m.deviceId) };
  }

  it("shows the install, without its keys or token", async () => {
    const { env } = rig();
    const response = await worker.fetch(request("POST", "/v1/devices/info", "{}"), env);
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ deviceId, label: "Pixel", platform: "android", pushToken: true, createdAt: 5, events: 2, latest: 7 });
  });

  it("forgets the install, its phone entry and its place on the address", async () => {
    const { env, stored, calls, members } = rig();
    const body = JSON.stringify({ registrationNumber: "22bce0001", mailbox: "k3f9x2ab7q" });
    const response = await worker.fetch(request("POST", "/v1/devices/delete", body), env);
    expect(response.status).toBe(204);
    expect(stored.has(deviceId)).toBe(false);
    expect(calls).toEqual([`22BCE0001:${deviceId}`]);
    expect(members()).toEqual(["other"]);
    // Gone: the next signed call must register again.
    const after = await worker.fetch(request("POST", "/v1/devices/info", "{}"), env);
    expect(await code(after)).toBe("unknown_device");
  });

  it("needs only the signature", async () => {
    const { env, stored, calls } = rig();
    expect((await worker.fetch(request("POST", "/v1/devices/delete", "{}"), env)).status).toBe(204);
    expect(stored.size).toBe(0);
    expect(calls).toEqual([]);
  });
});
