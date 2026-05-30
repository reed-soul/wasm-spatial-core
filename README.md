<div align="center">

# 🌍 wasm-spatial-core

**A high-performance WebAssembly engine that brings server-grade spatial computing to the browser.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/wasm-spatial-core)](https://www.npmjs.com/package/wasm-spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-%23A73737-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0.svg?logo=webassembly&logoColor=white)](https://webassembly.org)

![Lines](https://img.shields.io/badge/code-6.6K-blue)
![Tests](https://img.shields.io/badge/tests-131-success)
![Formats](https://img.shields.io/badge/formats-6-green)

*Offload server-side spatial computing to the client — free the cloud.*

[Quick Start](#-quick-start) · [API Reference](#-api-reference) · [Roadmap](./PLAN.md) · [Contributing](./CONTRIBUTING.md)

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
- 📦 **GeoJSON Parser** — Parse large FeatureCollections into flat `Float64Array` buffers
- 📡 **Streaming GeoJSON Parser** — Chunked processing with progress callbacks
- 🔍 **Spatial Index (R-Tree)** — Bounding box search, nearest-neighbor, K-nearest-neighbor (point index + edge index)
- 🗺️ **Vector Tile Slicing** — Frontend MVT tile generation from GeoJSON
- 🌐 **Cesium Native Adapter** — WGS84 → Cartesian3 (ECEF), polygon triangulation, 3D Tiles (b3dm)
- ☁️ **Point Cloud (LAS/PCD)** — Parse LAS headers & points, voxel grid & random decimation, COPC range-based access
- 🏗️ **IFC/BIM Geometry** (experimental) — Extract `IFCEXTRUDEDAREASOLID` mesh geometry
- 🔺 **glTF / GLB Writer** — Build glTF 2.0 scenes in WASM, export as GLB binary
- 📐 **Spatial Analysis** — Point/line buffering, bounding box, centroid, haversine distance, bearing, destination, midpoint on WGS-84
- 📐 **Topology Analysis** — Polygon area (spherical excess), polyline length, Douglas-Peucker simplification, point-in-ring
- ⚡ **Zero-Copy Architecture** — Data stays in WASM linear memory; JS gets typed array views
- 📊 **GPU-Ready Output** — Interleaved vertex buffers, indexed geometry for WebGL2/WebGPU
- 🧵 **Multi-threaded** (optional) — Web Workers + SharedArrayBuffer via Rayon
- 🔒 **Memory Management** — `memoryInfo()` API, 100 MB input size limit

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

console.log(core.version()); // "0.1.0"

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

- [Rust](https://rustup.rs/) (stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) ≥ 18

### Build

```bash
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core

# Standard single-threaded build
wasm-pack build --target web --release --out-dir pkg

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
| `*InPlace` variants | Zero-copy in-place mutation for all above |

### GeoJSON Processing

| Function | Description |
|----------|-------------|
| `parseGeoJsonCoords(input)` | Extract all coordinates as flat `Float64Array` |
| `countGeoJsonFeatures(input)` | Count features (fast pre-scan) |
| `parseGeoJsonStream(input, chunkSize, onChunk)` | Streaming parser with progress callback |
| `parseGeoJsonPerFeature(input)` | Per-feature coordinate arrays |
| `parseGeoJsonProperties(input)` | Extract all feature properties as JSON string |
| `parseGeoJsonFeatures(input)` | Structured per-feature result → `GeoJsonFeaturesResult` |

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
| `parseLasHeaderOnly(bytes)` | Header-only parse (COPC) |
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

### Utilities

| Function | Description |
|----------|-------------|
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
