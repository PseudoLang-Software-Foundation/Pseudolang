import "../common.just"

# Generate tmLanguage spec from Rust source
gen-tmlanguage:
  cd {{ROOT}} && bash scripts/tm/gen_tmlanguage.sh

# Copy tmLanguage to extension and package VSIX
build-vscode: gen-tmlanguage
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p extensions/vscode/syntaxes dist
  cp utils/tm/pseudolang.tmLanguage.json extensions/vscode/syntaxes/
  cp assets/Pseudolang-Logo.png extensions/vscode/icons/icon.png
  # Inject version from Cargo.toml into package.json
  sed -i 's/"version": "[^"]*"/"version": "{{VERSION}}"/' extensions/vscode/package.json
  cd extensions/vscode && bun run package
  # Restore placeholder version
  cd "{{ROOT}}"
  sed -i 's/"version": "[^"]*"/"version": "0.0.0"/' extensions/vscode/package.json

# Lint extension code with Biome
lint-vscode:
  cd {{ROOT}}/extensions/vscode && bun run lint

# Format extension code with Biome
fmt-vscode:
  cd {{ROOT}}/extensions/vscode && bun run format
