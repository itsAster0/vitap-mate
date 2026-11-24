// Popup-side live subscriptions over Convex. Opens a socket only while a valid key is saved
// or a pairing code is active, and tears it down as soon as that work stops.

import {
  FIRST_ANSWER_MS,
  UNREACHABLE_MESSAGE,
  openLive,
  queries,
  type AuthTrouble,
  type KeyView,
  type LiveClient,
} from "./live";

export type LiveOpener = (
  base: string,
  key?: string,
  onTrouble?: (why: AuthTrouble) => void,
  onAuth?: (signedIn: boolean) => void,
) => LiveClient;

/** Mutually exclusive phases of the popup's account watch. */
export type AccountLiveState =
  | { kind: "idle" }
  | { kind: "connecting"; account?: KeyView }
  | { kind: "ready"; account: KeyView }
  | { kind: "rejected" }
  | { kind: "unreachable"; account?: KeyView };

/**
 * Watches `extension:key` for a saved access key. Call only when a valid key exists;
 * returns a cleanup function that stops the subscription and closes the socket.
 */
export function watchAccount(
  base: string,
  key: string,
  onState: (state: AccountLiveState) => void,
  open: LiveOpener = openLive,
  firstAnswerMs = FIRST_ANSWER_MS,
): () => void {
  let stopped = false;
  let signedIn = false;
  let lastAccount: KeyView | undefined;

  const emit = (next: AccountLiveState) => {
    if (!stopped) onState(next);
  };

  emit({ kind: "connecting" });

  let client: LiveClient;
  try {
    client = open(
      base,
      key,
      (why) => {
        if (why === "key_unknown") {
          lastAccount = undefined;
          emit({ kind: "rejected" });
        } else {
          emit({ kind: "unreachable", account: lastAccount });
        }
      },
      (yes) => {
        signedIn = yes;
      },
    );
  } catch {
    emit({ kind: "unreachable" });
    return () => {
      stopped = true;
    };
  }

  const silent = setTimeout(
    () => emit({ kind: "unreachable", account: lastAccount }),
    firstAnswerMs,
  );

  const unsub = client.onUpdate(
    queries.key,
    {},
    (value) => {
      clearTimeout(silent);
      // Signed out (token fetch failed), every key reads as unknown: only trust a signed-in null.
      if (value === null && !signedIn) return;
      if (value === null) {
        lastAccount = undefined;
        emit({ kind: "rejected" });
      } else {
        lastAccount = value;
        emit({ kind: "ready", account: value });
      }
    },
    () => emit({ kind: "unreachable", account: lastAccount }),
  );

  return () => {
    stopped = true;
    clearTimeout(silent);
    unsub();
    void client.close();
  };
}

/**
 * Lazily opens an unauthenticated socket to watch `extension:pairing` for `codeHash`.
 * Immediately reports `UNREACHABLE_MESSAGE` if opening the socket fails.
 */
export function watchPairing(
  base: string,
  codeHash: string,
  handlers: { onReady: () => void; onError: (message: string) => void },
  open: LiveOpener = openLive,
  firstAnswerMs = FIRST_ANSWER_MS,
): () => void {
  let stopped = false;

  let client: LiveClient;
  try {
    client = open(base);
  } catch {
    handlers.onError(UNREACHABLE_MESSAGE);
    return () => {
      stopped = true;
    };
  }

  const silent = setTimeout(() => {
    if (!stopped) handlers.onError(UNREACHABLE_MESSAGE);
  }, firstAnswerMs);

  let unsub: (() => void) | undefined;
  try {
    unsub = client.onUpdate(
      queries.pairing,
      { codeHash },
      ({ ready }) => {
        clearTimeout(silent);
        if (!stopped && ready) handlers.onReady();
      },
      () => {
        clearTimeout(silent);
        if (!stopped) handlers.onError(UNREACHABLE_MESSAGE);
      },
    );
  } catch {
    clearTimeout(silent);
    void client.close();
    handlers.onError(UNREACHABLE_MESSAGE);
    return () => {
      stopped = true;
    };
  }

  return () => {
    if (stopped) return;
    stopped = true;
    clearTimeout(silent);
    unsub?.();
    void client.close();
  };
}
