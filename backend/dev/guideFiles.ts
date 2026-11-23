// The files under edge/guides as `GET /guides/*` serves them (shared by
// dev/gen-guides.ts and its test). `_headers` is Cloudflare's, not content.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("../../edge/guides", import.meta.url).pathname;
const TYPES: Record<string, string> = { json: "application/json", webp: "image/webp", png: "image/png", jpg: "image/jpeg", svg: "image/svg+xml" };

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

export async function guideFiles() {
  const files = [];
  for (const full of walk(ROOT).sort()) {
    const path = relative(ROOT, full);
    if (path === "_headers") continue;
    const bytes = readFileSync(full);
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new Uint8Array(bytes)));
    const etag = `"${Array.from(digest.slice(0, 16), (b) => b.toString(16).padStart(2, "0")).join("")}"`;
    files.push({ path, type: TYPES[path.split(".").pop() ?? ""] ?? "application/octet-stream", etag, base64: bytes.toString("base64") });
  }
  return files;
}
