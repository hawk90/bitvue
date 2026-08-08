#!/bin/bash
# Bitvue Electron Packaging Script
# Builds the release sidecar binary + frontend, then packages the Electron shell into a
# distributable (electron-builder). Local counterpart to .github/workflows/build-electron-app.yml.
#
# Usage: ./scripts/package_electron.sh [mac|linux|win]
#   No argument packages for the host platform.

set -euo pipefail

TARGET="${1:-}"

if [ ! -d "bitvue-desktop/node_modules" ]; then
    echo "❌ bitvue-desktop dependencies not installed. Run 'npm run setup' or './scripts/setup.sh' first."
    exit 1
fi
if [ ! -d "frontend/node_modules" ]; then
    echo "❌ frontend dependencies not installed. Run 'cd frontend && npm ci' first."
    exit 1
fi

echo "🔧 Building the sidecar (cargo build --release -p bitvue-sidecar)..."
cargo build --release -p bitvue-sidecar

echo "🎨 Building the frontend (npm run build)..."
(cd frontend && npm run build)

echo "📦 Packaging the Electron app..."
case "$TARGET" in
    mac)   (cd bitvue-desktop && npm run package:mac) ;;
    linux) (cd bitvue-desktop && npm run package:linux) ;;
    win)   (cd bitvue-desktop && npm run package:win) ;;
    "")    (cd bitvue-desktop && npm run package) ;;
    *)     echo "❌ Unknown target '$TARGET' — expected mac, linux, win, or no argument."; exit 1 ;;
esac

echo "✅ Done — see bitvue-desktop/release/"
