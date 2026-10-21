// Sealed boxes for forwarded OTP mail: the edge seals, the app and the
// engine open with vtop-shared's `mail_seal`. The format must match it
// byte for byte (see the test vector in mail-seal.test.ts).
//
// Box v1, base64url without padding:
// `0x01 | ephemeral public key (32) | nonce (12) | AES-256-GCM ciphertext`.
// The key is HKDF-SHA256 over the X25519 shared secret, salted with the
// ephemeral and recipient public keys, info `INFO`. The mailbox id is the
// associated data, so a box cannot be replayed to another mailbox.

import { gcm } from "@noble/ciphers/aes.js";
import { x25519 } from "@noble/curves/ed25519.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { randomBytes, utf8ToBytes } from "@noble/hashes/utils.js";

import { fromBase64Url, toBase64Url } from "./base64";

export const VERSION = 1;
export const INFO = utf8ToBytes("vitapmate mail-otp v1");
const HEADER_LEN = 1 + 32 + 12;

function key32(encoded: string): Uint8Array | null {
  const bytes = fromBase64Url(encoded.trim().replace(/=+$/, ""));
  return bytes?.length === 32 ? bytes : null;
}

/** True for a base64url X25519 public key. */
export function isPublicKey(encoded: string): boolean {
  return key32(encoded) !== null;
}

function aesKey(shared: Uint8Array, ephemeral: Uint8Array, recipient: Uint8Array): Uint8Array {
  const salt = new Uint8Array(64);
  salt.set(ephemeral, 0);
  salt.set(recipient, 32);
  return hkdf(sha256, shared, salt, INFO, 32);
}

/** Seals with a fresh ephemeral key and nonce; null for a bad key. */
export function seal(recipientPublic: string, mailboxId: string, plaintext: Uint8Array): string | null {
  return sealWith(recipientPublic, mailboxId, plaintext, randomBytes(32), randomBytes(12));
}

/** Seals with the given ephemeral secret and nonce. Only for test vectors:
 * reusing either breaks the encryption. */
export function sealWith(
  recipientPublic: string,
  mailboxId: string,
  plaintext: Uint8Array,
  ephemeralSecret: Uint8Array,
  nonce: Uint8Array,
): string | null {
  const recipient = key32(recipientPublic);
  if (!recipient) return null;
  const ephemeralPublic = x25519.getPublicKey(ephemeralSecret);
  const shared = x25519.getSharedSecret(ephemeralSecret, recipient);
  const key = aesKey(shared, ephemeralPublic, recipient);
  const ciphertext = gcm(key, nonce, utf8ToBytes(mailboxId)).encrypt(plaintext);
  const out = new Uint8Array(HEADER_LEN + ciphertext.length);
  out[0] = VERSION;
  out.set(ephemeralPublic, 1);
  out.set(nonce, 33);
  out.set(ciphertext, HEADER_LEN);
  return toBase64Url(out);
}

/** Opens a box sealed to `secret`; null when it does not open. The edge
 * never opens boxes: this is for the tests. */
export function open(secret: string, mailboxId: string, sealed: string): Uint8Array | null {
  const secretKey = key32(secret);
  const bytes = fromBase64Url(sealed.trim().replace(/=+$/, ""));
  if (!secretKey || !bytes || bytes.length < HEADER_LEN + 16 || bytes[0] !== VERSION) return null;
  const ephemeral = bytes.subarray(1, 33);
  const recipient = x25519.getPublicKey(secretKey);
  const key = aesKey(x25519.getSharedSecret(secretKey, ephemeral), ephemeral, recipient);
  try {
    return gcm(key, bytes.subarray(33, HEADER_LEN), utf8ToBytes(mailboxId)).decrypt(bytes.subarray(HEADER_LEN));
  } catch {
    return null;
  }
}
