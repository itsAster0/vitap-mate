import { describe, expect, it } from "vitest";

import { firstSixDigitCode, idFromAddress, isId, parseMail } from "./parse";

/** A minute after the test mails' `Date:`. */
const NOW = 1792231260;

function raw(from: string, subject: string, body: string, contentType: string): string {
  return [
    `From: ${from}`,
    "To: someone@gmail.com",
    `Subject: ${subject}`,
    "Date: Fri, 17 Oct 2026 10:00:00 +0000",
    "MIME-Version: 1.0",
    `Content-Type: ${contentType}; charset=UTF-8`,
    "Content-Transfer-Encoding: quoted-printable",
    "",
    body,
  ].join("\r\n");
}

describe("parseMail", () => {
  it("reads the mailbox id from otp+<id>@ addresses", () => {
    const id = "k3f9x2ab7q";
    expect(isId(id)).toBe(true);
    expect(idFromAddress(`otp+${id}@vmate.aster0.dev`)).toBe(id);
    expect(idFromAddress(` OTP+${id.toUpperCase()}@Vmate.Aster0.dev`)).toBe(id);
    expect(idFromAddress("someone@aster0.dev")).toBeNull();
    expect(idFromAddress("otp@vmate.aster0.dev")).toBeNull();
    expect(idFromAddress("otp+short@vmate.aster0.dev")).toBeNull();
    // l, o, 0 and 1 are never in an id.
    expect(idFromAddress("otp+k3f9x2ab7l@vmate.aster0.dev")).toBeNull();
  });

  it("reads six-digit codes as vtop-core does", () => {
    expect(firstSixDigitCode("Your OTP is 401295. Valid 5 min")).toBe("401295");
    expect(firstSixDigitCode("ref 12345678 then 654321")).toBe("654321");
    expect(firstSixDigitCode("no code 12345")).toBeNull();
  });

  it("finds a VTOP OTP in quoted-printable HTML", async () => {
    const mail = raw(
      "VTOP <noreply.sdc@vitap.ac.in>",
      "VTOP Login OTP",
      '<p style=3D"x">Ref 2026 Your OTP is <b>112233</b></p>',
      "text/html",
    );
    expect(await parseMail(mail, NOW)).toEqual({
      kind: "otp",
      code: "112233",
      sentAt: 1792231200,
    });
  });

  it("does not read styles and scripts", async () => {
    const mail = raw(
      "noreply.sdc@vitap.ac.in",
      "VTOP Login OTP",
      "<style>.a{width:100000px}</style><script>var x=999999</script>OTP 445566",
      "text/html",
    );
    expect(await parseMail(mail, NOW)).toMatchObject({ kind: "otp", code: "445566" });
  });

  it("reads Gmail's forwarding confirmation", async () => {
    const body = [
      "student@gmail.com has requested to automatically forward mail to your email",
      "Confirmation code: 987654321",
      "To allow, click the link below:",
      "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123",
    ].join("\r\n");
    const mail = raw(
      "Gmail Team <forwarding-noreply@google.com>",
      "(#987654321) Gmail Forwarding Confirmation - Receive Mail from student@gmail.com",
      body,
      "text/plain",
    );
    expect(await parseMail(mail, NOW)).toMatchObject({
      kind: "gmail_confirm",
      confirm: {
        code: "987654321",
        link: "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123",
        requester: "student@gmail.com",
      },
      sentAt: 1792231200,
    });
  });

  it("reads a Workspace forwarding confirmation with only a link", async () => {
    const body = [
      "student@vitapstudent.ac.in has requested to automatically forward mail to your email",
      "To allow, click the link below:",
      "https://mail.google.com/mail/vf-%5BANGjdJ8%5D-xyz789",
    ].join("\r\n");
    const mail = raw(
      "forwarding-noreply@google.com",
      "(VIT-AP University Forwarding Confirmation - Receive Mail from student@vitapstudent.ac.in",
      body,
      "text/plain",
    );
    expect(await parseMail(mail, NOW)).toMatchObject({
      kind: "gmail_confirm",
      confirm: {
        code: null,
        link: "https://mail.google.com/mail/vf-%5BANGjdJ8%5D-xyz789",
        requester: "student@vitapstudent.ac.in",
      },
      sentAt: 1792231200,
    });
  });

  it("ignores confirm links outside google.com or without https", async () => {
    const body =
      "Click https://mail.google.com.evil.example/mail/vf-abc or https://evilgoogle.com/mail/vf-abc or http://mail.google.com/mail/vf-abc or https://www.google.com/mail/vf-/../../url?q=https://evil.example";
    const mail = raw("forwarding-noreply@google.com", "Forwarding Confirmation", body, "text/plain");
    expect(await parseMail(mail, NOW)).toBeNull();
  });

  it("drops other senders and mail without a code", async () => {
    expect(await parseMail(raw("spam@example.com", "OTP 123456", "123456", "text/plain"), NOW)).toBeNull();
    expect(await parseMail(raw("noreply.sdc@vitap.ac.in", "Hello", "no code here", "text/plain"), NOW)).toBeNull();
  });

  it("uses now without a usable date", async () => {
    const mail = raw("noreply.sdc@vitap.ac.in", "OTP", "112233", "text/plain").replace(
      "Date: Fri, 17 Oct 2026 10:00:00 +0000",
      "Date: someday",
    );
    expect(await parseMail(mail, 42)).toMatchObject({ sentAt: 42 });
  });

  it("never dates a mail after now", async () => {
    const mail = raw("noreply.sdc@vitap.ac.in", "OTP", "112233", "text/plain").replace(
      "Date: Fri, 17 Oct 2026 10:00:00 +0000",
      "Date: Fri, 1 Jan 2100 00:00:00 +0000",
    );
    expect(await parseMail(mail, NOW)).toMatchObject({ sentAt: NOW });
  });
});
