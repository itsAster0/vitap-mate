// What a forwarded mail means for the phone. Only two kinds are relayed:
// VTOP's OTP mail, and Gmail's forwarding confirmation (needed once during
// setup). Everything else is dropped. The mail is parsed in memory and
// nothing about its content is logged.

import PostalMime, { type RawEmail } from "postal-mime";

export const VTOP_SENDER = "noreply.sdc@vitap.ac.in";
export const GMAIL_FORWARDING_SENDER = "forwarding-noreply@google.com";
const CONFIRM_LINK = "https://mail-settings.google.com/mail/vf-";

/** The plaintext sealed for the phone; the app reads it as JSON. */
export type Payload =
  | { kind: "otp"; code: string; sentAt: number }
  | {
      kind: "gmail_confirm";
      code: string | null;
      link: string | null;
      requester: string | null;
      sentAt: number;
    };

/** Lower-cases A-Z only, so indexes into the result match the input. */
function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (c) => c.toLowerCase());
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
  const start = text.indexOf(CONFIRM_LINK);
  if (start < 0) return null;
  const link = /^[^\s"'<>]*/.exec(text.slice(start))![0];
  return link.length > CONFIRM_LINK.length ? link : null;
}

function requester(subject: string): string | null {
  const label = "receive mail from ";
  const start = asciiLower(subject).indexOf(label);
  if (start < 0) return null;
  const word = subject.slice(start + label.length).trim().split(/\s+/)[0];
  return word?.includes("@") ? word : null;
}

/** The payload for a raw RFC 5322 mail, or null to drop it. `now` is used
 * when the mail has no usable `Date:`. */
export async function payloadFor(raw: RawEmail, now: number): Promise<Payload | null> {
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
  const sentAt = Number.isFinite(date) && date > 0 ? date : now;

  switch (from) {
    case VTOP_SENDER: {
      const code = firstSixDigitCode(text);
      return code === null ? null : { kind: "otp", code, sentAt };
    }
    case GMAIL_FORWARDING_SENDER: {
      const code = digitsAfter(text, "confirmation code:") ?? digitsAfter(text, "(#");
      const link = confirmLink(text);
      if (code === null && link === null) return null;
      return { kind: "gmail_confirm", code, link, requester: requester(subject), sentAt };
    }
    default:
      return null;
  }
}
