import { readFileSync } from "node:fs";
import { defineConfig } from "astro/config";

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
    // No `vite-plugin-wasm` / `vite-plugin-top-level-await`: the wasm binary is
    // fetched from `public/` at runtime rather than imported as a module, and
    // nothing in the graph uses top-level await. Both plugins re-emitted every
    // chunk through SWC *after* minification, shipping monaco and the app entry
    // unminified. The default build target (chrome111/firefox114/safari16.4)
    // supports top-level await natively anyway.
    optimizeDeps: {
      exclude: ["@bjorn3/browser_wasi_shim"],
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes("node_modules/monaco-editor")) return "monaco";
            if (id.includes("node_modules/@xterm/")) return "xterm";
          },
        },
      },
    },
  },
});
