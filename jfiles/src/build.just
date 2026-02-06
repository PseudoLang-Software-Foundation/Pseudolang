import "../common.just"

# Build commands

# Build debug (native)
build:
  cd {{ROOT}} && cargo build --features native

# Build release (native, for current platform)
release:
  cd {{ROOT}} && cargo build --release --features native

# Cross-compile Windows release (Linux only, requires cross)
build-windows:
  cd {{ROOT}} && cross build --release --target x86_64-pc-windows-gnu --features native

# Cross-compile Linux release (Windows only, requires cross)
build-linux:
  cd {{ROOT}} && cross build --release --target x86_64-unknown-linux-gnu --features native

# Build WASM (wasm-bindgen via wasm-pack)
build-wasm:
  cd {{ROOT}} && wasm-pack build --target web --release -- --features wasm

# Build WASI target
build-wasi:
  cd {{ROOT}} && cargo build --release --target wasm32-wasip1 --features wasi

# Build NSIS Windows installer (requires makensis and fplc.exe in installer/)
build-installer:
  cd {{ROOT}} && makensis -DVERSION={{VERSION}} installer/pseudolang.nsi

# Build all targets (native release + cross + wasm + wasi)
build-all: release build-wasm build-wasi
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p release/wasi release/wasm
  cp pkg/* release/wasm/ || true
  cp target/wasm32-wasip1/release/fplc.wasm release/wasi/ || true
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      cp target/release/fplc.exe release/fplc-x64.exe || true
      cp release/fplc-x64.exe installer/fplc.exe || true
      if command -v makensis >/dev/null 2>&1; then
        makensis -DVERSION={{VERSION}} installer/pseudolang.nsi
        echo "Installer built at release/installer/pseudolang-setup-x64.exe"
      else
        echo "makensis not found, skipping installer"
      fi
      if command -v cross >/dev/null 2>&1; then
        echo "Cross-compiling Linux target..."
        cross build --release --target x86_64-unknown-linux-gnu --features native
        cp target/x86_64-unknown-linux-gnu/release/fplc release/fplc-linux-x64 || true
      else
        echo "cross not found, skipping Linux cross-compilation"
      fi
      ;;
    *)
      cp target/release/fplc release/fplc-linux-x64 || true
      chmod +x release/fplc-linux-x64 || true
      if command -v cross >/dev/null 2>&1; then
        echo "Cross-compiling Windows target..."
        cross build --release --target x86_64-pc-windows-gnu --features native
        cp target/x86_64-pc-windows-gnu/release/fplc.exe release/fplc-x64.exe || true
      else
        echo "cross not found, skipping Windows cross-compilation"
      fi
      ;;
  esac
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
