# Browser demos

The interactive showcase lives at the **site landing page**:
[`/site/index.html`](../site/index.html) — a performance-driven scrolling
narrative with embedded mini-demos, a benchmark, a quick-start, a format
matrix, and the gallery below.

This `examples/` directory holds the standalone full-page demos, all unified
with the site brand shell (top nav + "← Back to site" link).

## Online (GitHub Pages)

| Page | URL |
|------|-----|
| **Site home** (brand showcase) | https://reed-soul.github.io/wasm-spatial-core/site/index.html |
| Demo hub (multi-tab playground) | https://reed-soul.github.io/wasm-spatial-core/examples/index.html |
| Interactive GeoJSON + CRS + R-tree | https://reed-soul.github.io/wasm-spatial-core/examples/demo/index.html |
| **Three.js Point Cloud** | https://reed-soul.github.io/wasm-spatial-core/examples/point-cloud-demo/index.html |
| **Cesium 3D Tiles Point Cloud** | https://reed-soul.github.io/wasm-spatial-core/examples/point-cloud-cesium/index.html |
| WASM vs JS benchmark | https://reed-soul.github.io/wasm-spatial-core/bench/browser/index.html |

See [docs/DEMO_SITE.md](../docs/DEMO_SITE.md) for deployment notes.

## Local

Demos import WASM from `../pkg/`. **`pkg/` is not in git** — build it first:

```bash
npm run build:pkg
# or: wasm-pack build --target web --release --out-dir pkg -- --features point-cloud,geotiff
```

| Page | URL (after `npm run demo`) |
|------|----------------------------|
| **Site home** | http://127.0.0.1:8080/site/index.html |
| Demo hub | http://127.0.0.1:8080/examples/index.html |
| Interactive GIS | http://127.0.0.1:8080/examples/demo/index.html |
| Three.js Point Cloud | http://127.0.0.1:8080/examples/point-cloud-demo/index.html |
| **WebGL Point Cloud** | http://127.0.0.1:8080/examples/webgl-pointcloud/index.html |
| Cesium 3D Tiles Point Cloud | http://127.0.0.1:8080/examples/point-cloud-cesium/index.html |
| Worker (COOP/COEP) | http://127.0.0.1:8080/examples/worker-demo/index.html |
| Cesium workflow (COPC streaming) | http://127.0.0.1:8080/examples/cesium-workflow/index.html |

```bash
npm run demo      # builds pkg + assembles _site, then serves on :8080
npm run demo:dev  # serve repo root directly
```

## Brand shell (`shared/site-shell.mjs`)

Every demo page imports `../shared/site-shell.mjs`, which injects the unified
top navigation (with logo, version badge, and a "← Back to site" link) and
loads the global `site/site.css` so all pages share the brand palette. Demo
pages keep their own page-specific CSS; only the shell + token refresh are
added.

## Point Cloud Demos

### WASM bindings helper (`shared/pc-bindings.mjs`)

Shared helpers for wasm-bindgen **getter-based** point-cloud APIs (`positions`, `colors`, `tileCount`, etc.). Used by Three.js / Cesium demos.

### Three.js Point Cloud Viewer (`point-cloud-demo/`)

Zero-dependency 3D point cloud viewer. Drag a `.las` file to render it in 3D (`.laz` requires a `laz-support` WASM build).

- No API keys or tokens required
- WASM-powered octree + pnts encoding
- Interactive orbit controls (zoom, rotate, pan)
- Height-based and intensity-based coloring modes

### Cesium 3D Tiles Point Cloud (`point-cloud-cesium/`)

Point cloud rendered on a 3D globe via Cesium and 3D Tiles.

- Drag-and-drop LAS upload (LAZ if built with `laz-support`)
- Full 3D Tiles pipeline (octree → pnts → tileset.json)
- Globe navigation with automatic fly-to

> **Removed:** the legacy `cesium-demo/` (pure-JS, no WASM) has been retired.
> **Moved:** the `webgpu-smoke/` diagnostic page was relocated to `tests/webgpu-smoke/` as a non-public test asset.
