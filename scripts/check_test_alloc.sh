#!/usr/bin/env bash
# Every test binary must install the capped allocator (bitvue_test_alloc::install!()), otherwise a
# runaway mutant in that binary is only caught by the slower RSS-polling guard. Lists the misses.
# Run by CI (rust-lint). A text match only: a mention in a comment passes, and binaries with a custom
# [[test]] path or under src/bin/ are not looked at -- extend the file list below if one appears.
cd "$(dirname "$0")/.."
skip='bitvue-test-alloc|bitvue-benchmarks|vendor'
missing=0
for c in crates/*/; do
  name=$(basename "$c"); echo "$name" | grep -Eq "^($skip)$" && continue
  for f in "$c"src/lib.rs "$c"src/main.rs "$c"tests/*.rs "$c"tests/*/main.rs; do
    [ -f "$f" ] || continue
    grep -q 'bitvue_test_alloc::install!' "$f" || { echo "missing install!(): $f"; missing=1; }
  done
done
[ $missing -eq 0 ] && echo "ok: every test binary installs the capped allocator"
exit $missing
