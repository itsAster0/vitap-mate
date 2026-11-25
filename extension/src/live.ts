// The extension's live link to the server: a Convex client on the bridge URL, signed in with the
// access key's JWT (`POST /v1/key/token`), for the backend's `extension:*` queries. It is opened
// only while something is happening (a sign-in, a pairing, the popup with a key) and closed after. The
// session's cookies and new keys still come over HTTP, once (bridge.ts).

import { ConvexClient } from "convex/browser";
import { makeFunctionReference } from "convex/server";
import { LoginError, UNKNOWN_KEY_MESSAGE, fetchWithTimeout } from "./bridge";

/** `extension:key`: the key's account, or null once it is revoked. Times in seconds. */
export type KeyView = {
  registrationNumber: string;
  keyLabel: string | null;
  keyCreatedAt: number;
  keyLastUsedAt: number | null;
  settings: { alwaysUseVault: boolean; phoneWaitSecs: number; vaultTtlSecs: number | null };
  phoneLinked: boolean;
  /** The key's own phone: the only one its sign-ins ask. */
  phone: { label: string | null; notifications: boolean } | null;
  savedCredentials: boolean;
  vaultSavedAt: number | null;
  credentialsRejectedAt: number | null;
  cachedSession: boolean;
};

/** What the server's sign-in is doing, while it signs in for a request. */
export type LoginStep = "signing_in" | "waiting_otp" | "checking_otp";

/** `extension:request`: a session request's progress, never its cookies. Times in ms. */
export type RequestView =
  | { status: "pending"; stage: "phone" | "server" | "starting"; serverDueAt: number | null; login: LoginStep | null; expiresAt: number }
  | { status: "ready"; from: string; expiresAt: number }
  | { status: "error"; code: string; error: string; expiresAt: number }
  | { status: "key_unknown" }
  | { status: "not_found" };

/** The newest build CI published on GitHub's `extension-latest` release. Times in ms. */
export type Release = {
  version: string;
  sha: string;
  subject: string | null;
  url: string;
  zipUrl: string | null;
  xpiUrl: string | null;
  publishedAt: number;
  checkedAt: number;
};

export const queries = {
  key: makeFunctionReference<"query", Record<string, never>, KeyView | null>("extension:key"),
  request: makeFunctionReference<"query", { requestId: string }, RequestView>("extension:request"),
  pairing: makeFunctionReference<"query", { codeHash: string }, { ready: boolean }>("extension:pairing"),
};

/** Why the key couldn't sign the socket in. */
export type AuthTrouble = "key_unknown" | "unreachable";

/** The part of ConvexClient used here, so tests can pass a fake. */
export type LiveClient = Pick<ConvexClient, "onUpdate" | "close">;

/** A client on `base`. With `key`, it signs in with the key's JWT and waits for that before any
 * query; `onTrouble` hears when the key is refused or the server can't be reached, and `onAuth`
 * whether the socket is signed in. A socket whose token fetch failed runs signed out, where the
 * key's queries answer as if the key were unknown, so callers check `onAuth` before believing that. */
export function openLive(
  base: string,
  key?: string,
  onTrouble?: (why: AuthTrouble) => void,
  onAuth?: (signedIn: boolean) => void,
): ConvexClient {
  const client = new ConvexClient(base.replace(/\/+$/, ""), {
    // Its logs carry the end of each token.
    logger: false,
    unsavedChangesWarning: false,
    skipConvexDeploymentUrlCheck: true,
    expectAuth: key !== undefined,
    // Without it, the client fetches a second token straight after the first, and the same token
    // back would stop its refreshes.
    initialAuthTokenReuse: true,
  });
  if (key !== undefined) client.setAuth(keyToken(base, key, onTrouble), onAuth);
  return client;
}

/** Fetches key JWTs. Never throws: a throw would leave the client's socket paused for good. */
function keyToken(base: string, key: string, onTrouble?: (why: AuthTrouble) => void) {
  return async (): Promise<string | null> => {
    try {
      const res = await fetchWithTimeout(
        `${base}/v1/key/token`,
        { method: "POST", headers: { Authorization: `Bearer ${key}` } },
        10_000,
      );
      if (res.status === 401) {
        onTrouble?.("key_unknown");
        return null;
      }
      if (!res.ok) {
        onTrouble?.("unreachable");
        return null;
      }
      const { jwt } = (await res.json()) as { jwt?: string };
      return typeof jwt === "string" ? jwt : null;
    } catch {
      onTrouble?.("unreachable");
      return null;
    }
  };
}

/** How long a socket may stay silent before a wait gives up on reaching the server. */
export const FIRST_ANSWER_MS = 10_000;

export const UNREACHABLE_MESSAGE = "Can't reach VITAP Mate's server. Check your connection and try again.";

/** Follows request `requestId` until it is ready, failed or expired, telling `onProgress` each
 * change. Resolves with the last view; rejects with a message to show. `expiresAt` is enforced
 * here, as a query doesn't run again just because time passed. */
export function waitForRequest(
  client: LiveClient,
  requestId: string,
  signal: AbortSignal,
  onProgress: (view: Extract<RequestView, { status: "pending" }>) => void,
  firstAnswerMs = FIRST_ANSWER_MS,
): Promise<Extract<RequestView, { status: "ready" }>> {
  if (signal.aborted) return Promise.reject(new LoginError("cancelled", CANCELLED));
  return new Promise((resolve, reject) => {
    let expiry: ReturnType<typeof setTimeout> | undefined;
    const silent = setTimeout(() => finish(new LoginError("unreachable", UNREACHABLE_MESSAGE)), firstAnswerMs);
    const stop = client.onUpdate(
      queries.request,
      { requestId },
      (view) => {
        clearTimeout(silent);
        switch (view.status) {
          case "pending":
            if (expiry === undefined) {
              expiry = setTimeout(() => finish(new LoginError("expired", EXPIRED)), Math.max(0, view.expiresAt - Date.now()));
            }
            onProgress(view);
            return;
          case "ready":
            return finish(null, view);
          case "error":
            return finish(
              new LoginError("failed", view.error.replace(/^[a-z_]+:\s*/, "") || "The sign-in failed. Try again."),
            );
          case "key_unknown":
            return finish(new LoginError("key_unknown", UNKNOWN_KEY_MESSAGE));
          case "not_found":
            return finish(new LoginError("expired", EXPIRED));
        }
      },
      () => finish(new LoginError("unreachable", UNREACHABLE_MESSAGE)),
    );
    const onAbort = () => finish(new LoginError("cancelled", CANCELLED));
    signal.addEventListener("abort", onAbort, { once: true });

    function finish(error: LoginError | null, view?: Extract<RequestView, { status: "ready" }>) {
      clearTimeout(silent);
      clearTimeout(expiry);
      signal.removeEventListener("abort", onAbort);
      stop();
      if (error) reject(error);
      else resolve(view!);
    }
  });
}

export const CANCELLED = "Sign-in cancelled.";
const EXPIRED = "Your phone did not answer in time. Open VITAP Mate once (so it can sign in) and try again.";

/** The pairing code as `keys:create` hashes it: without its dash, upper case. */
export async function pairCodeHash(code: string): Promise<string> {
  const plain = code.replace(/[\s-]/g, "").toUpperCase();
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(plain)));
  return [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** Where CI publishes each build (`.github/workflows/extension-build.yml`). GitHub's API allows
 * any origin, so this needs no host permission. */
export const RELEASE_API = "https://api.github.com/repos/itsAster0/vitap-mate/releases/tags/extension-latest";

/** Reads the `extension-latest` release: CI titles it "Browser extension <version> (<sha>)" and
 * bolds the commit's subject in the notes. Null when it doesn't look like that. */
export function parseRelease(json: unknown, checkedAt: number): Release | null {
  const r = json as {
    name?: string;
    body?: string;
    html_url?: string;
    published_at?: string;
    assets?: { name?: string; browser_download_url?: string; updated_at?: string }[];
  };
  const title = /^Browser extension (\d+(?:\.\d+){1,3}) \(([0-9a-f]{7,40})\)$/.exec(r?.name?.trim() ?? "");
  if (!title || typeof r.html_url !== "string") return null;
  const asset = (name: string) => r.assets?.find((a) => a.name === name);
  // The release is edited in place: the newest upload is this build's date.
  const times = (r.assets ?? []).map((a) => Date.parse(a.updated_at ?? "")).filter(Number.isFinite);
  return {
    version: title[1],
    sha: title[2],
    subject: /\*\*(.+?)\*\*/.exec(r.body ?? "")?.[1]?.trim() ?? null,
    url: r.html_url,
    zipUrl: asset("extension.zip")?.browser_download_url ?? null,
    xpiUrl: asset("extension.xpi")?.browser_download_url ?? null,
    publishedAt: times.length ? Math.max(...times) : Date.parse(r.published_at ?? "") || checkedAt,
    checkedAt,
  };
}

/** The newest build straight from GitHub; null when GitHub is blocked, rate limited or down, so a
 * network that blocks GitHub gets no update notice and no error. */
export async function releaseFromGitHub(timeoutMs = 10_000): Promise<Release | null> {
  try {
    const res = await fetchWithTimeout(RELEASE_API, { headers: { accept: "application/vnd.github+json" } }, timeoutMs);
    return res.ok ? parseRelease(await res.json(), Date.now()) : null;
  } catch {
    return null;
  }
}

/** This build's commit and when it was built, from vite.config.ts. */
export const BUILD = { sha: __BUILD_SHA__, at: __BUILD_AT__ };

/** Whether `release` is a newer build than this one: another commit, published after this build.
 * A local build of an older commit doesn't count as behind a release it predates. */
export function updateAvailable(release: Release | null, build = BUILD): boolean {
  if (!release || !build.sha) return false;
  const sameCommit = build.sha.startsWith(release.sha) || release.sha.startsWith(build.sha);
  return !sameCommit && release.publishedAt > build.at;
}
