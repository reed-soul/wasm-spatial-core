#!/usr/bin/env bash
# Fail if a public-facing WASM error return uses a raw string JsValue.
#
# ROADMAP_1_0.md §1: every wasm-bindgen failure must be a SpatialError with a
# typed `code`. `JsValue::from_str` is still allowed for non-error uses
# (property keys, Worker message fields, typed-array string values).
#
# Uses POSIX grep (basic regex) so GitHub-hosted runners do not need ripgrep.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

set +e
matches="$(grep -Rn --include='*.rs' 'Err([[:space:]]*JsValue::from_str' src 2>&1)"
status=$?
set -e

# grep: 0 = matches, 1 = no matches, 2 = error
if [[ "$status" -eq 2 ]]; then
  echo "error: hatch grep failed:" >&2
  echo "$matches" >&2
  exit 1
fi

if [[ "$status" -eq 0 ]]; then
  echo "error: WASM error returns must use SpatialError helpers, not JsValue::from_str:" >&2
  echo "$matches" >&2
  echo >&2
  echo "Use crate::errors::{parse_js, invalid_input_js, tile_js, geometry_js, terrain_js}" >&2
  echo "or Result<_, SpatialErrorDetail> (Into<JsValue> is implemented)." >&2
  exit 1
fi

echo "OK: no Err(JsValue::from_str) hatches in src/"
