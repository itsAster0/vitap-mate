import { x25519 } from "@noble/curves/ed25519.js";
import { describe, expect, it } from "vitest";

import { toBase64Url } from "./shared/base64";
import {
  MAX_LABEL_LEN,
  MAX_MEMBERS,
  MEMBER_STALE_SECS,
  type MailboxDoc,
  type MailboxMember,
  SEALED_TTL_SECS,
  address,
  addMember,
  clearExpired,
  fcmToken,
  freshSealed,
  idFromAddress,
  isId,
  memberBySecret,
  memberLabel,
  newId,
  nextExpiry,
  publicKey,
  setPublicKey,
  touch,
} from "./state";
import { isDeviceId } from "./common";

function member(device: string, seenAt: number): MailboxMember {
  return {
    deviceId: device,
    secretHash: `hash-${device}`,
    publicKey: `key-${device}`,
    label: null,
    sealed: null,
    sealedAt: null,
    addedAt: seenAt,
    seenAt,
  };
}

function mailbox(members: MailboxMember[]): MailboxDoc {
  return { id: "abcdefghij", ownerHash: null, members, createdAt: 0, updatedAt: 0 };
}

describe("mailbox ids", () => {
  it("round trip through addresses", () => {
    const id = newId();
    expect(isId(id)).toBe(true);
    expect(idFromAddress(address(id, "aster0.dev"))).toBe(id);
    expect(idFromAddress(` VTOP-OTP-${id.toUpperCase()}@Aster0.dev`)).toBe(id);
    expect(idFromAddress("someone@aster0.dev")).toBeNull();
    expect(idFromAddress("vtop-otp-short@aster0.dev")).toBeNull();
  });
});

describe("mailboxes", () => {
  it("let sealed boxes expire", () => {
    const m = member("a", 0);
    m.sealed = "AQ";
    m.sealedAt = 1000;
    expect(freshSealed(m, 1000 + SEALED_TTL_SECS)).toEqual({ sealed: "AQ", at: 1000 });
    expect(freshSealed(m, 1001 + SEALED_TTL_SECS)).toBeNull();
    const doc = mailbox([m]);
    expect(nextExpiry(doc)).toBe(1001 + SEALED_TTL_SECS);
    expect(clearExpired(doc, 1000 + SEALED_TTL_SECS)).toBe(false);
    expect(clearExpired(doc, 1001 + SEALED_TTL_SECS)).toBe(true);
    expect(doc.members[0].sealed).toBeNull();
  });

  it("drop the box on a new key", () => {
    const m = member("a", 0);
    m.sealed = "AQ";
    m.sealedAt = 1;
    setPublicKey(m, "key-a");
    expect(m.sealed).not.toBeNull();
    setPublicKey(m, "k2");
    expect([m.sealed, m.sealedAt]).toEqual([null, null]);
  });

  it("find members by secret", () => {
    const doc = mailbox([member("a", 0), member("b", 0)]);
    expect(memberBySecret(doc, "hash-b")).toBe(1);
    expect(memberBySecret(doc, "hash-c")).toBeNull();
  });

  it("replace the entry of a device joining again", () => {
    const now = 1_000_000;
    const doc = mailbox([member("a", now), member("b", now)]);
    addMember(doc, { ...member("a", now), publicKey: "new" }, now);
    expect(doc.members.length).toBe(2);
    expect(doc.members[1].deviceId).toBe("a");
    expect(doc.members[1].publicKey).toBe("new");
  });

  it("make room by dropping the least recently seen", () => {
    const now = 1_000_000;
    const doc = mailbox(Array.from({ length: MAX_MEMBERS }, (_, i) => member(`d${i}`, now - 100 + i)));
    addMember(doc, member("new", now), now);
    expect(doc.members.length).toBe(MAX_MEMBERS);
    expect(doc.members.every((m) => m.deviceId !== "d0")).toBe(true);
    expect(doc.members.at(-1)?.deviceId).toBe("new");
  });

  it("drop quiet devices when one joins", () => {
    const now = MEMBER_STALE_SECS * 2;
    const doc = mailbox([member("old", 1), member("recent", now - 10)]);
    addMember(doc, member("new", now), now);
    expect(doc.members.map((m) => m.deviceId)).toEqual(["recent", "new"]);
  });

  it("write visits at most hourly", () => {
    const doc = mailbox([member("a", 0)]);
    expect(touch(doc, 0, 10_000)).toBe(true);
    expect(touch(doc, 0, 10_000 + 60)).toBe(false);
    expect(touch(doc, 0, 10_000 + 3600)).toBe(true);
  });

  it("trim and cap labels", () => {
    expect(memberLabel("  Pixel 8 ")).toBe("Pixel 8");
    expect(memberLabel("  ")).toBeNull();
    expect(memberLabel(undefined)).toBeNull();
    expect(memberLabel("x".repeat(100))?.length).toBe(MAX_LABEL_LEN);
  });
});

describe("inputs", () => {
  it("are checked", () => {
    expect(isDeviceId("a".repeat(32))).toBe(true);
    expect(isDeviceId("A".repeat(32))).toBe(false);
    expect(isDeviceId("abc")).toBe(false);
    expect(fcmToken("  t ")).toEqual({ ok: true, value: "t" });
    expect(fcmToken(" ").ok).toBe(false);
    expect(fcmToken(undefined)).toEqual({ ok: true, value: null });
    expect(publicKey("nope").ok).toBe(false);
    const key = toBase64Url(x25519.keygen().publicKey);
    expect(publicKey(key)).toEqual({ ok: true, value: key });
  });
});
