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

# Cross-compile Linux amd64 release (Windows only, requires cross)
build-linux:
  cd {{ROOT}} && cross build --release --target x86_64-unknown-linux-gnu --features native

# Cross-compile Linux ARM64 release (requires cross)
build-linux-arm:
  cd {{ROOT}} && cross build --release --target aarch64-unknown-linux-gnu --features native

# Build macOS arm64 (Apple Silicon) release -- macOS host only
build-macos:
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  rustup target add aarch64-apple-darwin >/dev/null 2>&1 || true
  cargo build --release --target aarch64-apple-darwin --features native
  mkdir -p dist/release
  cp target/aarch64-apple-darwin/release/fpli dist/release/fpli-macos-arm64
  chmod +x dist/release/fpli-macos-arm64
  echo "Built dist/release/fpli-macos-arm64"

# Build macOS x86_64 (Intel) release -- macOS host only
build-macos-intel:
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  rustup target add x86_64-apple-darwin >/dev/null 2>&1 || true
  cargo build --release --target x86_64-apple-darwin --features native
  mkdir -p dist/release
  cp target/x86_64-apple-darwin/release/fpli dist/release/fpli-macos-amd64
  chmod +x dist/release/fpli-macos-amd64
  echo "Built dist/release/fpli-macos-amd64"

# Build a macOS universal (arm64 + x86_64) binary via lipo -- macOS host only
build-macos-universal: build-macos build-macos-intel
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p dist/release
  lipo -create \
    dist/release/fpli-macos-arm64 \
    dist/release/fpli-macos-amd64 \
    -output dist/release/fpli-macos-universal
  # Ad-hoc sign the fat binary. lipo copies each slice verbatim, so the fat
  # container itself carries no valid signature (`codesign --verify` reports
  # "not signed at all"). arm64 macOS can refuse to exec a Mach-O whose signature
  # is missing or invalidated, so always re-sign after lipo.
  codesign -s - -f dist/release/fpli-macos-universal
  chmod +x dist/release/fpli-macos-universal
  lipo -info dist/release/fpli-macos-universal

# Build WASM (wasm-bindgen via wasm-pack)
build-wasm:
  cd {{ROOT}} && wasm-pack build --out-dir dist/.tmp --target web --release -- --features wasm

# Build WASI target
build-wasi:
  cd {{ROOT}} && cargo build --release --target wasm32-wasip1 --features wasi

# Build NSIS Windows installer (requires makensis and fpli.exe in installer/nsis/)
build-installer:
  cd {{ROOT}} && makensis -DVERSION={{VERSION}} installer/nsis/pseudolang.nsi

# Build Debian package (requires cargo-deb, assumes release already built)
build-deb:
  cd {{ROOT}} && cargo deb --no-build

# Build all targets (native release + cross + wasm + wasi)
build-all: release build-wasm build-wasi
  #!/usr/bin/env bash
  set -euo pipefail
  cd "{{ROOT}}"
  mkdir -p dist/release/wasi dist/release/wasm
  cp dist/.tmp/fpli_lib_bg.wasm dist/release/wasm/ || true
  cp dist/.tmp/fpli_lib.js dist/release/wasm/ || true
  cp dist/.tmp/fpli_lib.d.ts dist/release/wasm/ || true
  cp dist/.tmp/fpli_lib_bg.wasm.d.ts dist/release/wasm/ || true
  cp target/wasm32-wasip1/release/fpli.wasm dist/release/wasi/ || true
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      cp target/release/fpli.exe dist/release/fpli-amd64.exe || true
      if command -v makensis >/dev/null 2>&1; then
        makensis -DVERSION={{VERSION}} installer/nsis/pseudolang.nsi
        echo "Installer built at dist/release/pseudolang-setup-amd64.exe"
      else
        echo "makensis not found, skipping installer"
      fi
      if command -v cross >/dev/null 2>&1; then
        echo "Cross-compiling Linux amd64 target..."
        cross build --release --target x86_64-unknown-linux-gnu --features native \
          && cp target/x86_64-unknown-linux-gnu/release/fpli dist/release/fpli-linux-amd64 \
          || echo "Cross-compilation failed, skipping Linux amd64 target"
        echo "Cross-compiling Linux ARM64 target..."
        cross build --release --target aarch64-unknown-linux-gnu --features native \
          && cp target/aarch64-unknown-linux-gnu/release/fpli dist/release/fpli-linux-arm64 \
          || echo "Cross-compilation failed, skipping Linux ARM64 target"
      else
        echo "cross not found, skipping Linux cross-compilation"
      fi
      ;;
    Darwin*)
      # macOS: never fall through to the Linux arm below -- target/release/fpli is
      # a Mach-O binary, so copying it to fpli-linux-amd64 and feeding it to
      # `cargo deb` would produce a mislabelled artifact and a broken .deb.
      echo "macOS host ($(uname -m)) - building macOS artifacts..."
      MAC_SLICES=1
      rustup target add aarch64-apple-darwin >/dev/null 2>&1 || true
      rustup target add x86_64-apple-darwin >/dev/null 2>&1 || true
      if cargo build --release --target aarch64-apple-darwin --features native; then
        cp target/aarch64-apple-darwin/release/fpli dist/release/fpli-macos-arm64
        chmod +x dist/release/fpli-macos-arm64
      else
        echo "macOS arm64 build failed, skipping"
        MAC_SLICES=0
      fi
      if cargo build --release --target x86_64-apple-darwin --features native; then
        cp target/x86_64-apple-darwin/release/fpli dist/release/fpli-macos-amd64
        chmod +x dist/release/fpli-macos-amd64
      else
        echo "macOS x86_64 build failed, skipping"
        MAC_SLICES=0
      fi
      if [ "$MAC_SLICES" -eq 1 ]; then
        lipo -create \
          dist/release/fpli-macos-arm64 \
          dist/release/fpli-macos-amd64 \
          -output dist/release/fpli-macos-universal
        # lipo leaves the fat container without a valid signature; re-sign ad-hoc.
        codesign -s - -f dist/release/fpli-macos-universal
        chmod +x dist/release/fpli-macos-universal
        lipo -info dist/release/fpli-macos-universal
      else
        echo "Skipping universal binary (a slice is missing)"
      fi
      if command -v cross >/dev/null 2>&1; then
        echo "Cross-compiling Linux amd64 target..."
        cross build --release --target x86_64-unknown-linux-gnu --features native \
          && cp target/x86_64-unknown-linux-gnu/release/fpli dist/release/fpli-linux-amd64 \
          || echo "Cross-compilation failed, skipping Linux amd64 target"
        echo "Cross-compiling Linux ARM64 target..."
        cross build --release --target aarch64-unknown-linux-gnu --features native \
          && cp target/aarch64-unknown-linux-gnu/release/fpli dist/release/fpli-linux-arm64 \
          || echo "Cross-compilation failed, skipping Linux ARM64 target"
      else
        echo "cross not found (needs Docker/Podman on macOS), skipping Linux cross-compilation"
      fi
      echo "Skipping Debian package: cargo deb on macOS would package a Mach-O binary"
      ;;
    *)
      cp target/release/fpli dist/release/fpli-linux-amd64 || true
      chmod +x dist/release/fpli-linux-amd64 || true
      if command -v cross >/dev/null 2>&1; then
        echo "Cross-compiling Windows target..."
        cross build --release --target x86_64-pc-windows-gnu --features native \
          && cp target/x86_64-pc-windows-gnu/release/fpli.exe dist/release/fpli-amd64.exe \
          || echo "Cross-compilation failed, skipping Windows target"
        echo "Cross-compiling Linux ARM64 target..."
        cross build --release --target aarch64-unknown-linux-gnu --features native \
          && cp target/aarch64-unknown-linux-gnu/release/fpli dist/release/fpli-linux-arm64 \
          || echo "Cross-compilation failed, skipping Linux ARM64 target"
      else
        echo "cross not found, skipping cross-compilation"
      fi
      if command -v cargo-deb >/dev/null 2>&1; then
        echo "Building Debian package..."
        cargo deb --no-build \
          && cp target/debian/*.deb dist/release/ \
          || echo "Debian package build failed"
      else
        echo "cargo-deb not found, skipping Debian package"
      fi
      ;;
  esac
  echo ""
  echo "Release {{VERSION}} built. Artifacts in dist/release/"

# Check Rust code without building
check:
  cd {{ROOT}} && cargo check

# Format and lint all code
fmt:
  cd {{ROOT}} && cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings
  cd {{ROOT}} && cargo fmt --all
  cd {{ROOT}} && taplo fmt
  cd {{ROOT}}/extensions/vscode && bun x biome check --write .
  cd {{ROOT}}/web && bun x biome check --write .

# Check formatting and linting without changes
fmt-check:
  cd {{ROOT}} && cargo fmt --check --all
  cd {{ROOT}} && cargo clippy --all-targets --all-features -- -D warnings
  cd {{ROOT}} && taplo fmt --check
  cd {{ROOT}}/extensions/vscode && bun x biome check .
  cd {{ROOT}}/web && bun x biome check .

# Clean build artifacts
clean:
  cd {{ROOT}} && cargo clean
  cd {{ROOT}} && rm -rf dist/.tmp dist/release
