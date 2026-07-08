# Architecture

This document is the single canonical description of how **wasm-spatial-core**
is structured. It consolidates what is otherwise spread across
[VISION.md](./VISION.md), [docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md),
and [ROADMAP_V2.md](./ROADMAP_V2.md). For *what* to build and *why*, read those;
for *how the pieces fit*, read this.

> **One-line summary:** every spatial format is parsed into buffers, optionally
> normalized through the **Spatial IR**, transformed by geometry / terrain /
> GPU kernels, and emitted as 3D Tiles (pnts / b3dm / i3dm), quantized-mesh
> terrain, or glTF — all inside WASM, all in the browser.

---

## 1. The three-layer split

This is the load-bearing design rule. Code lives in exactly one of three
layers; putting logic in the wrong layer is the most common architectural bug.

| Layer | Runs in | Owns | Does **not** own |
|-------|---------|------|------------------|
| **WASM core (Rust)** | WASM linear memory | Format I/O, Spatial IR, geometry edit, tile trees, correctness, memory budgeting | Viewer culling, rendering, DOM |
| **WebGPU (WGSL + JS)** | GPU | Throughput-bound kernels: point transform, heightfield flatten, mesh quadrics | The viewer's own draw calls |
| **JS (app)** | main thread + Workers | Init, worker pool, `AbortSignal`, progress, viewer wiring, business logic | Anything that needs spatial heavy lifting |

Two principles fall out of this:

- **Engine core, not product.** If only one vertical (parking, inspection,
  charging) needs it, it does **not** go in core. See
  [docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md).
- **Viewer stays in the viewer.** Frustum culling, polyline animation, and
  scene timelines are Cesium / Three.js / app concerns. The engine emits
  standard tiles; the viewer consumes them.

---

## 2. Data flow

```
                ┌─────────────────────────── formats (read) ────────────────────────────┐
                │                                                                          │
   LAS/LAZ/COPC │  GeoTIFF    GLB/glTF   GeoJSON  MVT   WKT/WKB   PLY/OBJ/PCD   E57  IFC │
        ▼       │     ▼          ▼          ▼      ▼       ▼          ▼          ▼    ▼  │
   point_cloud  │  geotiff   gltf_reader  geojson_ vector_  wkb_wkt      ply/obj     e57  │
        │       │     │       │+gltf_w   parser   tile.rs   │           │          │   │
        │       │     │       │  riter    (stream)           │           │          │   │
        ▼       │     ▼       │             │                │           │          │   │
   ┌─────────────────────────▼─────────────▼────────────────▼───────────▼──────────┘   │
   │                              Spatial IR (optional)                                  │
   │          SpatialChunk = PointCloudChunk | MeshChunk | HeightfieldChunk             │
   │          (+ Aabb, ChunkMeta, CRS/ENU frame)            ▲                            │
   └──────────────┬───────────────────────────┬───────────┼───────────────┬───────────┘
                  │                           │           │               │
            region select               geometry edit   terrain edit   SVD align
            (selectAabb)            clip/cap/QEM/OBB   cut/fill/flatten computeSvdAlignment
                  │                           │           │               │
                  ▼                           ▼           ▼               ▼
   ┌────────────────────────── transform / throughput layer ──────────────────────────┐
   │   CPU (Rust): octree, R-tree, quantization, tile patch                            │
   │   GPU (WGSL, JS-driven): transform_points, heightfield_flatten, mesh_quadrics    │
   └──────────────┬───────────────────────────────────┬──────────────────────────────┘
                  ▼                                    ▼
        ┌──── formats (write / generate) ────┐   ┌──── export ────┐
        │ pnts  b3dm  i3dm  + tileset.json   │   │ glTF/GLB        │
        │ quantized-mesh  TMS pyramid        │   │ (mesh/pc/terrain)│
        │ layer.json                         │   └─────────────────┘
        └────────────────┬───────────────────┘
                         ▼
                  Cesium / Three.js / your app
```

Two things to note about this diagram:

1. **The Spatial IR is optional.** The default npm build runs point-cloud and
   GeoJSON paths straight through to tiles without ever materializing a
   `SpatialChunk`. The IR exists for the `mesh-ingest` world (GLB read → edit →
   GLB/tile write) and for cross-format region selection. V1 pipelines never
   touch it.
2. **WebGPU is not a Rust dependency.** There is no `wgpu` crate. The Rust side
   (`webgpu.rs`, `mesh_qem_gpu.rs`) only exports **layout constants**,
   a **shader-bundle version**, and **CPU reference implementations** for
   parity testing. The real kernels are WGSL shaders (`shaders/*.wgsl`) driven
   by JS (`npm/webgpu.ts`). This keeps the default WASM binary small and the
   GPU path opt-in via the `webgpu` feature.

---

## 3. Module map

The crate is **flat** — every module sits directly under `src/`. There is no
`src/formats/` or `src/ir/`; feature flags do the namespacing instead. The
table groups modules by concern and shows which [feature flag](#4-feature-flags)
gates each.

### Format I/O (read)

| Module | Reads | Feature |
|--------|-------|---------|
| `point_cloud.rs` | LAS (header / chunked / per-point / progress / abort) | `point-cloud` |
| `point_cloud.rs` (LAZ path) | LAZ decompression via `laz` crate | `laz-support` |
| `copc_hierarchy.rs`, `point_cloud_stream.rs` | COPC hierarchy + spatial region queries | `laz-support` |
| `e57.rs` | E57 | `e57-support` |
| `ply.rs`, `obj.rs` | PLY, OBJ (vertices/normals) | `point-cloud` / `mesh-ingest` |
| `point_cloud.rs` (PCD path) | PCD ASCII + binary | `point-cloud` |
| `geotiff.rs` | GeoTIFF (LZW/deflate/none; strip+tile; no GDAL) | `geotiff` |
| `gltf_reader.rs` | GLB/glTF → Spatial IR mesh | `mesh-ingest` |
| `ifc_reader.rs` | IFC geometry (text-based) | `mesh-ingest` |
| `geojson_parser.rs`, `geojson_streaming.rs` | GeoJSON (incl. lazy/streaming) | default |
| `topojson.rs` | TopoJSON | default |
| `vector_tile.rs` | MVT decode (via `mvt` + `geozero`) | default |
| `wkb_wkt.rs` | WKT / WKB | default |
| `gpx.rs` | GPX tracks | default |

### Spatial IR + geometry edit

| Module | Responsibility | Feature |
|--------|----------------|---------|
| `spatial_ir.rs` | `SpatialChunk`, `PointCloudChunk`, `MeshChunk`, `HeightfieldChunk`, `Aabb`, `ChunkMeta`, `selectAabb` | `mesh-ingest` |
| `gltf_writer.rs` | `GltfBuilder`, `mesh_to_glb`, `point_cloud_to_glb`, `terrain_to_glb` | `mesh-ingest` |
| `chunk_export.rs` | IR → standalone tile / GLB export | `mesh-ingest` |
| `enu_frame.rs` | WGS84 ↔ local ENU frame | `mesh-ingest` |
| `svd_align.rs` | Weighted / RANSAC SVD point-cloud alignment (Umeyama) | `mesh-ingest` |
| `mesh_edit.rs`, `mesh_clip.rs`, `mesh_cap.rs` | OBB split, plane clip, hole capping (Euler χ) | `mesh-edit` |
| `mesh_qem.rs`, `mesh_qem_math.rs` | QEM mesh simplification | `mesh-edit` |
| `terrain_edit.rs` | Excavate / fill / flatten / feather + polygon mask raster | `terrain-edit` |

### Index, tiling, terrain encode

| Module | Responsibility | Feature |
|--------|----------------|---------|
| `octree.rs` | Octree builder + chunk reorder (`OctreeChunkBuilder`) | `point-cloud` |
| `pnts.rs` | 3D Tiles `.pnts` encoder + tileset generator (incremental / abort / parallel) | `point-cloud` |
| `b3dm.rs` | `.b3dm` (Batched 3D Model) + `.i3dm` (Instanced) encoders + tileset builders | default |
| `cesium_adapter.rs` | `generate_cesium_geometry`, `generate_3d_tile` | default |
| `quantized_mesh.rs` | quantized-mesh encode/decode (zig-zag, index stream) | `geotiff` |
| `terrain_tms.rs` | TMS pyramid + `layer.json` generator | `geotiff` |
| `tile_patch.rs` | `TilesetPatch` + `applyTilesetPatch` (incremental tile edits) | default |
| `spatial_index.rs` | `rstar`-based R-tree, kNN, grid index | default |

### Throughput

| Module | Responsibility | Feature |
|--------|----------------|---------|
| `webgpu.rs` | GPU layout constants, shader-bundle version, **CPU reference** implementations for parity | `webgpu` |
| `mesh_qem_gpu.rs` | CPU reference path for the GPU QEM kernels (parity testing) | `webgpu` |
| `quantization.rs` | Float32→Uint16 quantization (Draco alternative) | `point-cloud` |
| `shaders/*.wgsl` | `transform_points`, `heightfield_flatten`, `mesh_quadrics`, `mesh_edge_costs` | `webgpu` |

### Runtime, transforms, analysis

| Module | Responsibility | Feature |
|--------|----------------|---------|
| `coordinate.rs` | WGS84/Web-Mercator/UTM/GCJ02/BD09/CGCS2000 batch conversions (no `proj`) | default |
| `runtime.rs` | `WasmProcessingContext`, `JobOp`, `estimate_job_bytes`, memory budgeting | default |
| `worker.rs` | `WorkerHandle`, `WorkerOptions`, generated worker script | default |
| `spatial_analysis.rs` | centroid, bearing, haversine, geohash, hulls, clustering | default |
| `topology.rs` | polygon union/intersection, buffer, concave/convex hulls | default |
| `errors.rs` | `SpatialError` (prefer this over `JsValue` for new APIs) | default |
| `lib.rs` | `#[wasm_bindgen]` re-export hub + `init()` panic hook | default |

---

## 4. Feature flags

The default build is intentionally small. Everything beyond core formats and
coordinate transforms is opt-in. From `Cargo.toml`:

| Feature | Pulls in | In default npm package? |
|---------|----------|-------------------------|
| `point-cloud` | LAS/PLY/OBJ/PCD, octree, pnts | yes |
| `laz-support` | `point-cloud` + LAZ/COPC decompression | no (custom build) |
| `e57-support` | E57 reader | no |
| `geotiff` | GeoTIFF decode + quantized-mesh | yes |
| `mesh-ingest` | `point-cloud` + GLB read + Spatial IR | no |
| `terrain-edit` | `geotiff` + terrain deformation (W3) | no |
| `mesh-edit` | `mesh-ingest` + clip/cap/QEM (W5) | no |
| `webgpu` | `point-cloud` + WebGPU kernels (W4) | no |
| `multi-thread` | rayon + wasm-bindgen-rayon (needs COOP/COEP) | nightly build only |
| `single-thread` | (default) | yes |

CI runs `cargo test --all-features`, so the whole matrix is green on every push.
The published npm package builds a curated subset — see the
[`npm/package.json` `exports` map](./npm/package.json).

---

## 5. Cancellation and incremental output (W1)

Two cross-cutting mechanisms shape almost every long-running API:

- **Cancellable pipelines.** Long jobs accept an optional
  `shouldAbort: () => bool` callback (or JS `Function`). When aborted, they
  return `SpatialError::Cancelled` (`code === 'CANCELLED'`). The JS helper
  `createAbortChecker(signal)` bridges an `AbortSignal` to this callback.
- **Incremental tile edits.** Build a `TilesetPatch`, call `setTile(uri, bytes)`
  per changed tile, then `applyTilesetPatch(base, patch)`. Tiles whose URIs are
  not in the patch are preserved verbatim — no full rebuild.

---

## 6. What is explicitly out of scope

These are **not** missing features — they are deliberate non-goals, documented
in [docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md):

- **`proj` / arbitrary EPSG reprojection.** The engine ships curated, verified
  transforms for the coordinate systems it supports (WGS84, Web-Mercator, UTM,
  GCJ02, BD09, CGCS2000) plus ENU local frames. Arbitrary EPSG → EPSG via
  PROJ/ProjDB is out of scope.
- **Viewer concerns.** Frustum culling, polyline animation, scene timelines —
  the consumer (Cesium/Three) already does these.
- **Product logic.** Parking occupancy, MQTT/IoT, geofencing alerts,
  trajectory replay. These compose *on top of* the engine in your app.
- **3D Gaussian Splatting native pipeline.** 3DGS is an *input strategy*, not
  a core concern.

The backlog (research, maybe-never): NTv2/ProjDB grids, GPU BVH for custom
renderers, GPU QEM (CPU path is core first), WASM Component Model plugins.

---

## 7. Where to make changes

| If you are adding… | Touch… | Feature-gate under… | Watch out for… |
|--------------------|--------|---------------------|----------------|
| A new format reader | a new `src/<fmt>.rs` + a `parse*` export in `lib.rs` | the relevant feature | zero-copy `Float32Array` returns; `SpatialError` over `JsValue` |
| A geometry operation | `mesh_edit.rs` / `terrain_edit.rs` | `mesh-edit` / `terrain-edit` | keep a CPU reference if you add a GPU kernel |
| A GPU kernel | `shaders/*.wgsl` + JS in `npm/webgpu.ts` + a CPU reference in `webgpu.rs` | `webgpu` | the default build must still compile without `webgpu` |
| A new tile format | the encode path + tileset builder | relevant feature | the headless Cesium test (`tests/cesium-terrain.spec.mjs`) is the acceptance bar |
| A public API | `lib.rs` `#[wasm_bindgen]` | — | `pub mod test_exports` is `#[doc(hidden)]` — internals stay there, not in the public surface |

Before claiming work complete, run the
[CI parity table](./AGENTS.md#ci-parity): `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all-features`, `wasm-pack build`, and the Node smoke tests.
