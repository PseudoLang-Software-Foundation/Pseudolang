#!/bin/bash

set -e

chmod +x build_release.sh

mkdir -p release/installer
mkdir -p release/wasm
mkdir -p release/wasi

echo "Building native targets..."

echo "Building Windows target..."
cross build --release --target x86_64-pc-windows-gnu --features native || {
    echo "Windows build failed but continuing..."
}

echo "Building Linux target..."
cross build --release --target x86_64-unknown-linux-gnu --features native || {
    echo "Linux build failed but continuing..."
}

echo "Building WASM target..."
if command -v wasm-pack >/dev/null 2>&1; then
    wasm-pack build --target web --release -- --features wasm
    cp pkg/* release/wasm/
else
    echo "wasm-pack not found, skipping WASM build"
fi

echo "Building WASI target..."
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1 --features wasi
cp target/wasm32-wasip1/release/fplc.wasm release/wasi/

echo "Copying binaries to release folder..."

if [ -f "target/x86_64-pc-windows-gnu/release/fplc.exe" ]; then
    cp target/x86_64-pc-windows-gnu/release/fplc.exe release/fplc-x64.exe
    cp release/fplc-x64.exe installer/fplc.exe
fi

if [ -f "target/x86_64-unknown-linux-gnu/release/fplc" ]; then
    cp target/x86_64-unknown-linux-gnu/release/fplc release/fplc-linux-x64
    chmod +x release/fplc-linux-x64
fi

echo "Build complete! Binaries are in the release folder."
