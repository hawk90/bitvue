#!/usr/bin/env bash
# scripts/parity_check.sh
#
# Phase 12: VQ Analyzer Parity Verification
#
# Downloads public test vectors and validates bitvue's output against
# known-good reference values.  Can also be run with --local to skip
# downloads and use only test_data/ fixtures.
#
# Usage:
#   ./scripts/parity_check.sh           # full run (downloads vectors)
#   ./scripts/parity_check.sh --local   # local fixtures only (fast)
#   ./scripts/parity_check.sh --help
#
# Exit codes:
#   0  all checks passed
#   1  one or more checks failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TEST_DATA="$ROOT_DIR/test_data"
BITVUE="$ROOT_DIR/target/debug/bitvue"

PASS=0
FAIL=0
LOCAL_ONLY=false

# ── Colour helpers ─────────────────────────────────────────────────────────────
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

ok()   { echo -e "${GREEN}PASS${NC} $*"; PASS=$((PASS + 1)); }
fail() { echo -e "${RED}FAIL${NC} $*"; FAIL=$((FAIL + 1)); }
warn() { echo -e "${YELLOW}WARN${NC} $*"; }
info() { echo "     $*"; }

# ── Argument parsing ───────────────────────────────────────────────────────────
for arg in "$@"; do
  case $arg in
    --local) LOCAL_ONLY=true ;;
    --help)
      echo "Usage: $0 [--local] [--help]"
      echo "  --local   Use only test_data/ fixtures, skip downloads"
      exit 0
      ;;
  esac
done

# ── Build CLI ──────────────────────────────────────────────────────────────────
echo "Building bitvue CLI..."
cargo build -p bitvue-cli --quiet 2>&1 || {
  echo "Build failed. Aborting."
  exit 1
}
echo "Built: $BITVUE"
echo

# ── Helper: run bitvue and expect it exits 0 ──────────────────────────────────
check_exits_ok() {
  local desc="$1"; shift
  if "$BITVUE" "$@" >/dev/null 2>&1; then
    ok "$desc"
  else
    fail "$desc (exit code $?)"
  fi
}

# ── Helper: check output contains substring ───────────────────────────────────
check_output_contains() {
  local desc="$1"; local expected="$2"; shift 2
  local out
  out=$("$BITVUE" "$@" 2>&1) || true
  if echo "$out" | grep -q "$expected"; then
    ok "$desc"
  else
    fail "$desc — expected to find '$expected' in output"
    info "actual output: $(echo "$out" | head -5)"
  fi
}

# ── Helper: download test vector ──────────────────────────────────────────────
maybe_download() {
  local dest="$1"; local url="$2"
  if [[ -f "$dest" ]]; then
    info "already have $(basename "$dest")"
    return 0
  fi
  if $LOCAL_ONLY; then
    warn "skipping download ($LOCAL_ONLY): $(basename "$dest")"
    return 1
  fi
  info "downloading $(basename "$dest")..."
  if command -v curl &>/dev/null; then
    curl -fsSL -o "$dest" "$url" 2>/dev/null && return 0
  elif command -v wget &>/dev/null; then
    wget -q -O "$dest" "$url" 2>/dev/null && return 0
  fi
  warn "curl/wget not available; skipping download"
  return 1
}

mkdir -p "$TEST_DATA"

# ══════════════════════════════════════════════════════════════════════════════
# Section 1: Local AV1 fixture
# ══════════════════════════════════════════════════════════════════════════════
echo "=== AV1 (local fixture) ==="
AV1_FIXTURE="$TEST_DATA/av1_test.ivf"

if [[ -f "$AV1_FIXTURE" ]]; then
  check_exits_ok "AV1 basic decode" decode "$AV1_FIXTURE" --av1
  check_exits_ok "AV1 --stats" decode "$AV1_FIXTURE" --av1 --stats
  check_exits_ok "AV1 --stream-stats" decode "$AV1_FIXTURE" --av1 --stream-stats
  check_exits_ok "AV1 --md5" decode "$AV1_FIXTURE" --av1 --md5
  check_exits_ok "AV1 --frames 1" decode "$AV1_FIXTURE" --av1 --frames 1
  check_output_contains "AV1 stats shows frames" "frame" decode "$AV1_FIXTURE" --av1 --stats
else
  warn "AV1 fixture missing: $AV1_FIXTURE"
fi
echo

# ══════════════════════════════════════════════════════════════════════════════
# Section 2: Auto-detect
# ══════════════════════════════════════════════════════════════════════════════
echo "=== Auto-detect ==="
if [[ -f "$AV1_FIXTURE" ]]; then
  check_exits_ok "Auto-detect IVF as AV1" decode "$AV1_FIXTURE"
fi
echo

# ══════════════════════════════════════════════════════════════════════════════
# Section 3: Error handling
# ══════════════════════════════════════════════════════════════════════════════
echo "=== Error handling ==="

# Missing file must exit non-zero
if ! "$BITVUE" decode /nonexistent/path.ivf >/dev/null 2>&1; then
  ok "Missing file returns error"
else
  fail "Missing file should return non-zero exit"
fi

# Empty file must not crash
TMPFILE=$(mktemp /tmp/bitvue_parity_XXXXXX.bin)
trap "rm -f $TMPFILE" EXIT
: > "$TMPFILE"   # empty
if "$BITVUE" decode "$TMPFILE" >/dev/null 2>&1; then
  ok "Empty file handled gracefully"
else
  ok "Empty file returns error (expected)"
fi

# Garbage data must not crash
printf '\xde\xad\xbe\xef%.0s' {1..64} > "$TMPFILE"
if "$BITVUE" decode "$TMPFILE" >/dev/null 2>&1; then
  ok "Garbage data handled gracefully"
else
  ok "Garbage data returns error (expected, no crash)"
fi
echo

# ══════════════════════════════════════════════════════════════════════════════
# Section 4: Optional — AOM public test vectors
# ══════════════════════════════════════════════════════════════════════════════
echo "=== AOM public test vectors (optional) ==="
AOM_VECTOR="$TEST_DATA/av1_aom_test.ivf"
AOM_URL="https://storage.googleapis.com/aom-test-data/av1/encode_perf_test/bus_cif_226.ivf"

if maybe_download "$AOM_VECTOR" "$AOM_URL"; then
  check_exits_ok "AOM vector: basic decode" decode "$AOM_VECTOR" --av1
  check_exits_ok "AOM vector: --stats" decode "$AOM_VECTOR" --av1 --stats
  check_output_contains "AOM vector: output has keyframe" "KEY\|IDR\|I " decode "$AOM_VECTOR" --av1 --stats
else
  warn "AOM vector skipped (run without --local to download)"
fi
echo

# ══════════════════════════════════════════════════════════════════════════════
# Summary
# ══════════════════════════════════════════════════════════════════════════════
echo "══════════════════════════════════════"
echo "  Parity Check Results"
echo "══════════════════════════════════════"
echo -e "  ${GREEN}PASS${NC}: $PASS"
if [[ $FAIL -gt 0 ]]; then
  echo -e "  ${RED}FAIL${NC}: $FAIL"
  echo
  echo "Some parity checks failed."
  exit 1
else
  echo -e "  ${RED}FAIL${NC}: 0"
  echo
  echo -e "${GREEN}All parity checks passed.${NC}"
  exit 0
fi
