#!/usr/bin/env bash
# Cargo test runner that kills the test binary (and its children) once their combined RSS
# exceeds BITVUE_MEM_LIMIT_MB (default 2048; backstop to the 1 GiB allocator cap in bitvue-test-alloc). Normal bitvue-engine tests peak at ~50 MB.
#
# Why: cargo-mutants rewrites operators (e.g. `offset += n` -> `offset *= n`), which can turn a
# loop into an endless one that allocates on every pass. A mutant like that ate all 32 GB within
# seconds and kernel-panicked the machine twice on 2026-10-22 (watchdog timeout, compressor full).
# macOS ignores `ulimit -v`, so the limit has to be enforced by polling.
#
# Wired in via .cargo/mutants.toml (additional_cargo_args), so bare `cargo mutants` is guarded too. A killed binary exits 137,
# which cargo-mutants records as a failed (caught) test; the log line says why.
limit_mb=${BITVUE_MEM_LIMIT_MB:-2048}

"$@" &
pid=$!

tree_rss_kb() {
  ps -A -o pid=,ppid=,rss= | awk -v root="$pid" '
    { ppid[$1] = $2; rss[$1] = $3 }
    END {
      for (p in rss) {
        q = p
        while (q != "" && q != 0 && q != 1) { if (q == root) { sum += rss[p]; break }; q = ppid[q] }
      }
      print sum + 0
    }'
}

(
  while kill -0 "$pid" 2>/dev/null; do
    kb=$(tree_rss_kb)
    if [ "$kb" -gt $((limit_mb * 1024)) ]; then
      echo "MEM_GUARD: killed $1 at $((kb / 1024)) MB (limit ${limit_mb} MB)" >&2
      pkill -KILL -P "$pid" 2>/dev/null
      kill -KILL "$pid" 2>/dev/null
      exit 0
    fi
    sleep 0.05
  done
) &
watcher=$!

wait "$pid"
status=$?
kill "$watcher" 2>/dev/null
wait "$watcher" 2>/dev/null
exit "$status"
