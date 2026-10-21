import { describe, expect, it } from "vitest";

import { firstSixDigitCode, payloadFor } from "./mail";

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

describe("mail", () => {
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
    expect(await payloadFor(mail, 1)).toEqual({ kind: "otp", code: "112233", sentAt: 1792231200 });
  });

  it("does not read styles and scripts", async () => {
    const mail = raw(
      "noreply.sdc@vitap.ac.in",
      "VTOP Login OTP",
      "<style>.a{width:100000px}</style><script>var x=999999</script>OTP 445566",
      "text/html",
    );
    expect(await payloadFor(mail, 1)).toMatchObject({ kind: "otp", code: "445566" });
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
    expect(await payloadFor(mail, 1)).toEqual({
      kind: "gmail_confirm",
      code: "987654321",
      link: "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123",
      requester: "student@gmail.com",
      sentAt: 1792231200,
    });
  });

  it("drops other senders and mail without a code", async () => {
    expect(await payloadFor(raw("spam@example.com", "OTP 123456", "123456", "text/plain"), 1)).toBeNull();
    expect(await payloadFor(raw("noreply.sdc@vitap.ac.in", "Hello", "no code here", "text/plain"), 1)).toBeNull();
  });

  it("uses now without a usable date", async () => {
    const mail = raw("noreply.sdc@vitap.ac.in", "OTP", "112233", "text/plain").replace(
      "Date: Fri, 17 Oct 2026 10:00:00 +0000",
      "Date: someday",
    );
    expect(await payloadFor(mail, 42)).toMatchObject({ sentAt: 42 });
  });
});
