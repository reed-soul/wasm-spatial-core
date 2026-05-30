<div align="center">

# 🌍 wasm-spatial-core

**A high-performance WebAssembly engine that brings server-grade spatial computing to the browser.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-%23A73737-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0.svg?logo=webassembly&logoColor=white)](https://webassembly.org)

![Lines](https://img.shields.io/badge/code-11K-blue)
![Tests](https://img.shields.io/badge/tests-240%2B-success)
![Formats](https://img.shields.io/badge/formats-6-green)

*Offload server-side spatial computing to the client — free the cloud.*

[Quick Start](#-quick-start) · [Live Demo](#-live-demo) · [API Reference](#-api-reference) · [Roadmap](./PLAN.md) · [Contributing](./CONTRIBUTING.md)

</div>

---

## 🎯 The Problem

Modern Web3D and GIS applications face a fundamental bottleneck:

```
┌──────────┐   upload    ┌──────────┐   download   ┌──────────┐
│  Browser  │ ─────────► │  Server  │ ──────────► │  Browser  │
│ (idle CPU)│  raw data   │ (busy!)  │  processed   │ (render)  │
└──────────┘             └──────────┘              └──────────┘
```

- **100 MB GeoJSON** needs server-side coordinate projection before rendering
- **Point cloud scans** (LAS/LAZ) require decimation pipelines running on cloud VMs
- **BIM models** (IFC) must be pre-processed and converted server-side
- Every request adds latency, bandwidth costs, and compute bills

## 💡 The Solution

`wasm-spatial-core` moves heavy spatial computation **directly into the browser** via WebAssembly — compiled from Rust for near-native performance:

```
┌─────────────────────────────────────────────┐
│                  Browser                      │
│                                               │
│  ┌─────────┐  zero-copy  ┌───────────────┐   │
│  │   JS /   │ ◄─────────► │  WASM Engine  │   │
│  │ Three.js │  ArrayBuffer │  (Rust core)  │   │
│  │ Cesium   │             │               │   │
│  │ Mars3D   │  ┌──────────┤  • GeoJSON    │   │
│  └─────────┘  │ Float64   │  • CRS proj   │   │
│               │ Array     │  • Point cloud │   │
│  ┌─────────┐  │           │  • Tiling     │   │
│  │  WebGL / │◄─┘           │  • Decimation │   │
│  │  WebGPU  │              └───────────────┘   │
│  └─────────┘                                   │
└─────────────────────────────────────────────┘
         ↑  No server round-trip needed!
```

### Key Advantages

| | Traditional Server-Side | wasm-spatial-core |
|---|---|---|
| **Latency** | Network round-trip (100ms–10s) | Instant (local CPU) |
| **Bandwidth** | Upload raw + download processed | Zero transfer |
| **Server Cost** | $$$ per compute hour | $0 — runs on user's device |
| **Privacy** | Data leaves the device | Data never leaves the browser |
| **Offline** | ❌ Requires connection | ✅ Works offline |
| **Scalability** | Limited by server capacity | Scales with user count |

---

## ✨ Features

- 🔄 **Batch Coordinate Projection** — WGS-84 ↔ GCJ-02, WGS-84 ↔ BD-09, WGS-84 ↔ Web Mercator (EPSG:3857), WGS-84 ↔ CGCS2000
- 📦 **GeoJSON Parser & Writer** — Parse and generate GeoJSON FeatureCollections from/to flat `Float64Array` buffers
- 📡 **Streaming GeoJSON Parser** — Chunked processing with progress callbacks
- 📖 **Lazy GeoJSON Parser** — O(single feature) memory via manual JSON state machine — no full DOM parse
- 🔍 **Spatial Index (R-Tree)** — Bounding box search, nearest-neighbor, K-nearest-neighbor (point index + edge index)
- 📐 **Bounds Computation** — SIMD-style vectorized bounding box calculation (`computeBounds`, `computeBoundsMulti`)
- 🗺️ **Vector Tile Slicing & Decoding** — Frontend MVT tile generation and protobuf MVT decoding back to GeoJSON
- 🌐 **Cesium Native Adapter** — WGS84 → Cartesian3 (ECEF), polygon triangulation, 3D Tiles (b3dm)
- ☁️ **Point Cloud (LAS/PCD)** — Parse LAS headers & points, voxel grid & random decimation, COPC range-based access
- 🏗️ **IFC/BIM Geometry** (experimental) — Extract `IFCEXTRUDEDAREASOLID` mesh geometry
- 🔺 **glTF / GLB Writer** — Build glTF 2.0 scenes in WASM, export as GLB binary
- 📐 **Spatial Analysis** — Point/line buffering, bounding box, centroid, haversine distance, bearing, destination, midpoint on WGS-84
- 📐 **Topology Analysis** — Polygon area (spherical excess), polyline length, Douglas-Peucker simplification, point-in-ring, polygon boolean operations (intersection/union), TIN interpolation
- ✅ **Coordinate Quality** — Validation against CRS ranges, cleaning (remove/clamp/snap), deduplication with tolerance
- 📐 **Coordinate Sorting & Gridding** — Sort by lng/lat, spatial hash grid indexing
- 🔒 **Memory Management** — `memoryInfo()`, `getAllocatedBytes()`, dynamic `setInputSizeLimit()`
- ⚡ **Zero-Copy Architecture** — Data stays in WASM linear memory; JS gets typed array views
- 📊 **GPU-Ready Output** — Interleaved vertex buffers, indexed geometry for WebGL2/WebGPU
- 🧵 **Multi-threaded** (optional) — Web Workers + SharedArrayBuffer via Rayon

---

## 🌐 Live Demo

在浏览器中直接体验 WASM 引擎（坐标转换、GeoJSON、空间索引、性能对比等）：

| 演示 | 链接 |
|------|------|
| **演示中心**（推荐） | [reed-soul.github.io/wasm-spatial-core/examples/index.html](https://reed-soul.github.io/wasm-spatial-core/examples/index.html) |
| 完整交互 demo | […/examples/demo/index.html](https://reed-soul.github.io/wasm-spatial-core/examples/demo/index.html) |
| WASM vs JS 基准 | […/bench/browser/index.html](https://reed-soul.github.io/wasm-spatial-core/bench/browser/index.html) |

由 GitHub Actions 在 push 到 `master` 时自动部署；首次使用请在仓库 **Settings → Pages → Source: GitHub Actions** 中启用。  
也可部署到 Vercel（见 [docs/DEMO_SITE.md](./docs/DEMO_SITE.md)）。

本地预览：`bash scripts/build-demo-site.sh && npx http-server _site -p 8080 -c-1`

---

## 🚀 Quick Start

### Installation

```bash
npm install wasm-spatial-core
```

### Basic Usage

```typescript
import { loadSpatialCore } from "wasm-spatial-core";

// Initialize the WASM module (call once)
const core = await loadSpatialCore();

console.log(core.version()); // "0.2.0"

// Errors are structured objects (not plain strings):
// { name: "SpatialError", code: "PARSE_ERROR", message: "..." }
try {
  core.parseGeoJsonCoords("{ invalid");
} catch (e) {
  console.log(e.code, e.message);
}

// ── Batch coordinate conversion ──────────────────────────────
// Flat array: [lng0, lat0, lng1, lat1, ...]
const wgs84Coords = new Float64Array([
  116.404, 39.915,   // Beijing
  121.474, 31.230,   // Shanghai
  113.264, 23.129,   // Guangzhou
]);

// WGS-84 → GCJ-02 (China encrypted coordinates)
const gcj02 = core.batchWgs84ToGcj02(wgs84Coords);
console.log("GCJ-02:", gcj02);

// WGS-84 → Web Mercator (for Mapbox / Leaflet)
const mercator = core.batchWgs84ToMercator(wgs84Coords);
console.log("Mercator:", mercator);

// Zero-copy in-place transform (no allocation)
const mutableCoords = new Float64Array([116.404, 39.915]);
core.batchWgs84ToGcj02InPlace(mutableCoords);
// mutableCoords is now in GCJ-02

// ── GeoJSON parsing ──────────────────────────────────────────
const geojson = await fetch("/data/buildings.geojson").then(r => r.text());
const count = core.countGeoJsonFeatures(geojson);
const coords = core.parseGeoJsonCoords(geojson);

// Feed directly to WebGL
gl.bufferData(gl.ARRAY_BUFFER, coords, gl.STATIC_DRAW);
```

### Spatial Index

```typescript
const core = await loadSpatialCore();

// Build from flat coordinate array
const index = new core.SpatialIndex(coords);

// Bounding box query
const ids = index.searchBBox(116.0, 39.5, 117.0, 40.5);

// Nearest neighbor
const nearestId = index.nearestNeighbor(116.4, 39.9);

// K nearest neighbors
const kIds = index.kNearestNeighbors(116.4, 39.9, 5);
```

### Cesium Integration

```typescript
const core = await loadSpatialCore();
const geojson = await fetch("/buildings.geojson").then(r => r.text());

// Triangulate polygons for Cesium rendering
const geometry = core.generateCesiumGeometry(geojson, "height");

// Generate a b3dm 3D Tile
const tile = core.generate3DTile(geojson, "height");
const tileBytes = tile.toBytes();
```

---

## 🛠️ Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) stable **≥ 1.90** (see `rust-version` in `Cargo.toml`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) ≥ 18

### Build

`pkg/` is **not** committed to git — build it locally (or use the npm package) before running `examples/` demos.

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core

# Standard single-threaded build (or: npm run build:pkg)
wasm-pack build --target web --release --out-dir pkg

# Run examples hub + interactive demo (builds pkg, then serves on :8080)
npm run demo

# With point cloud support
wasm-pack build --target web --release --out-dir pkg -- --features point-cloud
```

### Multi-threaded Build

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

cd npm
npm run build:wasm:mt
```

> ⚠️ **Multi-threading requires** `SharedArrayBuffer`. Configure your server with:
> - `Cross-Origin-Opener-Policy: same-origin`
> - `Cross-Origin-Embedder-Policy: require-corp`
>
> Initialize the thread pool before calling parallel functions:
> ```typescript
> import { loadSpatialCore, initThreadPool } from "wasm-spatial-core";
> await loadSpatialCore();
> await initThreadPool(navigator.hardwareConcurrency);
> ```

---

## 📖 API Reference

### Coordinate Conversion

| Function | Description |
|----------|-------------|
| `batchWgs84ToGcj02(coords)` | WGS-84 → GCJ-02 |
| `batchGcj02ToWgs84(coords)` | GCJ-02 → WGS-84 |
| `batchWgs84ToBd09(coords)` | WGS-84 → BD-09 |
| `batchBd09ToWgs84(coords)` | BD-09 → WGS-84 |
| `batchGcj02ToBd09(coords)` | GCJ-02 → BD-09 |
| `batchBd09ToGcj02(coords)` | BD-09 → GCJ-02 |
| `batchWgs84ToMercator(coords)` | WGS-84 → EPSG:3857 |
| `batchMercatorToWgs84(coords)` | EPSG:3857 → WGS-84 |
| `batchWgs84ToCgcs2000(coords)` | WGS-84 → CGCS2000 (identity) |
| `batchWgs84ToGcj02Mercator(coords)` | WGS-84 → GCJ-02 → Mercator (pipeline) |
| `batchWgs84ToBd09Mercator(coords)` | WGS-84 → BD-09 → Mercator (pipeline) |
| `wgs84ToUtm(lng, lat)` | WGS-84 → UTM → `[zone, easting, northing, isN]` |
| `utmToWgs84(zone, easting, northing, isN)` | UTM → WGS-84 → `[lng, lat]` |
| `batchWgs84ToUtm(coords)` | WGS-84 → UTM (batch) |
| `batchUtmToWgs84(utmCoords)` | UTM → WGS-84 (batch) |
| `*InPlace` variants | Zero-copy in-place mutation for all above |

### Geohash

| Function | Description |
|----------|-------------|
| `geohashEncode(lng, lat, precision)` | Encode coordinate to geohash string |
| `geohashDecode(hash)` | Decode geohash → `[lng, lat, minLng, minLat, maxLng, maxLat]` |
| `geohashNeighbors(hash)` | 8 neighbors → `[N, NE, E, SE, S, SW, W, NW]` |

### Coordinate Normalization

| Function | Description |
|----------|-------------|
| `normalizeCoords(coords, bounds?)` | Normalize to `[0, 1]` range |
| `denormalizeCoords(coords, bounds)` | Restore from normalized coordinates |

### GeoJSON Processing

| Function | Description |
|----------|-------------|
| `parseGeoJsonCoords(input)` | Extract all coordinates as flat `Float64Array` |
| `countGeoJsonFeatures(input)` | Count features (fast pre-scan) |
| `parseGeoJsonStream(input, chunkSize, onChunk)` | Streaming parser with progress callback |
| `parseGeoJsonPerFeature(input)` | Per-feature coordinate arrays |
| `parseGeoJsonLazy(input)` | **Lazy iterator** — O(feature) memory, no full DOM parse |
| `parseGeoJsonProperties(input)` | Extract all feature properties as JSON string |
| `parseGeoJsonFeatures(input)` | Structured per-feature result → `GeoJsonFeaturesResult` |
| `geoJsonFromCoords(coords, geometryType)` | **Generate** GeoJSON Feature from coordinate buffer |
| `geoJsonFeatureCollection(coords, types, props)` | **Generate** FeatureCollection from multiple features |
| `filterGeoJsonByProperty(input, key, value)` | Filter features by property equality |
| `filterGeoJsonByBBox(input, minLng, minLat, maxLng, maxLat)` | Filter features by bounding box |
| `countGeoJsonByProperty(input, key)` | Count features per unique property value |
| `addProperty(input, key, value)` | Add a property to all features |
| `renameProperty(input, oldKey, newKey)` | Rename a property key |
| `removeProperty(input, key)` | Remove a property key |

**Lazy GeoJSON usage:**
```typescript
const iter = core.parseGeoJsonLazy(hugeGeoJsonStr);
let feature;
while ((feature = iter.nextFeature()) !== null) {
  gl.bufferSubData(gl.ARRAY_BUFFER, offset, feature);
  offset += feature.byteLength;
}
console.log(`Processed ${iter.total() - iter.remaining()} features`);
iter.free(); // release WASM memory
```

### Bounds Computation

| Function | Description |
|----------|-------------|
| `computeBounds(coords)` | Fast bounding box → `[minLng, minLat, maxLng, maxLat]` |
| `computeBoundsMulti(buffers)` | Merged bounds for multiple coordinate arrays |

### Spatial Index

| Class / Method | Description |
|----------------|-------------|
| `new SpatialIndex(coords)` | Build R-Tree from flat coordinate array |
| `.searchBBox(minX, minY, maxX, maxY)` | Bounding box range query → `Uint32Array` |
| `.nearestNeighbor(x, y)` | Nearest point → ID or `undefined` |
| `.kNearestNeighbors(x, y, k)` | K nearest → `Uint32Array` |
| `.size()` | Total point count |
| `new SpatialEdgeIndex(segments)` | Build R-Tree from line segments `[x0,y0,x1,y1,…]` |
| `.searchBBox(minX, minY, maxX, maxY)` | Edge bbox query → `Uint32Array` |
| `.nearestNeighbor(x, y)` | Nearest edge → ID or `undefined` |

### Vector Tiles

| Function | Description |
|----------|-------------|
| `new VectorTileEngine(geojson, options, layerName?)` | Create MVT engine |
| `new VectorTileOptions()` | Tile generation options |
| `.getTile(z, x, y)` | Get MVT PBF protobuf → `Uint8Array` |
| `decodeMvt(bytes)` | Decode MVT protobuf → `MvtLayer` (name, extent, features) |
| `decodeMvtToGeoJson(bytes)` | MVT → GeoJSON FeatureCollection string |

**MVT decoding usage:**
```typescript
const response = await fetch('/tiles/10/868/387.pbf');
const buffer = await response.arrayBuffer();
const layer = core.decodeMvt(new Uint8Array(buffer));
console.log(layer.name(), layer.extent(), layer.featureCount());
const feat = layer.featureAt(0);
console.log(feat.geometryType(), feat.geometry());
// Convert to GeoJSON
const geojson = core.decodeMvtToGeoJson(new Uint8Array(buffer));
```

### Cesium Adapter

| Function | Description |
|----------|-------------|
| `batchWgs84ToCartesian3(coords)` | WGS84 `[lng,lat,…]` → Cartesian3 `[x,y,z,…]` |
| `generateCesiumGeometry(geojson, heightProp?)` | Triangulate → `CesiumMeshGeometry` |
| `generate3DTile(geojson, heightProp?)` | Generate b3dm tile → `Cesium3DTile` |

### Point Cloud (LAS/PCD) *`--features point-cloud`*

| Function | Description |
|----------|-------------|
| `parseLasHeader(bytes)` | Parse LAS header |
| `parseLasPoints(bytes)` | Parse all points |
| `parseLasPointsWithProgress(bytes, onProgress)` | Parse with progress callback |
| `parseLasHeaderOnly(bytes)` | Header-only parse (COPC) |
| `computeLasPointOffset(header, idx, format)` | Byte offset of Nth point |
| `parseLasPointAt(bytes, offset, format)` | Parse single point |
| `parsePcdAscii(text)` | Parse ASCII PCD |
| `parsePcdBinary(bytes)` | Parse binary PCD |
| `decimateVoxelGrid(positions, colors, gridSize)` | Voxel grid decimation |
| `decimateVoxelGridWithProgress(positions, colors, gridSize, onProgress)` | Voxel decimation with progress |
| `decimateRandom(positions, colors, targetCount)` | Random sampling |
| `generateInterleavedVertexBuffer(positions, colors)` | GPU-ready interleaved buffer |
| `generateIndexedGeometry(positions, indices)` | GPU-ready indexed buffers |
| `colorizeByHeight(positions, minZ, maxZ)` | Height-based RGB coloring |
| `colorizeByIntensity(positions, minI, maxI)` | Intensity-based grayscale |
| `applyColorRamp(positions, colors)` | Apply color ramp to point cloud |
| `estimateNormals(positions, k)` | kNN normal estimation → unit normals |
| `flipNormals(normals, positions)` | Consistent orientation toward centroid |
| `computeLasPointOffset(header, idx, format)` | Byte offset of Nth point |
| `parseLasPointAt(bytes, offset, format)` | Parse single point |
| `parsePcdAscii(text)` | Parse ASCII PCD |
| `parsePcdBinary(bytes)` | Parse binary PCD |
| `decimateVoxelGrid(positions, colors, gridSize)` | Voxel grid decimation |
| `decimateRandom(positions, colors, targetCount)` | Random sampling |
| `generateInterleavedVertexBuffer(positions, colors)` | GPU-ready interleaved buffer |
| `generateIndexedGeometry(positions, indices)` | GPU-ready indexed buffers |

### IFC/BIM (Experimental)

| Function | Description |
|----------|-------------|
| `parseIfcGeometry(text)` | Extract meshes from IFC-SPF → `IfcGeometryResult` |

### glTF / GLB

| Function | Description |
|----------|-------------|
| `new GltfBuilder()` | Create scene builder |
| `.addMaterial(r, g, b, a)` | Add material → material index |
| `.addMesh(positions, indices, normals, materialIdx)` | Add triangle mesh |
| `.toGlb()` | Export as GLB binary → `Uint8Array` |
| `.toGltfJson()` | Export as glTF JSON string |

### Topology Analysis

| Function | Description |
|----------|-------------|
| `polygonArea(coords)` | Polygon area (m²) via spherical excess formula |
| `areaWithHoles(rings, ringSizes)` | Polygon area with holes |
| `polylineLength(coords)` | Line/polygon length (m) via Haversine |
| `simplifyDouglasPeucker(coords, tolerance)` | Douglas-Peucker simplification → `Float64Array` |
| `isPointInRing(px, py, ring)` | Point-in-polygon via ray-casting → `bool` |
| `polygonIntersection(ring1, ring2)` | **Boolean intersection** of two polygons → `Float64Array` |
| `polygonUnion(ring1, ring2)` | **Boolean union** of two polygons → `Float64Array` |
| `contains(ring, px, py)` | Point-in-polygon predicate |
| `disjoint(ring1, ring2)` | Two polygons are disjoint |
| `touches(ring1, ring2)` | Two polygons touch (boundary contact) |
| `polygonIntersects(ring1, ring2)` | Two polygons intersect |

### Spatial Analysis

| Function | Description |
|----------|-------------|
| `haversineDistance(lng1, lat1, lng2, lat2)` | Great-circle distance (m) |
| `bearing(lng1, lat1, lng2, lat2)` | Forward azimuth (degrees, 0=N) |
| `destination(lng, lat, bearing, distance)` | Direct geodesic → `[lng, lat]` |
| `midpoint(lng1, lat1, lng2, lat2)` | Great-circle midpoint → `[lng, lat]` |
| `bufferPoint(lng, lat, radius, segments?)` | Buffer a point → `Float64Array` |
| `bufferLineString(coords, radius, segments?)` | Buffer a line → `Float64Array` |
| `boundingBox(coords)` | Compute bbox → `[minLng, minLat, maxLng, maxLat]` |
| `centroid(coords)` | Compute centroid → `[lng, lat]` |

### TIN & Interpolation

| Function | Description |
|----------|-------------|
| `buildTin(coords, values)` | Build Delaunay TIN → `TinResult` |
| `tinInterpolate(tin, x, y)` | Interpolate value at point from TIN |

### Utilities

| Function | Description |
|----------|-------------|
| `version()` | Library version string |
| `memoryInfo()` | WASM memory usage → `MemoryInfo` |
| `getAllocatedBytes()` | WASM linear memory size (bytes) |
| `cgcs2000IsWgs84Compatible()` | Returns `true` (sub-cm accuracy) |
| `initThreadPool(numThreads)` | Init Rayon thread pool (multi-thread) |

### Memory Management

| Function | Description |
|----------|-------------|
| `setInputSizeLimit(bytes)` | Set max input size (default 100 MB) |
| `getInputSizeLimit()` | Query current input size limit |
| `getAllocatedBytes()` | Query WASM memory allocation |

### Coordinate Quality

| Function | Description |
|----------|-------------|
| `validateCoords(coords, crs)` | Validate against CRS range → `ValidationResult` |
| `cleanCoords(coords, strategy)` | Remove / clamp / snap invalid values |
| `deduplicateCoords(coords, tolerance)` | Remove near-duplicate points |

### Coordinate Sorting & Gridding

| Function | Description |
|----------|-------------|
| `sortCoordsByLng(coords)` | Sort pairs by longitude |
| `sortCoordsByLat(coords)` | Sort pairs by latitude |
| `gridIndex(coords, cellSizeDeg)` | Spatial hash grid IDs per point |
| `version()` | Library version string |
| `memoryInfo()` | WASM memory usage → `MemoryInfo` |
| `cgcs2000IsWgs84Compatible()` | Returns `true` (sub-cm accuracy) |
| `initThreadPool(numThreads)` | Init Rayon thread pool (multi-thread) |

> All coordinate functions accept and return **flat** `Float64Array` in
> `[lng, lat, lng, lat, …]` layout for maximum throughput and direct
> GPU buffer compatibility.

---

## ⚡ Performance

Preliminary benchmarks on Apple M1 (Chrome 120, 1M coordinate pairs):

| Operation | Pure JS (proj4js) | wasm-spatial-core | Speedup |
|-----------|-------------------|-------------------|---------|
| WGS84 → GCJ-02 | ~1,200 ms | ~45 ms | **~27×** |
| WGS84 → Mercator | ~800 ms | ~12 ms | **~67×** |
| GeoJSON parse (50 MB) | ~3,500 ms | ~320 ms | **~11×** |

> Benchmarks are indicative and will vary by hardware and browser.
> Run `cargo bench` for local results.

---

## 📋 Roadmap

See **[PLAN.md](./PLAN.md)** for the full development roadmap.

All three phases completed ✅ — Phase 1 (MVP), Phase 2 (Ecosystem), Phase 3 (Heterogeneous Data), plus backlogged features.

---

## 🤝 Contributing

Contributions are welcome! See [**CONTRIBUTING.md**](./CONTRIBUTING.md) for details.

- 🐛 [Report a bug](.github/ISSUE_TEMPLATE/bug_report.md)
- 💡 [Request a feature](.github/ISSUE_TEMPLATE/feature_request.md)

---

## 📄 License

[MIT License](./LICENSE) — © 2026 **智启未来 (Zhiqi Weilai)** — Qingxi

---

<div align="center">

**Built with 🦀 Rust + 🕸️ WebAssembly**

*Bringing the power of native spatial computing to every browser.*

</div>
