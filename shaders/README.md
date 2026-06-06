# WGSL compute shaders (Wave 4)

Versioned WebGPU compute kernels for **wasm-spatial-core**. Loaded by the
`wasm-spatial-core/webgpu` TypeScript module.

## Versioning (W4.6)

| File | Version | Workgroup | Purpose |
|------|---------|-----------|---------|
| `transform_points_v1.wgsl` | 1.0.0 | 256×1×1 | Mat4 × vec3 batch transform |
| `heightfield_flatten_v1.wgsl` | 1.0.0 | 8×8×1 | Masked flatten to target elevation |

Bump the `@version` comment and filename suffix (`_v2`) when changing binding
layouts or struct packing. `webGpuShaderVersion()` in WASM returns the bundle
version (`1.0.0`).

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

## CPU fallback

When `navigator.gpu` is unavailable or `preferGpu: false`, the `webgpu` module
delegates to WASM:

- `transformPointCloud` — point transform
- `flattenTerrain` — heightfield flatten (requires `terrain-edit` build)
