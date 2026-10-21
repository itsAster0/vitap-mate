import { describe, expect, it } from "vitest";

import { MAILBOX_SECRET_PREFIX, hash, hashesEqual, randomToken } from "./tokens";
import { accountId } from "./engine-api";

describe("tokens", () => {
  it("hashes as vtop-shared does", () => {
    // vtop-shared's `hashes_are_hex_sha256`.
    expect(hash("a")).toBe("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
  });

  it("makes prefixed random tokens", () => {
    const token = randomToken(MAILBOX_SECRET_PREFIX);
    expect(token).toMatch(/^vmb_[0-9a-f]{64}$/);
    expect(token).not.toBe(randomToken(MAILBOX_SECRET_PREFIX));
  });

  it("compares whole hashes", () => {
    expect(hashesEqual(hash("a"), hash("a"))).toBe(true);
    expect(hashesEqual(hash("a"), hash("b"))).toBe(false);
    expect(hashesEqual("ab", "abc")).toBe(false);
  });

  it("makes account ids as the engine does", () => {
    expect(accountId(" 23abc4567 ")).toBe("23ABC4567");
  });
});
