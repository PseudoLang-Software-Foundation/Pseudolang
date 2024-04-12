#!/bin/bash
chmod +x build_release.sh
cross build --target x86_64-unknown-linux-gnu --release
cross build --target x86_64-apple-darwin --release
cross build --target x86_64-pc-windows-msvc --release
