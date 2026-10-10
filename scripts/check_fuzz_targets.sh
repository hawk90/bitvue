#!/usr/bin/env bash
# scripts/check_fuzz_targets.sh
#
# fuzz/ is excluded from the cargo workspace (it needs its own Cargo.toml for cargo-fuzz), so
# `cargo build/test --workspace` never touches it. Renaming a crate or changing an API silently
# broke every target once (#98); this keeps that from happening again:
#
#   1. every fuzz/fuzz_targets/*.rs is registered as a [[bin]] in fuzz/Cargo.toml, and every
#      registered path exists (an unregistered target is never built, never fuzzed);
#   2. every target compiles against the current crates (`cargo check`, stable is enough).
#
# Usage: ./scripts/check_fuzz_targets.sh
# Exit codes: 0 all fine, 1 a target is unregistered/missing or does not compile.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/fuzz"

status=0

for file in fuzz_targets/*.rs; do
  name="$(basename "$file" .rs)"
  if ! grep -Eq "^name = \"$name\"$" Cargo.toml; then
    echo "fuzz/$file is not registered as a [[bin]] in fuzz/Cargo.toml" >&2
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
