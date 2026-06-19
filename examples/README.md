# Browser demos

## Online (GitHub Pages)

| Demo | URL |
|------|-----|
| **Hub** (multi-tab playground) | https://reed-soul.github.io/wasm-spatial-core/examples/index.html |
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

| Demo | URL (after `npm run demo`) |
|------|----------------------------|
| Hub | http://127.0.0.1:8080/examples/index.html |
| Interactive | http://127.0.0.1:8080/examples/demo/index.html |
| Three.js Point Cloud | http://127.0.0.1:8080/examples/point-cloud-demo/index.html |
| **WebGL Point Cloud** | http://127.0.0.1:8080/examples/webgl-pointcloud/index.html |
| Cesium 3D Tiles Point Cloud | http://127.0.0.1:8080/examples/point-cloud-cesium/index.html |
| Worker (COOP/COEP) | http://127.0.0.1:8080/examples/worker-demo/index.html |
| **WebGPU smoke** (Wave 4) | http://127.0.0.1:8080/examples/webgpu-smoke/index.html |

```bash
npm run demo      # builds pkg + assembles _site, then serves on :8080
npm run demo:dev  # serve repo root directly
```

## Point Cloud Demos

### WASM bindings helper (`shared/pc-bindings.mjs`)

Shared helpers for wasm-bindgen **getter-based** point-cloud APIs (`positions`, `colors`, `tileCount`, etc.). Used by Three.js / Cesium demos.

### Hub Point Cloud Tab (`index.html` → Point Cloud)

Interactive **3D WebGL preview** embedded in the demo hub:

- Orbit / pan / zoom (mouse + touch)
- WASM `decimateVoxelGrid` on synthetic terrain
- **Drag-drop LAS** (LAZ if built with `laz-support`) or one-click **Sample LAS** (`sample-data/demo_terrain.las`)
- Links to full viewers below

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
- Requires Cesium Ion token (free tier)

### `cesium-demo/` (legacy)

Pure JavaScript Cesium demo — does not load the WASM `pkg/`.
