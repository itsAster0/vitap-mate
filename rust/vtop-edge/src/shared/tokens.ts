// Random secrets and how they are stored: only their SHA-256, as hex.
// `hash` and `hashesEqual` match vtop-shared's `tokens`, which the engine
// uses to check the secret the edge sends it.

import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, randomBytes, utf8ToBytes } from "@noble/hashes/utils.js";

/** Personal access key for clients (extension, MCP). */
export const KEY_PREFIX = "vtm_";
/** The linked app's secret; proves account changes come from that phone. */
export const APP_SECRET_PREFIX = "vta_";
/** The secret a forwarding mailbox's phone holds. */
export const MAILBOX_SECRET_PREFIX = "vmb_";
/** The secret an app install's event channel uses. */
export const DEVICE_SECRET_PREFIX = "vdv_";

/** `prefix` + 64 hex characters from the system's random source. */
export function randomToken(prefix: string): string {
  return prefix + bytesToHex(randomBytes(32));
}

export function hash(secret: string): string {
  return bytesToHex(sha256(utf8ToBytes(secret)));
}

/** Compares two hashes without an early exit. */
export function hashesEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}
