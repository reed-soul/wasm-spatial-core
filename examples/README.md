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
# or: wasm-pack build --target web --release --out-dir pkg -- --features point-cloud
```

| Demo | URL (after `npm run demo`) |
|------|----------------------------|
| Hub | http://127.0.0.1:8080/examples/index.html |
| Interactive | http://127.0.0.1:8080/examples/demo/index.html |
| Three.js Point Cloud | http://127.0.0.1:8080/examples/point-cloud-demo/index.html |
| Cesium 3D Tiles Point Cloud | http://127.0.0.1:8080/examples/point-cloud-cesium/index.html |
| Worker (COOP/COEP) | http://127.0.0.1:8080/examples/worker-demo/index.html |

```bash
npm run demo      # builds pkg + assembles _site, then serves on :8080
npm run demo:dev  # serve repo root directly
```

## Point Cloud Demos

### Three.js Point Cloud Viewer (`point-cloud-demo/`)

Zero-dependency 3D point cloud viewer. Drag a `.las` or `.laz` file to render it in 3D.

- No API keys or tokens required
- WASM-powered octree + pnts encoding
- Interactive orbit controls (zoom, rotate, pan)
- Height-based and intensity-based coloring modes

### Cesium 3D Tiles Point Cloud (`point-cloud-cesium/`)

Point cloud rendered on a 3D globe via Cesium and 3D Tiles.

- Drag-and-drop LAS/LAZ file upload
- Full 3D Tiles pipeline (octree → pnts → tileset.json)
- Globe navigation with automatic fly-to
- Requires Cesium Ion token (free tier)

### `cesium-demo/` (legacy)

Pure JavaScript Cesium demo — does not load the WASM `pkg/`.
