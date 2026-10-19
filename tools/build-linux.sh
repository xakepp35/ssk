#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release -p ssk --jobs "${SSK_BUILD_JOBS:-2}"
ssk_bundle="$PWD/dist/ssk-linux-x64"
mkdir -p "$ssk_bundle"
cp target/release/ssk ssk.yml "$ssk_bundle/"
cp docs/LAN.md "$ssk_bundle/PLAY.md"
tar -C "$ssk_bundle" -czf dist/ssk-linux-x64.tar.gz ssk ssk.yml PLAY.md
printf 'Linux package: %s\n' "$PWD/dist/ssk-linux-x64.tar.gz"
