#!/bin/bash
set -e

# Bitvue Development Setup Script
# This script sets up the development environment for Bitvue

echo "🔧 Setting up Bitvue development environment..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install it from https://rustup.rs/"
    exit 1
fi

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install it from https://nodejs.org/"
    exit 1
fi

# Install Rust dependencies
echo "📦 Installing Rust dependencies..."
cargo fetch

# Install frontend dependencies
echo "📦 Installing frontend dependencies..."
cd frontend
npm install
cd ..

# Install Tauri CLI (if not already installed)
echo "📦 Checking Tauri CLI..."
if ! command -v cargo-tauri &> /dev/null; then
    echo "Installing Tauri CLI..."
    cargo install tauri-cli --version "^2.0.0"
fi

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file..."
    cat > .env << EOF
# Bitvue Environment Variables
# Add your custom variables here

NODE_ENV=development
EOF
fi

echo "✅ Setup complete!"
echo ""
echo "To start development:"
echo "  npm run dev        - Start frontend dev server"
echo "  npm run tauri:dev  - Start Tauri dev server (includes frontend)"
echo ""
echo "To build:"
echo "  npm run build      - Build frontend"
echo "  npm run tauri:build - Build Tauri app"
echo ""
echo "To run tests:"
echo "  npm run test       - Run frontend tests"
echo "  cargo test         - Run Rust tests"
