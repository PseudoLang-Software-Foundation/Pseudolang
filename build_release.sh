#!/bin/bash

chmod +x build_release.sh

mkdir -p release/installer

echo "Building for Windows..."
cross build --target x86_64-pc-windows-gnu --release

echo "Building for Linux..."
cross build --target x86_64-unknown-linux-gnu --release

echo "Copying binaries to release folder..."

cp target/x86_64-pc-windows-gnu/release/fplc.exe release/fplc-x64.exe
cp release/fplc-x64.exe installer/fplc.exe

cp target/x86_64-unknown-linux-gnu/release/fplc release/fplc-linux-x64

chmod +x release/fplc-linux-*

echo "Build complete! Binaries are in the release folder."
