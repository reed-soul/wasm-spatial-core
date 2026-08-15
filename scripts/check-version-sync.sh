#!/usr/bin/env bash
# Verify that every user-facing version string matches the Cargo.toml version.
#
# Cargo.toml is the single source of truth (it drives the wasm-pack generated
# package and the version() export). The files below duplicate the version for
# humans (site badges, JSON-LD, npm manifest) and MUST be bumped together —
# see RELEASE.md for the full release checklist.
#
# Exits non-zero on the first category of drift. CI runs this on every push.
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
if [ -z "$VERSION" ]; then
  echo "✗ could not read version from Cargo.toml" >&2
  exit 1
fi

fail=0

# check <file> <min-occurrences>
check() {
  local file=$1 min=$2 count
  if [ ! -f "$file" ]; then
    echo "✗ $file: missing" >&2
    fail=1
    return
  fi
  count=$(grep -oF "$VERSION" "$file" | wc -l | tr -d ' ')
  if [ "$count" -lt "$min" ]; then
    echo "✗ $file: expected ≥$min occurrence(s) of $VERSION, found $count" >&2
    fail=1
  else
    echo "✓ $file ($count)"
  fi
}

check npm/package.json 1
check site/llms.txt 1
check site/index.html 2
check site/shared/site-nav.mjs 1
check site/shared/brand/og-image.svg 1
check examples/shared/site-shell.mjs 1

# Guard the regression from commits 719998e/17e51d0/600efe1: npm build outputs
# (including ~1MB WASM binaries) must never be tracked by git.
if [ -n "$(git ls-files npm/pkg npm/pkg-node)" ]; then
  echo "✗ npm/pkg or npm/pkg-node is tracked by git — build artifacts must not be committed" >&2
  fail=1
else
  echo "✓ npm build outputs untracked"
fi

exit $fail
