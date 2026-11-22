import { x25519 } from "@noble/curves/ed25519.js";
import { hexToBytes, utf8ToBytes } from "@noble/hashes/utils.js";
import { describe, expect, it } from "vitest";

import { fromBase64Url, toBase64Url } from "./base64";
import vector from "../../../rust/vtop-shared/vectors/mail_seal.json";
import { isPublicKey, open, seal, sealWith } from "./mail-seal";

// The inputs and box vtop-shared's `vector_matches_the_worker` checks too:
// the app and the engine open what the edge seals only while these agree.
const VECTOR_SECRET = hexToBytes(vector.secret);
const VECTOR_EPHEMERAL = hexToBytes(vector.ephemeral);
const VECTOR_NONCE = hexToBytes(vector.nonce);
const VECTOR_ID = vector.mailboxId;
const VECTOR_PLAINTEXT = vector.plaintext;
const VECTOR_PUBLIC = vector.public;
const VECTOR_BOX = vector.box;

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
