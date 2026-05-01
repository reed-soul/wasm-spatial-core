<div align="center">

# 🌍 wasm-spatial-core

**A high-performance WebAssembly engine that brings server-grade spatial computing to the browser.**

[![CI](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml/badge.svg)](https://github.com/reed-soul/wasm-spatial-core/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/@anthropic-wasm/spatial-core)](https://www.npmjs.com/package/@anthropic-wasm/spatial-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-🦀-orange.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0.svg?logo=webassembly&logoColor=white)](https://webassembly.org)

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

- 🔄 **Batch Coordinate Projection** — WGS-84 ↔ GCJ-02, WGS-84 → Web Mercator (EPSG:3857)
- 📦 **GeoJSON Parser** — Parse large FeatureCollections into flat `Float64Array` buffers
- ⚡ **Zero-Copy Architecture** — Data stays in WASM linear memory; JS gets typed array views
- 📊 **GPU-Ready Output** — Flat `[x, y, x, y, …]` buffers upload directly to WebGL vertex attributes

### Coming Soon

- 🗺️ Frontend vector tile slicing (MVT)
- 🌐 Cesium 3D Tiles / Mars3D adapters
- ☁️ LAS/LAZ point cloud parsing & decimation
- 🏗️ IFC/BIM geometry extraction
- 🧵 Multi-threaded via Web Workers + SharedArrayBuffer

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

# Build the WASM package
wasm-pack build --target web --release

# Run Rust tests
cargo test

# Run WASM tests in a headless browser
wasm-pack test --headless --chrome
```

---

## 📖 API Reference

### Coordinate Conversion

| Function | Description |
|----------|-------------|
| `batchWgs84ToGcj02(coords: Float64Array): Float64Array` | WGS-84 → GCJ-02 (China) |
| `batchGcj02ToWgs84(coords: Float64Array): Float64Array` | GCJ-02 → WGS-84 |
| `batchWgs84ToMercator(coords: Float64Array): Float64Array` | WGS-84 → EPSG:3857 |

### GeoJSON Processing

| Function | Description |
|----------|-------------|
| `parseGeoJsonCoords(geojson: string): Float64Array` | Extract all coordinates as flat array |
| `countGeoJsonFeatures(geojson: string): number` | Count features (fast pre-scan) |

### Utilities

| Function | Description |
|----------|-------------|
| `version(): string` | Library version |

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

See **[PLAN.md](./PLAN.md)** for the full three-phase development roadmap:

1. **Phase 1 (MVP)**: Zero-copy pipeline, GeoJSON parser, CRS projection engine
2. **Phase 2**: Cesium/Mars3D adapters, frontend tiling, spatial indexing
3. **Phase 3**: LAS/LAZ point cloud, IFC/BIM, GPU-ready buffer generation

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
