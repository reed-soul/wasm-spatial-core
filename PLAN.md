# 🗺️ wasm-spatial-core — Development Roadmap

> **Mission**: Offload heavy spatial computation from the cloud to the browser.
> Every cycle saved on the server is a cycle that belongs to the user.

---

## Phase 1 · MVP — Core Infrastructure & Zero-Copy Pipeline ✅

**Goal**: Establish the foundational memory-sharing architecture and deliver the
two most universally demanded spatial primitives — GeoJSON parsing and CRS
projection — at near-native speed inside the browser.

| Task | Description | Status |
|------|-------------|--------|
| **1.1 Zero-copy memory bridge** | Design and implement the `Float64Array`-backed data exchange protocol between JS and WASM. All public APIs must operate on flat typed arrays — no JSON serialisation on the hot path. | ✅ Done |
| **1.2 GeoJSON streaming parser** | Parse arbitrarily large GeoJSON payloads (100 MB+) inside WASM memory. Output: flat `[lng, lat, …]` buffers ready for GPU upload. Support `FeatureCollection`, `Feature`, and bare `Geometry`. | ✅ Done |
| **1.3 Coordinate projection engine** | Batch WGS-84 ↔ GCJ-02, WGS-84 ↔ BD-09, WGS-84 → EPSG:3857 (Web Mercator). All operating on `Float64Array` with zero per-point JS↔WASM overhead. | ✅ Done |
| **1.4 Benchmark harness** | Automated benchmarks comparing WASM vs. pure-JS implementations (e.g. `proj4js`, `@turf/turf`) for 100k / 1M / 10M coordinate transforms. | ✅ Done |
| **1.5 npm package & CI** | wasm-pack → npm publish pipeline, GitHub Actions CI with clippy + fmt + test + wasm-build. | ✅ Done |
| **1.6 Documentation & examples** | API docs (rustdoc + TypeDoc), runnable browser demo page. | ✅ Done |

---

## Phase 2 · Ecosystem Integration — Web3D Engine Adapters ✅

**Goal**: Bridge the gap between raw spatial data and what rendering engines
actually consume.

| Task | Description | Status |
|------|-------------|--------|
| **2.1 Cesium 3D Tiles adapter** | Generate quantized-mesh terrain tiles or Batched 3D Model (b3dm) payloads from raw geometry directly in the browser. | ✅ Done |
| **2.2 Mars3D / SuperMap integration** | Adapter layer for Mars3D's internal data structures. | ⬜ Deferred |
| **2.3 Frontend spatial indexing** | R-tree based spatial index built in WASM for viewport-driven progressive loading. | ✅ Done |
| **2.4 Lightweight vector tile slicing** | Slice large GeoJSON into Mapbox Vector Tile (MVT) format on the client. | ✅ Done |
| **2.5 Multi-threaded via Web Workers** | Partition datasets across workers using `SharedArrayBuffer`. | ✅ Done |

---

## Phase 3 · Heterogeneous Data — Point Cloud & BIM ✅

**Goal**: Bring point cloud and BIM pre-processing to the browser. Users
drag-and-drop a raw scan file and see it rendered in seconds.

| Task | Description | Status |
|------|-------------|--------|
| **3.1 LAS/LAZ parser** | Stream-parse multi-GB LAS files with chunked reading. | ✅ Done |
| **3.2 Point cloud decimation** | Voxel grid / random sampling decimation. Output: thinned `Float32Array` buffers. | ✅ Done |
| **3.3 PCD format support** | Parse ASCII and binary PCD files. | ✅ Done |
| **3.4 IFC geometry extraction** | Experimental: extract mesh geometry from IFC-SPF `IFCEXTRUDEDAREASOLID` entities via text parsing. Triangulate to indexed meshes. | ✅ Done |
| **3.5 GPU-ready buffer generation** | Interleaved vertex buffers (`position + normal + color`) for WebGL2 / WebGPU. | ✅ Done |
| **3.6 Streaming & progress API** | Progress callback for multi-second parses. | ✅ Done |

---

## Backlog & Completed Extras ✅

| Feature | Description | Status |
|---------|-------------|--------|
| **glTF / GLB writer** | Convert any geometry to glTF/GLB in-browser. | ✅ Done |
| **Spatial analysis** | Buffering, bounding box, centroid. | ✅ Done |
| **COPC (range-based access)** | LAS header-only parsing + point offset computation for range-based `fetch`. | ✅ Done |
| **Memory management** | `memoryInfo()` API, input size validation (100MB limit). | ✅ Done |
| **Error handling** | All WASM functions return `Result<T, JsValue>`. | ✅ Done |

---

## What's Next

Potential future directions (not committed):

- 🔒 **WASM Component Model** — modular architecture for composable spatial processing
- 🧪 **WASI Preview 2** support for Node.js / Deno server-side usage
- 🌐 **Full COPC streaming** — LAZ decompression + hierarchical LOD (requires heavy deps)
- 🏗️ **Full IFC support** — integrate `ifc-rs` for comprehensive BIM data access
- 🗺️ **TopoJSON support** — arc-based topology format
- 📐 **Geodesic calculations** — distance, bearing, great-circle interpolation
- 🧮 **Projection grid files** — NTv2 / ProjDB for high-accuracy local projections

---

## Stats

| Metric | Value |
|--------|-------|
| Source lines | ~5600+ |
| Unit tests | 100 |
| Integration tests | 8 |
| Supported formats | GeoJSON, MVT, LAS, PCD, IFC, glTF/GLB |
| Coordinate systems | WGS-84, GCJ-02, BD-09, CGCS2000, Web Mercator |

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md)
(coming soon) for guidelines.

---

*© 2026 智启未来 (Zhiqi Weilai) — Qingxi · MIT License*
