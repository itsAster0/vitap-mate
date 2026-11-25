import { afterEach, describe, expect, it, vi } from "vitest";
import { PHONE_HINT, createBackground, createSerialQueue } from "./background";
import type { LiveClient, RequestView } from "./live";
import type { PageMessage, StateMessage } from "./messages";
import { LOGIN_STATE_KEY, type LoginState } from "./state";

const ACCESS_KEY = `vtm_${"ab".repeat(32)}`;
const VTOP_CONTENT_URL = "https://vtop.vitap.ac.in/vtop/content";

function deferred<T = void>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

type MockChromeOptions = {
  local?: Record<string, unknown>;
  cookies?: Partial<chrome.cookies.Cookie>[];
  notifyGranted?: boolean;
  onSessionSet?: (items: Record<string, unknown>) => Promise<void> | void;
};

function installMockChrome(opts: MockChromeOptions = {}) {
  const localStore = new Map<string, unknown>(Object.entries(opts.local ?? {}));
  const sessionStore = new Map<string, unknown>();
  let jar: chrome.cookies.Cookie[] = (opts.cookies ?? []).map((c) => ({
    domain: "vtop.vitap.ac.in",
    hostOnly: true,
    httpOnly: true,
    name: "JSESSIONID",
    path: "/vtop",
    sameSite: "unspecified",
    secure: true,
    session: true,
    storeId: "0",
    value: "old",
    ...c,
  }));

  const tabMessages: { tabId: number; message: PageMessage }[] = [];
  const broadcasts: StateMessage[] = [];
  const tabsUpdated: { tabId: number; url?: string }[] = [];
  const tabsCreated: { url?: string }[] = [];
  const notifications: { id: string; title: string; message: string }[] = [];
  const clearedNotifications: string[] = [];

  const mockChrome = {
    storage: {
      local: {
        get: vi.fn(async (keys: string | string[] | null) => {
          if (keys === null) return Object.fromEntries(localStore);
          const list = Array.isArray(keys) ? keys : [keys];
          return Object.fromEntries(
            list.filter((k) => localStore.has(k)).map((k) => [k, structuredClone(localStore.get(k))]),
          );
        }),
        set: vi.fn(async (items: Record<string, unknown>) => {
          for (const [k, v] of Object.entries(items)) localStore.set(k, structuredClone(v));
        }),
      },
      session: {
        get: vi.fn(async (key: string) =>
          sessionStore.has(key) ? { [key]: structuredClone(sessionStore.get(key)) } : {},
        ),
        set: vi.fn(async (items: Record<string, unknown>) => {
          await opts.onSessionSet?.(items);
          for (const [k, v] of Object.entries(items)) sessionStore.set(k, structuredClone(v));
        }),
      },
    },
    cookies: {
      getAll: vi.fn(async () => [...jar]),
      set: vi.fn(async (details: chrome.cookies.SetDetails) => {
        jar = jar.filter((c) => c.name !== details.name);
        const added: chrome.cookies.Cookie = {
          domain: details.domain ?? "vtop.vitap.ac.in",
          name: details.name ?? "",
          value: details.value ?? "",
          path: details.path ?? "/vtop",
          secure: details.secure ?? true,
          hostOnly: !details.domain,
          httpOnly: details.httpOnly ?? true,
          session: details.expirationDate === undefined,
          sameSite: details.sameSite ?? "unspecified",
          storeId: details.storeId ?? "0",
        };
        jar.push(added);
        return added;
      }),
      remove: vi.fn(async ({ name }: { name: string }) => {
        const before = jar.length;
        jar = jar.filter((c) => c.name !== name);
        return jar.length < before ? ({ name } as chrome.cookies.CookieDetails) : null;
      }),
    },
    tabs: {
      query: vi.fn(async () => [{ id: 7 }]),
      sendMessage: vi.fn(async (tabId: number, message: PageMessage) => {
        tabMessages.push({ tabId, message });
      }),
      update: vi.fn(async (tabId: number, props: { url?: string }) => {
        tabsUpdated.push({ tabId, url: props.url });
      }),
      create: vi.fn(async (props: { url?: string }) => {
        tabsCreated.push({ url: props.url });
      }),
    },
    runtime: {
      sendMessage: vi.fn(async (message: StateMessage) => {
        broadcasts.push(structuredClone(message));
      }),
      getPlatformInfo: vi.fn(async () => ({ os: "mac", arch: "arm64" })),
    },
    permissions: {
      contains: vi.fn(async () => opts.notifyGranted ?? false),
      onAdded: { addListener: vi.fn() },
    },
    notifications: {
      create: vi.fn((id: string, options: { title: string; message: string }) => {
        notifications.push({ id, title: options.title, message: options.message });
      }),
      clear: vi.fn(async (id: string) => {
        clearedNotifications.push(id);
        return true;
      }),
      onClicked: { addListener: vi.fn() },
    },
    action: {
      openPopup: vi.fn(async () => {}),
      setBadgeText: vi.fn(async () => {}),
      setBadgeBackgroundColor: vi.fn(async () => {}),
    },
  };

  vi.stubGlobal("chrome", mockChrome);

  return {
    mockChrome,
    localStore,
    sessionStore,
    tabMessages,
    broadcasts,
    tabsUpdated,
    tabsCreated,
    notifications,
    clearedNotifications,
    cookieValues: () => Object.fromEntries(jar.map((c) => [c.name, c.value])),
    persistedState: () => sessionStore.get(LOGIN_STATE_KEY) as LoginState | undefined,
  };
}

function responseWithUrl(body: unknown, init: ResponseInit & { url?: string } = {}) {
  const { url = VTOP_CONTENT_URL, ...rest } = init;
  const res = typeof body === "string" ? new Response(body, rest) : Response.json(body, rest);
  Object.defineProperty(res, "url", { value: url });
  return res;
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("createSerialQueue", () => {
  it("runs concurrent tasks strictly in FIFO order", async () => {
    const enqueue = createSerialQueue();
    const firstGate = deferred();
    const order: string[] = [];

    const first = enqueue(async () => {
      order.push("start:1");
      await firstGate.promise;
      order.push("end:1");
      return 1;
    });
    const second = enqueue(async () => {
      order.push("start:2");
      order.push("end:2");
      return 2;
    });

    await Promise.resolve();
    expect(order).toEqual(["start:1"]);

    firstGate.resolve();
    await expect(Promise.all([first, second])).resolves.toEqual([1, 2]);
    expect(order).toEqual(["start:1", "end:1", "start:2", "end:2"]);
  });

  it("continues processing queued tasks after a rejection", async () => {
    const enqueue = createSerialQueue();
    const bad = enqueue(async () => {
      throw new Error("boom");
    });
    const good = enqueue(async () => "ok");

    await expect(bad).rejects.toThrow("boom");
    await expect(good).resolves.toBe("ok");
  });
});

describe("report() serialization", () => {
  it("prevents concurrent progress updates from interleaving and persisting stale state", async () => {
    const firstWriteGate = deferred();
    let writeIndex = 0;
    const savedPhases: string[] = [];

    const env = installMockChrome({
      onSessionSet: async (items) => {
        writeIndex += 1;
        const snapshot = items[LOGIN_STATE_KEY] as LoginState;
        if (writeIndex === 1) {
          await firstWriteGate.promise;
        }
        savedPhases.push(`${snapshot.phase}:${snapshot.text}`);
      },
    });

    let tick = 1000;
    const bg = createBackground({ now: () => (tick += 100) });

    // Fire two updates concurrently; the first write is delayed while the second is already queued.
    const p1 = bg.report(7, "Waiting for phone", { phase: "phone", via: "phone" });
    const p2 = bg.report(7, "Server signing in", { phase: "server", via: "server" });

    firstWriteGate.resolve();
    await Promise.all([p1, p2]);

    expect(savedPhases).toEqual(["phone:Waiting for phone", "server:Server signing in"]);
    expect(env.persistedState()?.phase).toBe("server");
    expect(env.broadcasts.map((m) => m.state.phase)).toEqual(["phone", "server"]);
    expect(env.tabMessages.map((m) => (m.message.type === "VTOP_MATE_LOADING" ? m.message.text : ""))).toEqual([
      "Waiting for phone",
      "Server signing in",
    ]);
  });
});

describe("background login flow", () => {
  it("short-circuits when the browser already has a valid VTOP session", async () => {
    const env = installMockChrome({
      local: { token: ACCESS_KEY },
      cookies: [{ name: "JSESSIONID", value: "alive" }],
    });
    const fetchMock = vi.fn(async () => responseWithUrl("<html>ok</html>", { status: 200, url: VTOP_CONTENT_URL }));
    vi.stubGlobal("fetch", fetchMock);

    let tick = 1000;
    const bg = createBackground({ browserName: "Firefox", now: () => (tick += 100) });

    await bg.runLogin(7);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(env.tabsUpdated).toEqual([{ tabId: 7, url: VTOP_CONTENT_URL }]);
    const state = bg.getState();
    expect(state.phase).toBe("done");
    expect(state.phase === "done" && state.via).toBe("browser");
    expect(state.log.some((e) => e.text.includes("Firefox is still signed in"))).toBe(true);
    expect(env.localStore.get("lastLogin")).toMatchObject({ via: "browser" });
  });

  it("installs cookies from an immediate ready session and navigates to VTOP", async () => {
    const env = installMockChrome({ local: { token: ACCESS_KEY } });
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        return responseWithUrl({
          status: "ready",
          cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "fresh-1", path: "/vtop", hostOnly: true }],
          source: { from: "phone", cached: true, madeAt: "2026-10-18T10:00:00Z" },
        });
      }
      return responseWithUrl("<html>content</html>", { status: 200, url: VTOP_CONTENT_URL });
    });
    vi.stubGlobal("fetch", fetchMock);

    const bg = createBackground({ browserName: "Chrome" });
    await bg.runLogin(9);

    expect(env.cookieValues()).toEqual({ JSESSIONID: "fresh-1" });
    expect(env.tabsUpdated).toEqual([{ tabId: 9, url: VTOP_CONTENT_URL }]);
    expect(bg.getState().phase).toBe("done");
    const doneState = bg.getState();
    expect(doneState.phase === "done" && doneState.via).toBe("phone");
    expect(doneState.phase === "done" && doneState.source).toMatchObject({ from: "phone", cached: true });
    expect(env.tabMessages.at(-1)?.message).toEqual({
      type: "VTOP_MATE_LOADING",
      text: "Signed in",
      tone: "ok",
    });
  });

  it("follows a pending live request across phone -> server handover and notifies with browser name", async () => {
    const env = installMockChrome({
      local: { token: ACCESS_KEY, notify: true },
      notifyGranted: true,
    });
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        return responseWithUrl({ status: "pending", requestId: "req-1" }, { status: 202 });
      }
      if (url.endsWith("/v1/session/requests/req-1")) {
        return responseWithUrl({
          status: "ready",
          cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "srv-1", path: "/vtop", hostOnly: true }],
          source: { from: "server", cached: false },
        });
      }
      return responseWithUrl("<html>ok</html>", { status: 200, url: VTOP_CONTENT_URL });
    });
    vi.stubGlobal("fetch", fetchMock);

    let pushView!: (view: RequestView) => void;
    const subscribed = deferred();
    const fakeClient: LiveClient = {
      onUpdate: ((_q: unknown, _a: unknown, cb: (v: RequestView) => void) => {
        pushView = cb;
        subscribed.resolve();
        return () => {};
      }) as LiveClient["onUpdate"],
      close: async () => {},
    };

    const bg = createBackground({
      browserName: "Firefox",
      openLive: () => fakeClient,
    });

    const loginPromise = bg.runLogin(5);
    await subscribed.promise;

    pushView({
      status: "pending",
      stage: "phone",
      serverDueAt: Date.now() + 15_000,
      login: null,
      expiresAt: Date.now() + 120_000,
    });
    pushView({
      status: "pending",
      stage: "server",
      serverDueAt: null,
      login: "waiting_otp",
      expiresAt: Date.now() + 120_000,
    });
    pushView({ status: "ready", from: "server", expiresAt: Date.now() + 120_000 });

    await loginPromise;

    expect(env.notifications).toContainEqual({
      id: "phone",
      title: "Approve on your phone",
      message: "Firefox is asking for a VTOP session. Open VITAP Mate on your phone.",
    });
    expect(env.clearedNotifications).toContain("phone");
    expect(bg.getState().phase).toBe("done");
    expect(bg.getState().log.some((e) => e.text === "Your phone didn't answer; the server takes over")).toBe(true);
    expect(env.cookieValues()).toEqual({ JSESSIONID: "srv-1" });
  });

  it("waits on the phone without serverDueAt when storage is off", async () => {
    const env = installMockChrome({ local: { token: ACCESS_KEY } });
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        return responseWithUrl({ status: "pending", requestId: "req-off", stage: "phone", serverDueAt: null }, { status: 202 });
      }
      if (url.endsWith("/v1/session/requests/req-off")) {
        return responseWithUrl({
          status: "ready",
          cacheable: false,
          cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "p-1", path: "/vtop", hostOnly: true }],
          source: { from: "phone", cached: false },
        });
      }
      return responseWithUrl("<html>ok</html>", { status: 200, url: VTOP_CONTENT_URL });
    });
    vi.stubGlobal("fetch", fetchMock);

    let pushView!: (view: RequestView) => void;
    const subscribed = deferred();
    const fakeClient: LiveClient = {
      onUpdate: ((_q: unknown, _a: unknown, cb: (v: RequestView) => void) => {
        pushView = cb;
        subscribed.resolve();
        return () => {};
      }) as LiveClient["onUpdate"],
      close: async () => {},
    };

    const bg = createBackground({ browserName: "Chrome", openLive: () => fakeClient });
    const loginPromise = bg.runLogin(5);
    await subscribed.promise;

    pushView({
      status: "pending",
      stage: "phone",
      serverDueAt: null,
      pushSent: true,
      login: null,
      expiresAt: Date.now() + 120_000,
    });
    pushView({ status: "ready", from: "phone", expiresAt: Date.now() + 120_000 });

    await loginPromise;

    const phoneMsg = env.tabMessages.find(
      (m) => m.message.type === "VTOP_MATE_LOADING" && m.message.text === "Waiting for your phone…",
    );
    expect(phoneMsg).toBeDefined();
    expect(phoneMsg?.message.type === "VTOP_MATE_LOADING" && phoneMsg.message.serverDueAt).toBeUndefined();
    // It says to open the app, which asks at once; the push can be slow.
    expect(phoneMsg?.message.type === "VTOP_MATE_LOADING" && phoneMsg.message.hint).toBe(PHONE_HINT);
    expect(env.cookieValues()).toEqual({ JSESSIONID: "p-1" });
  });

  it("expires and retries once when VTOP bounces the first session", async () => {
    const env = installMockChrome({ local: { token: ACCESS_KEY } });
    let sessionCall = 0;
    let probeCall = 0;
    const expiredCalls: string[] = [];

    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        sessionCall += 1;
        return responseWithUrl({
          status: "ready",
          cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: `s-${sessionCall}`, path: "/vtop", hostOnly: true }],
          source: { from: "server", cached: sessionCall === 1 },
        });
      }
      if (url.endsWith("/v1/session/expire")) {
        expiredCalls.push(url);
        return responseWithUrl({ ok: true });
      }
      probeCall += 1;
      return probeCall === 1
        ? responseWithUrl("<html>login</html>", { status: 200, url: "https://vtop.vitap.ac.in/vtop/login" })
        : responseWithUrl("<html>content</html>", { status: 200, url: VTOP_CONTENT_URL });
    });
    vi.stubGlobal("fetch", fetchMock);

    const bg = createBackground();
    await bg.runLogin(3);

    expect(expiredCalls).toHaveLength(1);
    expect(sessionCall).toBe(2);
    expect(env.cookieValues()).toEqual({ JSESSIONID: "s-2" });
    expect(bg.getState().phase).toBe("done");
  });

  it("guards incoming messages, handles CANCEL_LOGIN, and reports pairing errors", async () => {
    const env = installMockChrome({ local: {} });
    const bg = createBackground();
    const sendResponse = vi.fn();

    // Unknown / malformed messages are ignored.
    expect(bg.handleMessage(null, {}, sendResponse)).toBeUndefined();
    expect(bg.handleMessage({ type: "BOGUS" }, {}, sendResponse)).toBeUndefined();
    expect(sendResponse).not.toHaveBeenCalled();

    // LOGIN_WITH_TOKEN without a saved key replies with ok: false and sends VTOP_MATE_ERROR to tab.
    const replyDone = deferred<unknown>();
    vi.spyOn(console, "error").mockImplementation(() => {});
    const isAsync = bg.handleMessage({ type: "LOGIN_WITH_TOKEN", source: "popup" }, { tab: { id: 12 } as chrome.tabs.Tab }, (res) => {
      replyDone.resolve(res);
    });
    expect(isAsync).toBe(true);
    await expect(replyDone.promise).resolves.toEqual({
      ok: false,
      error: "Pair the extension with VITAP Mate first.",
    });
    expect(env.tabMessages).toContainEqual({
      tabId: 12,
      message: { type: "VTOP_MATE_ERROR", text: "Pair the extension with VITAP Mate first." },
    });

    // CHECK_VTOP_SESSION works through the guard.
    const checkDone = deferred<unknown>();
    bg.handleMessage({ type: "CHECK_VTOP_SESSION" }, {}, (res) => checkDone.resolve(res));
    await expect(checkDone.promise).resolves.toEqual({ signedIn: false });
  });

  it("cancels an in-flight login on CANCEL_LOGIN without firing a failure notification", async () => {
    const env = installMockChrome({
      local: { token: ACCESS_KEY, notify: true },
      notifyGranted: true,
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => responseWithUrl({ status: "pending", requestId: "req-cancel" }, { status: 202 })),
    );

    const subscribed = deferred();
    const fakeClient = {
      onUpdate: () => {
        subscribed.resolve();
        return () => {};
      },
      close: async () => {},
    } as unknown as LiveClient;

    const bg = createBackground({ openLive: () => fakeClient });
    const loginPromise = bg.runLogin(4);
    await subscribed.promise;

    bg.handleMessage({ type: "CANCEL_LOGIN" }, {}, () => {});

    await expect(loginPromise).rejects.toThrow("Sign-in cancelled.");
    expect(bg.getState().phase).toBe("error");
    expect(bg.getState().log.at(-1)?.text).toBe("Cancelled");
    expect(env.notifications.find((n) => n.id === "failed")).toBeUndefined();
  });

  it("aborts POST /v1/session on CANCEL_LOGIN and hides the tab badge instead of showing an error", async () => {
    const env = installMockChrome({ local: { token: ACCESS_KEY } });
    const started = deferred<AbortSignal>();
    vi.stubGlobal(
      "fetch",
      vi.fn((_input: RequestInfo | URL, init?: RequestInit) => {
        started.resolve(init!.signal!);
        return new Promise<Response>((_res, rej) => {
          init?.signal?.addEventListener("abort", () => rej(new DOMException("Aborted", "AbortError")), {
            once: true,
          });
        });
      }),
    );

    const bg = createBackground();
    const reply = deferred<unknown>();
    bg.handleMessage({ type: "LOGIN_WITH_TOKEN", source: "content" }, { tab: { id: 8 } as chrome.tabs.Tab }, (res) => {
      reply.resolve(res);
    });

    const signal = await started.promise;
    bg.handleMessage({ type: "CANCEL_LOGIN" }, {}, () => {});

    expect(signal.aborted).toBe(true);
    await expect(reply.promise).resolves.toEqual({ ok: false, error: "Sign-in cancelled." });
    expect(env.tabMessages.at(-1)).toEqual({ tabId: 8, message: { type: "VTOP_MATE_HIDE_LOADING" } });
    expect(env.tabMessages.some((m) => m.message.type === "VTOP_MATE_ERROR")).toBe(false);
  });

  it("aborts browserSignedIn probe, expireSession, and skips goToVtopContent on CANCEL_LOGIN", async () => {
    // 1. Cancel while browserSignedIn probes VTOP: aborts the probe and never starts /v1/session.
    const env1 = installMockChrome({
      local: { token: ACCESS_KEY },
      cookies: [{ name: "JSESSIONID", value: "alive" }],
    });
    const probeStarted = deferred<AbortSignal>();
    const fetch1 = vi.fn((_input: RequestInfo | URL, init?: RequestInit) => {
      probeStarted.resolve(init!.signal!);
      return new Promise<Response>((_res, rej) => {
        init?.signal?.addEventListener("abort", () => rej(new DOMException("Aborted", "AbortError")), { once: true });
      });
    });
    vi.stubGlobal("fetch", fetch1);
    const bg1 = createBackground();
    const login1 = bg1.runLogin(6);
    const probeSignal = await probeStarted.promise;
    bg1.handleMessage({ type: "CANCEL_LOGIN" }, {}, () => {});
    expect(probeSignal.aborted).toBe(true);
    await expect(login1).rejects.toThrow("Sign-in cancelled.");
    expect(fetch1).toHaveBeenCalledTimes(1);
    expect(env1.tabsUpdated).toEqual([]);

    // 2. Cancel during expireSession after a bounced probe: aborts /v1/session/expire and does not retry.
    const env2 = installMockChrome({ local: { token: ACCESS_KEY } });
    const expireStarted = deferred<AbortSignal>();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const fetch2 = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        return Promise.resolve(
          responseWithUrl({
            status: "ready",
            cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "s-1", path: "/vtop", hostOnly: true }],
            source: { from: "server", cached: true },
          }),
        );
      }
      if (url.endsWith("/v1/session/expire")) {
        expireStarted.resolve(init!.signal!);
        return new Promise<Response>((_res, rej) => {
          init?.signal?.addEventListener("abort", () => rej(new DOMException("Aborted", "AbortError")), { once: true });
        });
      }
      return Promise.resolve(responseWithUrl("<html>login</html>", { status: 200, url: "https://vtop.vitap.ac.in/vtop/login" }));
    });
    vi.stubGlobal("fetch", fetch2);
    const bg2 = createBackground();
    const login2 = bg2.runLogin(7);
    const expireSignal = await expireStarted.promise;
    bg2.handleMessage({ type: "CANCEL_LOGIN" }, {}, () => {});
    expect(expireSignal.aborted).toBe(true);
    await expect(login2).rejects.toThrow("Sign-in cancelled.");
    expect(fetch2.mock.calls.filter(([u]) => String(u).endsWith("/v1/session"))).toHaveLength(1);
    expect(warn).not.toHaveBeenCalled();
    expect(env2.tabsUpdated).toEqual([]);

    // 3. Cancel right before goToVtopContent after cookies are verified: does not navigate the tab.
    const env3 = installMockChrome({ local: { token: ACCESS_KEY } });
    const bg3 = createBackground();
    const fetch3 = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/session")) {
        return responseWithUrl({
          status: "ready",
          cookies: [{ domain: "vtop.vitap.ac.in", name: "JSESSIONID", value: "s-ok", path: "/vtop", hostOnly: true }],
          source: { from: "phone", cached: false },
        });
      }
      // Cancel just as the post-install VTOP probe succeeds.
      bg3.handleMessage({ type: "CANCEL_LOGIN" }, {}, () => {});
      return responseWithUrl("<html>content</html>", { status: 200, url: VTOP_CONTENT_URL });
    });
    vi.stubGlobal("fetch", fetch3);
    await expect(bg3.runLogin(8)).rejects.toThrow("Sign-in cancelled.");
    expect(env3.tabsUpdated).toEqual([]);
    expect(env3.tabsCreated).toEqual([]);
  });

  it("handles OPEN_EXTENSION_PANEL safely, synchronously, bound to chrome.action, and without windowId on Firefox", async () => {
    const env = installMockChrome();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const bg = createBackground({ browserName: "Chrome" });

    // Missing openPopup (Chrome < 127): does not throw.
    (env.mockChrome.action as { openPopup?: unknown }).openPopup = undefined;
    expect(() =>
      bg.handleMessage({ type: "OPEN_EXTENSION_PANEL" }, { tab: { id: 1, windowId: 2 } as chrome.tabs.Tab }, () => {}),
    ).not.toThrow();

    // Must be called synchronously and bound to chrome.action; sync throw on options falls back synchronously.
    const calls: { thisOk: boolean; args: unknown[] }[] = [];
    const openPopup = vi.fn(function (this: unknown, ...args: unknown[]) {
      calls.push({ thisOk: this === env.mockChrome.action, args });
      if (args.length > 0) throw new TypeError("Unexpected options");
      return Promise.resolve();
    });
    env.mockChrome.action.openPopup = openPopup;
    expect(() =>
      bg.handleMessage({ type: "OPEN_EXTENSION_PANEL" }, { tab: { id: 1, windowId: 2 } as chrome.tabs.Tab }, () => {}),
    ).not.toThrow();
    // Both the primary call and the sync-throw fallback ran synchronously (no microtask deferral).
    expect(calls).toEqual([
      { thisOk: true, args: [{ windowId: 2 }] },
      { thisOk: true, args: [] },
    ]);
    expect(warn).not.toHaveBeenCalled();

    // Firefox calls openPopup() with no arguments directly, synchronously, bound to chrome.action.
    calls.length = 0;
    const ffBg = createBackground({ browserName: "Firefox" });
    ffBg.handleMessage({ type: "OPEN_EXTENSION_PANEL" }, { tab: { id: 1, windowId: 2 } as chrome.tabs.Tab }, () => {});
    expect(calls).toEqual([{ thisOk: true, args: [] }]);
    expect(warn).not.toHaveBeenCalled();
  });
});
