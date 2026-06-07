# ROADMAP V2 — Unified Web3D Spatial Engine

> **Vision:** [VISION.md](./VISION.md)  
> **Boundary:** [docs/ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md) — core vs application  
> **Completed wedge:** [ROADMAP_V1.md](./ROADMAP_V1.md) (point cloud → 3D Tiles)  
> **Platform:** Latest Chrome only · WASM + WebGPU · 3D Tiles distribution

---

## Principles

1. **Core only** — no product plugins (parking, inspection, geofence, timelines).  
2. **Compose, don’t duplicate** — if i3dm / Douglas–Peucker / point-in-polygon already exist, apps use them.  
3. **Replace desktop pre-processing** — ingest, edit geometry, emit tiles/glTF.  
4. **Viewer stays in the viewer** — frustum cull, polylines, MQTT are app concerns.

---

## Waves Overview

| Wave | Theme | Outcome |
|------|-------|---------|
| **W1** | Core runtime & incremental output | Tile patch, cancellable jobs, memory budget |
| **W2** | Spatial IR + GLB ingest | Unified chunks, read/edit glTF, region select |
| **W3** | Terrain deformation | Cut, flatten, fill on heightfields |
| **W4** | WebGPU compute core | GPU deform / transform / decimate kernels |
| **W5** | Mesh geometry edit | Clip, QEM decimate, cap holes |

**Start here after V1:** W2 (Spatial IR) — everything else hangs off one internal representation.

---

## Wave 1 — Core Runtime & Incremental Output

**Goal:** Pipeline infrastructure shared by all formats — **not** IoT, parking, or live twin product APIs.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W1.1 | Tile patch protocol | Incremental `tileset.json` / content URI diff |
| W1.2 | `AbortSignal` jobs | Cancellable Worker + WASM long tasks |
| W1.3 | Memory arena / job budget | Optional buffer reuse + `estimateJobBytes` |

### Issue templates

[docs/issues/WAVE_1.md](./docs/issues/WAVE_1.md)

### Exit criteria

- [ ] Single-tile edit → patch ≪ full tileset  
- [ ] Abort during 10M-point parse returns without panic  

---

## Wave 2 — Spatial IR + GLB Ingest

**Goal:** All formats converge to one internal **Spatial IR** before export or edit.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W2.1 | `SpatialChunk` enum | `PointCloudChunk`, `MeshChunk`, `HeightfieldChunk` |
| W2.2 | Chunk metadata | CRS, AABB, version, byte budget |
| W2.3 | GLB/glTF reader | Parse mesh attributes (mirror `gltf_writer`) |
| W2.4 | Region select | By AABB or polygon extrusion |
| W2.5 | Chunk export | IR → glTF, pnts/b3dm subset, tile patch |
| W2.6 | ENU / local frame | Site-scale precision; anchor + local offsets |
| W2.7 | SVD 3D alignment | Umeyama similarity from control point pairs |

### Issue templates

[docs/issues/WAVE_2.md](./docs/issues/WAVE_2.md)

### Exit criteria

- [ ] GLB → IR → GLB round-trip (positions + indices)  
- [ ] Submesh select by AABB → standalone GLB  

---

## Wave 3 — Terrain Deformation

**Goal:** Browser-native flatten / cut / fill (replaces GIS/desktop terrain tools for common edits).

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W3.1 | Polygon mask on heightfield | Inside/outside raster |
| W3.2 | Cut (excavate) | Depth or target elevation |
| W3.3 | Flatten | Target height inside polygon |
| W3.4 | Fill | Raise to target height |
| W3.5 | Edge feather | Blend ramp at boundary |
| W3.6 | Re-encode terrain tiles | Deformed grid → quantized-mesh pyramid |
| W3.7 | Golden raster tests | Reference height grids in CI |

### Issue templates

[docs/issues/WAVE_3.md](./docs/issues/WAVE_3.md)

### Exit criteria

- [ ] 2048×2048 flatten &lt; 500 ms WASM release  
- [ ] Cesium terrain demo loads re-encoded pyramid  

---

## Wave 4 — WebGPU Compute Core

**Goal:** Throughput for geometry-bound work. **Not** viewer culling.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W4.1 | `GpuContext` bootstrap | `navigator.gpu` device + buffer import |
| W4.2 | Buffer layout contract | Shared WASM ↔ GPU layouts |
| W4.3 | Point transform kernel | Batch `Mat4 × vec3` |
| W4.4 | Heightfield kernel | Parallel W3 ops on GPU |
| W4.5 | Fallback policy | Same API → WASM when no GPU |
| W4.6 | WGSL versioning | `shaders/` + subgroup feature detect |

**Removed from core:** frustum cull (Cesium/Three handle this).

### Issue templates

[docs/issues/WAVE_4.md](./docs/issues/WAVE_4.md)

### Exit criteria

- [ ] 10M point transform: GPU faster than WASM SIMD on discrete GPU  
- [ ] Feature `webgpu` optional; default build unchanged  

---

## Wave 5 — Mesh Geometry Edit

**Goal:** Replace Blender/CloudCompare for common mesh split + decimate.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W5.1 | OBB / half-space classifier | Triangle inside/outside |
| W5.2 | Mesh split (phase 1) | Inside/outside index buffers |
| W5.3 | Plane clip (phase 2) | UV/normal interpolation |
| W5.4 | Cap holes | Planar ear-clipping |
| W5.5 | QEM decimate (CPU) | Garland–Heckbert to target ratio |
| W5.6 | UV seam preservation | Penalize seam edge collapses |

**Backlog (not W5):** GPU QEM — CPU path first.

### Issue templates

[docs/issues/WAVE_5.md](./docs/issues/WAVE_5.md)

### Exit criteria

- [ ] 500k → 50k triangles QEM on sample mesh  
- [ ] OBB split correct on unit cube fixture  

---

## Feature Flags (Planned)

| Feature | Modules | Default |
|---------|---------|---------|
| `point-cloud` | LAS/LAZ, octree, pnts | off (npm build enables) |
| `geotiff` | Terrain | off |
| `mesh-ingest` | GLB read, Spatial IR | off |
| `terrain-edit` | W3 deformation | off |
| `webgpu` | W4 compute | off |
| `mesh-edit` | W5 clip/QEM | off |

No `live-twin` flag — instance export stays under existing 3D Tiles / i3dm APIs.

---

## How to File Work

1. Read [ENGINE_BOUNDARY.md](./docs/ENGINE_BOUNDARY.md) — confirm the work is core  
2. Pick a deliverable (default start: **W2.1**)  
3. Copy issue block from [docs/issues/](./docs/issues/)  
4. Labels: `roadmap-v2`, `wave-N`, `engine`  

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-06 | Initial V2 roadmap |
| 2026-06-06 | Removed trajectory/geofence from engine |
| 2026-06-06 | Removed instance/parking twin wave; W1 = runtime only; trimmed frustum cull, GPU QEM, SVD from core path |
