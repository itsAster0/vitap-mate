import { describe, expect, it } from "vitest";

import { KEY_PREFIX, hash, hashesEqual, randomToken } from "./tokens";
import { accountId } from "./engine-api";
import vectors from "../../../rust/vtop-shared/vectors/tokens.json";

describe("tokens", () => {
  it("hashes as vtop-shared does", () => {
    // vtop-shared's `hashes_are_hex_sha256`.
    for (const v of vectors.hashes) expect(hash(v.secret)).toBe(v.hash);
  });

  it("makes prefixed random tokens", () => {
    const token = randomToken(KEY_PREFIX);
    expect(token).toMatch(/^vtm_[0-9a-f]{64}$/);
    expect(token).not.toBe(randomToken(KEY_PREFIX));
  });

  it("compares whole hashes", () => {
    expect(hashesEqual(hash("a"), hash("a"))).toBe(true);
    expect(hashesEqual(hash("a"), hash("b"))).toBe(false);
    expect(hashesEqual("ab", "abc")).toBe(false);
  });

  it("makes account ids as the engine does", () => {
    for (const v of vectors.accountIds) expect(accountId(v.registrationNumber)).toBe(v.id);
  });
});
