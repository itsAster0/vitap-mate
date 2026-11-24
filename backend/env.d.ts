// Convex functions read their deployment's environment through `process.env`;
// this declares it without pulling in @types/node.
declare const process: { env: Record<string, string | undefined>; argv: string[] };

// The few Node APIs dev/guideFiles.ts and dev/demo.ts use (they run under bun and
// vitest), so the project needs no @types/node.
declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
  export function readFileSync(path: string): Uint8Array & { toString(encoding: "base64"): string };
  export function writeFileSync(path: string, data: string): void;
  export function readdirSync(path: string): string[];
  export function statSync(path: string): { isDirectory(): boolean };
}
declare module "node:path" {
  export function join(...parts: string[]): string;
  export function relative(from: string, to: string): string;
}
