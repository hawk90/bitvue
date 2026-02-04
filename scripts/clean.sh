#!/bin/bash
# Bitvue Clean Script
# Cleans all build artifacts and dependencies

echo "🧹 Cleaning Bitvue build artifacts..."

# Clean Rust build artifacts
echo "Cleaning Rust artifacts..."
cargo clean

# Clean frontend
echo "Cleaning frontend..."
cd frontend
rm -rf node_modules dist .vite
cd ..

# Clean any remaining build artifacts
echo "Cleaning additional artifacts..."
find . -type d -name "target" -exec rm -rf {} + 2>/dev/null || true
find . -type d -name "dist" -exec rm -rf {} + 2>/dev/null || true
find . -type d -name ".vite" -exec rm -rf {} + 2>/dev/null || true

echo "✅ Clean complete!"
echo ""
echo "To reinstall dependencies:"
echo "  ./scripts/setup.sh"
echo "  or:"
echo "  cd frontend && npm install"
