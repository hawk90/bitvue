#!/bin/bash
# Consolidate bitvue-core tests from tests/ to src/tests/
# This solves the linker OOM issue (signal 7 [Bus error])

set -e

# Get script directory (repository root)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

CORE_DIR="$REPO_ROOT/crates/bitvue-core"
TESTS_DIR="$CORE_DIR/tests"
SRC_TESTS_DIR="$CORE_DIR/src/tests"

echo "=== Consolidating bitvue-core tests ==="
echo "  Repository root: $REPO_ROOT"
echo "  Core directory: $CORE_DIR"
echo ""

# Step 1: Create src/tests/ directory
echo "Step 1: Creating $SRC_TESTS_DIR..."
mkdir -p "$SRC_TESTS_DIR"

# Step 2: Move all test files to src/tests/
echo "Step 2: Moving test files to src/tests/..."
cd "$TESTS_DIR" || exit 1

# Count files to move
FILE_COUNT=$(find . -maxdepth 1 -name "*.rs" -type f | grep -v "all_tests.rs" | wc -l | tr -d ' ')
echo "  Found $FILE_COUNT test files to move"

# Move all .rs files except all_tests.rs (which was just created)
for file in *.rs; do
    if [ "$file" != "all_tests.rs" ]; then
        mv "$file" "$SRC_TESTS_DIR/"
    fi
done

echo "  Moved $FILE_COUNT files"

# Step 3: Create mod.rs in src/tests/
echo "Step 3: Creating src/tests/mod.rs..."
cat > "$SRC_TESTS_DIR/mod.rs" << 'EOF'
//! Consolidated integration tests for bitvue-core
//!
//! These tests were moved from tests/ to src/tests/ to solve
//! linker OOM issues in CI (signal 7 [Bus error]).
//!
//! In tests/, each file is compiled as a separate binary.
//! In src/tests/, all tests are compiled into the lib.rs binary.

#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]

// Re-export all test modules
EOF

# Add module declarations for each test file
cd "$SRC_TESTS_DIR"
for file in *.rs; do
    if [ "$file" != "mod.rs" ]; then
        module_name="${file%.rs}"
        echo "pub mod $module_name;" >> "mod.rs"
    fi
done

# Count modules added
MODULE_COUNT=$(grep -c "^pub mod" "mod.rs" || true)
echo "  Created mod.rs with $MODULE_COUNT module declarations"

# Step 4: Add tests module to lib.rs
echo "Step 4: Adding tests module to lib.rs..."
cd "$CORE_DIR"
if ! grep -q "^pub mod tests;" src/lib.rs; then
    echo "" >> src/lib.rs
    echo "// Consolidated tests (moved from tests/ to avoid linker OOM)" >> src/lib.rs
    echo "#[cfg(test)]" >> src/lib.rs
    echo "pub mod tests;" >> src/lib.rs
    echo "  Added #[cfg(test)] pub mod tests; to lib.rs"
else
    echo "  tests module already exists in lib.rs"
fi

# Step 5: Clean up tests/ directory
echo "Step 5: Cleaning up tests/ directory..."
cd "$TESTS_DIR"
rm -f all_tests.rs  # Remove the file we created earlier

# Create a minimal integration test that just re-exports the main tests
cat > "integration_tests.rs" << 'EOF'
//! Minimal integration tests for bitvue-core
//!
//! The actual tests are in src/tests/ (compiled as part of lib.rs)
//! to avoid linker OOM from compiling 4500+ separate test binaries.

use bitvue_core::*;

#[test]
fn lib_can_be_imported() {
    // Basic smoke test that the library compiles and imports work
    let _selection = SelectionState::default();
}

#[test]
fn basic_types_work() {
    let frame = FrameType::I;
    assert!(matches!(frame, FrameType::I));
}
EOF

echo "  Created minimal integration_tests.rs"
echo ""
echo "=== Summary ==="
echo "  Moved $FILE_COUNT test files from tests/ to src/tests/"
echo "  Tests now compile into lib.rs (single binary)"
echo "  tests/ now contains only minimal integration tests"
echo ""
echo "Next: Run 'cargo test -p bitvue-core' to verify"
