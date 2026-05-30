<div align="center">

# 🌍 wasm-spatial-core

**A high-performance WebAssembly engine that brings server-grade spatial computing to the browser.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/@anthropic-wasm/spatial-core)](https://www.npmjs.com/package/@anthropic-wasm/spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-🦀-orange.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0.svg?logo=webassembly&logoColor=white)](https://webassembly.org)

<!-- stats -->
![Lines](https://img.shields.io/badge/lines-6.5K-blue)
![Tests](https://img.shields.io/badge/tests-108-success)
![Formats](https://img.shields.io/badge/formats-6-green)


*将服务端算力下放至客户端，释放云端压力*
*Offload server-side computing to the client — free the cloud.*

[Quick Start](#-quick-start) · [API Reference](#-api-reference) · [Roadmap](./PLAN.md) · [Contributing](#-contributing)

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
- **Point cloud scans** (LAS/LAZ) require decimation pipelines running on expensive cloud VMs
- **BIM models** (IFC) must be pre-processed and converted server-side
- Every user request adds latency, bandwidth costs, and cloud compute bills

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

### Available Now (v0.1)

- 🔄 **Batch Coordinate Projection** — WGS-84 ↔ GCJ-02, WGS-84 ↔ BD-09, WGS-84 ↔ Web Mercator (EPSG:3857)
- 📦 **GeoJSON Parser** — Parse large FeatureCollections into flat `Float64Array` buffers
- 📡 **Streaming GeoJSON Parser** — Chunked processing with progress callbacks for large files
- 🔍 **Spatial Index (R-Tree)** — Bounding box search, nearest-neighbor, K-nearest-neighbor queries
- 🗺️ **Vector Tile Slicing** — Frontend MVT tile generation for offline tiling
- 🌐 **Cesium Native Adapter** — WGS84 → Cartesian3 (ECEF), polygon triangulation (earcut), 3D Tiles (b3dm)
- ☁️ **Point Cloud (LAS/PCD)** — Parse LAS headers & points, voxel grid & random decimation, PCD format support
- 🌐 **COPC Range-Based Access** — LAS header-only parsing + point offset computation for range-based `fetch`
- 🏗️ **IFC/BIM Geometry** (experimental) — Extract `IFCEXTRUDEDAREASOLID` mesh geometry from IFC-SPF text
- 🔺 **glTF / GLB Writer** — Build glTF 2.0 scenes in WASM, export as GLB binary
- 📐 **Spatial Analysis** — Point/line buffering, bounding box, centroid on WGS-84
- ⚡ **Zero-Copy Architecture** — Data stays in WASM linear memory; JS gets typed array views
- 📊 **GPU-Ready Output** — Interleaved vertex buffers, indexed geometry for WebGL2/WebGPU
- 🧵 **Multi-threaded** (optional) — Web Workers + SharedArrayBuffer via Rayon
- 🔒 **Memory Management** — `memoryInfo()` API, 100MB input size limit

---

## 🚀 Quick Start

### Installation

```bash
npm install @anthropic-wasm/spatial-core
```

### Basic Usage

```typescript
import { loadSpatialCore } from "@anthropic-wasm/spatial-core";

// Initialize the WASM module (call once)
const core = await loadSpatialCore();

// Print version
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

// ── GeoJSON parsing ──────────────────────────────────────────
const geojson = await fetch("/data/buildings.geojson").then(r => r.text());

// Count features (fast — useful for progress reporting)
const count = core.countGeoJsonFeatures(geojson);
console.log(`Parsing ${count} features...`);

// Extract all coordinates as a flat Float64Array
const coords = core.parseGeoJsonCoords(geojson);
console.log(`Extracted ${coords.length / 2} coordinate pairs`);

// Feed directly to WebGL
gl.bufferData(gl.ARRAY_BUFFER, coords, gl.STATIC_DRAW);
```

### Using with Cesium

```typescript
import { loadSpatialCore } from "@anthropic-wasm/spatial-core";

const core = await loadSpatialCore();
const geojson = await fetch("/large-dataset.geojson").then(r => r.text());

// Parse & project in WASM — orders of magnitude faster than JS
const coords = core.parseGeoJsonCoords(geojson);
const mercatorCoords = core.batchWgs84ToMercator(coords);

// Create Cesium entities from the projected coordinates
for (let i = 0; i < mercatorCoords.length; i += 2) {
  viewer.entities.add({
    position: Cesium.Cartesian3.fromDegrees(
      mercatorCoords[i],
      mercatorCoords[i + 1]
    ),
    point: { pixelSize: 4 },
  });
}
```

---

## 🛠️ Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) ≥ 18

### Build

```bash
# Clone the repository
git clone https://github.com/reed-soul/wasm-spatial-core.git
cd wasm-spatial-core

# Build the standard single-threaded WASM package
wasm-pack build --target web --release
```

### Build with Multi-threading

To enable Web Workers and `SharedArrayBuffer` for extreme performance, you must use the `nightly` Rust toolchain and the custom `build:wasm:mt` NPM script:

```bash
# Ensure you have the nightly toolchain and rust-src component
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Build the multi-threaded version
cd npm
npm run build:wasm:mt
```

> **⚠️ IMPORTANT:** The multi-threaded version relies on `SharedArrayBuffer`. Your web server must be configured with the following HTTP headers:
> - `Cross-Origin-Opener-Policy: same-origin`
> - `Cross-Origin-Embedder-Policy: require-corp`
> 
> You must also initialize the thread pool on the JS side before calling any parallel functions:
> ```typescript
> import { loadSpatialCore, initThreadPool } from "@anthropic-wasm/spatial-core";
> await loadSpatialCore();
> await initThreadPool(navigator.hardwareConcurrency);
> ```

---

## 📖 API Reference

### Coordinate Conversion

| Function | Description |
|----------|-------------|
| `batchWgs84ToGcj02(coords)` | WGS-84 → GCJ-02 (China) |
| `batchGcj02ToWgs84(coords)` | GCJ-02 → WGS-84 |
| `batchWgs84ToBd09(coords)` | WGS-84 → BD-09 (Baidu) |
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
| `parseGeoJsonCoords(geojson)` | Extract all coordinates as flat `Float64Array` |
| `countGeoJsonFeatures(geojson)` | Count features (fast pre-scan) |
| `parseGeoJsonStream(input, chunkSize, onChunk)` | Streaming chunked parser with progress callback |
| `parseGeoJsonPerFeature(input)` | Parse into per-feature coordinate arrays |

### Spatial Index

| Function | Description |
|----------|-------------|
| `new SpatialIndex(coords)` | Build R-Tree from flat coordinate array |
| `.searchBBox(minX, minY, maxX, maxY)` | Bounding box range query → `Uint32Array` of IDs |
| `.nearestNeighbor(x, y)` | Find nearest point → ID or `null` |
| `.kNearestNeighbors(x, y, k)` | K nearest neighbors → `Uint32Array` of IDs |
| `.size()` | Total point count |

### Vector Tiles

| Function | Description |
|----------|-------------|
| `new VectorTileEngine(geojson, options)` | Create MVT tile engine from GeoJSON |
| `.getTile(z, x, y)` | Get MVT (PBF) protobuf for tile → `Uint8Array` |

### Cesium Adapter

| Function | Description |
|----------|-------------|
| `batchWgs84ToCartesian3(coords)` | WGS84 `[lng,lat,…]` → Cartesian3 `[x,y,z,…]` |
| `wgs84ToCartesian3Single(lng, lat, height)` | Single point → `[x, y, z]` |
| `generateCesiumGeometry(geojson, heightProp?)` | Triangulate polygons → `CesiumMeshGeometry` |
| `generate3DTile(geojson, options?)` | Generate 3D Tiles (b3dm) → `Cesium3DTile` |

### Point Cloud (LAS/PCD)

| Function | Description |
|----------|-------------|
| `parseLasHeader(bytes)` | Parse LAS header → `LasHeader` |
| `parseLasPoints(bytes)` | Parse all LAS points → `LasPointCloud` |
| `parsePcdAscii(text)` | Parse ASCII PCD → `PcdPointCloud` |
| `parsePcdBinary(bytes)` | Parse binary PCD → `PcdPointCloud` |
| `decimateVoxelGrid(positions, colors, gridSize)` | Voxel grid decimation |
| `decimateRandom(positions, colors, targetCount)` | Random sampling |
| `generateInterleavedVertexBuffer(positions, colors)` | GPU-ready interleaved buffer |
| `generateIndexedGeometry(positions, indices)` | GPU-ready indexed buffers |

### COPC (Range-Based Access)

| Function | Description |
|----------|-------------|
| `parseLasHeaderOnly(bytes)` | Header-only parse for range-based access → `LasHeaderInfo` |
| `computeLasPointOffset(header, pointIndex, format)` | Byte offset of Nth point |
| `parseLasPointAt(bytes, offset, format)` | Parse single point at offset → `PointData` |

### IFC/BIM (Experimental)

| Function | Description |
|----------|-------------|
| `parseIfcGeometry(ifcText)` | Extract meshes from IFC-SPF → `IfcGeometryResult` |

### glTF / GLB

| Function | Description |
|----------|-------------|
| `new GltfWriter()` | Create glTF scene builder |
| `.addMesh(positions, indices, material?)` | Add triangle mesh |
| `.addMeshWithBbox(positions, indices, bbox, material?)` | Add mesh with bounding box |
| `.buildGlb()` | Export as GLB binary → `Uint8Array` |

### Spatial Analysis

| Function | Description |
|----------|-------------|
| `boundingBox(coords, dimensions)` | Compute bounding box → `[min, max]` |
| `bufferPoints(coords, radius)` | Buffer points on a sphere → `[lng, lat, …]` |
| `bufferLine(coords, radius)` | Buffer a line on a sphere → `[lng, lat, …]` |
| `centroid(coords, dimensions)` | Compute centroid → `[x, y, …]` |

### Utilities

| Function | Description |
|----------|-------------|
| `version()` | Library version |
| `memoryInfo()` | WASM memory usage → `MemoryInfo` |
| `cgcs2000IsWgs84Compatible()` | Returns `true` (sub-cm accuracy) |

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

*Benchmarks are indicative and will vary by hardware and browser. Formal
benchmark suite coming in Phase 1.*

---

## 📋 Roadmap

See **[PLAN.md](./PLAN.md)** for the full development roadmap.

All three phases completed ✅ — Phase 1 (MVP), Phase 2 (Ecosystem), Phase 3 (Heterogeneous Data), plus backlogged features (glTF, spatial analysis, COPC).

---

## 🤝 Contributing

Contributions are welcome! Whether it's:

- 🐛 Bug reports and feature requests
- 📝 Documentation improvements
- 🔧 Code contributions
- 🧪 Benchmark data and real-world test cases

Please see our [Contributing Guide](./CONTRIBUTING.md) (coming soon).

---

## 📄 License

[MIT License](./LICENSE) — © 2026 **智启未来 (Zhiqi Weilai)** — Qingxi

---

<div align="center">

**Built with 🦀 Rust + 🕸️ WebAssembly**

*Bringing the power of native spatial computing to every browser.*

</div>
