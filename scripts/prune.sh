#!/bin/bash
# Reclaim disk without a full rebuild.
#
# target/debug/incremental is the part of target/ that grows without bound (it was 7.9 GB of a
# 12 GB target/, ~60% of a fresh build's size) because every check/clippy/test/doc variant keeps
# its own incremental sessions. It is only a cache: deleting it costs one slower rebuild, nothing
# else. For a from-scratch clean (also node_modules/dist) use scripts/clean.sh instead.
set -euo pipefail
cd "$(dirname "$0")/.."

size() { du -sk "$1" 2>/dev/null | awk '{printf "%.1f GB", $1/1048576}'; }

[ -d target ] || { echo "no target/ here, nothing to prune"; exit 0; }
before=$(size target)
rm -rf target/debug/incremental target/release/incremental
echo "target/: $before -> $(size target)"
