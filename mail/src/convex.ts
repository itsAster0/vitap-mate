// The mail's way to Convex: `POST {CONVEX_URL}/internal/mail` with `MAIL_TOKEN`
// (docs/plans/2026-11-23-backend-v3-api.md, "Mail Worker → Convex").

import type { Parsed } from "./parse";

export const MAIL_PATH = "/internal/mail";
/** Longest wait for Convex on one try; the mail server, or the queue, tries again. */
const POST_TIMEOUT_MS = 10_000;

/** The body Convex takes. `receivedAt` (ms) is the mail's own date, capped at its arrival,
 * so a retry from the queue carries the same time as the first try. */
export type MailBody = {
  addressId: string;
  receivedAt: number;
  otp: string | null;
  gmailConfirm: { code: string | null; link: string | null; requester: string | null } | null;
};

export type MailEnv = {
  CONVEX_URL: string;
  MAIL_TOKEN: string;
  FALLBACK_FORWARD?: string;
};

export type Fetch = (input: string, init: RequestInit) => Promise<Response>;

/** `stored`; `no_mailbox` (Convex has no such address); `retry` for anything else. */
export type Outcome = "stored" | "no_mailbox" | "retry";

export function mailBody(addressId: string, mail: Parsed): MailBody {
  return {
    addressId,
    receivedAt: mail.sentAt * 1000,
    otp: mail.kind === "otp" ? mail.code : null,
    gmailConfirm: mail.kind === "gmail_confirm" ? mail.confirm : null,
  };
}

/** An unexpected failure, at error level, with the error's name, message and the top of its
 * stack. A SyntaxError's message is left out, as it quotes the text it could not parse. Never
 * pass mail bodies, codes or tokens. */
export function logError(message: string, err?: unknown): void {
  if (!(err instanceof Error)) {
    console.error(err === undefined ? message : `${message}: ${typeof err}`);
    return;
  }
  const frames = (err.stack ?? "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.startsWith("at "))
    .slice(0, 3);
  const text = err instanceof SyntaxError ? "" : err.message.replace(/\s+/g, " ").slice(0, 200);
  console.error(`${message}: ${err.name}${text ? `: ${text}` : ""}${frames.length ? ` (${frames.join("; ")})` : ""}`);
}

export async function postMail(env: MailEnv, body: MailBody, fetch: Fetch): Promise<Outcome> {
  let response: Response;
  try {
    response = await fetch(`${env.CONVEX_URL.replace(/\/+$/, "")}${MAIL_PATH}`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${env.MAIL_TOKEN}` },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(POST_TIMEOUT_MS),
    });
  } catch (e) {
    logError("mail: Convex unreachable", e);
    return "retry";
  }
  if (response.status === 200) return "stored";
  if (response.status === 404 && (await errorCode(response)) === "mailbox_not_found") return "no_mailbox";
  logError(`mail: Convex answered ${response.status}`);
  return "retry";
}

async function errorCode(response: Response): Promise<string | null> {
  try {
    const body: unknown = await response.json();
    if (typeof body === "object" && body !== null && "code" in body && typeof body.code === "string") {
      return body.code;
    }
  } catch {}
  return null;
}
