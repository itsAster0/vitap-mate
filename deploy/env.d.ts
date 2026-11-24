// The one Node API alchemy.run.ts uses, so the project needs no @types/node.
declare module "node:child_process" {
  export function execFileSync(
    file: string,
    args: string[],
    options: { cwd: string; encoding: "utf8"; stdio: ("ignore" | "pipe")[] },
  ): string;
}
