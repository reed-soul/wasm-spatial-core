# 🗺️ wasm-spatial-core — Development Roadmap

> **Mission**: Offload heavy spatial computation from the cloud to the browser.
> Every cycle saved on the server is a cycle that belongs to the user.

---

## Phase 1 · MVP — Core Infrastructure & Zero-Copy Pipeline

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

**Exit Criteria**: ✅ Achieved — A published npm package that can parse a 50 MB GeoJSON file
and project 1M coordinates in < 500 ms on a mid-range laptop browser.

---

## Phase 2 · Ecosystem Integration — Web3D Engine Adapters

**Goal**: Bridge the gap between raw spatial data and what rendering engines
like **Cesium**, **Mars3D**, **Mapbox GL**, and **deck.gl** actually consume.
Implement lightweight frontend tiling and format conversion so that users can
skip the traditional server-side tile pipeline entirely for moderate datasets.

| Task | Description | Status |
|------|-------------|--------|
| **2.1 Cesium 3D Tiles adapter** | Generate quantized-mesh terrain tiles or Batched 3D Model (b3dm) payloads from raw geometry directly in the browser. | 🟡 In Progress |
| **2.2 Mars3D / SuperMap integration** | Adapter layer for Mars3D's internal data structures. Direct `ArrayBuffer` → Mars3D entity pipeline. | ⬜ Planned |
| **2.3 Frontend spatial indexing** | R-tree or grid-based spatial index built in WASM. Enables viewport-driven progressive loading without a tile server. | ✅ Done |
| **2.4 Lightweight vector tile slicing** | Slice large GeoJSON into Mapbox Vector Tile (MVT) format on the client. Eliminate the need for `tippecanoe` or PostGIS for small-to-medium datasets. | 🟡 In Progress |
| **2.5 Multi-threaded via Web Workers** | Partition large datasets and distribute parsing / projection across multiple workers using `SharedArrayBuffer`. | 🟡 In Progress |

**Exit Criteria**: A Cesium-based demo that loads a 200 MB GeoJSON, projects,
indexes, and renders it entirely in the browser with no backend support.

---

## Phase 3 · Heterogeneous Data — Point Cloud & BIM

**Goal**: Bring heavy-duty point cloud (LAS / LAZ / PCD) and BIM (IFC)
pre-processing to the browser. Users drag-and-drop a raw scan file and see it
rendered in seconds — no upload, no server, no waiting.

| Task | Description | Status |
|------|-------------|--------|
| **3.1 LAS/LAZ parser** | Integrate `las-rs` (and potentially `laz-rs` for compressed LAZ). Stream-parse multi-GB files with constant memory via chunked reading. | ⬜ Planned |
| **3.2 Point cloud decimation** | Octree-based spatial decimation (voxel grid / random sampling). Output: thinned `Float32Array` position + color buffers for direct WebGL draw. | ⬜ Planned |
| **3.3 PCD format support** | Parse ASCII and binary PCD files. Convert to the same canonical buffer layout for engine-agnostic rendering. | ⬜ Planned |
| **3.4 IFC geometry extraction** | Experimental: extract mesh geometry from IFC-SPF files using `ifc-rs` or a custom parser. Output: indexed triangle buffers. | ⬜ Planned |
| **3.5 GPU-ready buffer generation** | Produce interleaved vertex buffers (`position + normal + color`) in the exact layout expected by WebGL2 / WebGPU `vertexBufferLayout`, eliminating all CPU-side re-packing in JS. | ⬜ Planned |
| **3.6 Streaming & progress API** | Expose a progress callback / `ReadableStream`-compatible interface so the UI can show a progress bar during multi-second parses. | ⬜ Planned |

**Exit Criteria**: A browser demo where the user drops a 500 MB LAS file,
sees a decimated 3D point cloud render within 5 seconds, with no data leaving
the machine.

---

## Future Ideas (Backlog)

- 🌐 **COPC (Cloud-Optimized Point Cloud)** streaming reader
- 🏗️ **glTF / GLB** writer — convert any geometry to glTF in-browser
- 📐 **Spatial analysis** — buffering, intersection, union via `geo` crate
- 🔒 **WASM Component Model** — future-proof modular architecture
- 🧪 **WASI Preview 2** support for Node.js / Deno server-side usage

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md)
(coming soon) for guidelines.

---

*© 2026 智启未来 (Zhiqi Weilai) — Qingxi · MIT License*
