// Parses forwarded email for `otp+<id>@` addresses. Only two kinds go on: VTOP's OTP mail,
// and Gmail's forwarding confirmation (needed once during setup). Everything else is dropped.
// The mail is parsed in memory and nothing about its content is logged.

import PostalMime, { type RawEmail } from "postal-mime";

export const VTOP_SENDER = "noreply.sdc@vitap.ac.in";
export const GMAIL_FORWARDING_SENDER = "forwarding-noreply@google.com";
// Gmail sends mail-settings.google.com; Workspace accounts (VIT-AP's) send
// another google.com host and no code, so match any google.com host.
const CONFIRM_LINK = /https:\/\/(?:[a-z0-9-]+\.)*google\.com\/mail\/vf-[A-Za-z0-9%._~-]+(?![^\s"'<>])/i;

// Addresses are `otp+<id>@<domain>`: one Email Routing rule (`otp@<domain>`) catches
// every plus address, as a subdomain can have no catch-all.
export const ADDRESS_PREFIX = "otp+";
export const ID_LEN = 10;
/** No `l`, `o`, `0` or `1`, so an id read aloud or typed is unambiguous. */
export const ID_ALPHABET = "abcdefghijkmnpqrstuvwxyz23456789";

export type GmailConfirm = { code: string | null; link: string | null; requester: string | null };

/** What a mail says, or null to drop it. `sentAt` is in seconds. */
export type Parsed =
  | { kind: "otp"; code: string; sentAt: number }
  | { kind: "gmail_confirm"; confirm: GmailConfirm; sentAt: number };

/** Lower-cases A-Z only, so indexes into the result match the input. */
function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (c) => c.toLowerCase());
}

export function isId(id: string): boolean {
  return id.length === ID_LEN && [...id].every((c) => ID_ALPHABET.includes(c));
}

/** The mailbox id in a recipient address, if it is one of ours. */
export function idFromAddress(recipient: string): string | null {
  const local = asciiLower(recipient.trim().split("@")[0] ?? "");
  if (!local.startsWith(ADDRESS_PREFIX)) return null;
  const id = local.slice(ADDRESS_PREFIX.length);
  return isId(id) ? id : null;
}

/** The first run of exactly six digits, as vtop-core's Gmail reader. */
export function firstSixDigitCode(text: string): string | null {
  return text.split(/[^0-9]/).find((run) => run.length === 6) ?? null;
}

function htmlToText(html: string): string {
  const lower = asciiLower(html);
  let out = "";
  let i = 0;
  while (i < html.length) {
    const tag = ["<style", "<script"].find((t) => lower.startsWith(t, i));
    if (tag) {
      const close = `</${tag.slice(1)}`;
      const end = lower.indexOf(close, i);
      i = end < 0 ? html.length : end + close.length;
      out += " ";
      continue;
    }
    if (html[i] === "<") {
      const end = html.indexOf(">", i);
      i = end < 0 ? html.length : end + 1;
      out += " ";
      continue;
    }
    out += html[i];
    i += 1;
  }
  return out.replaceAll("&nbsp;", " ").replaceAll("&amp;", "&");
}

/** Digits following `label` (case-insensitive), after optional spaces. */
function digitsAfter(text: string, label: string): string | null {
  const start = asciiLower(text).indexOf(label);
  if (start < 0) return null;
  return /^\s*([0-9]+)/.exec(text.slice(start + label.length))?.[1] ?? null;
}

function confirmLink(text: string): string | null {
  return CONFIRM_LINK.exec(text)?.[0] ?? null;
}

function requester(subject: string): string | null {
  const label = "receive mail from ";
  const start = asciiLower(subject).indexOf(label);
  if (start < 0) return null;
  const word = subject.slice(start + label.length).trim().split(/\s+/)[0];
  return word?.includes("@") ? word : null;
}

/** What a raw RFC 5322 mail says, or null to drop it. `sentAt` is the mail's `Date:`,
 * never later than `now` (the sender writes it, and a date ahead would pass every "sent
 * since" check), or `now` without one. */
export async function parseMail(raw: RawEmail, now: number): Promise<Parsed | null> {
  let message;
  try {
    message = await PostalMime.parse(raw);
  } catch {
    return null;
  }
  const sender = message.from?.address ?? message.from?.group?.[0]?.address ?? "";
  const from = asciiLower(sender);
  const subject = message.subject ?? "";
  const text = [subject, message.text ?? "", message.html ? htmlToText(message.html) : ""].join("\n");
  const date = message.date ? Math.floor(Date.parse(message.date) / 1000) : Number.NaN;
  const sentAt = Number.isFinite(date) && date > 0 ? Math.min(date, now) : now;

  switch (from) {
    case VTOP_SENDER: {
      const code = firstSixDigitCode(text);
      return code === null ? null : { kind: "otp", code, sentAt };
    }
    case GMAIL_FORWARDING_SENDER: {
      const code = digitsAfter(text, "confirmation code:") ?? digitsAfter(text, "(#");
      const link = confirmLink(text);
      if (code === null && link === null) return null;
      return { kind: "gmail_confirm", confirm: { code, link, requester: requester(subject) }, sentAt };
    }
    default:
      return null;
  }
}
