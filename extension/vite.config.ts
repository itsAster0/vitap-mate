import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, loadEnv, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

/** Writes the bridge origin from VITE_BRIDGE_URL into the built manifest. */
function bridgeHostPermission(bridgeUrl: string): Plugin {
  return {
    name: "bridge-host-permission",
    apply: "build",
    closeBundle() {
      const path = resolve(__dirname, "dist/manifest.json");
      const manifest = JSON.parse(readFileSync(path, "utf8"));
      manifest.host_permissions = [
        "https://vtop.vitap.ac.in/*",
        `${new URL(bridgeUrl).origin}/*`,
      ];
      writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
    },
  };
}

export default defineConfig(({ mode }) => ({
  plugins: [
    svelte(),
    tailwindcss(),
    bridgeHostPermission(
      loadEnv(mode, __dirname, "VITE_").VITE_BRIDGE_URL ??
        "https://vtop-bridge.aster0.dev",
    ),
  ],
  base: "./",
  build: {
    outDir: "dist",
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
}));
