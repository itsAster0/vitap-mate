import { test } from "node:test";
import assert from "node:assert/strict";
import PostalMime from "postal-mime";
import { firstSixDigitCode, mailboxId, payloadFor } from "../src/parse.js";

test("mailbox ids", () => {
  assert.equal(mailboxId("vtop-otp-k3f9x2ab7q@aster0.dev"), "k3f9x2ab7q");
  assert.equal(mailboxId("VTOP-OTP-K3F9X2AB7Q@aster0.dev"), "k3f9x2ab7q");
  assert.equal(mailboxId("hello@aster0.dev"), null);
  assert.equal(mailboxId("vtop-otp-short@aster0.dev"), null);
  assert.equal(mailboxId("vtop-otp-k3f9x2ab7l@aster0.dev"), null); // no "l"
});

test("six digit rule matches vtop-core", () => {
  assert.equal(firstSixDigitCode("Your OTP is 401295. Valid 5 min"), "401295");
  assert.equal(firstSixDigitCode("ref 12345678 then 654321"), "654321");
  assert.equal(firstSixDigitCode("no code 12345"), null);
});

const raw = (from, subject, body, type = "text/plain") =>
  [
    `From: ${from}`,
    "To: someone@gmail.com",
    `Subject: ${subject}`,
    "Date: Fri, 17 Oct 2026 10:00:00 +0000",
    "MIME-Version: 1.0",
    `Content-Type: ${type}; charset=UTF-8`,
    "Content-Transfer-Encoding: quoted-printable",
    "",
    body,
  ].join("\r\n");

test("VTOP OTP mail, quoted-printable HTML", async () => {
  const email = await PostalMime.parse(
    raw(
      "VTOP <noreply.sdc@vitap.ac.in>",
      "VTOP Login OTP",
      '<p style=3D"x">Ref 2026 Your OTP is <b>112233</b></p>',
      "text/html",
    ),
  );
  assert.deepEqual(payloadFor(email), { kind: "otp", code: "112233", sentAt: 1792231200 });
});

test("Gmail forwarding confirmation", async () => {
  const email = await PostalMime.parse(
    raw(
      "Gmail Team <forwarding-noreply@google.com>",
      "(#987654321) Gmail Forwarding Confirmation - Receive Mail from student@gmail.com",
      [
        "student@gmail.com has requested to automatically forward mail to your email",
        "Confirmation code: 987654321",
        "To allow, click the link below:",
        "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123",
      ].join("\r\n"),
    ),
  );
  const payload = payloadFor(email);
  assert.equal(payload.kind, "gmail_confirm");
  assert.equal(payload.code, "987654321");
  assert.equal(payload.link, "https://mail-settings.google.com/mail/vf-%5BANGjdJ8%5D-abc123");
  assert.equal(payload.requester, "student@gmail.com");
});

test("other senders are dropped", async () => {
  const email = await PostalMime.parse(raw("spam@example.com", "OTP 123456", "123456"));
  assert.equal(payloadFor(email), null);
});
