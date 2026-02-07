import "../common.just"

# Build the web IDE (copies WASI binary + tmLanguage, then builds Astro)
build-web: build-wasi gen-tmlanguage
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p web/public
  cp target/wasm32-wasip1/release/fpli.wasm web/public/fpli.wasm
  cp utils/tm/pseudolang.tmLanguage.json web/public/pseudolang.tmLanguage.json
  cd web && bun run build

# Dev server for web IDE (copies assets then starts Astro dev)
dev-web: build-wasi gen-tmlanguage
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p web/public
  cp target/wasm32-wasip1/release/fpli.wasm web/public/fpli.wasm
  cp utils/tm/pseudolang.tmLanguage.json web/public/pseudolang.tmLanguage.json
  cd web && bun run dev

# Lint web code with Biome
lint-web:
  cd {{ROOT}}/web && bun x biome check .

# Format web code with Biome
fmt-web:
  cd {{ROOT}}/web && bun x biome check --write .
