#!/bin/bash
# Generate module declarations for all test files in bitvue-core/tests/
# This consolidates 4526+ test files into a single test binary

set -e

TEST_DIR="crates/bitvue-core/tests"
OUTPUT_FILE="$TEST_DIR/all_tests.rs"
HEADER_FILE="$TEST_DIR/test_modules_header.rs"

echo "Generating module declarations for test files..."

# Create the header
cat > "$OUTPUT_FILE" << 'EOF'
// Consolidated integration tests for bitvue-core
//
// This file consolidates all 4526+ test files into a single test binary
// to avoid linker OOM errors in CI (signal 7 [Bus error]).
//
// Each test file is included as a module using #[path = "..."]
//
// Regenerate with: ./scripts/generate_test_modules.sh

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

EOF

# Generate module declarations for each .rs file (excluding all_tests.rs itself)
cd "$TEST_DIR"
for file in *.rs; do
    if [ "$file" != "all_tests.rs" ] && [ "$file" != "test_modules_header.rs" ]; then
        # Convert filename to module name (remove .rs extension)
        module_name="${file%.rs}"
        # Escape special characters in filename for #[path]
        escaped_file="$file"
        echo "#[path = \"$escaped_file\"]" >> "$OUTPUT_FILE"
        echo "mod $module_name;" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi
done

# Count modules
module_count=$(grep -c "^mod " "$OUTPUT_FILE" || true)
echo "Generated $module_count module declarations in $OUTPUT_FILE"
echo ""
echo "Next steps:"
echo "1. Rename all test files to exclude them from normal compilation:"
echo "   cd $TEST_DIR"
echo "   for f in *.rs; do mv \"\$f\" \"\${f}.rs.bak\"; done"
echo "   mv all_tests.rs.bak all_tests.rs"
echo ""
echo "2. Or use .cargo-ignore approach:"
echo "   Create $TEST_DIR/.cargo-ignore pattern: *.rs.bak"
