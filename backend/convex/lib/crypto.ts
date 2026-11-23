// Hashes and random secrets with Web Crypto only, which every Convex runtime
// has. `sha256Hex` and `hashesEqual` match vtop-shared's `tokens`, so stored
// hashes line up with the engine's and the old edge's.

import { toBase64Url } from "./base64";

const utf8 = new TextEncoder();

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

export function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}

export async function sha256(data: string | Uint8Array): Promise<Uint8Array> {
  const bytes = typeof data === "string" ? utf8.encode(data) : new Uint8Array(data);
  return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
}

export async function sha256Hex(data: string | Uint8Array): Promise<string> {
  return bytesToHex(await sha256(data));
}

/** Compares two hashes without an early exit. */
export function hashesEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

export function randomBytes(n: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(n));
}

export function randomHex(n: number): string {
  return bytesToHex(randomBytes(n));
}

export function randomBase64Url(n: number): string {
  return toBase64Url(randomBytes(n));
}

/** The device id of an Ed25519 signing key: the first 16 bytes of its SHA-256, hex. */
export async function deviceIdFor(signKey: Uint8Array): Promise<string> {
  return (await sha256Hex(signKey)).slice(0, 32);
}
