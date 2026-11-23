// Sealed boxes for forwarded OTP mail, the format of vtop-shared's
// `mail_seal` byte for byte (the test vector in mailSeal.test.ts), so the
// app and the engine open them as they open the old edge's.
//
// Box v1, base64url without padding:
// `0x01 | ephemeral public key (32) | nonce (12) | AES-256-GCM ciphertext`.
// The key is HKDF-SHA256 over the X25519 shared secret, salted with the
// ephemeral and recipient public keys, info `INFO`. The mailbox id is the
// associated data, so a box cannot be replayed to another mailbox.

import { fromBase64Url, toBase64Url } from "./base64";
import { randomBytes } from "./crypto";

export const VERSION = 1;
export const INFO = new TextEncoder().encode("vitapmate mail-otp v1");
const HEADER_LEN = 1 + 32 + 12;
// PKCS#8 for an X25519 private key: Web Crypto imports raw secrets only this way.
const PKCS8_X25519 = [0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x04, 0x22, 0x04, 0x20];

function key32(encoded: string): Uint8Array | null {
  const bytes = fromBase64Url(encoded.trim().replace(/=+$/, ""));
  return bytes?.length === 32 ? bytes : null;
}

/** True for a base64url X25519 public key. */
export function isPublicKey(encoded: string): boolean {
  return key32(encoded) !== null;
}

async function privateKey(secret: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey("pkcs8", new Uint8Array([...PKCS8_X25519, ...secret]), { name: "X25519" }, true, ["deriveBits"]);
}

/** The X25519 public key of a 32-byte secret. */
export async function publicKeyOf(secret: Uint8Array): Promise<Uint8Array> {
  const jwk = await crypto.subtle.exportKey("jwk", await privateKey(secret));
  return fromBase64Url(jwk.x ?? "") ?? new Uint8Array();
}

async function shared(secret: Uint8Array, peer: Uint8Array): Promise<Uint8Array> {
  const pub = await crypto.subtle.importKey("raw", new Uint8Array(peer), { name: "X25519" }, false, []);
  return new Uint8Array(await crypto.subtle.deriveBits({ name: "X25519", public: pub }, await privateKey(secret), 256));
}

async function aesKey(secret: Uint8Array, ephemeral: Uint8Array, recipient: Uint8Array, usage: KeyUsage): Promise<CryptoKey> {
  const salt = new Uint8Array(64);
  salt.set(ephemeral, 0);
  salt.set(recipient, 32);
  const ikm = await crypto.subtle.importKey("raw", new Uint8Array(secret), "HKDF", false, ["deriveBits"]);
  const bits = await crypto.subtle.deriveBits({ name: "HKDF", hash: "SHA-256", salt, info: INFO }, ikm, 256);
  return crypto.subtle.importKey("raw", bits, "AES-GCM", false, [usage]);
}

/** Seals with a fresh ephemeral key and nonce; null for a bad key. */
export function seal(recipientPublic: string, mailboxId: string, plaintext: Uint8Array): Promise<string | null> {
  return sealWith(recipientPublic, mailboxId, plaintext, randomBytes(32), randomBytes(12));
}

/** Seals with the given ephemeral secret and nonce. Only for test vectors:
 * reusing either breaks the encryption. */
export async function sealWith(
  recipientPublic: string,
  mailboxId: string,
  plaintext: Uint8Array,
  ephemeralSecret: Uint8Array,
  nonce: Uint8Array,
): Promise<string | null> {
  const recipient = key32(recipientPublic);
  if (!recipient) return null;
  const ephemeralPublic = await publicKeyOf(ephemeralSecret);
  const key = await aesKey(await shared(ephemeralSecret, recipient), ephemeralPublic, recipient, "encrypt");
  const aad = new TextEncoder().encode(mailboxId);
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv: new Uint8Array(nonce), additionalData: aad }, key, new Uint8Array(plaintext)));
  const out = new Uint8Array(HEADER_LEN + ciphertext.length);
  out[0] = VERSION;
  out.set(ephemeralPublic, 1);
  out.set(nonce, 33);
  out.set(ciphertext, HEADER_LEN);
  return toBase64Url(out);
}

/** Opens a box sealed to `secret`; null when it does not open. The backend
 * never opens boxes: this is for the tests. */
export async function open(secret: string, mailboxId: string, sealed: string): Promise<Uint8Array | null> {
  const secretKey = key32(secret);
  const bytes = fromBase64Url(sealed.trim().replace(/=+$/, ""));
  if (!secretKey || !bytes || bytes.length < HEADER_LEN + 16 || bytes[0] !== VERSION) return null;
  try {
    const ephemeral = bytes.subarray(1, 33);
    const recipient = await publicKeyOf(secretKey);
    const key = await aesKey(await shared(secretKey, ephemeral), ephemeral, recipient, "decrypt");
    const aad = new TextEncoder().encode(mailboxId);
    const iv = new Uint8Array(bytes.subarray(33, HEADER_LEN));
    return new Uint8Array(await crypto.subtle.decrypt({ name: "AES-GCM", iv, additionalData: aad }, key, new Uint8Array(bytes.subarray(HEADER_LEN))));
  } catch {
    return null;
  }
}
