#!/usr/bin/env bash
# Assemble static files for GitHub Pages / Vercel (demo site).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "${SKIP_WASM_BUILD:-0}" != "1" ]]; then
  echo "Building WASM package → pkg/"
  wasm-pack build --target web --release --out-dir pkg
fi

if [[ ! -f pkg/wasm_spatial_core.js ]]; then
  echo "error: pkg/wasm_spatial_core.js not found. Run wasm-pack build first." >&2
  exit 1
fi

OUT="${SITE_OUTPUT:-_site}"
rm -rf "$OUT"
mkdir -p "$OUT"

cp -r pkg "$OUT/pkg"
# wasm-pack writes pkg/.gitignore with "*" — breaks gh-pages deploy (files never committed).
rm -f "$OUT/pkg/.gitignore"
if [[ ! -f "$OUT/pkg/wasm_spatial_core_bg.wasm" ]]; then
  echo "error: $OUT/pkg/wasm_spatial_core_bg.wasm missing after copy" >&2
  exit 1
fi
echo "WASM size: $(du -h "$OUT/pkg/wasm_spatial_core_bg.wasm" | cut -f1)"
cp -r examples "$OUT/examples"
mkdir -p "$OUT/bench"
cp -r bench/browser "$OUT/bench/browser"

cat >"$OUT/index.html" <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>wasm-spatial-core</title>
  <meta http-equiv="refresh" content="0; url=examples/index.html" />
  <link rel="canonical" href="examples/index.html" />
  <script>
    location.replace('examples/index.html' + location.search + location.hash);
  </script>
</head>
<body>
  <p><a href="examples/index.html">wasm-spatial-core — open interactive demos</a></p>
</body>
</html>
HTML

touch "$OUT/.nojekyll"

echo "Demo site ready at $OUT/"
echo "  Hub:       examples/index.html"
echo "  Full demo: examples/demo/index.html"
echo "  Benchmark: bench/browser/index.html"
