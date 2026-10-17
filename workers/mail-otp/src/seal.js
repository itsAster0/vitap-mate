// Sealed boxes, the same format as rust/vtop-core/src/mail_seal.rs:
// base64url(0x01 | ephemeral public key (32) | nonce (12) | AES-256-GCM
// ciphertext), key = HKDF-SHA256(X25519 shared secret,
// salt = ephemeral public | recipient public, info = INFO), AAD = mailbox id.

import { x25519 } from "@noble/curves/ed25519.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";

const VERSION = 1;
const INFO = new TextEncoder().encode("vitapmate mail-otp v1");

export function fromBase64Url(text) {
  const base64 = text.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
  return Uint8Array.from(atob(padded), (char) => char.charCodeAt(0));
}

export function toBase64Url(bytes) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * Seals `plaintext` (a string) to `recipientPublic` (base64url) for
 * `mailboxId`. `ephemeralSecret` and `nonce` are only passed by tests.
 */
export async function seal(
  recipientPublic,
  mailboxId,
  plaintext,
  ephemeralSecret = crypto.getRandomValues(new Uint8Array(32)),
  nonce = crypto.getRandomValues(new Uint8Array(12)),
) {
  const recipient = fromBase64Url(recipientPublic);
  if (recipient.length !== 32) throw new Error("bad public key");
  const ephemeralPublic = x25519.getPublicKey(ephemeralSecret);
  const shared = x25519.getSharedSecret(ephemeralSecret, recipient);
  const salt = new Uint8Array(64);
  salt.set(ephemeralPublic, 0);
  salt.set(recipient, 32);
  const keyBytes = hkdf(sha256, shared, salt, INFO, 32);
  const key = await crypto.subtle.importKey("raw", keyBytes, "AES-GCM", false, ["encrypt"]);
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt(
      { name: "AES-GCM", iv: nonce, additionalData: new TextEncoder().encode(mailboxId) },
      key,
      new TextEncoder().encode(plaintext),
    ),
  );
  const out = new Uint8Array(1 + 32 + 12 + ciphertext.length);
  out[0] = VERSION;
  out.set(ephemeralPublic, 1);
  out.set(nonce, 33);
  out.set(ciphertext, 45);
  return toBase64Url(out);
}
