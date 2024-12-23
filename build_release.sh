#!/bin/bash

set -e

chmod +x build_release.sh

mkdir -p release/installer release/wasi release/wasm/raw release/wasm/bindgen

echo "Building native targets..."

echo "Building Windows target..."
cross build --release --target x86_64-pc-windows-gnu --features native || {
    echo "Windows build failed but continuing..."
}

echo "Building Linux target..."
cross build --release --target x86_64-unknown-linux-gnu --features native || {
    echo "Linux build failed but continuing..."
}

echo "Building WASM targets..."
rustup target add wasm32-unknown-unknown

echo "Building raw WASM..."
cargo build --release --target wasm32-unknown-unknown --features wasm

echo "Building WASM with bindgen..."
if command -v wasm-pack >/dev/null 2>&1; then
    wasm-pack build --target web --release -- --features "wasm bindgen"
else
    echo "wasm-pack not found, skipping bindgen WASM build"
fi

echo "Building WASI target..."
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1 --features wasi

echo "Building Windows installer..."
if command -v makensis >/dev/null 2>&1; then
    cd installer
    makensis pseudolang.nsi
    cd ..
    echo "Windows installer built successfully"
else
    echo "makensis not found, skipping installer build"
    echo "To build the installer, please install NSIS (Nullsoft Scriptable Install System)"
fi

echo "Copying binaries to release folder..."

cp target/wasm32-unknown-unknown/release/fplc.wasm release/wasm/raw/
cp pkg/* release/wasm/bindgen/
cp target/wasm32-wasip1/release/fplc.wasm release/wasi/

if [ -f "target/x86_64-pc-windows-gnu/release/fplc.exe" ]; then
    cp target/x86_64-pc-windows-gnu/release/fplc.exe release/fplc-x64.exe
    cp release/fplc-x64.exe installer/fplc.exe
fi

if [ -f "target/x86_64-unknown-linux-gnu/release/fplc" ]; then
    cp target/x86_64-unknown-linux-gnu/release/fplc release/fplc-linux-x64
    chmod +x release/fplc-linux-x64
fi

echo "Build complete! Binaries are in the release folder."
