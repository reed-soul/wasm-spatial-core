# Engine Boundary — Core vs Application

> **Rule:** If only one vertical (parking, inspection, charging) needs it, it does **not** go in wasm-spatial-core.  
> **Rule:** If Cesium/Three/your app can do it in &lt;50 lines of JS without spatial heavy lifting, it stays **out**.

---

## ✅ Engine core (this repo)

| Category | Examples |
|----------|----------|
| **Format I/O** | LAS/LAZ, GeoTIFF, GLB read/write, 3D Tiles encode |
| **Spatial IR** | `SpatialChunk`, region select, CRS / ENU |
| **Geometry** | Terrain cut/flatten/fill, mesh clip, QEM decimate |
| **Index & tiles** | Octree, R-tree, LOD, **incremental tile patch** |
| **Throughput** | WebGPU kernels for deform / transform / decimate |
| **Runtime** | Zero-copy buffers, Workers, **AbortSignal**, memory limits |

---

## ❌ Application layer (your next project)

| Category | Why not engine | What to use instead |
|----------|----------------|---------------------|
| Parking occupancy UI | Business semantics | App state + existing `createInstancedTilesetI3dm` |
| Real-time MQTT / WebSocket | I/O & protocols | App + viewer |
| Inspection path replay | Viewer polyline | Cesium `PolylineCollection` + app buffer |
| Geofencing alerts | Business rules | App + existing `isPointInRing` if needed |
| Frustum culling | Viewer job | Cesium/Three built-in culling |
| Scene timelines (“2027 no tree”) | Product workflow | App scene graph |
| Auth, dashboards, multi-tenant | Product | App |

---

## ⚠️ Already in engine — do not rebuild

| Capability | API |
|------------|-----|
| Instanced models (static export) | `encodeI3dmTile`, `createInstancedTilesetI3dm` |
| 2D path simplify | `simplifyDouglasPeucker` |
| Point in polygon | `isPointInRing`, `polygonIntersects` |
| Point cloud → tiles | `generateTileset`, `buildOctree` |

Apps compose these. No `InstanceLayer`, `setOccupied`, or `TrajectoryBuffer` in core.

---

## Backlog (research / maybe never)

- NTv2 / ProjDB high-precision grids  
- 3D Gaussian splatting native pipeline  
- GPU BVH for custom renderers  
- GPU QEM (CPU QEM is core first)  
- SVD site registration (unless photogrammetry alignment becomes blocking)  
- WASM Component Model plugins  
