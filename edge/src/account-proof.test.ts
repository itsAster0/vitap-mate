import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { accountObject } from "../test/account-harness";
import { memorySql } from "../test/memory-sql";
import { type Socket, parseVerdict, proofReply, relay } from "./account-proof";
import { type AccountClaims, TOKEN_HEADER, TOKEN_TTL_SECS, verifyToken } from "./account-token";
import { deviceIdFor, signedMessage } from "./auth";
import type { Env } from "./common";
import { Device, type DeviceInfo, PROOF_TTL_SECS } from "./device";
import worker from "./index";
import { fromBase64Url, toBase64Url } from "./shared/base64";
import type { ProofVerdict } from "./shared/engine-api";

const T = 1_800_000_000;
const SECRET = "test-secret";

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(T * 1000);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

/** A registered `Device` object over in-memory SQLite and key-value storage. */
async function deviceObject(id = "d1") {
  const kv = new Map<string, unknown>();
  const storage = {
    sql: memorySql(),
    kv: {
      get: (key: string) => kv.get(key),
      put: (key: string, value: unknown) => void kv.set(key, structuredClone(value)),
      delete: (key: string) => kv.delete(key),
    },
  };
  const ctx = { storage, getWebSockets: () => [], setWebSocketAutoResponse: () => {} };
  (globalThis as Record<string, unknown>).WebSocketRequestResponsePair ??= class {};
  const device = new Device(ctx as unknown as DurableObjectState, {} as Env);
  await device.register({ id, signKey: "k", sealKey: "s", label: null, platform: "android", fcmToken: null });
  return device;
}

/** A challenge from `device`, failing the test when it gives none. */
async function started(device: Device): Promise<{ challenge: string; expiresAt: number }> {
  const proof = await device.startProof();
  if (typeof proof === "string") throw new Error(`no challenge: ${proof}`);
  return proof;
}

/** Moves the fake clock on by `secs`. */
function later(secs: number) {
  vi.setSystemTime(Date.now() + secs * 1000);
}

function claimsOf(token: string): AccountClaims {
  return JSON.parse(new TextDecoder().decode(fromBase64Url(token.split(".")[1])!)) as AccountClaims;
}

describe("proof challenges", () => {
  it("are 128-bit hex, good once", async () => {
    const device = await deviceObject();
    const { challenge, expiresAt } = (await started(device));
    expect(challenge).toMatch(/^[0-9a-f]{32}$/);
    expect(expiresAt).toBe(T + PROOF_TTL_SECS);
    expect(await device.takeProof(challenge)).toBe(true);
    expect(await device.takeProof(challenge)).toBe(false);
  });

  it("expire after two minutes", async () => {
    const device = await deviceObject();
    const { challenge } = (await started(device));
    vi.setSystemTime((T + PROOF_TTL_SECS) * 1000);
    expect(await device.takeProof(challenge)).toBe(false);
  });

  it("are used up by a wrong try, replaced by a new one, and one device's only", async () => {
    const device = await deviceObject();
    const first = (await started(device)).challenge;
    expect(await device.takeProof("0".repeat(32))).toBe(false);
    expect(await device.takeProof(first)).toBe(false);
    later(5);
    const old = (await started(device)).challenge;
    later(5);
    const fresh = (await started(device)).challenge;
    expect(fresh).not.toBe(old);
    expect(await device.takeProof(old)).toBe(false);
    // Another install's object never saw it.
    const other = await deviceObject("d2");
    later(5);
    const mine = (await started(device)).challenge;
    expect(await other.takeProof(mine)).toBe(false);
    expect(await device.takeProof(mine)).toBe(true);
  });

  it("need a registered install", async () => {
    const kv = { get: () => undefined, put: () => {}, delete: () => false };
    const ctx = { storage: { sql: memorySql(), kv }, getWebSockets: () => [], setWebSocketAutoResponse: () => {} };
    const device = new Device(ctx as unknown as DurableObjectState, {} as Env);
    expect(await device.startProof()).toBe("unknown");
  });

  it("are rate limited per install, since each holds an engine slot", async () => {
    const device = await deviceObject();
    expect(typeof (await device.startProof())).toBe("object");
    expect(await device.startProof()).toBe("too_many");
    for (let i = 1; i < 20; i++) {
      later(5);
      expect(typeof (await device.startProof())).toBe("object");
    }
    later(5);
    expect(await device.startProof()).toBe("too_many");
    // An hour after the first, one more.
    vi.setSystemTime((T + 3600) * 1000);
    expect(typeof (await device.startProof())).toBe("object");
  });

  it("let only one of two sockets use a challenge", async () => {
    const device = await deviceObject();
    const { challenge } = await started(device);
    const both = await Promise.all([device.takeProof(challenge), device.takeProof(challenge)]);
    expect(both.filter(Boolean)).toHaveLength(1);
  });
});

const ok: ProofVerdict = { status: "ok", registrationNumber: "22bce0001", ownerHash: "oh1" };

/** A real `Account` for 22BCE0001 with phone d1, its Index entries in a map. */
function accounts() {
  const index = new Map<string, string>();
  const INDEX = {
    getByName: (name: string) => ({
      get: async () => index.get(name) ?? null,
      put: async (value: string) => void index.set(name, value),
    }),
  };
  const { account } = accountObject(undefined, { ACCOUNT_TOKEN_SECRET: SECRET, INDEX });
  const env = {
    ACCOUNT_TOKEN_SECRET: SECRET,
    INDEX,
    ACCOUNT: { getByName: (name: string) => (name === "22BCE0001" ? account : null) },
  } as unknown as Env;
  return { env, account, index };
}

describe("a proof's verdict", () => {
  it("mints the account token as a cookie check does", async () => {
    const { env, account, index } = accounts();
    const reply = await proofReply(env, "d1", ok);
    expect(reply).toMatchObject({ registrationNumber: "22BCE0001", expiresAt: (T + TOKEN_TTL_SECS) * 1000 });
    const token = reply.accountToken as string;
    expect(claimsOf(token)).toEqual({ sub: "22BCE0001", dev: "d1", iat: T, exp: T + TOKEN_TTL_SECS });
    expect(verifyToken(env, token, "d1", T + 60)).toMatchObject({ sub: "22BCE0001" });
    expect(await account.tokenOwner("d1", T)).toBe("oh1");
    expect(index.get("student:oh1")).toBe("22BCE0001");
    // The token works on the account routes as one from a cookie check.
    const claims = claimsOf(token);
    const device: DeviceInfo = { id: "d1", signKey: "", sealKey: "", label: null, platform: "android", fcmToken: "" };
    const view = await account.app("account", { checked: null, token: claims, mailboxId: null, body: {}, device });
    expect(await view.json()).toMatchObject({ registrationNumber: "22BCE0001" });
    expect(view.headers.has(TOKEN_HEADER)).toBe(false);
  });

  it("replaces the device's earlier token, cookie or proof", async () => {
    const { env, account } = accounts();
    const checked = { status: "valid" as const, registrationNumber: "22BCE0001", ownerHash: "oh1" };
    const device: DeviceInfo = { id: "d1", signKey: "", sealKey: "", label: null, platform: "android", fcmToken: "" };
    const byCookie = await account.app("account", { checked, mailboxId: null, body: {}, device });
    const old = claimsOf(byCookie.headers.get(TOKEN_HEADER)!);
    vi.setSystemTime((T + 5) * 1000);
    const fresh = claimsOf((await proofReply(env, "d1", ok)).accountToken as string);
    expect(await account.tokenOwner("d1", old.iat)).toBeNull();
    expect(await account.tokenOwner("d1", fresh.iat)).toBe("oh1");
  });

  it("mints nothing without a proof or a secret", async () => {
    const { env } = accounts();
    expect(await proofReply(env, "d1", { status: "failed", reason: "VTOP answered 302" })).toMatchObject({
      code: "proof_failed",
      error: expect.stringContaining("302"),
    });
    expect(await proofReply(env, "d1", null)).toMatchObject({ code: "proof_failed" });
    const { account } = accountObject(undefined, { INDEX: env.INDEX });
    const noSecret = { ACCOUNT: { getByName: () => account } } as unknown as Env;
    expect(await proofReply(noSecret, "d1", ok)).toMatchObject({ code: "token_unavailable" });
  });

  it("is read strictly", () => {
    expect(parseVerdict(JSON.stringify(ok))).toEqual(ok);
    expect(parseVerdict('{"status":"failed","reason":"x"}')).toEqual({ status: "failed", reason: "x" });
    for (const bad of ["", "{", '{"status":"ok"}', '{"status":"ok","registrationNumber":1,"ownerHash":"o"}', "[]"]) {
      expect(parseVerdict(bad), bad).toBeNull();
    }
  });
});

/** A socket that records what it was sent and lets the test fire its events. */
function fakeSocket() {
  const listeners: Record<string, Array<(e: { data: unknown }) => void>> = {};
  const socket = {
    sent: [] as unknown[],
    closed: null as number | null,
    send: (data: unknown) => void socket.sent.push(data),
    close: (code?: number) => void (socket.closed ??= code ?? 1000),
    addEventListener: (type: string, fn: (e: { data: unknown }) => void) => void (listeners[type] ??= []).push(fn),
    fire: (type: string, data?: unknown) => {
      for (const fn of listeners[type] ?? []) fn({ data });
    },
  };
  return socket;
}

describe("the relay", () => {
  const bytes = (n: number) => new Uint8Array([n]).buffer;

  it("passes binary both ways and gives the phone the reply, never the verdict", async () => {
    const phone = fakeSocket();
    const engine = fakeSocket();
    const verdicts: Array<ProofVerdict | null> = [];
    relay(phone as unknown as Socket, engine as unknown as Socket, async (v) => {
      verdicts.push(v);
      return { accountToken: "tok", expiresAt: 1 };
    });
    phone.fire("message", bytes(1));
    engine.fire("message", bytes(2));
    expect(engine.sent).toEqual([bytes(1)]);
    expect(phone.sent).toEqual([bytes(2)]);
    engine.fire("message", JSON.stringify(ok));
    await vi.waitFor(() => expect(phone.closed).toBe(1000));
    expect(verdicts).toEqual([ok]);
    expect(phone.sent.slice(1)).toEqual([JSON.stringify({ accountToken: "tok", expiresAt: 1 })]);
    expect(engine.closed).toBe(1000);
    // Nothing more goes either way.
    phone.fire("message", bytes(3));
    expect(engine.sent).toHaveLength(1);
  });

  it("ends both on text from the phone or either side closing", async () => {
    for (const [from, type, data] of [
      ["phone", "message", "{}"],
      ["phone", "close", undefined],
      ["engine", "close", undefined],
      ["engine", "error", undefined],
    ] as const) {
      const phone = fakeSocket();
      const engine = fakeSocket();
      let replied = false;
      relay(phone as unknown as Socket, engine as unknown as Socket, async () => {
        replied = true;
        return {};
      });
      (from === "phone" ? phone : engine).fire(type, data);
      expect([phone.closed, engine.closed].every((c) => c !== null), `${from} ${type}`).toBe(true);
      expect(engine.sent).toEqual([]);
      expect(replied).toBe(false);
    }
  });

  it("closes both when a send throws, even while the token is minted", async () => {
    // A send to a socket that already closed throws in workerd.
    const phone = fakeSocket();
    const engine = fakeSocket();
    engine.send = () => {
      throw new Error("closed");
    };
    relay(phone as unknown as Socket, engine as unknown as Socket, async () => ({}));
    phone.fire("message", bytes(1));
    expect([phone.closed, engine.closed]).toEqual([1011, 1011]);

    // The phone goes while the reply is pending: no unhandled rejection, both closed.
    const gone = fakeSocket();
    const engine2 = fakeSocket();
    let finish: (answer: Record<string, unknown>) => void = () => {};
    relay(gone as unknown as Socket, engine2 as unknown as Socket, () => new Promise((done) => (finish = done)));
    engine2.fire("message", JSON.stringify(ok));
    gone.send = () => {
      throw new Error("closed");
    };
    finish({ accountToken: "tok" });
    await vi.waitFor(() => expect(engine2.closed).toBe(1000));

    // And a reply that fails still answers and closes.
    const phone3 = fakeSocket();
    const engine3 = fakeSocket();
    relay(phone3 as unknown as Socket, engine3 as unknown as Socket, async () => {
      throw new Error("db down");
    });
    engine3.fire("message", JSON.stringify(ok));
    await vi.waitFor(() => expect(phone3.closed).toBe(1000));
    expect(String(phone3.sent[0])).toContain("internal_error");
  });
});

// Through the Worker, signed as the app signs.
const seed = Uint8Array.from({ length: 32 }, (_, i) => i + 1);
const signKey = ed25519.getPublicKey(seed);
const deviceId = deviceIdFor(signKey);

function signed(method: string, pathWithQuery: string, body = "", headers: Record<string, string> = {}): Request {
  const time = String(T);
  const sig = toBase64Url(ed25519.sign(signedMessage(method, pathWithQuery, time, new TextEncoder().encode(body)), seed));
  return new Request(`https://vmate.aster0.dev${pathWithQuery}`, {
    method,
    headers: { "x-vm-time": time, "x-vm-sig": sig, "x-vm-device": deviceId, ...headers },
    body: method === "GET" ? undefined : body,
  });
}

async function workerEnv() {
  const info: DeviceInfo = {
    id: deviceId,
    signKey: toBase64Url(signKey),
    sealKey: toBase64Url(x25519.keygen().publicKey),
    label: null,
    platform: "android",
    fcmToken: "",
  };
  const device = await deviceObject(deviceId);
  const engine: string[] = [];
  vi.stubGlobal("fetch", async (url: string) => {
    engine.push(url);
    return Response.json({ code: "proof_busy" }, { status: 503 });
  });
  const env = {
    ENGINE_URL: "http://engine",
    INTERNAL_TOKEN: "s",
    DEVICE: { getByName: (id: string) => (id === deviceId ? Object.assign(device, { info: async () => info }) : null) },
  } as unknown as Env;
  return { env, engine, device };
}

describe("the proof routes", () => {
  it("hand out a challenge, then use it up before asking the engine", async () => {
    const { env, engine } = await workerEnv();
    const start = await worker.fetch(signed("POST", "/v1/account/prove/start", "{}"), env);
    expect(start.status).toBe(200);
    const { challenge, expiresAt } = (await start.json()) as { challenge: string; expiresAt: number };
    expect(expiresAt).toBe((T + PROOF_TTL_SECS) * 1000);
    const path = `/v1/account/prove?challenge=${challenge}`;
    const ws = { upgrade: "websocket" };

    const plain = await worker.fetch(signed("GET", path), env);
    expect(plain.status).toBe(426);
    // The engine is busy: the challenge was used all the same.
    const busy = await worker.fetch(signed("GET", path, "", ws), env);
    expect(busy.status).toBe(503);
    expect(((await busy.json()) as { code: string }).code).toBe("proof_busy");
    expect(engine).toEqual([`http://engine/engine/proof?device=${deviceId}&challenge=${challenge}`]);
    const again = await worker.fetch(signed("GET", path, "", ws), env);
    expect(again.status).toBe(401);
    expect(((await again.json()) as { code: string }).code).toBe("challenge_invalid");
    expect(engine).toHaveLength(1);
  });

  it("refuse an unsigned or another device's request", async () => {
    const { env, device, engine } = await workerEnv();
    const { challenge } = (await started(device));
    const path = `/v1/account/prove?challenge=${challenge}`;
    const forged = signed("GET", path, "", { upgrade: "websocket", "x-vm-sig": "A".repeat(86) });
    expect((await worker.fetch(forged, env)).status).toBe(401);
    // Signed for another query: the signature covers it.
    const other = signed("GET", "/v1/account/prove?challenge=0", "", { upgrade: "websocket" });
    const swapped = new Request(`https://vmate.aster0.dev${path}`, other);
    expect((await worker.fetch(swapped, env)).status).toBe(401);
    expect(engine).toEqual([]);
    expect(await device.takeProof(challenge)).toBe(true);
  });
});
