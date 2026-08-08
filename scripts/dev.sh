#!/bin/bash
# Bitvue Development Script
# Starts the Electron dev shell (spawns bitvue-sidecar, loads the renderer).
# See docs/DEVELOPMENT_PHASES.md for the Tauri->Electron migration; src-tauri retired 2026-08-08.

echo "🚀 Starting Bitvue development environment..."

if [ ! -d "bitvue-desktop/node_modules" ]; then
    echo "❌ bitvue-desktop dependencies not installed. Run 'npm run setup' or './scripts/setup.sh' first."
    exit 1
fi

echo "Building the sidecar (cargo build -p bitvue-sidecar)..."
cargo build -p bitvue-sidecar

cd bitvue-desktop && npm run electron
