// The sample bodies vtop-shared's `samples_round_trip` reads and writes
// back unchanged, checked against the edge's types: a field renamed on one
// side fails here or there.
import { describe, expect, it } from "vitest";

import samples from "../../../rust/vtop-shared/vectors/engine.json";
import { isLoginOutcome } from "../account";
import type {
  GmailGrant,
  LoginDone,
  LoginStart,
  OtpAwait,
  OtpAwaited,
  SealVault,
  SealedVault,
  SessionCheck,
  SessionChecked,
} from "./engine-api";

/** What a field holds; `?` when it may be left out. */
type Field = "string" | "string?" | "number" | "boolean?" | "nullable string" | "object" | "object?";

/** Every field of `T`, so tsc fails when the type gains or loses one. */
type Shape<T> = { [K in keyof Required<T>]: Field };

const GMAIL: Shape<GmailGrant> = {
  refreshToken: "string",
  clientId: "string",
  clientSecret: "string?",
  deleteAfterReading: "boolean?",
};

const SHAPES = {
  SessionCheck: { cookies: "string?", sealedSession: "string?", account: "string?" } satisfies Shape<SessionCheck>,
  SessionChecked: {
    status: "string",
    registrationNumber: "string?",
    cookies: "string?",
    csrfToken: "string?",
    sealedSession: "string?",
    ownerHash: "string?",
  } satisfies Shape<SessionChecked>,
  SealVault: { account: "string", username: "string", password: "string", gmail: "object?", otp: "string?" } satisfies Shape<SealVault>,
  SealedVault: { sealedVault: "string" } satisfies Shape<SealedVault>,
  LoginStart: { account: "string", sealedVault: "string", requestId: "string" } satisfies Shape<LoginStart>,
  LoginDone: { account: "string", requestId: "string", outcome: "object" } satisfies Shape<LoginDone>,
  OtpAwait: { ownerHash: "string", publicKey: "string", since: "number", waitSecs: "number" } satisfies Shape<OtpAwait>,
  OtpAwaited: { sealed: "nullable string" } satisfies Shape<OtpAwaited>,
};

/** The fields of `value` that break `shape`. */
function problems(value: Record<string, unknown>, shape: Record<string, Field>): string[] {
  const out = Object.keys(value)
    .filter((k) => !(k in shape))
    .map((k) => `unknown field ${k}`);
  for (const [key, field] of Object.entries(shape)) {
    const v = value[key];
    if (v === undefined) {
      if (!field.endsWith("?")) out.push(`missing ${key}`);
      continue;
    }
    const kind = field.replace("?", "").replace("nullable ", "");
    const ok = (field.startsWith("nullable") && v === null) || (typeof v === kind && v !== null);
    if (!ok) out.push(`${key} is not ${field}`);
  }
  return out;
}

type Sample = Record<string, unknown>;
const all = samples as unknown as Record<string, Sample[] | string>;

describe("engine samples", () => {
  for (const [name, shape] of Object.entries(SHAPES)) {
    it(`fit ${name}`, () => {
      const list = all[name] as Sample[];
      expect(list.length).toBeGreaterThan(0);
      for (const sample of list) expect(problems(sample, shape), JSON.stringify(sample)).toEqual([]);
    });
  }

  it("have only known statuses, OTP methods and Gmail grants", () => {
    for (const s of all.SessionChecked as Sample[]) expect(["valid", "expired", "unknown"]).toContain(s.status);
    for (const s of all.SealVault as Sample[]) {
      if (s.otp !== undefined) expect(["forwarding", "gmail"]).toContain(s.otp);
      if (s.gmail !== undefined) expect(problems(s.gmail as Sample, GMAIL)).toEqual([]);
    }
  });

  it("have login outcomes the edge accepts", () => {
    const statuses = new Set<string>();
    for (const s of all.LoginDone as Sample[]) {
      expect(isLoginOutcome(s.outcome), JSON.stringify(s.outcome)).toBe(true);
      statuses.add((s.outcome as { status: string }).status);
    }
    // One of each kind of `LoginOutcome`.
    expect([...statuses].sort()).toEqual(
      ["failed", "forwarding_required", "gmail_revoked", "invalid_credentials", "ok", "vault_unusable"],
    );
  });

  it("are all checked here or are the engine's own", () => {
    // `PageFetch` and `PageBatch` pass through the edge unread.
    const names = Object.keys(all).filter((k) => k !== "comment" && k !== "PageFetch" && k !== "PageBatch");
    expect(names.sort()).toEqual(Object.keys(SHAPES).sort());
  });
});
