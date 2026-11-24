import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, loadEnv, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

/** CI uploads it next to extension.xpi on the `extension-latest` release. */
const FIREFOX_UPDATES =
  "https://github.com/itsAster0/vitap-mate/releases/download/extension-latest/firefox-updates.json";

/**
 * Writes the server origin from VITE_BRIDGE_URL into the built manifest. The
 * Firefox build swaps the service worker for an event page: Chrome rejects
 * `background.scripts` under MV3, and Firefox doesn't run service workers.
 * Firefox gets the update manifest CI puts on the GitHub release, so it updates
 * the signed .xpi by itself; Chrome gets its minimum version (116 keeps a service worker alive
 * while its WebSocket talks), which Firefox's add-on checks would flag.
 */
function finishManifest(
  outDir: string,
  bridgeUrl: string,
  firefox: boolean,
): Plugin {
  return {
    name: "finish-manifest",
    apply: "build",
    closeBundle() {
      const path = resolve(import.meta.dirname, outDir, "manifest.json");
      const manifest = JSON.parse(readFileSync(path, "utf8"));
      const origin = new URL(bridgeUrl).origin;
      manifest.host_permissions = ["https://vtop.vitap.ac.in/*", `${origin}/*`];
      if (firefox) {
        const { service_worker, ...background } = manifest.background;
        manifest.background = { ...background, scripts: [service_worker] };
        manifest.browser_specific_settings.gecko.update_url = FIREFOX_UPDATES;
      } else {
        manifest.minimum_chrome_version = "116";
      }
      writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
    },
  };
}

/** The commit this build is from: CI's GITHUB_SHA, else git's HEAD, else none. The popup compares
 * it with the newest release to offer an update. */
function buildSha(): string {
  if (process.env.GITHUB_SHA) return process.env.GITHUB_SHA.slice(0, 12);
  try {
    return execSync("git rev-parse --short=12 HEAD", { stdio: ["ignore", "pipe", "ignore"] }).toString().trim();
  } catch {
    return "";
  }
}

export default defineConfig(({ mode }) => {
  const firefox = mode === "firefox";
  const outDir = firefox ? "dist-firefox" : "dist";
  return {
    define: {
      __BUILD_SHA__: JSON.stringify(buildSha()),
      __BUILD_AT__: JSON.stringify(Date.now()),
    },
    plugins: [
      svelte(),
      finishManifest(
        outDir,
        loadEnv(mode, import.meta.dirname, "VITE_").VITE_BRIDGE_URL ??
          "https://vmate.aster0.dev",
        firefox,
      ),
    ],
    base: "./",
    build: {
      outDir,
      emptyOutDir: true,
      rollupOptions: {
        input: {
          popup: "index.html",
          background: "src/background.js",
          content: "src/content.js",
        },
        output: {
          entryFileNames: "[name].js",
          chunkFileNames: "chunks/[name].js",
          assetFileNames: "assets/[name].[ext]",
        },
      },
    },
  };
});
