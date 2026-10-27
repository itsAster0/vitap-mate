import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, loadEnv, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

/**
 * Writes the bridge origin from VITE_BRIDGE_URL into the built manifest. The
 * Firefox build swaps the service worker for an event page: Chrome rejects
 * `background.scripts` under MV3, and Firefox doesn't run service workers.
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
      manifest.host_permissions = [
        "https://vtop.vitap.ac.in/*",
        `${new URL(bridgeUrl).origin}/*`,
      ];
      if (firefox) {
        const { service_worker, ...background } = manifest.background;
        manifest.background = { ...background, scripts: [service_worker] };
      }
      writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
    },
  };
}

export default defineConfig(({ mode }) => {
  const firefox = mode === "firefox";
  const outDir = firefox ? "dist-firefox" : "dist";
  return {
    plugins: [
      svelte(),
      tailwindcss(),
      finishManifest(
        outDir,
        loadEnv(mode, import.meta.dirname, "VITE_").VITE_BRIDGE_URL ??
          "https://vmate-api.aster0.dev",
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
