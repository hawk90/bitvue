#!/bin/bash
# Bitvue Development Script
# Starts both Rust backend watch and frontend dev server

echo "🚀 Starting Bitvue development environment..."

# Check if setup has been run
if [ ! -d "frontend/node_modules" ]; then
    echo "❌ Frontend dependencies not installed. Run 'npm run setup' or './scripts/setup.sh' first."
    exit 1
fi

# Run Tauri dev (which starts both frontend and backend)
cd frontend && npm run tauri:dev
