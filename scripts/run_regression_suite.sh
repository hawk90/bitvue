#!/usr/bin/env bash
# scripts/run_regression_suite.sh
#
# Phase 12: Unified Regression Suite
#
# Runs all bitvue regression and parity checks in one command:
#   1. cargo test  — unit + integration tests (all workspace crates)
#   2. parity_check.sh --local  — CLI-level parity checks (local fixtures only)
#
# Usage:
#   ./scripts/run_regression_suite.sh           # full suite
#   ./scripts/run_regression_suite.sh --fast    # skip slow tests (no doc-tests)
#   ./scripts/run_regression_suite.sh --parity-full  # include network downloads
#   ./scripts/run_regression_suite.sh --help
#
# Exit codes:
#   0  all checks passed
#   1  one or more checks failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# ── Colour helpers ─────────────────────────────────────────────────────────────
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

header() { echo -e "\n${BOLD}${CYAN}══ $* ══${NC}"; }
ok()     { echo -e "${GREEN}PASS${NC} $*"; }
fail()   { echo -e "${RED}FAIL${NC} $*"; }
warn()   { echo -e "${YELLOW}WARN${NC} $*"; }

# ── Argument parsing ───────────────────────────────────────────────────────────
FAST=false
PARITY_LOCAL="--local"

for arg in "$@"; do
  case $arg in
    --fast)         FAST=true ;;
    --parity-full)  PARITY_LOCAL="" ;;
    --help)
      echo "Usage: $0 [--fast] [--parity-full] [--help]"
      echo "  --fast          Skip doc-tests, run only lib/bin/integration tests"
      echo "  --parity-full   Run parity_check.sh without --local (downloads test vectors)"
      exit 0
      ;;
  esac
done

PASS_STAGES=0
FAIL_STAGES=0

run_stage() {
  local name="$1"; shift
  header "$name"
  if "$@"; then
    ok "$name"
    PASS_STAGES=$((PASS_STAGES + 1))
  else
    fail "$name"
    FAIL_STAGES=$((FAIL_STAGES + 1))
  fi
}

# ── Stage 1: cargo test (all workspace crates) ─────────────────────────────────
cd "$ROOT_DIR"

CARGO_TEST_FLAGS=()
if $FAST; then
  CARGO_TEST_FLAGS+=("--lib" "--bins" "--tests")
fi

run_stage "cargo test (workspace)" \
  cargo test --workspace "${CARGO_TEST_FLAGS[@]+"${CARGO_TEST_FLAGS[@]}"}" 2>&1

# ── Stage 2: parity_check.sh ──────────────────────────────────────────────────
# shellcheck disable=SC2086
run_stage "parity_check.sh" \
  bash "$SCRIPT_DIR/parity_check.sh" $PARITY_LOCAL

# ── Summary ────────────────────────────────────────────────────────────────────
echo
echo -e "${BOLD}══════════════════════════════════════${NC}"
echo -e "${BOLD}  Regression Suite Results${NC}"
echo -e "${BOLD}══════════════════════════════════════${NC}"
echo -e "  ${GREEN}PASS${NC}: $PASS_STAGES stage(s)"
if [[ $FAIL_STAGES -gt 0 ]]; then
  echo -e "  ${RED}FAIL${NC}: $FAIL_STAGES stage(s)"
  echo
  echo -e "${RED}Regression suite FAILED.${NC}"
  exit 1
else
  echo -e "  ${RED}FAIL${NC}: 0 stage(s)"
  echo
  echo -e "${GREEN}All regression checks passed.${NC}"
  exit 0
fi
