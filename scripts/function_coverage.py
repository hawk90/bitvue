#!/usr/bin/env python3
"""Per-function and per-file production LINE coverage of one crate (test code excluded).

Reads the profile data and test binaries that `scripts/branch_coverage.py <crate>` leaves behind
(/tmp/<crate>.profdata, target/llvm-cov-target), so run that first. Counts are unioned (max) over all
test binaries. Line rule (same as llvm-cov): a line's count is the largest count among the region
entries starting on it and the segment already in force when the line begins; a line with no entry
takes the count of the wrapping segment.

usage: function_coverage.py <crate> [--min-lines N] [--files REGEX] [--uncovered FILE:A-B] [--verify]
  --min-lines N       list functions longer than N lines (default 100); exit criterion: none below 90%
  --files REGEX       restrict the per-file table to matching paths
  --uncovered F:A-B   print the unexecuted lines of src/F between lines A and B
  --verify            compare every production line with `llvm-cov show` and exit 1 on a mismatch

A function's span runs from its first to its last region, so it includes nested closures; llvm-cov's
own per-function "Lines" column leaves those out. The executed/missed split is the same.
"""
import glob, json, os, re, subprocess, sys

TEST_FILE = re.compile(r"(^|/)(tests?|test_support)\.rs$|_tests?\.rs$")


def arg(flag, default=None):
    return sys.argv[sys.argv.index(flag) + 1] if flag in sys.argv else default


def main():
    if len(sys.argv) < 2 or sys.argv[1].startswith("-"):
        sys.exit(__doc__)
    crate = sys.argv[1]
    min_lines = int(arg("--min-lines", 100))
    root = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
    os.chdir(root)
    sysroot = subprocess.check_output(["rustc", "+nightly", "--print", "sysroot"], text=True).strip()
    tools = glob.glob(sysroot + "/lib/rustlib/*/bin")[0]
    prof = f"/tmp/{crate}.profdata"
    bins = [p for p in glob.glob(f"target/llvm-cov-target/debug/build/{crate}/*/out/*")
            if os.path.isfile(p) and os.access(p, os.X_OK) and not re.search(r"\.(o|d|rmeta|rlib|dylib|so)$", p)]
    if not bins or not os.path.exists(prof):
        sys.exit(f"no coverage data for {crate}: run `python3 scripts/branch_coverage.py {crate}` first")
    src = f"crates/{crate}/src/"

    lines = {}   # path -> {line: max count over binaries}
    funcs = {}   # (path, name, start) -> [start, end]
    for b in bins:
        d = subprocess.run([f"{tools}/llvm-cov", "export", b, "-instr-profile=" + prof, "-format=text"],
                           capture_output=True, text=True)
        if d.returncode:
            continue
        data = json.loads(d.stdout)["data"][0]
        for f in data["files"]:
            path = os.path.relpath(f["filename"], root)
            segs = f["segments"]  # [line, col, count, hasCount, isRegionEntry, isGap]
            if not path.startswith(src) or not segs:
                continue
            by_line = {}
            for s in segs:
                by_line.setdefault(s[0], []).append(s)
            wrapped = None  # segment in force at the start of the line
            cur = lines.setdefault(path, {})
            for ln in range(segs[0][0], segs[-1][0] + 1):
                starts = by_line.get(ln, [])
                entries = [s[2] for s in starts if s[3] and s[4] and not s[5]]
                inherited = [wrapped[2]] if wrapped is not None and wrapped[3] else []
                counts = entries + inherited if entries else inherited
                if starts:
                    wrapped = starts[-1]
                if counts:
                    cur[ln] = max(cur.get(ln, 0), max(counts))
        for fn in data["functions"]:
            own = [r for r in fn["regions"] if r[5] == 0]  # file id 0 = the function's own file
            path = os.path.relpath(fn["filenames"][0], root)
            if not own or not path.startswith(src):
                continue
            start, end = min(r[0] for r in own), max(r[2] for r in own)
            e = funcs.setdefault((path, fn["name"], start), [start, end])
            e[1] = max(e[1], end)

    # Inline unit tests live after the first #[cfg(test)]; everything from there on is test code.
    cutoff = {}
    for path in lines:
        text = open(path).read().split("\n")
        cutoff[path] = next((i + 1 for i, l in enumerate(text) if l.strip().startswith("#[cfg(test)]")),
                            len(text) + 1)

    rows, seen = [], set()
    for (path, name, start), (s, e) in funcs.items():
        if TEST_FILE.search(path) or s >= cutoff[path] or (path, s, e) in seen:
            continue
        ex = [c for ln, c in lines[path].items() if s <= ln <= e]
        if not ex:
            continue
        seen.add((path, s, e))
        rows.append((e - s + 1, len(ex), sum(1 for c in ex if c > 0), path[len(src):], s))
    rows.sort(key=lambda r: -r[0])
    big = [r for r in rows if r[0] > min_lines]
    print(f"functions measured: {len(rows)}; longer than {min_lines} lines: {len(big)}")
    print("span  exec  cov%   file:line  function")
    for span, n, cov, path, s in big:
        text = open(src + path).read().split("\n")
        sig = next((l.strip() for l in text[s - 1:s + 6] if re.search(r"\bfn\b", l)), text[s - 1].strip())
        print(f"{span:4d} {n:5d} {100 * cov / n:5.1f}%  {path}:{s}  {sig[:80]}")
    low = [r for r in big if 100 * r[2] / r[1] < 90]
    print(f"\nlonger than {min_lines} lines and below 90% line coverage: {len(low)}")

    pattern = arg("--files")
    print("\nper-file line coverage:")
    for path in sorted(lines):
        if TEST_FILE.search(path) or (pattern and not re.search(pattern, path)):
            continue
        ex = [c for ln, c in lines[path].items() if ln < cutoff[path]]
        if ex:
            print(f"{100 * sum(1 for c in ex if c > 0) / len(ex):5.1f}%  {len(ex):5d}  {path[len(src):]}")

    spec = arg("--uncovered")
    if spec:
        f, rng = spec.rsplit(":", 1)
        a, b = map(int, rng.split("-"))
        text = open(src + f).read().split("\n")
        for ln in range(a, b + 1):
            if lines[src + f].get(ln) == 0:
                print(f"{ln:5d} NOT RUN  {text[ln - 1].rstrip()[:110]}")

    if "--verify" in sys.argv:
        objs = [bins[0]] + [x for b in bins[1:] for x in ("-object", b)]
        total = bad = 0
        for path in sorted(lines):
            if TEST_FILE.search(path):
                continue
            out = subprocess.run([f"{tools}/llvm-cov", "show", *objs, "-instr-profile=" + prof, path],
                                 capture_output=True, text=True).stdout
            ref = {int(m.group(1)): m.group(2) != "0"
                   for m in (re.match(r"\s*(\d+)\|\s*([\d.]+[kMGT]?)\|", l) for l in out.split("\n")) if m}
            for ln, c in lines[path].items():
                if ln >= cutoff[path]:
                    continue
                total += 1
                if ref.get(ln) != (c > 0):
                    bad += 1
                    if bad <= 25:
                        print(f"MISMATCH {path}:{ln} ours={c > 0} llvm-cov={ref.get(ln)}")
        print(f"\nverify: {total} production lines compared with `llvm-cov show`, {bad} mismatches")
        sys.exit(1 if bad else 0)


main()
