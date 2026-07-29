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
  # Inject version from Cargo.toml into package.json.
  # `sed -i` with no suffix is a GNU extension; BSD sed (macOS) requires an
  # argument. `-i.bak` + rm is the form both accept.
  sed -i.bak 's/"version": "[^"]*"/"version": "{{VERSION}}"/' extensions/vscode/package.json
  rm -f extensions/vscode/package.json.bak
  cd extensions/vscode && bun run package
  # Restore placeholder version
  cd "{{ROOT}}"
  sed -i.bak 's/"version": "[^"]*"/"version": "0.0.0"/' extensions/vscode/package.json
  rm -f extensions/vscode/package.json.bak

# Lint extension code with Biome
lint-vscode:
  cd {{ROOT}}/extensions/vscode && bun run lint

# Format extension code with Biome
fmt-vscode:
  cd {{ROOT}}/extensions/vscode && bun run format
