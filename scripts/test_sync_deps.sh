#!/usr/bin/env bash
# test_sync_deps.sh — checks the git-hook guards of scripts/sync-deps.sh in a throwaway repository
# with a fake `npm` that only records that it was called (#112). Needs git; no network, no lefthook.
#
#   scripts/test_sync_deps.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/sync_deps_test.XXXXXX")
trap 'rm -rf "$work"' EXIT
fail=0

mkdir -p "$work/bin" "$work/repo/scripts"
cat > "$work/bin/npm" <<SHIM
#!/usr/bin/env bash
echo "npm \$*" >> "$work/npm_calls"
SHIM
chmod +x "$work/bin/npm"
cp "$root/scripts/sync-deps.sh" "$work/repo/scripts/"
cd "$work/repo"
git init -q -b main
git config user.email t@example.com
git config user.name t
mkdir app
echo '{"name":"app"}' > app/package.json
echo '{"v":1}' > app/package-lock.json
git add -A && git commit -q -m "base"

export PATH="$work/bin:$PATH"
unset CI SYNC_DEPS_SKIP

calls() { [ -f "$work/npm_calls" ] && wc -l < "$work/npm_calls" | tr -d ' ' || echo 0; }
reset() { rm -f "$work/npm_calls"; }
check() {  # check <name> <expected call count>
  local got; got=$(calls)
  if [ "$got" = "$2" ]; then echo "ok   $1 ($got install)"; else echo "FAIL $1: expected $2 install(s), got $got"; fail=1; fi
  reset
}

# a second branch whose lockfile differs, and the commit that is the "old HEAD" for checkouts
git checkout -q -b other
echo '{"v":2}' > app/package-lock.json
git commit -q -am "lockfile change"
git checkout -q main
reset
old=$(git rev-parse main)

# post-checkout, branch checkout (flag 1), lockfile differs -> installs once
git checkout -q other
scripts/sync-deps.sh --hook post-checkout "$old" 1
check "branch checkout with a changed lockfile" 1

# post-checkout, file checkout (flag 0) -> nothing
scripts/sync-deps.sh --hook post-checkout "$old" 0
check "file checkout" 0

# post-checkout during a rebase -> nothing (post-rewrite handles the end of the rebase)
mkdir -p "$(git rev-parse --git-path rebase-merge)"
scripts/sync-deps.sh --hook post-checkout "$old" 1
check "checkout during a rebase" 0
rmdir "$(git rev-parse --git-path rebase-merge)"

# post-rewrite after an amend -> nothing, whatever ORIG_HEAD is
git update-ref ORIG_HEAD "$old"
scripts/sync-deps.sh --hook post-rewrite amend
check "post-rewrite after an amend" 0

# post-rewrite at the end of a rebase -> installs once
scripts/sync-deps.sh --hook post-rewrite rebase
check "post-rewrite after a rebase" 1

# the plain forms keep working
scripts/sync-deps.sh --since "$old"
check "--since" 1
scripts/sync-deps.sh
check "default range (ORIG_HEAD..HEAD)" 1

# an unknown hook name is ignored with a message, exit 0
scripts/sync-deps.sh --hook bogus >/dev/null 2>&1
check "unknown hook" 0

exit $fail
