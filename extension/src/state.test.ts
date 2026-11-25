import { describe, expect, it } from "vitest";
import {
  INTERRUPTED_ERROR,
  LOGIN_STATE_KEY,
  createIdleState,
  getSessionStorage,
  isInProgressPhase,
  isLoginState,
  loadLoginState,
  normalizeRestoredLoginState,
  saveLoginState,
  type InProgressPhase,
  type LoginState,
  type SessionStorageApi,
} from "./state";

function fakeSessionStorage(initial: Record<string, unknown> = {}) {
  const store = new Map<string, unknown>(Object.entries(initial));
  let writes = 0;
  const api: SessionStorageApi = {
    async get(key) {
      return store.has(key) ? { [key]: structuredClone(store.get(key)) } : {};
    },
    async set(items) {
      writes += 1;
      for (const [k, v] of Object.entries(items)) {
        store.set(k, structuredClone(v));
      }
    },
  };
  return {
    api,
    read: () => store.get(LOGIN_STATE_KEY),
    writeCount: () => writes,
  };
}

describe("login state persistence and recovery", () => {
  it("persists and restores terminal done state intact", async () => {
    const storage = fakeSessionStorage();
    const done: LoginState = {
      phase: "done",
      text: "Opening VTOP…",
      via: "server",
      source: { from: "server", cached: true, madeAt: 1_700_000_000_000 },
      startedAt: 1000,
      finishedAt: 2500,
      log: [
        { at: 1000, text: "Checking Chrome's VTOP session…" },
        { at: 2500, text: "Signed in" },
      ],
    };

    await saveLoginState(done, storage.api);
    const restored = await loadLoginState(false, storage.api, 9000);

    expect(restored).toEqual(done);
    expect(storage.writeCount()).toBe(1);
  });

  it("persists and restores terminal error state intact", async () => {
    const storage = fakeSessionStorage();
    const failed: LoginState = {
      phase: "error",
      text: "Waiting for your phone…",
      via: "phone",
      startedAt: 1000,
      finishedAt: 6000,
      error: "Your phone did not answer in time.",
      failedPhase: "phone",
      log: [
        { at: 1000, text: "Waiting for your phone…" },
        { at: 6000, text: "Failed" },
      ],
    };

    await saveLoginState(failed, storage.api);
    const restored = await loadLoginState(false, storage.api, 9000);

    expect(restored).toEqual(failed);
    expect(storage.writeCount()).toBe(1);
  });

  it("restores idle state intact", async () => {
    const storage = fakeSessionStorage();
    const idle = createIdleState();

    await saveLoginState(idle, storage.api);
    const restored = await loadLoginState(false, storage.api, 9000);

    expect(restored).toEqual(idle);
    expect(storage.writeCount()).toBe(1);
  });

  it.each<InProgressPhase>(["check", "request", "phone", "server", "cookies", "open"])(
    "normalizes interrupted in-progress phase %s when no login is in flight",
    async (phase) => {
      expect(isInProgressPhase(phase)).toBe(true);
      const storage = fakeSessionStorage();
      const midLogin: LoginState =
        phase === "phone"
          ? {
              phase: "phone",
              text: "Working…",
              via: "phone",
              startedAt: 1000,
              deadline: 121_000,
              serverDueAt: 16_000,
              log: [{ at: 1000, text: "Working…" }],
            }
          : phase === "server"
            ? {
                phase: "server",
                text: "Working…",
                via: "server",
                startedAt: 1000,
                deadline: 121_000,
                loginStep: "waiting_otp",
                log: [{ at: 1000, text: "Working…" }],
              }
            : phase === "check"
              ? {
                  phase: "check",
                  text: "Working…",
                  startedAt: 1000,
                  log: [{ at: 1000, text: "Working…" }],
                }
              : {
                  phase,
                  text: "Working…",
                  via: "phone",
                  startedAt: 1000,
                  log: [{ at: 1000, text: "Working…" }],
                };

      await saveLoginState(midLogin, storage.api);
      const restored = await loadLoginState(false, storage.api, 5000);

      expect(restored).toEqual({
        phase: "error",
        text: INTERRUPTED_ERROR,
        via: phase === "check" ? undefined : phase === "server" ? "server" : "phone",
        source: undefined,
        handedOver: undefined,
        startedAt: 1000,
        finishedAt: 5000,
        error: INTERRUPTED_ERROR,
        failedPhase: phase,
        log: [
          { at: 1000, text: "Working…" },
          { at: 5000, text: "Interrupted", failed: true },
        ],
      });

      // Persisted back as terminal error, so a subsequent restart does not re-normalize.
      expect(storage.writeCount()).toBe(2);
      const restoredAgain = await loadLoginState(false, storage.api, 8000);
      expect(restoredAgain).toEqual(restored);
      expect(storage.writeCount()).toBe(2);
    },
  );

  it("rejects phase-invalid states in isLoginState", () => {
    expect(isLoginState({ phase: "error", text: "x", startedAt: 1, finishedAt: 2, log: [] })).toBe(false);
    expect(isLoginState({ phase: "done", text: "x", startedAt: 1, log: [] })).toBe(false);
    expect(isLoginState({ phase: "phone", text: "x", startedAt: 1, via: "server", log: [] })).toBe(false);
    expect(isLoginState({ phase: "server", text: "x", startedAt: 1, via: "server", loginStep: "bogus", log: [] })).toBe(
      false,
    );
  });

  it("keeps in-progress state intact when a login is currently in flight", () => {
    const active: LoginState = {
      phase: "phone",
      text: "Waiting for your phone…",
      via: "phone",
      startedAt: 1000,
      deadline: 121_000,
      log: [{ at: 1000, text: "Waiting for your phone…" }],
    };

    const result = normalizeRestoredLoginState(active, true, 5000);
    expect(result).toEqual({ state: active, normalized: false });
  });

  it("gracefully no-ops when chrome.storage.session is unavailable", async () => {
    expect(getSessionStorage()).toBeUndefined();
    await expect(saveLoginState(createIdleState(), undefined)).resolves.toBeUndefined();
    await expect(loadLoginState(false, undefined)).resolves.toBeNull();
  });

  it("handles failing or malformed session storage gracefully", async () => {
    const broken: SessionStorageApi = {
      async get() {
        throw new Error("storage unavailable");
      },
      async set() {
        throw new Error("quota exceeded");
      },
    };
    await expect(saveLoginState(createIdleState(), broken)).resolves.toBeUndefined();
    await expect(loadLoginState(false, broken)).resolves.toBeNull();

    const malformed = fakeSessionStorage({ [LOGIN_STATE_KEY]: { phase: "bogus", text: 123 } });
    await expect(loadLoginState(false, malformed.api)).resolves.toBeNull();
  });
});
