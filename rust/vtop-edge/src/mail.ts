// What a forwarded mail means for the phone. Only two kinds are relayed:
// VTOP's OTP mail, and Gmail's forwarding confirmation (needed once during
// setup). Everything else is dropped. The mail is parsed in memory and
// nothing about its content is logged.

import PostalMime, { type RawEmail } from "postal-mime";

export const VTOP_SENDER = "noreply.sdc@vitap.ac.in";
export const GMAIL_FORWARDING_SENDER = "forwarding-noreply@google.com";
// Gmail sends mail-settings.google.com; Workspace accounts (VIT-AP's) send
// another google.com host and no code, so match any google.com host.
const CONFIRM_LINK = /https?:\/\/(?:[a-z0-9-]+\.)*google\.com\/mail\/vf-[^\s"'<>]+/i;

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
  return CONFIRM_LINK.exec(text)?.[0] ?? null;
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

/** TEMPORARY: the layout of a dropped mail from Gmail's forwarding sender,
 * to fix the parser for Workspace accounts. Digits and addresses are masked;
 * no code, link or body text is kept. */
export async function dropShape(raw: RawEmail): Promise<string> {
  let message;
  try {
    message = await PostalMime.parse(raw);
  } catch {
    return "unparsable";
  }
  const mask = (t: string) => t.replace(/[^\s@<>"]+@[^\s@<>"]+/g, "<addr>").replace(/[0-9]/g, "9");
  const text = message.text ?? "";
  const html = message.html ?? "";
  const plain = htmlToText(html);
  const around = (t: string, label: string) => {
    const i = asciiLower(t).indexOf(label);
    return i < 0 ? null : mask(t.slice(Math.max(0, i - 20), i + 40)).replace(/\s+/g, " ");
  };
  return JSON.stringify({
    from: mask(message.from?.address ?? message.from?.group?.[0]?.address ?? "") === "<addr>" ? asciiLower(message.from?.address ?? "").split("@")[1] : "none",
    fromIsGmail: asciiLower(message.from?.address ?? "") === GMAIL_FORWARDING_SENDER,
    subject: mask(message.subject ?? ""),
    textLen: text.length,
    htmlLen: html.length,
    vfInText: text.includes("/mail/vf-"),
    vfInHtml: html.includes("/mail/vf-"),
    vfHostsText: [...new Set([...text.matchAll(/([a-z]+:\/\/[a-z0-9.-]*)\/mail\/vf-/gi)].map((m) => m[1]))].slice(0, 4),
    linkHosts: [...new Set([...html.matchAll(/https?:\/\/([a-z0-9.-]+)\/[a-z]*\/?(vf|[a-z]{0,6})/gi)].map((m) => `${m[1]}/${m[2]}`))].slice(0, 8),
    codeLabelText: around(text, "code"),
    codeLabelHtml: around(plain, "code"),
    confirmHtml: around(plain, "confirm"),
  });
}
