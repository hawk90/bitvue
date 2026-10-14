#!/usr/bin/env python3
"""Production-code branch coverage of one workspace crate.

usage: scripts/branch_coverage.py <crate> [--top N]

Needs the nightly toolchain with llvm-tools (`--branch` is unstable). cargo-llvm-cov 0.7.1 reports 0%
with the new cargo build-dir layout (test binaries under target/llvm-cov-target/debug/build/<pkg>/<hash>/out/),
so this runs the tests through it with --no-report and calls llvm-cov directly.
Test code is excluded: everything after a file's first `#[cfg(test)]`, and tests.rs / *_tests.rs / test_support.rs.
"""
import glob, json, os, re, subprocess, sys, time

crate = sys.argv[1]
top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 25
root = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
os.chdir(root)

def run(cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw).stdout

run(["cargo", "+nightly", "llvm-cov", "clean", "--workspace"])
started = time.time()
for targets in (["--lib", "--tests"], ["--bins", "--tests"]):  # binary-only crates have no --lib
    if subprocess.run(["cargo", "+nightly", "llvm-cov", "--branch", "-p", crate, *targets, "--no-report"],
                      capture_output=True, text=True).returncode == 0:
        break
else:
    sys.exit(f"{crate}: tests did not build or pass")
tools = run(["rustc", "+nightly", "--print", "sysroot"]).strip() + "/lib/rustlib/aarch64-apple-darwin/bin"
tools = tools if os.path.isdir(tools) else glob.glob(run(["rustc", "+nightly", "--print", "sysroot"]).strip() + "/lib/rustlib/*/bin")[0]
prof = f"/tmp/{crate}.profdata"
run([f"{tools}/llvm-profdata", "merge", "-o", prof] + glob.glob("target/llvm-cov-target/*.profraw"))
bins = [p for p in glob.glob(f"target/llvm-cov-target/debug/build/{crate}/*/out/*")
        if os.access(p, os.X_OK) and not re.search(r"\.(o|d|rmeta|rlib|dylib|so)$", p) and os.path.isfile(p)
        and os.path.getmtime(p) >= started - 1]  # `clean` keeps old binaries in the new build-dir layout
if not bins:
    sys.exit("no test binaries found")
# One export per binary: a combined multi-object export does not add up the counts per file. The merged
# profile gives every binary the counts of all runs, so take the per-branch maximum over the binaries.
merged = {}  # path -> {(l1, c1, l2, c2): [true_count, false_count]}
for binary in bins:
    done = subprocess.run([f"{tools}/llvm-cov", "export", binary, "-instr-profile=" + prof, "-format=text"],
                          capture_output=True, text=True)
    if done.returncode != 0:
        continue  # e.g. a test binary that links no instrumented code
    exported = json.loads(done.stdout)
    for f in exported["data"][0]["files"]:
        slot = merged.setdefault(f["filename"], {})
        for b in f.get("branches", []):
            cur = slot.setdefault(tuple(b[:4]), [0, 0])
            cur[0] = max(cur[0], b[4])
            cur[1] = max(cur[1], b[5])

src = f"crates/{crate}/src/"
rows = []
for filename, branches in merged.items():
    path = os.path.relpath(filename, root)
    if not path.startswith(src) or re.search(r"(^|/)(tests?|test_support)\.rs$|_tests?\.rs$", path):
        continue
    lines = open(path).read().split("\n")
    limit = next((i + 1 for i, l in enumerate(lines) if l.strip().startswith("#[cfg(test)]")), len(lines) + 1)
    total = missed = 0
    for key, counts in branches.items():
        if key[0] >= limit:
            continue
        for taken in counts:
            total += 1
            missed += taken == 0
    if total:
        rows.append((missed, total, path[len(src):]))
tb = sum(r[1] for r in rows); tm = sum(r[0] for r in rows)
print(f"{crate}: production branch outcomes {tb}, never taken {tm}, covered {100 * (tb - tm) / tb:.1f}%" if tb else f"{crate}: no branches")
for missed, total, path in sorted(rows, reverse=True)[:top]:
    print(f"{missed:5d}/{total:<5d} never taken ({100 * (total - missed) / total:5.1f}% covered)  {path}")
