// Drafts a release's notes from the commits since the last one (`mise run notes [since]`):
// prints the app-facing commits by kind, and stubs this versionCode's Play changelog and its
// entry in the edge's updates.json when they don't exist yet. The drafts are commit subjects, to
// rewrite in the students' words before a release. Writes nothing that is already there.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const root = new URL("..", import.meta.url).pathname;
const git = (...args: string[]) =>
  execFileSync("git", args, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();

/** The last release: the newest `v*` tag, else the newest `chore(release)` commit. */
function lastRelease(): string {
  try {
    return git("describe", "--tags", "--abbrev=0", "--match", "v*");
  } catch {
    const commit = git("log", "--format=%H", "-1", "--extended-regexp", "--grep=^chore\\(release\\)");
    if (!commit) throw new Error("no v* tag or chore(release) commit; pass the revision to start from");
    return commit;
  }
}

type Kind = "New" | "Improved" | "Fixed";
const KINDS: Record<string, Kind> = { feat: "New", perf: "Improved", fix: "Fixed" };
/** Scopes that never reach the app. */
const SERVER = new Set(["edge", "extension", "ci", "gateway", "bridge", "engine", "mcp"]);

const since = process.argv[2] || lastRelease();
const grouped: Record<Kind, string[]> = { New: [], Improved: [], Fixed: [] };
for (const subject of git("log", "--no-merges", "--format=%s", `${since}..HEAD`).split("\n")) {
  const m = /^(\w+)(?:\(([^)]*)\))?!?:\s*(.+)$/.exec(subject);
  const kind = m && KINDS[m[1]];
  if (!m || !kind || SERVER.has(m[2] ?? "")) continue;
  grouped[kind].push(m[3].charAt(0).toUpperCase() + m[3].slice(1));
}

const yaml = readFileSync(`${root}app/module.yaml`, "utf8");
const code = Number(/versionCode:\s*(\d+)/.exec(yaml)?.[1]);
const version = /versionName:\s*"?([\d.]+)/.exec(yaml)?.[1];
if (!code || !version) throw new Error("no versionCode or versionName in app/module.yaml");

console.log(`${version} (${code}), commits since ${since}:`);
for (const [kind, items] of Object.entries(grouped)) {
  if (items.length) console.log(`\n${kind}\n${items.map((i) => `  • ${i}`).join("\n")}`);
}

const changelog = `${root}fastlane/metadata/android/en-GB/changelogs/${code}.txt`;
if (existsSync(changelog)) {
  console.log(`\nkept ${changelog}`);
} else {
  writeFileSync(changelog, Object.values(grouped).flat().map((i) => `• ${i}`).join("\n") + "\n");
  console.log(`\nwrote ${changelog}`);
}
const chars = [...readFileSync(changelog, "utf8").trimEnd()].length;
if (chars > 500) console.log(`  ${chars} characters: Play takes at most 500`);

const updatesPath = `${root}rust/vtop-edge/guides/updates.json`;
const updates = JSON.parse(readFileSync(updatesPath, "utf8"));
if (updates.releases.some((r: { code: number }) => r.code === code)) {
  console.log(`kept ${code} in ${updatesPath}`);
} else {
  const sections = Object.entries(grouped)
    .filter(([, items]) => items.length)
    .map(([title, items]) => ({ title, items }));
  updates.releases.unshift({ version, code, summary: `${version} …`, sections });
  writeFileSync(updatesPath, JSON.stringify(updates, null, 2) + "\n");
  console.log(`added ${code} to ${updatesPath}`);
}
