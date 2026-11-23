// The shared test vectors in rust/vtop-shared/vectors: the app, the engine and
// the backend agree byte for byte only while these pass on every side.

import { describe, expect, it } from "vitest";
import deviceSig from "../../../rust/vtop-shared/vectors/device_sig.json";
import mailSeal from "../../../rust/vtop-shared/vectors/mail_seal.json";
import tokens from "../../../rust/vtop-shared/vectors/tokens.json";
import { fromBase64Url, toBase64Url } from "./base64";
import { deviceIdFor, hashesEqual, hexToBytes, sha256Hex } from "./crypto";
import { isPublicKey, open, publicKeyOf, seal, sealWith } from "./mailSeal";
import { accountId } from "./util";
import { signedMessage, verifyVm1 } from "./vm1";

async function x25519Pair() {
  const pair = (await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"])) as CryptoKeyPair;
  const jwk = await crypto.subtle.exportKey("jwk", pair.privateKey);
  return { secret: jwk.d!, public: jwk.x! };
}

describe("device_sig (vm1)", () => {
  it("builds the message and checks the signature of the vector", async () => {
    const body = new TextEncoder().encode(deviceSig.body);
    const message = await signedMessage(deviceSig.method, deviceSig.path, String(deviceSig.time), body);
    expect(new TextDecoder().decode(message)).toBe(deviceSig.message);
    expect(await sha256Hex(new Uint8Array())).toBe(deviceSig.emptyBodySha256);
    const key = fromBase64Url(deviceSig.public)!;
    const req = { method: deviceSig.method, pathWithQuery: deviceSig.path, time: String(deviceSig.time), sig: deviceSig.sig, body };
    expect(await verifyVm1(req, key, deviceSig.time)).toBe("ok");
    expect(await verifyVm1(req, key, deviceSig.time + 121)).toBe("stale_time");
    expect(await verifyVm1({ ...req, body: new TextEncoder().encode("{}") }, key, deviceSig.time)).toBe("bad_signature");
    expect(await verifyVm1({ ...req, sig: "short" }, key, deviceSig.time)).toBe("bad_signature");
    expect(await deviceIdFor(key)).toMatch(/^[0-9a-f]{32}$/);
  });
});

describe("mail seal", () => {
  it("matches the Rust test vector byte for byte", async () => {
    const secret = hexToBytes(mailSeal.secret);
    expect(toBase64Url(await publicKeyOf(secret))).toBe(mailSeal.public);
    const sealed = await sealWith(mailSeal.public, mailSeal.mailboxId, new TextEncoder().encode(mailSeal.plaintext), hexToBytes(mailSeal.ephemeral), hexToBytes(mailSeal.nonce));
    expect(sealed).toBe(mailSeal.box);
    const opened = await open(toBase64Url(secret), mailSeal.mailboxId, mailSeal.box);
    expect(new TextDecoder().decode(opened!)).toBe(mailSeal.plaintext);
  });

  it("round trips, and refuses the wrong mailbox, key or an altered box", async () => {
    const keys = await x25519Pair();
    expect(isPublicKey(keys.public)).toBe(true);
    const sealed = (await seal(keys.public, "abc", new TextEncoder().encode("401295")))!;
    expect(new TextDecoder().decode((await open(keys.secret, "abc", sealed))!)).toBe("401295");
    expect(await open(keys.secret, "abd", sealed)).toBeNull();
    expect(await open((await x25519Pair()).secret, "abc", sealed)).toBeNull();
    const bytes = fromBase64Url(sealed)!;
    bytes[bytes.length - 1] ^= 1;
    expect(await open(keys.secret, "abc", toBase64Url(bytes))).toBeNull();
    expect(await open(keys.secret, "abc", "AQ")).toBeNull();
  });

  it("checks public keys", async () => {
    expect(isPublicKey("nope")).toBe(false);
    expect(isPublicKey(` ${mailSeal.public}= `)).toBe(true);
    expect(await seal("nope", "abc", new TextEncoder().encode("x"))).toBeNull();
  });
});

describe("tokens", () => {
  it("hashes and makes account ids as vtop-shared does", async () => {
    for (const v of tokens.hashes) expect(await sha256Hex(v.secret)).toBe(v.hash);
    for (const v of tokens.accountIds) expect(accountId(v.registrationNumber)).toBe(v.id);
    expect(hashesEqual("ab", "ab")).toBe(true);
    expect(hashesEqual("ab", "abc")).toBe(false);
  });
});
