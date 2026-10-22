#!/usr/bin/env bash
# Every test binary must install the capped allocator (bitvue_test_alloc::install!()), otherwise a
# runaway mutant in that binary is only caught by the slower RSS-polling guard. Lists the misses.
cd "$(dirname "$0")/.."
skip='bitvue-test-alloc|bitvue-test-data|bitvue-benchmarks|vendor'
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
