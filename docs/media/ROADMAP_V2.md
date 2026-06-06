# ROADMAP V2 — Unified Web3D Spatial Engine

> **Vision:** [VISION.md](./VISION.md)  
> **Completed wedge:** [ROADMAP_V1.md](./ROADMAP_V1.md) (point cloud → 3D Tiles)  
> **Platform:** Latest Chrome only · WASM + WebGPU · 3D Tiles distribution

---

## Waves Overview

| Wave | Theme | Outcome | Depends on |
|------|-------|---------|------------|
| **W1** | Live twin primitives | Instances, 3D trajectories, geofence, tile patches | V1 tileset output |
| **W2** | Spatial IR + GLB ingest | Unified chunks, read/edit glTF, region select | — |
| **W3** | Terrain deformation | Cut, flatten, fill on heightfields | W2 IR (heightfield chunk) |
| **W4** | WebGPU compute core | GPU kernels + WASM orchestration | W2 IR |
| **W5** | Advanced mesh edit | OBB clip, QEM decimate, cap holes | W2 + W4 |

Waves are **sequential in priority**, not strict blockers — W1 can ship while W2 is in progress.

---

## Wave 1 — Live Twin Primitives

**Goal:** Support real-time digital twin scenarios (trajectories, facility instances, occupancy) without regenerating full tilesets every frame.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W1.1 | `InstanceLayer` API | Wrap i3dm semantics: template GLB + slot table `{ id, transform, visible, metadata }` |
| W1.2 | In-place pose update | `updateInstance(id, matrix)` mutates GPU-ready buffers or patch blob |
| W1.3 | Occupancy / visibility | `setVisible(id, bool)`, `setOccupied(slotId, bool)` for parking-style twins |
| W1.4 | 3D trajectory stream | `TrajectoryBuffer.push(t, x,y,z)`, ring buffer, max length |
| W1.5 | 3D RDP simplify | Ramer–Douglas–Peucker on `f64`/`f32` XYZ polylines (extend existing 2D geo RDP) |
| W1.6 | Geofence events | `checkGeofence(trajectory, polygon) → enter/exit/dwell events` using existing topology |
| W1.7 | Tile patch protocol | Incremental `tileset.json` / content URI diff instead of full rebuild |
| W1.8 | `AbortSignal` jobs | Long pipelines honour cancellation across Worker + WASM |

### Issue templates

Copy from [docs/issues/WAVE_1.md](./docs/issues/WAVE_1.md)

### Exit criteria

- [ ] 10k instance slots update at 60 Hz path without full tileset regen (benchmark)  
- [ ] Drone path 100k points → RDP → renderable polyline < 100 ms (M2 class, WASM)  
- [ ] Geofence unit tests for enter/exit on synthetic polygon  

---

## Wave 2 — Spatial IR + GLB Ingest

**Goal:** All formats converge to one internal **Spatial IR** before export.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W2.1 | `SpatialChunk` enum | `PointCloudChunk`, `MeshChunk`, `HeightfieldChunk`, `InstanceGroupChunk` |
| W2.2 | Chunk metadata | CRS, AABB, version, source format, byte budget |
| W2.3 | GLB/glTF reader | Parse positions, indices, normals, UVs, materials (read path mirrors `gltf_writer`) |
| W2.4 | Region select | Select by AABB, polygon extrusion, or triangle ID range |
| W2.5 | Chunk export | IR → glTF, IR → pnts/b3dm subset, IR → tile patch |
| W2.6 | ENU / local frame | Tangent-plane origin; large-coordinate stability for site-scale scenes |
| W2.7 | SVD alignment | 3+ anchor pairs geo ↔ local → `Mat4` registration |

### Issue templates

[docs/issues/WAVE_2.md](./docs/issues/WAVE_2.md)

### Exit criteria

- [ ] Round-trip: GLB → IR → GLB (positions + indices preserved)  
- [ ] Select submesh by AABB; export as standalone GLB  
- [ ] SVD alignment error < 1 cm on synthetic anchor set  

---

## Wave 3 — Terrain Deformation

**Goal:** Browser-native “flatten / cut / fill” on elevation grids (alternative to editing splats or mesh in Blender).

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W3.1 | Polygon mask on heightfield | Rasterize polygon to grid; mark inside/outside |
| W3.2 | Cut (excavate) | Lower inside polygon by depth or to absolute elevation |
| W3.3 | Flatten | Set inside polygon to target height with blend ramp at edge |
| W3.4 | Fill | Raise inside polygon to target height |
| W3.5 | Smooth transition | Feather N cells at boundary (cosine or linear) |
| W3.6 | Re-encode terrain tiles | Deformed heightfield → quantized-mesh pyramid |
| W3.7 | WASM tests + golden rasters | Compare against reference height grids |

### Issue templates

[docs/issues/WAVE_3.md](./docs/issues/WAVE_3.md)

### Exit criteria

- [ ] 2048×2048 heightfield flatten < 500 ms (WASM, release)  
- [ ] Re-encoded terrain loads in Cesium demo without seams at LOD 0  

---

## Wave 4 — WebGPU Compute Core

**Goal:** Introduce `webgpu` feature module; offload throughput work to compute shaders.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W4.1 | `GpuContext` bootstrap | `navigator.gpu` adapter/device, WASM buffer import |
| W4.2 | Buffer contract | Shared layout: positions, indices, height grids ↔ GPU buffers |
| W4.3 | Point transform kernel | Batch `Mat4 × vec3` on millions of points |
| W4.4 | Heightfield kernel | Parallel flatten/cut on GPU grid |
| W4.5 | Frustum cull kernel | AABB vs frustum → visible instance IDs |
| W4.6 | Fallback policy | Chrome without adapter → WASM path (same API, slower) |
| W4.7 | WGSL shader crate / embed | Versioned shaders, subgroup ops where available |

### Issue templates

[docs/issues/WAVE_4.md](./docs/issues/WAVE_4.md)

### Exit criteria

- [ ] 10M point transform faster on GPU than WASM SIMD on M2-class GPU (benchmark)  
- [ ] Feature `webgpu` optional; default build unchanged  

---

## Wave 5 — Advanced Mesh Edit

**Goal:** Replace Blender for common “split / simplify / cap” workflows on photogrammetry meshes.

### Deliverables

| ID | Capability | Description |
|----|------------|-------------|
| W5.1 | OBB tester | Classify triangles inside/outside oriented box |
| W5.2 | Mesh split (phase 1) | Export inside/outside index buffers without UV interp |
| W5.3 | Plane clip (phase 2) | Sutherland–Hodgman style cut; interpolate UV/normal |
| W5.4 | Cap holes | Ear-clipping on boundary loops (planar caps) |
| W5.5 | QEM decimate | Garland–Heckbert edge collapse; target ratio |
| W5.6 | UV seam preservation | Penalize collapses across UV boundaries |
| W5.7 | GPU QEM (optional) | W4 integration for large meshes |

### Issue templates

[docs/issues/WAVE_5.md](./docs/issues/WAVE_5.md)

### Exit criteria

- [ ] 500k triangle mesh → 50k via QEM; visual sanity on sample asset  
- [ ] OBB split produces two watertight-ish parts on cube fixture  

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
| `live-twin` | W1 instances/trajectory | off |

---

## How to File Work

1. Pick a wave deliverable (e.g. W1.4)  
2. Open [docs/issues/WAVE_N.md](./docs/issues/) and copy the matching issue block  
3. Label: `roadmap-v2`, `wave-N`, `engine`  
4. PR must include tests + benchmark or explicit N/A  

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-06 | Initial V2 roadmap from product vision |
