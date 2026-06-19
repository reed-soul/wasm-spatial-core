<div align="center">

# wasm-spatial-core

**Drag a LAS file into your browser → Cesium 3D. No server needed.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

![Lines](https://img.shields.io/badge/code-33K-blue)
![Tests](https://img.shields.io/badge/tests-661%20(npm%20build)-success)
![Formats](https://img.shields.io/badge/formats-10%2B%20(npm)%20|%2015%2B%20(engine)-green)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/demo.png">
  <img alt="wasm-spatial-core demo" src="docs/demo.png" width="600">
</picture>

**[🌐 Live Demo](https://reed-soul.github.io/wasm-spatial-core/)** ·
[📦 npm](https://www.npmjs.com/package/wasm-spatial-core) ·
[📖 API Docs](https://reed-soul.github.io/wasm-spatial-core/docs/) ·
[🗺️ Roadmap](./ROADMAP_V1.md) · [🔭 Vision](./VISION.md)

**🧪 Try it now:**

```html
<script type="module">
  import init, { parsePointCloudAuto, buildOctree, generateTileset }
    from 'https://esm.run/wasm-spatial-core';
  await init();
  // LAS file → parse → octree → 3D Tiles — all in-browser (LAZ needs custom build)
</script>
```

</div>

---

## ✨ What is this?

🚀 **LAS/PLY/OBJ → 3D Tiles** in the browser (LAZ/COPC/E57 via optional build features)
🏔️ **GeoTIFF → Quantized-Mesh Terrain** in the browser
🗜️ **Draco point cloud compression** (Google draco3d integration)
⚡ **100M points in 8.5 seconds** (release build, native)
🔒 **Zero server, zero upload, zero dependencies**

`wasm-spatial-core` is a high-performance WebAssembly engine that compiles spatial computing from Rust to run directly in the browser. Point cloud parsing, octree spatial partitioning, 3D Tiles generation, Draco compression, GeoTIFF terrain decoding, coordinate projection, GeoJSON processing — all at near-native speed.

---

## 🚀 Quick Start

```bash
npm install wasm-spatial-core
```

```js
import init, {
  parsePointCloudAuto,
  buildOctree,
  generateTileset,
} from 'wasm-spatial-core';

await init();

// Parse LAS (default npm). LAZ/COPC need --features laz-support at build time.
const cloud = parsePointCloudAuto(lasBytes);

// Build octree → 3D Tiles
const tiles = generateTileset(
  cloud.positions(),
  50000,  // max points per node
  10      // max depth
);
```

### Draco Compression (optional)

```bash
npm install draco3d
```

```js
import { loadSpatialCore, compressTilesetWithDraco } from 'wasm-spatial-core';
import { createEncoderModule } from 'draco3d';

const wasm = await loadSpatialCore();
const encoderModule = await createEncoderModule({});
const tileset = wasm.generateTileset(positions, 50000, 21);

const results = compressTilesetWithDraco(tileset, encoderModule, {
  quantizationBits: 11,
  onProgress: (i, total, orig, comp) => {
    console.log(`Tile ${i + 1}/${total}: ${(comp / orig * 100).toFixed(0)}%`);
  },
});
```

### What's in the npm package?

`npm install wasm-spatial-core` ships a **prebuilt WASM binary** compiled with
`point-cloud` + `geotiff`. That gives you:

| Included in npm | Not in npm (custom `wasm-pack` build) |
|-----------------|---------------------------------------|
| LAS, PLY, OBJ, PCD parsing | LAZ / COPC (`laz-support`) |
| Octree + 3D Tiles (pnts) | E57 (`e57-support`) |
| GeoTIFF → quantized-mesh terrain | Terrain deformation (`terrain-edit`) |
| Coordinates, GeoJSON, MVT, spatial analysis | Mesh QEM / clip (`mesh-edit`) |

**Format counts:** **10+** read/write paths in the default npm build (LAS/PLY/OBJ/PCD, GeoJSON, MVT, WKT/WKB, GeoTIFF, GPX, TopoJSON, 3D Tiles/glTF output, …). **15+** when optional format features are enabled (LAZ/COPC, E57, GLB ingest, …).

Runtime checks: `supportsLaz()`, `supportsGeotiff()`, `lazStatus()`.

CI runs **`cargo test --all-features`** (~840 tests across the full matrix).
The npm build runs **661 tests** for the shipped feature set — the badge above
reflects the npm build, not `--all-features`.

---

## 🎯 Core Pipelines

### Point Cloud → 3D Tiles

```
LAS / PLY / OBJ  (npm default)
LAZ / COPC / E57 (optional build features — see table above)
        │
        ▼
  ┌──────────────┐
  │ WASM Parser   │  Browser-side; format set depends on build features
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ Octree Build  │  8-way spatial partitioning
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ pnts Encoder  │  3D Tiles Point Cloud binary
  └──────┬───────┘
         ▼
  ┌──────────────┐     ┌──────────────┐
  │ tileset.json  │     │ Draco Compress │  Optional (~20% ratio)
  └──────┬───────┘     └──────┬───────┘
         │                      │
         ▼                      ▼
  Cesium / Three.js — interactive 3D
```

### GeoTIFF → Terrain Tiles

```
GeoTIFF (.tif)
        │
        ▼
  ┌──────────────┐
  │ WASM Parser   │  Float32/16/8, strip/tile, DEFLATE
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ Quantized-Mesh │  Cesium terrain binary format
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ tileset.json   │  LOD pyramid (zoom 0..N)
  └───────────────┘
```

---

## ⚡ Performance

Benchmarks on **Apple M2 / Mac mini 4**, see [PERFORMANCE.md](./PERFORMANCE.md) for details.

### Point Cloud Pipeline (LAS → Octree → 3D Tiles)

| Dataset | Points | Parse | Octree | Tileset | Total |
|---------|--------|-------|--------|---------|-------|
| sample.las | 1,065 | — | < 1 ms | < 1 ms | — |
| Synthetic | 500K | 36 ms | 117 ms | 49 ms | 205 ms |
| Synthetic | 10M | 1.1 s | 2.9 s | 740 ms | 4.8 s |
| **Synthetic** | **100M** | **0.4 s** | **6.0 s** | **0.8 s** | **8.5 s** |

> 100M-point benchmark: release build, single-thread native (Rust). WASM will be slower but still well under 30 seconds.

### Draco Compression

| Dataset | Points | Uncompressed | Draco (q=11) | Ratio |
|---------|--------|-------------|-------------|-------|
| Synthetic | 50K | 600 KB | 121 KB | **20.2%** |
| Synthetic | 50K | 600 KB | 38 KB (q=8) | **6.3%** |

> Encoder WASM: **362 KB** (Google draco3d, Apache-2.0). Point order may differ after encoding, but position-color pairing is preserved.

### Coordinate Conversion (vs Pure JS)

| Operation | Pure JS | WASM | Speedup |
|-----------|---------|------|---------|
| WGS84 → GCJ-02 | ~1,200 ms | ~45 ms | **~27×** |
| WGS84 → Mercator | ~800 ms | ~12 ms | **~67×** |

---

## 📦 Format Support

### Point Cloud

| Format | Read | Feature Flag |
|--------|------|-------------|
| LAS (1.2–1.4, Format 0–6) | ✅ | `point-cloud` |
| LAZ (compressed) | ✅ | `laz-support` |
| COPC (Cloud Optimized) | ✅ | `laz-support` |
| PLY (ASCII + binary) | ✅ | `point-cloud` |
| OBJ | ✅ | `point-cloud` |
| PCD (ASCII + binary) | ✅ | `point-cloud` |
| E57 | ✅ | `e57-support` |

### Vector & Geometry

| Format | Read | Write |
|--------|------|-------|
| GeoJSON | ✅ | ✅ |
| MVT (Vector Tiles) | ✅ | ✅ |
| WKT / WKB | ✅ | ✅ |
| GeoTIFF (Terrain) | ✅ | — |
| glTF 2.0 / GLB | — | ✅ |
| 3D Tiles (pnts) | — | ✅ |
| 3D Tiles (b3dm) | — | ✅ |
| 3D Tiles (quantized-mesh) | — | ✅ |

### Coordinate Systems

| System | Direction |
|--------|-----------|
| WGS-84 ↔ GCJ-02 / BD-09 | ✅ |
| WGS-84 ↔ Web Mercator (EPSG:3857) | ✅ |
| WGS-84 ↔ CGCS2000 | ✅ |
| WGS-84 ↔ UTM | ✅ |

### Spatial Analysis

R-Tree / Octree indexing, bounding box / KNN queries, haversine / vincenty distance, polygon boolean ops, Douglas-Peucker simplification, convex / concave hull, DBSCAN / grid clustering, TIN interpolation, and more.

---

## 📸 Demos

| Demo | URL |
|------|-----|
| **🏠 Landing Page** | https://reed-soul.github.io/wasm-spatial-core/ |
| **Point Cloud** (LAS/PLY/OBJ; LAZ in custom builds) | https://reed-soul.github.io/wasm-spatial-core/point-cloud/ |
| **Cesium 3D Tiles** | https://reed-soul.github.io/wasm-spatial-core/cesium-workflow/ |
| **Terrain Viewer** (GeoTIFF) | https://reed-soul.github.io/wasm-spatial-core/terrain/ |

Run locally: `npm run demo`

---

## 📖 API Reference

### Point Cloud → 3D Tiles

```typescript
import { loadSpatialCore } from 'wasm-spatial-core';
const wasm = await loadSpatialCore();

// Auto-detect format
const cloud = wasm.parsePointCloudAuto(bytes);
console.log(cloud.count());        // point count
console.log(cloud.positions());    // Float32Array [x,y,z,...]
console.log(cloud.colors());       // Uint8Array [r,g,b,...] | null

// Octree
const octree = wasm.buildOctree(cloud.positions(), 50000, 10);
console.log(octree.nodeCount());   // node count
console.log(octree.depth());       // tree depth

// 3D Tiles tileset
const tileset = wasm.generateTileset(cloud.positions(), 50000, 10);
console.log(tileset.tileCount());  // tile count
console.log(tileset.tilesetJson()); // tileset.json string
```

### Draco Compression

```typescript
import { compressTilesetWithDraco, buildDracoTileset } from 'wasm-spatial-core';
import { createEncoderModule } from 'draco3d';

const encoderModule = await createEncoderModule({});

// Compress all tiles
const results = compressTilesetWithDraco(tileset, encoderModule, {
  quantizationBits: 11,   // 8–18, default 11
  encodeSpeed: 5,         // 0–10, default 5
  decodeSpeed: 5,         // 0–10, default 5
  compressColors: false,  // also compress RGB (default false)
});

// Or build a complete compressed tileset
const { tiles, totalCompressedSize, compressionRatio } =
  buildDracoTileset(tileset, encoderModule);
```

### Coordinate Conversion

```typescript
const coords = new Float64Array([116.404, 39.915, 121.474, 31.230]);
const gcj02 = wasm.batchWgs84ToGcj02(coords);       // batch transform
wasm.batchWgs84ToGcj02InPlace(mutable);             // zero-copy in-place
const [zone, easting, northing] = wasm.wgs84ToUtm(116.404, 39.915);
```

### GeoJSON

```typescript
// Chunked output: parses the full JSON first, then emits coordinate batches
// (progress callbacks + lower peak coord memory — not byte-stream input).
wasm.parseGeoJsonStream(hugeGeojson, 500, (chunk, processed, total) => { /* ... */ });

// Lower memory per iteration: one feature at a time (input string still required)
const iter = wasm.parseGeoJsonLazy(hugeGeojson);
```

**[📖 Full API Docs](https://reed-soul.github.io/wasm-spatial-core/docs/)**

---

## 🛠️ Build from Source

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core

# Point cloud + GeoTIFF
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud,geotiff

# Run demos
npm run demo
```

### Feature Flags

| Feature | In npm | Default crate | Description |
|---------|--------|---------------|-------------|
| `single-thread` | ✅ | ✅ | Zero-config, works everywhere |
| `point-cloud` | ✅ | ❌ | LAS/PLY/OBJ/PCD + octree + 3D Tiles |
| `geotiff` | ✅ | ❌ | GeoTIFF terrain + quantized-mesh |
| `multi-thread` | ❌ | ❌ | Web Workers + SharedArrayBuffer |
| `laz-support` | ❌ | ❌ | LAZ/COPC decompression (+ ~400 KB WASM) |
| `e57-support` | ❌ | ❌ | E57 format |
| `terrain-edit` | ❌ | ❌ | Heightfield flatten/deform (requires `geotiff`) |
| `mesh-ingest` | ❌ | ❌ | Spatial IR + GLB ingest (Wave 2) |
| `mesh-edit` | ❌ | ❌ | Mesh QEM / OBB split (requires `mesh-ingest`) |
| `draco-support` | ❌ | ❌ | Draco compression API (JS-side via draco3d) |

---

## 📋 Roadmap

| Doc | Scope |
|-----|-------|
| **[VISION.md](./VISION.md)** | Product vision — Web3D spatial **compute engine** (core only) |
| **[ROADMAP_V2.md](./ROADMAP_V2.md)** | Active plan — Waves 1–5 (runtime, IR, terrain, GPU, mesh edit) |
| **[docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md)** | What is **in** the engine vs your application |
| **[ROADMAP_V1.md](./ROADMAP_V1.md)** | ✅ Completed — point cloud → 3D Tiles browser pipeline |

**V1 highlights (done):** LAS → octree → 3D Tiles (npm) · LAZ/COPC/E57 (optional builds) · GeoTIFF terrain · Draco · multi-thread WASM · Node.js batch API

**V2 next:** spatial IR · terrain/mesh edit (source available; not in default npm) · WebGPU · incremental tiles — see [issue templates](./docs/issues/) (start at **W2**)

---

## 🤝 Contributing

See [**CONTRIBUTING.md**](./CONTRIBUTING.md).

- 🐛 [Report a bug](.github/ISSUE_TEMPLATE/bug_report.md)
- 💡 [Request a feature](.github/ISSUE_TEMPLATE/feature_request.md)

---

## 📄 License

[MIT License](./LICENSE) — © 2026 Zhiqi Weilai

---

<div align="center">

**Built with 🦀 Rust + 🕸️ WebAssembly**

*Native spatial computing in every browser.*

</div>
