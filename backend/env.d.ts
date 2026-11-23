// Convex functions read their deployment's environment through `process.env`;
// this declares it without pulling in @types/node.
declare const process: { env: Record<string, string | undefined> };

// The few Node APIs dev/guideFiles.ts uses (it runs under bun and vitest), so
// the project needs no @types/node.
declare module "node:fs" {
  export function readFileSync(path: string): Uint8Array & { toString(encoding: "base64"): string };
  export function readdirSync(path: string): string[];
  export function statSync(path: string): { isDirectory(): boolean };
}
declare module "node:path" {
  export function join(...parts: string[]): string;
  export function relative(from: string, to: string): string;
}
