// The help content under /guides/ (Static Assets, see API.md): what the app
// parses, so a bad edit fails here instead of on phones.
import { describe, expect, it } from "vitest";

// Node's fs, typed by hand: the tsconfig has the Workers types only.
const { existsSync, readFileSync } = (await import("node:fs" as string)) as {
  existsSync(path: string): boolean;
  readFileSync(path: string, encoding: "utf8"): string;
};

const GUIDES = decodeURIComponent(new URL("../guides/", (import.meta as { url: string }).url).pathname);
const ICONS = ["mail-check", "refresh-cw", "triangle-alert", "info"];
const TONES = ["lab", "warning", "success", "danger", "accent"];

type Callout = { icon: string; tone: string; text: string };
type Block =
  | ({ type: "callout" } & Callout)
  | { type: "step"; title: string; body: string; image?: string; caption?: string; callout?: Callout };
type Forwarding = { schema: number; title: string; subtitle: string; blocks: Block[] };
type Section = { title: string; items: string[] };
type Feature = string | { text: string; since: number };
type Updates = {
  schema: number;
  features: { title: string; items: Feature[] }[];
  releases: { version: string; code: number; summary: string; sections: Section[] }[];
};

/** The versionCode of "x.y.z" before its build number, as `app/module.yaml` gives them (3.0.2 → 3000200, build 201 → 3000201). */
const codeOf = (version: string) => {
  const [major, minor, patch] = version.split(".").map(Number);
  return major * 1_000_000 + minor * 1_000 + patch * 100;
};

const read = <T>(name: string): T => JSON.parse(readFileSync(GUIDES + name, "utf8")) as T;

function expectCallout(c: Callout) {
  expect(ICONS).toContain(c.icon);
  expect(TONES).toContain(c.tone);
  expect(c.text).not.toBe("");
}

describe("guides/forwarding.json", () => {
  const guide = read<Forwarding>("forwarding.json");

  it("is schema 1 with a title", () => {
    expect(guide.schema).toBe(1);
    expect(guide.title).not.toBe("");
    expect(guide.blocks.length).toBeGreaterThan(0);
  });

  it("has known icons and tones, and every image exists", () => {
    for (const block of guide.blocks) {
      if (block.type === "callout") {
        expectCallout(block);
        continue;
      }
      expect(block.type).toBe("step");
      expect(block.title).not.toBe("");
      expect(block.body).not.toBe("");
      if (block.callout) expectCallout(block.callout);
      if (block.image !== undefined) {
        expect(block.image).not.toMatch(/^\/|\.\./);
        expect(existsSync(GUIDES + block.image), block.image).toBe(true);
      }
    }
  });

  it("asks the app for the sender in the filter step", () => {
    expect(guide.blocks.some((b) => b.type === "step" && b.body.includes("{sender}"))).toBe(true);
  });
});

describe("guides/updates.json", () => {
  const updates = read<Updates>("updates.json");
  const filled = (s: Section) => s.title !== "" && s.items.length > 0 && s.items.every((i) => i !== "");

  it("is schema 1 with features and releases", () => {
    expect(updates.schema).toBe(1);
    expect(updates.features.length).toBeGreaterThan(0);
    for (const section of updates.features) {
      expect(section.title).not.toBe("");
      expect(section.items.length).toBeGreaterThan(0);
      for (const item of section.items) {
        if (typeof item === "string") expect(item).not.toBe("");
        else {
          expect(item.text).not.toBe("");
          expect(Number.isInteger(item.since) && item.since >= 3_000_000).toBe(true);
        }
      }
    }
    expect(updates.releases.length).toBeGreaterThan(0);
  });

  it("has a version, summary and sections for each release", () => {
    for (const release of updates.releases) {
      expect(release.version).toMatch(/^\d+\.\d+\.\d+$/);
      expect(release.summary).not.toBe("");
      expect(release.sections.length).toBeGreaterThan(0);
      expect(release.sections.every(filled)).toBe(true);
    }
    expect(new Set(updates.releases.map((r) => r.version)).size).toBe(updates.releases.length);
  });

  it("gives each release its versionCode, newest first, so an app shows only what it has", () => {
    const codes = updates.releases.map((r) => r.code);
    for (const release of updates.releases) {
      expect(Number.isInteger(release.code)).toBe(true);
      // The build digits may be set; the version must match the rest.
      expect(release.code - (release.code % 100)).toBe(codeOf(release.version));
    }
    expect([...codes].sort((a, b) => b - a)).toEqual(codes);
    expect(new Set(codes).size).toBe(codes.length);
  });
});
