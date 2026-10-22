#!/usr/bin/env bash
# Run cargo-mutants with sane jobs/timeout and report which mutants the memory guards killed.
# The guards themselves are NOT optional and not in this script: they are wired into
# .cargo/mutants.toml (scripts/mem_guard_runner.sh) and into every test binary
# (crates/bitvue-test-alloc), so a bare `cargo mutants` is protected too. This wrapper only adds
# convenience. Background: a runaway mutant kernel-panicked the machine twice on 2026-10-22.
# Extra args go to cargo-mutants, e.g. `scripts/run_mutants.sh -p bitvue-engine -f extractor.rs`.
set -uo pipefail
cd "$(dirname "$0")/.."

out_dir=mutants.out
prev=""
for a in "$@"; do
  case "$prev" in -o|--output) out_dir="$a/mutants.out" ;; esac
  case "$a" in --output=*) out_dir="${a#--output=}/mutants.out" ;; esac
  prev="$a"
done

# No fixed --timeout: it would also cap the baseline, and the first launch of freshly built test
# binaries is slow on macOS (Gatekeeper scan, ~8 s each). The floor lives in .cargo/mutants.toml.
cargo mutants --jobs "${MUTANTS_JOBS:-2}" "$@"
status=$?

# Killed-by-guard report. cargo-mutants counts a mutant whose test binary was killed as "caught"
# (failed test), so without this a runaway mutant looks like an ordinary catch.
python3 - "$out_dir" <<'PY'
import json, sys
out = sys.argv[1]
try:
    outcomes = json.load(open(f"{out}/outcomes.json"))["outcomes"]
except (OSError, ValueError):
    sys.exit(0)
hits = []
for o in outcomes:
    s = o.get("scenario")
    if not isinstance(s, dict):
        continue
    try:
        log = open(f"{out}/{o['log_path']}", errors="replace").read()
    except OSError:
        continue
    for tag in ("TEST_ALLOC", "MEM_GUARD"):
        if tag + ":" in log:
            m = s["Mutant"]
            hits.append(f"{m['name']}  [{tag}]")
            break
report = f"{out}/memory_guard_killed.txt"
open(report, "w").write("".join(h + "\n" for h in hits))
print(f"\nmemory-guard kills: {len(hits)} (runaway allocation; counted as 'caught' above) -> {report}")
for h in hits:
    print("  " + h)
PY
exit $status
