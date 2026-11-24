import { describe, expect, it, vi } from "vitest";

import { MAIL_PATH, type MailBody, logError } from "./convex";
import { type Deps, type Env, type IncomingMail, type Queued, type QueuedMessage, RETRY_FOR_MS, backoff, handleEmail, handleQueue } from "./index";

/** A minute after the test mail's `Date:`, in ms. */
const NOW = 1792231260_000;
const ID = "k3f9x2ab7q";
const TOKEN = "mail-token-mail-token-mail-token";

const OTP_MAIL = [
  "From: VTOP <noreply.sdc@vitap.ac.in>",
  `To: otp+${ID}@vmate.aster0.dev`,
  "Subject: VTOP Login OTP",
  "Date: Fri, 17 Oct 2026 10:00:00 +0000",
  "Content-Type: text/plain; charset=UTF-8",
  "",
  "Your OTP is 112233.",
].join("\r\n");

const OTP_BODY: MailBody = {
  addressId: ID,
  receivedAt: 1792231200_000,
  otp: "112233",
  gmailConfirm: null,
};

type Call = { url: string; init: RequestInit };

/** Convex answering each POST from `answers` in turn (a thrown answer is a network error). */
function convex(...answers: Array<Response | Error>): { fetch: Deps["fetch"]; calls: Call[] } {
  const calls: Call[] = [];
  return {
    calls,
    fetch: async (url, init) => {
      calls.push({ url, init });
      const answer = answers.shift() ?? new Error("no more answers");
      if (answer instanceof Error) throw answer;
      return answer;
    },
  };
}

const json = (status: number, body: unknown) => Response.json(body, { status });

function env(fallback = ""): Env & { queued: Queued[] } {
  const queued: Queued[] = [];
  return {
    CONVEX_URL: "https://convex.example/",
    MAIL_TOKEN: TOKEN,
    FALLBACK_FORWARD: fallback,
    queued,
    MAIL_QUEUE: {
      send: async (body: Queued) => {
        queued.push(body);
      },
      sendBatch: async () => {},
      metrics: async () => ({ backlogCount: 0, backlogBytes: 0 }),
    } as unknown as Queue<Queued>,
  };
}

function mail(to: string, raw = OTP_MAIL): IncomingMail & { forwarded: string[]; rejected: string[] } {
  const forwarded: string[] = [];
  const rejected: string[] = [];
  return {
    to,
    raw: new Response(raw).body as ReadableStream<Uint8Array>,
    forwarded,
    rejected,
    forward: async (rcptTo: string) => {
      forwarded.push(rcptTo);
      return { messageId: "x" };
    },
    setReject: (reason: string) => {
      rejected.push(reason);
    },
  } as IncomingMail & { forwarded: string[]; rejected: string[] };
}

describe("handleEmail", () => {
  it("posts an OTP to Convex with the mail token", async () => {
    const { fetch, calls } = convex(json(200, { ok: true }));
    const e = env();
    const m = mail(`otp+${ID}@vmate.aster0.dev`);
    await handleEmail(m, e, { fetch, now: () => NOW });
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe(`https://convex.example${MAIL_PATH}`);
    expect(new Headers(calls[0]?.init.headers).get("authorization")).toBe(`Bearer ${TOKEN}`);
    expect(JSON.parse(String(calls[0]?.init.body))).toEqual(OTP_BODY);
    expect(e.queued).toEqual([]);
    expect([m.forwarded, m.rejected]).toEqual([[], []]);
  });

  it("forwards mail Convex has no mailbox for, or rejects it without a fallback", async () => {
    for (const [fallback, forwarded, rejected] of [
      ["me@example.com", ["me@example.com"], []],
      ["", [], ["Unknown address"]],
    ] as const) {
      const { fetch } = convex(json(404, { code: "mailbox_not_found", error: "No such forwarding address." }));
      const e = env(fallback);
      const m = mail(`otp+${ID}@vmate.aster0.dev`);
      await handleEmail(m, e, { fetch, now: () => NOW });
      expect([m.forwarded, m.rejected]).toEqual([forwarded, rejected]);
      expect(e.queued).toEqual([]);
    }
  });

  it("forwards other addresses without asking Convex", async () => {
    const { fetch, calls } = convex();
    const m = mail("someone@vmate.aster0.dev");
    await handleEmail(m, env("me@example.com"), { fetch, now: () => NOW });
    expect(m.forwarded).toEqual(["me@example.com"]);
    expect(calls).toEqual([]);
  });

  it("drops mail that is neither an OTP nor a confirmation", async () => {
    const { fetch, calls } = convex();
    const e = env("me@example.com");
    const m = mail(`otp+${ID}@vmate.aster0.dev`, OTP_MAIL.replace("noreply.sdc@vitap.ac.in", "spam@example.com"));
    await handleEmail(m, e, { fetch, now: () => NOW });
    expect(calls).toEqual([]);
    expect([m.forwarded, m.rejected, e.queued]).toEqual([[], [], []]);
  });

  it("queues the mail when Convex fails or can't be reached", async () => {
    for (const answer of [json(500, { code: "internal_error" }), json(404, { code: "not_found" }), new TypeError("fetch failed")]) {
      const { fetch } = convex(answer);
      const e = env("me@example.com");
      const m = mail(`otp+${ID}@vmate.aster0.dev`);
      await handleEmail(m, e, { fetch, now: () => NOW });
      expect(e.queued).toEqual([{ mail: OTP_BODY, queuedAt: NOW }]);
      expect(m.forwarded).toEqual([]);
    }
  });
});

type Fake = QueuedMessage & { acked: boolean; retried: number[] };

function queued(body: unknown, attempts: number): Fake {
  const message: Fake = {
    body,
    attempts,
    acked: false,
    retried: [],
    ack() {
      message.acked = true;
    },
    retry(options) {
      message.retried.push(options?.delaySeconds ?? 0);
    },
  };
  return message;
}

describe("handleQueue", () => {
  const body: Queued = { mail: OTP_BODY, queuedAt: NOW };

  it("acks what Convex stores, or has no mailbox for", async () => {
    const stored = queued(body, 1);
    const orphan = queued(body, 1);
    const { fetch, calls } = convex(json(200, {}), json(404, { code: "mailbox_not_found" }));
    await handleQueue([stored, orphan], env(), { fetch, now: () => NOW + 10_000 });
    expect(calls).toHaveLength(2);
    expect([stored.acked, orphan.acked]).toEqual([true, true]);
  });

  it("retries with a doubling delay while Convex fails", async () => {
    const message = queued(body, 3);
    const { fetch } = convex(json(503, {}));
    await handleQueue([message], env(), { fetch, now: () => NOW + 60_000 });
    expect(message.acked).toBe(false);
    expect(message.retried).toEqual([40]);
    expect([1, 2, 3, 4, 5, 9].map(backoff)).toEqual([10, 20, 40, 80, 120, 120]);
  });

  it("never waits past ten minutes, and gives up after them", async () => {
    const late = queued(body, 9);
    const { fetch } = convex(new TypeError("fetch failed"));
    await handleQueue([late], env(), { fetch, now: () => NOW + RETRY_FOR_MS - 30_000 });
    expect(late.retried).toEqual([30]);

    const expired = queued(body, 12);
    const none = convex();
    await handleQueue([expired], env(), { fetch: none.fetch, now: () => NOW + RETRY_FOR_MS });
    expect(expired.acked).toBe(true);
    expect(none.calls).toEqual([]);
  });

  it("drops a message it can't read", async () => {
    const junk = queued({ hello: 1 }, 1);
    const { fetch, calls } = convex();
    await handleQueue([junk], env(), { fetch, now: () => NOW });
    expect(junk.acked).toBe(true);
    expect(calls).toEqual([]);
  });
});

describe("logError", () => {
  it("omits SyntaxError messages so unparsed payloads never leak to logs", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      logError("mail: bad json", new SyntaxError("Unexpected token 'SECRET_OTP_112233'"));
      expect(spy).toHaveBeenCalledTimes(1);
      const line = String(spy.mock.calls[0]?.[0]);
      expect(line).toContain("SyntaxError");
      expect(line).not.toContain("SECRET_OTP_112233");
    } finally {
      spy.mockRestore();
    }
  });
});
