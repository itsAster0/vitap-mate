import { afterEach, describe, expect, it, vi } from "vitest";
import { UNREACHABLE_MESSAGE, type KeyView, type LiveClient } from "./live";
import { watchAccount, watchPairing, type AccountLiveState } from "./popupLive";

afterEach(() => vi.useRealTimers());

const SAMPLE_KEY_VIEW: KeyView = {
  registrationNumber: "22BCE0001",
  keyLabel: "Laptop",
  keyCreatedAt: 1000,
  keyLastUsedAt: 2000,
  settings: { alwaysUseVault: false, phoneWaitSecs: 15, vaultTtlSecs: null },
  phoneLinked: true,
  phone: { label: "Pixel", notifications: true },
  savedCredentials: true,
  vaultSavedAt: 1500,
  credentialsRejectedAt: null,
  cachedSession: false,
};

function fakeLive() {
  let onData: ((v: unknown) => void) | undefined;
  let onErr: (() => void) | undefined;
  let unsubscribed = 0;
  let closed = 0;
  const client: LiveClient = {
    onUpdate: ((_q: unknown, _a: unknown, cb: (v: unknown) => void, err?: () => void) => {
      onData = cb;
      onErr = err;
      return () => {
        unsubscribed += 1;
      };
    }) as LiveClient["onUpdate"],
    close: async () => {
      closed += 1;
    },
  };
  return {
    client,
    push: (v: unknown) => onData?.(v),
    fail: () => onErr?.(),
    unsubscribed: () => unsubscribed,
    closed: () => closed,
  };
}

describe("watchAccount", () => {
  it("tracks ready, signed-in rejection, unreachable, and cleans up on stop", () => {
    vi.useFakeTimers();
    const fake = fakeLive();
    let setAuth: ((signedIn: boolean) => void) | undefined;
    const states: AccountLiveState[] = [];

    const stop = watchAccount(
      "https://vmate.aster0.dev",
      "vtm_key",
      (s) => states.push(s),
      (_base, _key, _onTrouble, onAuth) => {
        setAuth = onAuth;
        return fake.client;
      },
      5000,
    );

    expect(states.at(-1)).toEqual({ kind: "connecting" });

    // Null while signed out is ignored so a transient auth failure doesn't look revoked.
    fake.push(null);
    expect(states).toHaveLength(1);

    setAuth?.(true);
    fake.push(SAMPLE_KEY_VIEW);
    expect(states.at(-1)).toEqual({ kind: "ready", account: SAMPLE_KEY_VIEW });

    fake.push(null);
    expect(states.at(-1)).toEqual({ kind: "rejected" });

    stop();
    expect(fake.unsubscribed()).toBe(1);
    expect(fake.closed()).toBe(1);
  });

  it("marks unreachable after silent timeout", () => {
    vi.useFakeTimers();
    const fake = fakeLive();
    const states: AccountLiveState[] = [];

    const stop = watchAccount(
      "https://vmate.aster0.dev",
      "vtm_key",
      (s) => states.push(s),
      () => fake.client,
      2000,
    );

    vi.advanceTimersByTime(2000);
    expect(states.at(-1)).toEqual({ kind: "unreachable", account: undefined });
    stop();
  });
});

describe("watchPairing", () => {
  it("opens lazily, fires onReady, and closes cleanly on stop", () => {
    const fake = fakeLive();
    const opened: string[] = [];
    const onReady = vi.fn();
    const onError = vi.fn();

    const stop = watchPairing(
      "https://vmate.aster0.dev",
      "hash",
      { onReady, onError },
      (base) => {
        opened.push(base);
        return fake.client;
      },
      5000,
    );

    expect(opened).toEqual(["https://vmate.aster0.dev"]);
    fake.push({ ready: false });
    expect(onReady).not.toHaveBeenCalled();

    fake.push({ ready: true });
    expect(onReady).toHaveBeenCalledTimes(1);
    expect(onError).not.toHaveBeenCalled();

    stop();
    expect(fake.unsubscribed()).toBe(1);
    expect(fake.closed()).toBe(1);
  });

  it("reports UNREACHABLE_MESSAGE immediately when opening the socket throws", () => {
    const onReady = vi.fn();
    const onError = vi.fn();

    const stop = watchPairing(
      "https://vmate.aster0.dev",
      "hash",
      { onReady, onError },
      () => {
        throw new Error("socket unavailable");
      },
    );

    expect(onError).toHaveBeenCalledWith(UNREACHABLE_MESSAGE);
    expect(onReady).not.toHaveBeenCalled();
    stop();
  });
});
