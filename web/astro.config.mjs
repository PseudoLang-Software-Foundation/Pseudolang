import { readFileSync } from "node:fs";
import { defineConfig } from "astro/config";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

const cargoToml = readFileSync("../Cargo.toml", "utf-8");
const versionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
const PSEUDOLANG_VERSION = versionMatch ? versionMatch[1] : "0.0.0";

export default defineConfig({
  output: "static",
  base: process.env.ASTRO_BASE ?? "/",
  server: { port: 8076 },
  vite: {
    server: {
      headers: {
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "credentialless",
      },
    },
    define: {
      __PSEUDOLANG_VERSION__: JSON.stringify(PSEUDOLANG_VERSION),
    },
    plugins: [wasm(), topLevelAwait()],
    optimizeDeps: {
      exclude: ["@bjorn3/browser_wasi_shim"],
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks: {
            monaco: ["monaco-editor"],
            xterm: ["@xterm/xterm", "@xterm/addon-fit", "@xterm/addon-webgl"],
          },
        },
      },
    },
  },
});
