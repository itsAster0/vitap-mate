// rust/vtop-shared/vectors/server_codes.json lists the error codes the app matches (vtop-ffi's
// ServerCode; a cargo test keeps that enum and the list in step). Each one it expects from us
// has to be one these functions throw or answer with, and each one from the engine one the
// engine sends, or the app would wait for a code that never comes.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import codes from "../../../rust/vtop-shared/vectors/server_codes.json";

function sources(dir: string, ext: string): string {
  return readdirSync(dir)
    .map((name) => join(dir, name))
    .flatMap((path) => {
      if (statSync(path).isDirectory()) return name(path) === "_generated" ? [] : [sources(path, ext)];
      return path.endsWith(ext) && !path.endsWith(`.test${ext}`) && !path.endsWith("/tests.rs") ? [readFileSync(path, "utf8")] : [];
    })
    .join("\n");
}

const name = (path: string) => path.split("/").pop();
const root = new URL("../../..", import.meta.url).pathname;

describe("server codes the app matches", () => {
  it("are each thrown or answered by the backend", () => {
    const backend = sources(join(root, "backend/convex"), ".ts");
    const emitted = (code: string) =>
      new RegExp(`(fail\\(|error\\(\\d+, |refused: |code: |\\? )"${code}"`).test(backend);
    expect(codes.backend.filter((code) => !emitted(code))).toEqual([]);
  });

  it("from the engine are each sent by it", () => {
    const engine = sources(join(root, "rust/vtop-bridge/src"), ".rs");
    expect(codes.engine.filter((code) => !engine.includes(`"${code}"`))).toEqual([]);
  });

  it("made on the phone are never the backend's", () => {
    const backend = sources(join(root, "backend/convex"), ".ts");
    expect(codes.phone.filter((code) => backend.includes(`"${code}"`))).toEqual([]);
  });
});
