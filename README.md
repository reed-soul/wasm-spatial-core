<div align="center">

# wasm-spatial-core

**Drag a point cloud file into your browser, get interactive 3D. No server needed.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

![Lines](https://img.shields.io/badge/code-30K-blue)
![Tests](https://img.shields.io/badge/tests-633-success)
![Formats](https://img.shields.io/badge/formats-15-green)

**[🌐 Live Demo](https://reed-soul.github.io/wasm-spatial-core/)** ·
[📦 npm](https://www.npmjs.com/package/wasm-spatial-core) ·
[📖 API Reference](#-api-reference) ·
[🗺️ Roadmap](./ROADMAP_V1.md)

**🧪 Try it now — no build needed:**

```html
<script type="module">
  import init, { parsePointCloudAuto, buildOctree, generateTileset }
    from 'https://esm.run/wasm-spatial-core';
  await init();
  // Drop a LAS/LAZ file, parse → octree → 3D Tiles — all in-browser.
</script>
```

</div>

---

## ✨ What is this?

🚀 **LAS/LAZ/COPC/E57/PLY/OBJ → 3D Tiles + GeoTIFF → Terrain** in the browser
⚡ **10M points in seconds**, not minutes
🔒 **Zero server, zero upload, zero dependencies** — your data never leaves the device

`wasm-spatial-core` is a high-performance WebAssembly engine that moves heavy spatial computing from the server to the browser. Point cloud parsing, octree spatial partitioning, 3D Tiles generation, GeoTIFF terrain decoding, quantized-mesh encoding, coordinate projection, GeoJSON processing — all compiled from Rust for near-native performance.

---

## 🚀 Quick Start

```js
import init, {
  parsePointCloudAuto,
  buildOctree,
  generateTileset,
} from 'wasm-spatial-core';

await init();

// Parse any point cloud format (LAS, LAZ, COPC, PLY, OBJ...)
const cloud = parsePointCloudAuto(lasBytes);

// Build octree for spatial partitioning
const octree = buildOctree(cloud.positions());

// Generate 3D Tiles tileset (pnts + tileset.json)
const tiles = generateTileset(octree, cloud.positions(), cloud.colors());
```

**npm install:**

```bash
npm install wasm-spatial-core
```

**Build from source:**

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud
```

---

## 🎯 Core Capability: Point Cloud → 3D Tiles

> Drag a LAS/LAZ file into the browser → get a Cesium-ready 3D Tiles tileset.
> **Zero server, zero upload, zero dependencies.**

## 🏔️ Core Capability: GeoTIFF → Terrain Tiles

> Drag a GeoTIFF elevation file into the browser → get Cesium quantized-mesh terrain tiles.
> **Hand-written parser, zero external TIFF dependencies, WASM-optimized.**

```
File Drop (GeoTIFF .tif)
        │
        ▼
  ┌──────────────┐
  │ GeoTIFF Parser │  ← Float32/16/8, strip/tile, DEFLATE
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │ Quantized-Mesh │  ← Cesium terrain binary format
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │ tileset.json   │  ← LOD pyramid (zoom 0..N)
  └───────────────┘
```

```
File Drop (LAS/LAZ/COPC/E57/PLY/OBJ)
        │
        ▼
  ┌──────────────┐
  │ WASM Parser   │  ← Full format support, browser-side
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │ Octree Build  │  ← 8-way spatial partitioning
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │ pnts Encoder  │  ← 3D Tiles Point Cloud binary format
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │ tileset.json  │  ← Recursive hierarchy with LOD
  └──────┬───────┘
         │
         ▼
  Cesium / Three.js — interactive 3D rendering
```

---

## 📸 Demos

| Demo | URL |
|------|-----|
| **🏠 Landing Page** | https://reed-soul.github.io/wasm-spatial-core/ |
| **Unified Demo** (GeoJSON + CRS + R-tree) | https://reed-soul.github.io/wasm-spatial-core/demo/ |
| **Three.js Point Cloud** (LAS/LAZ/PLY/OBJ/E57) | https://reed-soul.github.io/wasm-spatial-core/point-cloud/ |
| **Terrain Viewer** (GeoTIFF) | https://reed-soul.github.io/wasm-spatial-core/terrain/ |
| **Demos (legacy)** | https://reed-soul.github.io/wasm-spatial-core/examples/index.html |

Run locally: `npm run demo` (builds `pkg/` and serves on port 8080).

---

## 📦 Format Support

### Point Cloud Formats

| Format | Read | Write | Feature Flag |
|--------|------|-------|--------------|
| LAS (1.2–1.4, Format 0–6) | ✅ | — | `point-cloud` |
| LAZ (compressed) | ✅ | — | `laz-support` |
| COPC (Cloud Optimized) | ✅ | — | `laz-support` |
| PLY (ASCII + binary) | ✅ | — | `point-cloud` |
| OBJ | ✅ | — | `point-cloud` |
| PCD (ASCII + binary) | ✅ | — | `point-cloud` |
| E57 | ✅ | — | `e57-support` |

### Vector & Geometry Formats

| Format | Read | Write |
|--------|------|-------|
| GeoJSON | ✅ | ✅ |
| MVT (Vector Tiles) | ✅ | ✅ |
| WKT / WKB | ✅ | ✅ |
| TopoJSON | ✅ | — |
| GPX | ✅ | — |
| GeoTIFF (Terrain) | ✅ | — |
| glTF 2.0 / GLB | — | ✅ |
| 3D Tiles (b3dm) | — | ✅ |
| 3D Tiles (pnts) | — | ✅ |
| 3D Tiles (quantized-mesh) | — | ✅ |

### Coordinate Systems

| System | Direction |
|--------|-----------|
| WGS-84 ↔ GCJ-02 | ✅ Both ways |
| WGS-84 ↔ BD-09 | ✅ Both ways |
| WGS-84 ↔ Web Mercator (EPSG:3857) | ✅ Both ways |
| WGS-84 ↔ CGCS2000 | ✅ Both ways |
| WGS-84 ↔ UTM | ✅ Both ways |

### Spatial Analysis

| Capability | Status |
|------------|--------|
| R-Tree spatial index (point + edge) | ✅ |
| Octree spatial partitioning | ✅ |
| Bounding box / KNN queries | ✅ |
| Haversine / Vincenty distance | ✅ |
| Point / line buffer | ✅ |
| Polygon boolean ops (intersection/union) | ✅ |
| Douglas-Peucker simplification | ✅ |
| Convex / concave hull | ✅ |
| DBSCAN / grid clustering | ✅ |
| TIN interpolation | ✅ |

---

## ⚡ Performance

Benchmarks on **Apple M2 (macOS, debug)**, see [PERFORMANCE.md](./PERFORMANCE.md) for details.

| Operation | Pure JS | wasm-spatial-core | Speedup |
|-----------|---------|-------------------|---------|
| WGS84 → GCJ-02 | ~1,200 ms | ~45 ms | **~27×** |
| WGS84 → Mercator | ~800 ms | ~12 ms | **~67×** |
| GeoJSON parse (50 MB) | ~3,500 ms | ~320 ms | **~11×** |

### Point Cloud Pipeline (LAS → Octree → 3D Tiles)

| Dataset | Points | Octree Build | Tileset Gen | Output |
|---------|--------|-------------|-------------|--------|
| sample.las | 1,065 | < 1 ms | < 1 ms | 4 tiles, 8 KB |
| Synthetic | 100K | 24 ms | 4 ms | 97 tiles, 1.15 MB |
| Synthetic | 1M | 270 ms | 41 ms | 401 tiles, 11.5 MB |
| Synthetic | 10M | 2,944 ms | — | octree only |

### Real File Test: `sample.las`

Using `tests/fixtures/sample.las` (1,065 points, LAS 1.2, format 3 with color):

```
Header:  version 1.2, format 3, 1,065 points
Octree:  17 nodes, depth 2, 15 leaves
Tiles:   15 pnts tiles, 16.89 KB

> Benchmarks are indicative. Run `cargo bench` for local results.

---

## 📖 API Reference

### Point Cloud → 3D Tiles

```typescript
const core = await loadSpatialCore();

// Auto-detect format (LAS/LAZ/COPC/PLY/OBJ)
const cloud = core.parsePointCloudAuto(bytes);
console.log(cloud.count());          // point count
console.log(cloud.positions());      // Float32Array [x,y,z,...]
console.log(cloud.colors());         // Uint8Array [r,g,b,...]

// Build octree
const octree = core.buildOctree(cloud.positions());
console.log(octree.nodeCount());     // node count
console.log(octree.depth());         // tree depth

// Generate 3D Tiles tileset
const tileset = core.generateTileset(
  cloud.positions(),
  50000,  // max points per node
  10,     // max depth
  cloud.colors()
);
console.log(tileset.tileCount());    // tile count
console.log(tileset.totalBytes());    // total size
console.log(tileset.tilesetJson());  // tileset.json string

// Encode individual pnts tile
const tileBytes = tileset.tile(0);

// View-dependent LOD
const visible = core.getVisibleTiles(
  cloud.positions(),
  cameraX, cameraY, cameraZ,
  60,     // FOV
  1920,   // screen width
  1080,   // screen height
  50000,  // max points per node
  10,     // max depth
  1.0     // SSE threshold (pixels)
);
```

### Coordinate Conversion

```typescript
const coords = new Float64Array([116.404, 39.915, 121.474, 31.230]);

// Batch transform
const gcj02 = core.batchWgs84ToGcj02(coords);

// Zero-copy in-place
const mutable = new Float64Array([116.404, 39.915]);
core.batchWgs84ToGcj02InPlace(mutable);

// Pipeline transforms
const gcjMercator = core.batchWgs84ToGcj02Mercator(coords);

// UTM
const [zone, easting, northing, isN] = core.wgs84ToUtm(116.404, 39.915);
```

### GeoJSON Processing

```typescript
const coords = core.parseGeoJsonCoords(geojsonStr);
const count = core.countGeoJsonFeatures(geojsonStr);

// Streaming for large files
core.parseGeoJsonStream(geojsonStr, 65536, (chunk, processed, total) => {
  console.log(`${processed}/${total} features`);
});

// Lazy parser — O(single feature) memory
const iter = core.parseGeoJsonLazy(hugeGeoJsonStr);
let feature;
while ((feature = iter.nextFeature()) !== null) {
  // process feature...
}
iter.free();
```

### Spatial Index

```typescript
const index = new core.SpatialIndex(coords);

// Bounding box query
const ids = index.searchBBox(116.0, 39.5, 117.0, 40.5);

// Nearest neighbor
const nearest = index.nearestNeighbor(116.4, 39.9);

// K nearest
const k5 = index.kNearestNeighbors(116.4, 39.9, 5);
```

### Cesium Integration

```typescript
// WGS84 → Cartesian3
const cartesian = core.batchWgs84ToCartesian3(coords);

// Triangulate polygons
const geometry = core.generateCesiumGeometry(geojson, "height");

// Generate b3dm tile
const tile = core.generate3DTile(geojson, "height");
const tileBytes = tile.toBytes();
```

### Full API List

See the detailed tables in the [`npm/` README](./npm/README.md) for the complete categorized API reference.

---

## 🛠️ Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) stable **≥ 1.90**
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- [Node.js](https://nodejs.org/) ≥ 18

### Build

```bash
# Standard build (coordinate + GeoJSON)
wasm-pack build --target web --release --out-dir pkg

# With point cloud support (LAS/LAZ/COPC/PLY/OBJ + octree + 3D Tiles)
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud

# With LAZ decompression (adds ~400KB to WASM)
wasm-pack build --target web --release --out-dir pkg -- --features laz-support

# Run demos
npm run demo
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `single-thread` | ✅ | Zero-config, works everywhere |
| `multi-thread` | ❌ | Web Workers + SharedArrayBuffer (requires atomics + bulk-memory) |
| `point-cloud` | ❌ | LAS/PCD/PLY/OBJ parsing + octree + 3D Tiles |
| `laz-support` | ❌ | LAZ/COPC decompression (implies `point-cloud`) |
| `e57-support` | ❌ | E57 format support (architectural/industrial scans) |
| `geotiff` | ❌ | GeoTIFF terrain parsing + quantized-mesh + hillshade |
| `draco-support` | ❌ | Draco compression status API |

---

## 📋 Roadmap

See **[ROADMAP_V1.md](./ROADMAP_V1.md)** for the full development roadmap.

- ✅ **Phase A**: Point cloud core pipeline (LAS/LAZ/COPC/PLY/OBJ, octree, pnts, tileset, demos)
- ✅ **Phase B1-B2**: LOD optimization, screen-space error, view-dependent loading
- ✅ **Phase B3**: WebWorker parallelism (WASM streaming, chunked processing)
- ✅ **Phase C1**: E57 format support (read + Three.js rendering)
- ✅ **Phase C2**: GeoTIFF terrain pipeline (parser, quantized-mesh, hillshade, contour)
- ✅ **Phase C3**: COPC full support (header, chunk reader, region queries)
- ✅ **Phase D**: npm publish preparation + GitHub Pages deployment
- 🔜 **Phase E1**: WASM multi-thread (atomics + SharedArrayBuffer)

---

## 🤝 Contributing

Contributions are welcome! See [**CONTRIBUTING.md**](./CONTRIBUTING.md) for details.

- 🐛 [Report a bug](.github/ISSUE_TEMPLATE/bug_report.md)
- 💡 [Request a feature](.github/ISSUE_TEMPLATE/feature_request.md)

---

## 📄 License

[MIT License](./LICENSE) — © 2026 Zhiqi Weilai

---

<div align="center">

**Built with 🦀 Rust + 🕸️ WebAssembly**

*Bringing the power of native spatial computing to every browser.*

</div>
