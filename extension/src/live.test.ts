import { afterEach, describe, expect, it, vi } from "vitest";
import { LoginError, UNKNOWN_KEY_MESSAGE, type LoginErrorKind } from "./bridge";
import {
  CANCELLED,
  UNREACHABLE_MESSAGE,
  pairCodeHash,
  parseRelease,
  releaseFromGitHub,
  updateAvailable,
  waitForRequest,
  type LiveClient,
  type Release,
  type RequestView,
} from "./live";

afterEach(() => vi.useRealTimers());

/** A client whose one subscription the test drives by hand. */
function fakeClient() {
  let push: ((view: RequestView) => void) | undefined;
  let fail: (() => void) | undefined;
  let stopped = 0;
  const client = {
    onUpdate: (_q: unknown, _a: unknown, onUpdate: (v: RequestView) => void, onError?: () => void) => {
      push = onUpdate;
      fail = onError;
      return () => void stopped++;
    },
    close: async () => {},
  } as unknown as LiveClient;
  return { client, push: (v: RequestView) => push!(v), fail: () => fail!(), stopped: () => stopped };
}

const pending = (over: Partial<Extract<RequestView, { status: "pending" }>> = {}): RequestView => ({
  status: "pending",
  stage: "phone",
  serverDueAt: null,
  login: null,
  expiresAt: Date.now() + 60_000,
  ...over,
});

describe("waitForRequest", () => {
  it("reports each step and resolves when ready", async () => {
    const f = fakeClient();
    const seen: string[] = [];
    const wait = waitForRequest(f.client, "r", new AbortController().signal, (v) => seen.push(`${v.stage}:${v.login}`));
    f.push(pending());
    f.push(pending({ stage: "server", login: "waiting_otp" }));
    f.push({ status: "ready", from: "server", expiresAt: 0 });
    await expect(wait).resolves.toMatchObject({ status: "ready", from: "server" });
    expect(seen).toEqual(["phone:null", "server:waiting_otp"]);
    expect(f.stopped()).toBe(1);
  });

  it("rejects with typed LoginError kinds and messages", async () => {
    for (const [view, kind, message] of [
      [{ status: "error", code: "x", error: "phone_timeout: Open the app.", expiresAt: 0 }, "failed", "Open the app."],
      [{ status: "key_unknown" }, "key_unknown", UNKNOWN_KEY_MESSAGE],
      [{ status: "not_found" }, "expired", /did not answer/],
    ] as [RequestView, LoginErrorKind, string | RegExp][]) {
      const f = fakeClient();
      const wait = waitForRequest(f.client, "r", new AbortController().signal, () => {});
      f.push(view);
      await expect(wait).rejects.toThrow(LoginError);
      await expect(wait).rejects.toMatchObject({ kind });
      await expect(wait).rejects.toThrow(message);
    }
    const f = fakeClient();
    const wait = waitForRequest(f.client, "r", new AbortController().signal, () => {});
    f.fail();
    await expect(wait).rejects.toMatchObject({ kind: "unreachable", message: UNREACHABLE_MESSAGE });
  });

  it("gives up on a silent socket, at expiry, and when cancelled", async () => {
    vi.useFakeTimers();
    const silent = waitForRequest(fakeClient().client, "r", new AbortController().signal, () => {}, 1000);
    vi.advanceTimersByTime(1000);
    await expect(silent).rejects.toMatchObject({ kind: "unreachable", message: UNREACHABLE_MESSAGE });

    const f = fakeClient();
    const expiring = waitForRequest(f.client, "r", new AbortController().signal, () => {});
    f.push(pending({ expiresAt: Date.now() + 5000 }));
    vi.advanceTimersByTime(5000);
    await expect(expiring).rejects.toMatchObject({ kind: "expired" });

    const abort = new AbortController();
    const cancelled = waitForRequest(fakeClient().client, "r", abort.signal, () => {});
    abort.abort();
    await expect(cancelled).rejects.toMatchObject({ kind: "cancelled", message: CANCELLED });

    // Already-aborted signal rejects at once without subscribing.
    const preAborted = new AbortController();
    preAborted.abort();
    const onUpdate = vi.fn();
    const unusedClient = { onUpdate, close: async () => {} } as unknown as LiveClient;
    await expect(waitForRequest(unusedClient, "r", preAborted.signal, () => {})).rejects.toMatchObject({
      kind: "cancelled",
      message: CANCELLED,
    });
    expect(onUpdate).not.toHaveBeenCalled();
  });
});

describe("pairCodeHash", () => {
  it("hashes the code as keys:create does: no dash, upper case", async () => {
    const hash = await pairCodeHash("abcde-fghjk");
    expect(hash).toBe(await pairCodeHash("ABCDEFGHJK"));
    // sha256("ABCDEFGHJK")
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("updateAvailable", () => {
  const release: Release = {
    version: "3.0.1",
    sha: "8086105",
    subject: null,
    url: "u",
    zipUrl: null,
    xpiUrl: null,
    publishedAt: 2000,
    checkedAt: 2000,
  };

  it("is true for another commit published after this build", () => {
    expect(updateAvailable(release, { sha: "aaaaaaaaaaaa", at: 1000 })).toBe(true);
  });

  it("is false for the same commit, an older release, no build sha or no release", () => {
    expect(updateAvailable(release, { sha: "8086105abcde", at: 1000 })).toBe(false);
    expect(updateAvailable(release, { sha: "aaaaaaaaaaaa", at: 3000 })).toBe(false);
    expect(updateAvailable(release, { sha: "", at: 1000 })).toBe(false);
    expect(updateAvailable(null, { sha: "aaaaaaaaaaaa", at: 1000 })).toBe(false);
  });
});

describe("the newest build from GitHub", () => {
  const github = {
    name: "Browser extension 3.0.1 (8086105)",
    body: "Built from `8086105`.\n\n**feat(extension): live sign-in**",
    html_url: "https://github.com/o/r/releases/tag/extension-latest",
    published_at: "2026-10-18T16:10:04Z",
    assets: [{ name: "extension.zip", browser_download_url: "https://x/extension.zip", updated_at: "2026-11-24T10:00:00Z" }],
  };

  it("reads CI's release", () => {
    expect(parseRelease(github, 5)).toMatchObject({
      version: "3.0.1",
      sha: "8086105",
      subject: "feat(extension): live sign-in",
      zipUrl: "https://x/extension.zip",
      xpiUrl: null,
      publishedAt: Date.parse("2026-11-24T10:00:00Z"),
    });
    expect(parseRelease({ ...github, name: "Other" }, 5)).toBeNull();
  });

  it("answers null when GitHub is blocked or refuses", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("blocked"))));
    expect(await releaseFromGitHub()).toBeNull();
    vi.stubGlobal("fetch", vi.fn(async () => new Response("rate limited", { status: 403 })));
    expect(await releaseFromGitHub()).toBeNull();
    vi.stubGlobal("fetch", vi.fn(async () => Response.json(github)));
    expect(await releaseFromGitHub()).toMatchObject({ sha: "8086105" });
    vi.unstubAllGlobals();
  });

});
