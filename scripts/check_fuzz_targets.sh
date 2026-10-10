#!/usr/bin/env bash
# scripts/check_fuzz_targets.sh
#
# fuzz/ is excluded from the cargo workspace (it needs its own Cargo.toml for cargo-fuzz), so
# `cargo build/test --workspace` never touches it. Renaming a crate or changing an API silently
# broke every target once (#98); this keeps that from happening again:
#
#   1. every fuzz/fuzz_targets/*.rs is the `path` of a [[bin]] in fuzz/Cargo.toml, and every
#      registered path exists (an unregistered target is never built, never fuzzed);
#   2. every target compiles against the current crates (`cargo check`, stable is enough).
#
# Usage: ./scripts/check_fuzz_targets.sh
# Exit codes: 0 all fine; 1 a target is unregistered or its file is missing; cargo's own code
# (101) when a target does not compile.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/fuzz"

status=0

# Match on `path`, not `name`: cargo builds the file a [[bin]] points at, so an entry named after
# one target but pointing at another would leave the first one unbuilt.
shopt -s nullglob
files=(fuzz_targets/*.rs)
shopt -u nullglob
if [[ ${#files[@]} -eq 0 ]]; then
  echo "fuzz/fuzz_targets has no targets" >&2
  exit 1
fi
for file in "${files[@]}"; do
  if ! grep -Fxq "path = \"$file\"" Cargo.toml; then
    echo "fuzz/$file is not the path of any [[bin]] in fuzz/Cargo.toml" >&2
    status=1
  fi
done

while read -r path; do
  if [[ ! -f "$path" ]]; then
    echo "fuzz/Cargo.toml registers $path, which does not exist" >&2
    status=1
  fi
done < <(sed -n 's/^path = "\(fuzz_targets\/.*\)"$/\1/p' Cargo.toml)

if [[ $status -ne 0 ]]; then
  exit "$status"
fi

cargo check --bins --locked
