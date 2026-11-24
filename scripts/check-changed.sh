#!/usr/bin/env bash
# Runs the checks for the parts of the repo the working copy changes (`mise run check-changed`).
# Files come from `jj diff --name-only`, else `git diff --name-only HEAD` plus untracked files.
# -n prints the checks without running them. Every check runs; the exit is non-zero if any failed.
set -uo pipefail
cd "$(dirname "$0")/.."

dry=false
[ "${1:-}" = "-n" ] && dry=true

if [ -d .jj ] && command -v jj >/dev/null; then
  files=$(jj diff --name-only)
else
  files=$( { git diff --name-only HEAD; git ls-files --others --exclude-standard; } | sort -u)
fi

crates=() whole=false deny=false kotlin=false backend=false workers=false deploy=false ext=false
while IFS= read -r f; do
  case "$f" in
    backend/*) backend=true ;;
    mail/* | proxy/*) workers=true ;;
    deploy/*) deploy=true ;;
    rust/vtop-shared/vectors/*) backend=true crates+=("vtop-shared") ;;
    rust/vtop-convex/* | rust/vps/* | rust/*.md) ;;
    rust/Cargo.toml | rust/Cargo.lock | rust/deny.toml) whole=true deny=true ;;
    rust/.cargo/*) whole=true ;;
    rust/*/*)
      crate=${f#rust/}
      crate=${crate%%/*}
      # Workspace members' package names are their directory names.
      if [ -f "rust/$crate/Cargo.toml" ]; then
        [[ " ${crates[*]-} " == *" $crate "* ]] || crates+=("$crate")
        [ "$f" = "rust/$crate/Cargo.toml" ] && deny=true
      fi
      ;;
    core/* | design/* | features/* | app/* | project.yaml | detekt.yml) kotlin=true ;;
    extension/*) ext=true ;;
  esac
done <<< "$files"

failed=()
# run <dir> <command...>: runs it in <dir> and notes a failure.
run() {
  local dir=$1
  shift
  echo "==> ($dir) $*"
  $dry && return
  (cd "$dir" && "$@") || failed+=("($dir) $*")
}

if $whole; then
  pkgs=(--workspace)
else
  pkgs=()
  for c in ${crates[@]+"${crates[@]}"}; do pkgs+=(-p "$c"); done
fi
if [ ${#pkgs[@]} -gt 0 ]; then
  fmt=(cargo fmt --check)
  $whole || fmt+=("${pkgs[@]}")
  run rust "${fmt[@]}"
  run rust cargo clippy "${pkgs[@]}" --all-targets -- -D warnings
  run rust cargo test "${pkgs[@]}"
fi
$deny && run rust cargo deny check
if $kotlin; then
  run . ./kotlin test
  run . mise run detekt
fi
$backend && run . mise run check-backend
$workers && run . mise run check-workers
$deploy && run . mise run check-deploy
$ext && run . mise run check-ext

if [ ${#pkgs[@]} -eq 0 ] && ! $deny && ! $kotlin && ! $backend && ! $workers && ! $deploy && ! $ext; then
  echo "no checks for the changed files"
fi
if [ ${#failed[@]} -gt 0 ]; then
  printf 'failed: %s\n' "${failed[@]}" >&2
  exit 1
fi
