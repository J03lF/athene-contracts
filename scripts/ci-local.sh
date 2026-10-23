#!/usr/bin/env bash
# Local run of the CI quality gates (.github/workflows/ci.yml), step 1.1 of the
# Athene quality overhaul. Works without any push: siblings are resolved from
# the local tree through the same ../../ path patches CI recreates.
#
#   scripts/ci-local.sh            all gates
#   scripts/ci-local.sh fmt clippy only the named gates (fmt clippy test deny)
#
# Honours CARGO_TARGET_DIR. cargo-deny is taken from $CARGO_DENY, else PATH;
# when missing, the deny gate is reported as SKIPPED (not passed).
set -uo pipefail

cd "$(dirname "$0")/.."
REPO_DIR=$(pwd)

gates=("$@")
[ ${#gates[@]} -eq 0 ] && gates=(fmt clippy test deny)

declare -a results=()
failed=0
run_gate() {
  local name=$1; shift
  echo "==> $name: $*"
  if "$@"; then results+=("$name: ok"); else results+=("$name: FAILED"); failed=1; fi
}

# CI builds siblings at the SHAs pinned in ci.yml; warn when the local
# checkout differs, because then "green here" is not "green in CI".
check_sibling() {
  local var=$1 dir=$2
  local pinned
  pinned=$(sed -n "s/^  $var: \([0-9a-f]\{40\}\).*/\1/p" .github/workflows/ci.yml)
  [ -z "$pinned" ] && return 0
  if [ ! -d "$dir/.git" ]; then echo "warning: sibling $dir missing"; return 0; fi
  local head dirty=""
  head=$(git -C "$dir" rev-parse HEAD)
  [ -n "$(git -C "$dir" status --porcelain --untracked-files=no)" ] && dirty=" + uncommitted changes"
  if [ "$head" != "$pinned" ] || [ -n "$dirty" ]; then
    echo "warning: $dir is at ${head:0:12}$dirty, CI pins ${pinned:0:12} ($var in ci.yml)"
  fi
}
check_sibling MODULE_KIT_SHA ../../module-kit
check_sibling CONTRACTS_SHA ../../athene-contracts

for gate in "${gates[@]}"; do
  case $gate in
    fmt)    run_gate fmt cargo fmt --all --check ;;
    clippy) run_gate clippy cargo clippy --all-targets --locked -- -D warnings ;;
    test)   run_gate test cargo test --all-targets --locked ;;
    deny)
      deny_bin=${CARGO_DENY:-$(command -v cargo-deny || true)}
      if [ -z "$deny_bin" ]; then
        echo "==> deny: cargo-deny not installed (set CARGO_DENY=/path/to/cargo-deny)"
        results+=("deny: SKIPPED (cargo-deny missing)")
      else
        run_gate deny "$deny_bin" deny --locked check
      fi ;;
    *) echo "unknown gate: $gate" >&2; exit 2 ;;
  esac
done

echo
echo "== ci-local summary ($REPO_DIR)"
printf '  %s\n' "${results[@]}"
exit $failed
