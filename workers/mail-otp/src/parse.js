// What a forwarded mail means for the phone. Only two kinds are relayed:
// VTOP's OTP mail, and Gmail's forwarding confirmation (needed once during
// setup). Everything else is dropped.

export const VTOP_SENDER = "noreply.sdc@vitap.ac.in";
export const GMAIL_FORWARDING_SENDER = "forwarding-noreply@google.com";
const RECIPIENT = /^vtop-otp-([a-km-np-z2-9]{10})@/i;

/** The mailbox id in `vtop-otp-<id>@domain`, or null. */
export function mailboxId(recipient) {
  const match = RECIPIENT.exec(recipient.trim());
  return match ? match[1].toLowerCase() : null;
}

/** The first run of exactly six digits, as vtop-core's Gmail reader. */
export function firstSixDigitCode(text) {
  for (const match of text.matchAll(/\d+/g)) {
    if (match[0].length === 6) return match[0];
  }
  return null;
}

function htmlToText(html) {
  return html
    .replace(/<(style|script)[\s\S]*?<\/\1>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&");
}

function sentAt(email) {
  const parsed = Date.parse(email.date ?? "");
  return Number.isFinite(parsed) ? Math.floor(parsed / 1000) : Math.floor(Date.now() / 1000);
}

/**
 * The plaintext to seal for a parsed mail (postal-mime shape), or null to
 * drop it.
 */
export function payloadFor(email) {
  const from = (email.from?.address ?? "").toLowerCase();
  const text = [email.subject ?? "", email.text ?? "", htmlToText(email.html ?? "")].join("\n");

  if (from === VTOP_SENDER) {
    const code = firstSixDigitCode(text);
    return code ? { kind: "otp", code, sentAt: sentAt(email) } : null;
  }
  if (from === GMAIL_FORWARDING_SENDER) {
    const code = /confirmation code:\s*(\d+)/i.exec(text)?.[1] ?? /\(#(\d+)\)/.exec(text)?.[1];
    const link = /https:\/\/mail-settings\.google\.com\/mail\/vf-[^\s"'<>]+/.exec(text)?.[0];
    const requester = /receive mail from\s+(\S+@\S+)/i.exec(email.subject ?? "")?.[1];
    if (!code && !link) return null;
    return {
      kind: "gmail_confirm",
      code: code ?? null,
      link: link ?? null,
      requester: requester ?? null,
      sentAt: sentAt(email),
    };
  }
  return null;
}
