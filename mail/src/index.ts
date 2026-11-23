// The Email Worker for the forwarding addresses (`otp+<id>@vmate.aster0.dev`). It keeps
// no state: each OTP or Gmail confirmation goes to Convex's `/internal/mail`, which seals
// it to the mailbox's devices. When Convex doesn't take it, the mail waits on a Queue and
// is tried again with backoff for up to RETRY_FOR_MS. Mail to other addresses, or to an
// address Convex doesn't know, goes to FALLBACK_FORWARD (rejected without one).

import { type Fetch, type MailBody, type MailEnv, mailBody, postMail } from "./convex";
import { idFromAddress, parseMail } from "./parse";

/** An OTP is useless after a few minutes; nothing is kept longer than this. */
export const RETRY_FOR_MS = 10 * 60 * 1000;
const FIRST_DELAY_SECS = 10;
const MAX_DELAY_SECS = 120;

/** A mail Convex didn't take yet, with when it was first queued. */
export type Queued = { mail: MailBody; queuedAt: number };

export type Env = MailEnv & { MAIL_QUEUE: Queue<Queued> };

export type Deps = { fetch: Fetch; now: () => number };

/** What the handler needs of an incoming mail, so tests can pass their own. */
export type IncomingMail = Pick<ForwardableEmailMessage, "to" | "raw" | "forward" | "setReject">;

/** What the consumer needs of a queued message. */
export type QueuedMessage = Pick<Message<unknown>, "body" | "attempts" | "ack" | "retry">;

async function fallback(message: IncomingMail, env: Env): Promise<void> {
  const to = env.FALLBACK_FORWARD ?? "";
  if (to) await message.forward(to);
  else message.setReject("Unknown address");
}

/** Throwing makes the sending server retry later, so only a failed enqueue throws. */
export async function handleEmail(message: IncomingMail, env: Env, deps: Deps): Promise<void> {
  const id = idFromAddress(message.to);
  if (id === null) return fallback(message, env);
  const now = deps.now();
  const raw = await new Response(message.raw).arrayBuffer();
  const parsed = await parseMail(raw, Math.floor(now / 1000));
  if (parsed === null) {
    console.log("mail: dropped a mail for a mailbox");
    return;
  }
  const body = mailBody(id, parsed);
  switch (await postMail(env, body, deps.fetch)) {
    case "stored":
      console.log(`mail: stored a ${parsed.kind}`);
      return;
    case "no_mailbox":
      console.log("mail: no such mailbox");
      return fallback(message, env);
    case "retry":
      await env.MAIL_QUEUE.send({ mail: body, queuedAt: now });
      console.log(`mail: queued a ${parsed.kind}`);
  }
}

/** Seconds before try `attempts + 1`: doubling from FIRST_DELAY_SECS, at most MAX_DELAY_SECS. */
export function backoff(attempts: number): number {
  return Math.min(FIRST_DELAY_SECS * 2 ** Math.max(0, attempts - 1), MAX_DELAY_SECS);
}

function isQueued(body: unknown): body is Queued {
  if (typeof body !== "object" || body === null) return false;
  const { mail, queuedAt } = body as Record<string, unknown>;
  return typeof queuedAt === "number" && typeof mail === "object" && mail !== null && typeof (mail as Record<string, unknown>).addressId === "string";
}

export async function handleQueue(messages: readonly QueuedMessage[], env: Env, deps: Deps): Promise<void> {
  await Promise.all(
    messages.map(async (message) => {
      const { body } = message;
      if (!isQueued(body)) {
        console.log("mail: dropped an unreadable queued mail");
        message.ack();
        return;
      }
      if (deps.now() - body.queuedAt >= RETRY_FOR_MS) {
        console.log("mail: gave up on a queued mail");
        message.ack();
        return;
      }
      switch (await postMail(env, body.mail, deps.fetch)) {
        case "stored":
          console.log("mail: stored a queued mail");
          message.ack();
          return;
        case "no_mailbox":
          // The raw mail is gone by now, so there is nothing to forward.
          console.log("mail: queued mail has no mailbox");
          message.ack();
          return;
        case "retry": {
          const left = body.queuedAt + RETRY_FOR_MS - deps.now();
          const delaySeconds = Math.min(backoff(message.attempts), Math.ceil(left / 1000));
          if (delaySeconds <= 0) {
            console.log("mail: gave up on a queued mail");
            message.ack();
            return;
          }
          message.retry({ delaySeconds });
        }
      }
    }),
  );
}

const live: Deps = { fetch: (input, init) => fetch(input, init), now: () => Date.now() };

export default {
  email: (message: ForwardableEmailMessage, env: Env) => handleEmail(message, env, live),
  queue: (batch: MessageBatch<unknown>, env: Env) => handleQueue(batch.messages, env, live),
} satisfies ExportedHandler<Env, unknown>;
