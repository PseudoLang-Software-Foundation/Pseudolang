import "../common.just"

# Build commands

# Build debug (native)
build:
  cd {{ROOT}} && cargo build --features native

# Build release (native)
release:
  cd {{ROOT}} && cargo build --release --features native

# Cross-compile Windows release
build-windows:
  cd {{ROOT}} && cross build --release --target x86_64-pc-windows-gnu --features native

# Cross-compile Linux release
build-linux:
  cd {{ROOT}} && cross build --release --target x86_64-unknown-linux-gnu --features native

# Build raw WASM
build-wasm:
  cd {{ROOT}} && cargo build --release --target wasm32-unknown-unknown --features wasm

# Build WASM with wasm-bindgen
build-wasm-bindgen:
  cd {{ROOT}} && wasm-pack build --target web --release -- --features "wasm bindgen"

# Build WASI target
build-wasi:
  cd {{ROOT}} && cargo build --release --target wasm32-wasip1 --features wasi

# Build NSIS Windows installer (requires makensis and fplc.exe in installer/)
build-installer:
  cd {{ROOT}} && makensis -DVERSION={{VERSION}} installer/pseudolang.nsi

# Full release: all targets + installer + copy artifacts
build-all: build-windows build-linux build-wasm build-wasm-bindgen build-wasi
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p release/installer release/wasi release/wasm/raw release/wasm/bindgen
  cp target/x86_64-pc-windows-gnu/release/fplc.exe release/fplc-x64.exe || true
  cp release/fplc-x64.exe installer/fplc.exe || true
  cp target/x86_64-unknown-linux-gnu/release/fplc release/fplc-linux-x64 || true
  chmod +x release/fplc-linux-x64 || true
  cp target/wasm32-unknown-unknown/release/fplc.wasm release/wasm/raw/ || true
  cp pkg/* release/wasm/bindgen/ || true
  cp target/wasm32-wasip1/release/fplc.wasm release/wasi/ || true
  if command -v makensis >/dev/null 2>&1; then
    makensis -DVERSION={{VERSION}} installer/pseudolang.nsi
    echo "Installer built at release/installer/pseudolang-setup-x64.exe"
  else
    echo "makensis not found, skipping installer"
  fi
  echo ""
  echo "Release {{VERSION}} built. Artifacts in release/"

# Check Rust code without building
check:
  cd {{ROOT}} && cargo check

# Format and lint Rust code
fmt:
  cd {{ROOT}} && cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings
  cd {{ROOT}} && cargo fmt --all
  cd {{ROOT}} && taplo fmt

# Check formatting and linting without changes
fmt-check:
  cd {{ROOT}} && cargo fmt --check --all
  cd {{ROOT}} && cargo clippy --all-targets --all-features -- -D warnings
  cd {{ROOT}} && taplo fmt --check

# Clean build artifacts
clean:
  cd {{ROOT}} && cargo clean
  cd {{ROOT}} && rm -rf release pkg
