import { x25519 } from "@noble/curves/ed25519.js";
import { utf8ToBytes } from "@noble/hashes/utils.js";
import { describe, expect, it } from "vitest";

import { fromBase64Url, toBase64Url } from "./base64";
import { isPublicKey, open, seal, sealWith } from "./mail-seal";

// The inputs and box of vtop-shared's `vector_matches_the_worker`: the app
// and the engine open what the edge seals only while these agree.
const VECTOR_SECRET = new Uint8Array(32).fill(7);
const VECTOR_EPHEMERAL = new Uint8Array(32).fill(9);
const VECTOR_NONCE = new Uint8Array(12).fill(3);
const VECTOR_ID = "k3f9x2ab7q";
const VECTOR_PLAINTEXT = '{"kind":"otp","code":"401295","sentAt":1760000000}';
const VECTOR_PUBLIC = "E75P6uryBMf9M1j8nAByGIHRdCeBKCJ-xnTzf3_pe20";
const VECTOR_BOX =
  "AVfbSzWfI65eFG5OJRIFZwRyJQY0jBUMFHU9DJM9BNQhAwMDAwMDAwMDAwMDnR9M8IRFhOh0oPy3nhcTWiVq8Xs8ocPlv23PRQgSBmfqtiQ3PzC4XkT7cks8QLqiNiJ4346hVr5T_nZryOFoQRCE";

function keypair() {
  const { secretKey, publicKey } = x25519.keygen();
  return { secret: toBase64Url(secretKey), public: toBase64Url(publicKey) };
}

describe("mail seal", () => {
  it("matches the Rust test vector byte for byte", () => {
    expect(toBase64Url(x25519.getPublicKey(VECTOR_SECRET))).toBe(VECTOR_PUBLIC);
    const sealed = sealWith(VECTOR_PUBLIC, VECTOR_ID, utf8ToBytes(VECTOR_PLAINTEXT), VECTOR_EPHEMERAL, VECTOR_NONCE);
    expect(sealed).toBe(VECTOR_BOX);
    const opened = open(toBase64Url(VECTOR_SECRET), VECTOR_ID, VECTOR_BOX);
    expect(new TextDecoder().decode(opened!)).toBe(VECTOR_PLAINTEXT);
  });

  it("round trips", () => {
    const keys = keypair();
    expect(isPublicKey(keys.public)).toBe(true);
    const sealed = seal(keys.public, "abc", utf8ToBytes("401295"))!;
    expect(new TextDecoder().decode(open(keys.secret, "abc", sealed)!)).toBe("401295");
  });

  it("rejects the wrong mailbox or key", () => {
    const keys = keypair();
    const sealed = seal(keys.public, "abc", utf8ToBytes("x"))!;
    expect(open(keys.secret, "abd", sealed)).toBeNull();
    expect(open(keypair().secret, "abc", sealed)).toBeNull();
  });

  it("rejects altered, short and future boxes", () => {
    const keys = keypair();
    const bytes = fromBase64Url(seal(keys.public, "abc", utf8ToBytes("x"))!)!;
    bytes[bytes.length - 1] ^= 1;
    expect(open(keys.secret, "abc", toBase64Url(bytes))).toBeNull();
    expect(open(keys.secret, "abc", "AQ")).toBeNull();
    expect(open(keys.secret, "abc", "not base64!")).toBeNull();
    bytes[0] = 2;
    expect(open(keys.secret, "abc", toBase64Url(bytes))).toBeNull();
  });

  it("checks public keys", () => {
    expect(isPublicKey("nope")).toBe(false);
    expect(isPublicKey(` ${VECTOR_PUBLIC}= `)).toBe(true);
    expect(seal("nope", "abc", utf8ToBytes("x"))).toBeNull();
  });
});
