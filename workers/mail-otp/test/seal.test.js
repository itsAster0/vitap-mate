import { test } from "node:test";
import assert from "node:assert/strict";
import { x25519 } from "@noble/curves/ed25519.js";
import { seal, toBase64Url } from "../src/seal.js";

// The vector from rust/vtop-core/src/mail_seal.rs.
const SECRET = new Uint8Array(32).fill(7);
const EPHEMERAL = new Uint8Array(32).fill(9);
const NONCE = new Uint8Array(12).fill(3);
const ID = "k3f9x2ab7q";
const PLAINTEXT = '{"kind":"otp","code":"401295","sentAt":1760000000}';
const PUBLIC = "E75P6uryBMf9M1j8nAByGIHRdCeBKCJ-xnTzf3_pe20";
const BOX =
  "AVfbSzWfI65eFG5OJRIFZwRyJQY0jBUMFHU9DJM9BNQhAwMDAwMDAwMDAwMDnR9M8IRFhOh0oPy3nhcTWiVq8Xs8ocPlv23PRQgSBmfqtiQ3PzC4XkT7cks8QLqiNiJ4346hVr5T_nZryOFoQRCE";

test("public key matches Rust", () => {
  assert.equal(toBase64Url(x25519.getPublicKey(SECRET)), PUBLIC);
});

test("box matches the Rust vector byte for byte", async () => {
  assert.equal(await seal(PUBLIC, ID, PLAINTEXT, EPHEMERAL, NONCE), BOX);
});

test("random boxes differ", async () => {
  assert.notEqual(await seal(PUBLIC, ID, PLAINTEXT), await seal(PUBLIC, ID, PLAINTEXT));
});
