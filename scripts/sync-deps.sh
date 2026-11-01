#!/usr/bin/env bash
# sync-deps.sh — install dependencies whose lockfile changed, so a `git pull` /
# `git checkout` leaves node_modules and the cargo cache matching the tree.
#
#   scripts/sync-deps.sh                 # compare ORIG_HEAD..HEAD (post-merge)
#   scripts/sync-deps.sh --since <rev>   # compare <rev>..HEAD
#   scripts/sync-deps.sh --all           # install everything (bootstrap / fresh clone)
#   scripts/sync-deps.sh --hook post-rewrite <rewrite-type>
#   scripts/sync-deps.sh --hook post-checkout <old-head> <branch-flag>
#
# The two --hook forms are what lefthook.toml calls, with git's own hook arguments, and skip the
# runs that would be wrong or repeated (#112):
#   * post-rewrite also fires for `git commit --amend`, which does not move ORIG_HEAD, so the
#     range would be stale: only `rebase` is handled;
#   * post-checkout fires for file checkouts (branch flag 0) and, during a rebase, for each
#     checkout of the commit being rebased onto; post-rewrite runs once when the rebase ends.
#
# Runs npm ci / pnpm install --frozen-lockfile / cargo fetch only for the lockfiles that
# changed. Never runs in CI. Exit code is 0 even if an install fails, so a hook can't
# block git; failures are printed.
set -u
cd "$(git rev-parse --show-toplevel)" || exit 0
[ -n "${CI:-}" ] && exit 0
[ -n "${SYNC_DEPS_SKIP:-}" ] && exit 0

mode="range"; since="ORIG_HEAD"
case "${1:-}" in
  --all) mode="all" ;;
  --since) since="${2:-ORIG_HEAD}" ;;
  --hook)
    case "${2:-}" in
      post-rewrite)
        [ "${3:-}" = rebase ] || exit 0 ;;
      post-checkout)
        [ "${4:-}" = 1 ] || exit 0
        for dir in rebase-merge rebase-apply; do
          [ -d "$(git rev-parse --git-path "$dir")" ] && exit 0
        done
        since="${3:-ORIG_HEAD}" ;;
      *) echo "sync-deps: unknown hook '${2:-}'" >&2; exit 0 ;;
    esac ;;
esac

changed() {  # print tracked paths matching the pathspecs that changed in the range (or all, in --all mode)
  if [ "$mode" = all ]; then git ls-files -- "$@"
  else git rev-parse -q --verify "$since" >/dev/null 2>&1 || return 0
       git diff --name-only "$since" HEAD -- "$@" 2>/dev/null
  fi
}

run() {  # run <dir> <label> <cmd...>
  local dir=$1 label=$2; shift 2
  echo "sync-deps: $label in ./$dir"
  ( cd "$dir" && "$@" ) || echo "sync-deps: WARNING: '$*' failed in ./$dir — run it by hand"
}

while IFS= read -r lock; do [ -n "$lock" ] || continue
  d=$(dirname "$lock"); [ -f "$d/package.json" ] || continue
  run "$d" "npm ci" npm ci --no-fund --no-audit
done < <(changed 'package-lock.json' '*/package-lock.json' '**/package-lock.json' | grep -v node_modules | sort -u)

while IFS= read -r lock; do [ -n "$lock" ] || continue
  d=$(dirname "$lock"); [ -f "$d/package.json" ] || continue
  run "$d" "pnpm install --frozen-lockfile" pnpm install --frozen-lockfile
done < <(changed 'pnpm-lock.yaml' '*/pnpm-lock.yaml' '**/pnpm-lock.yaml' | grep -v node_modules | sort -u)

while IFS= read -r lock; do [ -n "$lock" ] || continue
  d=$(dirname "$lock"); [ -f "$d/Cargo.toml" ] || continue
  command -v cargo >/dev/null 2>&1 || continue
  run "$d" "cargo fetch" cargo fetch --quiet
done < <(changed 'Cargo.lock' '*/Cargo.lock' '**/Cargo.lock' | sort -u)
exit 0
