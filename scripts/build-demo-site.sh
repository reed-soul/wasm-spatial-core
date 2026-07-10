#!/usr/bin/env bash
# Assemble static files for GitHub Pages / Vercel (demo site).
#
# Output layout:
#   _site/
#   ├── index.html            → redirects to site/index.html (brand showcase)
#   ├── site/                 ← NEW: brand landing page + shared brand assets
#   ├── examples/             ← standalone demos (unified shell)
#   ├── pkg/                  ← WASM module
#   ├── bench/                ← browser benchmark
#   ├── point-cloud/          → short redirect to examples/point-cloud-demo
#   ├── cesium-workflow/      → short redirect
#   ├── terrain/              → short redirect
#   └── demos/                → short redirect to examples/index.html (demo hub)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FEATURES="${SITE_FEATURES:-point-cloud,geotiff,laz-support,mesh-ingest}"

if [[ "${SKIP_WASM_BUILD:-0}" != "1" ]]; then
  echo "Building WASM package → pkg/  (features: $FEATURES)"
  wasm-pack build --target web --release --out-dir pkg -- --features "$FEATURES"
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

# Brand showcase site (landing page + shared brand/mini-demo assets)
if [[ -d site ]]; then
  cp -r site "$OUT/site"
  echo "Site: copied site/ → $OUT/site/"
else
  echo "⚠️ site/ not found — brand showcase will be missing" >&2
fi

cp -r examples "$OUT/examples"
# Copy hand-written docs/ (the typedoc API docs are generated separately by CI
# into gh-pages/docs/; this ships the manual overview + demo card page so the
# nav "Docs" link resolves locally and on the deployed site).
if [[ -d docs ]]; then
  mkdir -p "$OUT/docs"
  cp -r docs/. "$OUT/docs/"
fi
mkdir -p "$OUT/bench"
if [[ -d bench/browser ]]; then
  cp -r bench/browser "$OUT/bench/browser"
else
  echo "⚠️ bench/browser/ not found — skipping benchmark page"
fi

# Root index → brand showcase (site/index.html)
cat >"$OUT/index.html" <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>wasm-spatial-core — The fastest spatial engine in the browser</title>
  <meta http-equiv="refresh" content="0; url=site/index.html" />
  <link rel="canonical" href="site/index.html" />
  <script>
    location.replace('site/index.html' + location.search + location.hash);
  </script>
</head>
<body>
  <p><a href="site/index.html">wasm-spatial-core — open the showcase</a></p>
</body>
</html>
HTML

write_demo_redirect() {
  local slug="$1"
  local target="$2"
  mkdir -p "$OUT/$slug"
  cat >"$OUT/$slug/index.html" <<HTML
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>wasm-spatial-core</title>
  <meta http-equiv="refresh" content="0; url=$target" />
  <link rel="canonical" href="$target" />
  <script>
    location.replace('$target' + location.search + location.hash);
  </script>
</head>
<body>
  <p><a href="$target">wasm-spatial-core — open demo</a></p>
</body>
</html>
HTML
}

# Short URLs used in README / API docs → canonical example pages.
write_demo_redirect demos            ../examples/index.html
write_demo_redirect point-cloud      ../examples/point-cloud-demo/index.html
write_demo_redirect cesium-workflow  ../examples/cesium-workflow/index.html
write_demo_redirect terrain          ../examples/terrain-demo/index.html

touch "$OUT/.nojekyll"

echo "Site ready at $OUT/"
echo "  Brand showcase:  site/index.html   (root / redirects here)"
echo "  Demo hub:        examples/index.html   (/demos)"
echo "  Three.js PCloud: examples/point-cloud-demo/index.html"
echo "  Cesium PCloud:   examples/point-cloud-cesium/index.html"
echo "  Cesium workflow: examples/cesium-workflow/index.html"
echo "  Terrain:         examples/terrain-demo/index.html"
echo "  Benchmark:       bench/browser/index.html"
