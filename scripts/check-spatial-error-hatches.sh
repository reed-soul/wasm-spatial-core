#!/usr/bin/env bash
# Fail if a public-facing WASM error return uses a raw string JsValue.
#
# ROADMAP_1_0.md §1: every wasm-bindgen failure must be a SpatialError with a
# typed `code`. `JsValue::from_str` is still allowed for non-error uses
# (property keys, Worker message fields, typed-array string values).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "error: ripgrep (rg) is required" >&2
  exit 1
fi

matches="$(rg -n --glob '*.rs' 'Err\(\s*JsValue::from_str' src || true)"

if [[ -n "$matches" ]]; then
  echo "error: WASM error returns must use SpatialError helpers, not JsValue::from_str:" >&2
  echo "$matches" >&2
  echo >&2
  echo "Use crate::errors::{parse_js, invalid_input_js, tile_js, geometry_js, terrain_js}" >&2
  echo "or Result<_, SpatialErrorDetail> (Into<JsValue> is implemented)." >&2
  exit 1
fi

echo "OK: no Err(JsValue::from_str) hatches in src/"
