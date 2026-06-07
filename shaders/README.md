# WGSL compute shaders (Wave 4)

Versioned WebGPU compute kernels for **wasm-spatial-core**. Loaded by the
`wasm-spatial-core/webgpu` TypeScript module.

## Versioning (W4.6)

| File | Version | Workgroup | Purpose |
|------|---------|-----------|---------|
| `transform_points_v1.wgsl` | 1.0.0 | 256×1×1 | Mat4 × vec3 batch transform |
| `heightfield_flatten_v1.wgsl` | 1.0.0 | 8×8×1 | Masked flatten to target elevation |
| `mesh_quadrics_v1.wgsl` | 1.1.0 | 256×1×1 | Per-triangle quadric accumulation |
| `mesh_edge_costs_v1.wgsl` | 1.1.0 | 256×1×1 | Per-edge QEM collapse cost |

Bump the `@version` comment and filename suffix (`_v2`) when changing binding
layouts or struct packing. `webGpuShaderVersion()` in WASM returns the bundle
version (`1.1.0`).

### Subgroup features

Kernels do **not** require `subgroups`. The JS `GpuContext` checks
`adapter.features` before enabling subgroup-optimized paths in future versions.

## Buffer layout contract (W4.2)

Layouts are mirrored in Rust (`src/webgpu.rs`) and TypeScript (`npm/webgpu.ts`).

| Buffer | Type | Stride | Layout |
|--------|------|--------|--------|
| Positions | `Float32Array` | 12 B | `[x0,y0,z0, x1,y1,z1, …]` |
| Indices | `Uint32Array` | 4 B | triangle list |
| Heights | `Float32Array` | 4 B | row-major `heights[row×width + col]` |
| Mask | `Uint8Array` | 1 B | `0` outside, `1` inside (uploaded as `u32` on GPU) |
| Matrix | `Float32Array` | 64 B | 16 floats, **column-major** (WebGL / glTF) |
| Quadrics | `Float32Array` | 40 B / vertex | 10 floats, symmetric 4×4 upper triangle |

### Transform kernel bindings

| Binding | Visibility | Content |
|---------|------------|---------|
| 0 | uniform | `mat4x4<f32>` + `point_count: u32` (256 B padded) |
| 1 | storage read | input positions (`point_count × 3` f32) |
| 2 | storage read_write | output positions |

### Flatten kernel bindings

| Binding | Visibility | Content |
|---------|------------|---------|
| 0 | uniform | `width`, `height`, `target: f32` |
| 1 | storage read | mask (`width × height` u32, 0/1) |
| 2 | storage read_write | heights (`width × height` f32) |

### QEM quadrics kernel bindings (`mesh_quadrics_v1`)

| Binding | Visibility | Content |
|---------|------------|---------|
| 0 | uniform | `tri_count`, `vertex_count` |
| 1 | storage read | positions (`vertex_count × 3` f32) |
| 2 | storage read | indices (`tri_count × 3` u32) |
| 3 | storage read_write | quadrics (`vertex_count × 10` atomic u32, bitcast f32) |

### QEM edge costs kernel bindings (`mesh_edge_costs_v1`)

| Binding | Visibility | Content |
|---------|------------|---------|
| 0 | uniform | `edge_count`, `vertex_count` |
| 1 | storage read | positions |
| 2 | storage read | quadrics (`vertex_count × 10` f32) |
| 3 | storage read | edges (`edge_count × 2` u32) |
| 4 | storage read_write | costs (`edge_count` f32) |

## CPU fallback

When `navigator.gpu` is unavailable or `preferGpu: false`, the `webgpu` module
delegates to WASM:

- `transformPointCloud` — point transform
- `flattenTerrain` — heightfield flatten (requires `terrain-edit` build)
- `simplifyMeshQem` — QEM decimation (requires `mesh-edit` build)

GPU QEM (W5.7) uses `accumulateQuadrics` + `evaluateEdgeCosts` on the GPU with a
CPU collapse loop in `simplifyMeshQem` when `preferGpu: true`.
