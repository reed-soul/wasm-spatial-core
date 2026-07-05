// Sync with shaders/*.wgsl and npm/webgpu.ts — browser demos import from here.

export const TRANSFORM_POINTS_WGSL_V1 = `struct TransformParams {
  matrix: mat4x4<f32>,
  point_count: u32,
}
@group(0) @binding(0) var<uniform> params: TransformParams;
@group(0) @binding(1) var<storage, read> positions_in: array<f32>;
@group(0) @binding(2) var<storage, read_write> positions_out: array<f32>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.point_count) { return; }
  let base = i * 3u;
  let v = vec4<f32>(positions_in[base], positions_in[base+1u], positions_in[base+2u], 1.0);
  let out = params.matrix * v;
  positions_out[base] = out.x;
  positions_out[base+1u] = out.y;
  positions_out[base+2u] = out.z;
}`;

/** @version 1.0.1 — keep in sync with shaders/heightfield_flatten_v1.wgsl
 *
 * v1.0.1: renamed `target` → `target_height`. `target` is a reserved keyword
 * in WGSL (used for @interpolate etc.), so the v1.0.0 shader failed to
 * compile in real Chrome with "Error while parsing WGSL: 'target' is a
 * reserved keyword". Caught by tests/webgpu-bench.spec.mjs on M4 macOS.
 */
export const HEIGHTFIELD_FLATTEN_WGSL_V1 = `struct FlattenParams {
  width: u32,
  height: u32,
  target_height: f32,
  _pad: u32,
}
@group(0) @binding(0) var<uniform> params: FlattenParams;
@group(0) @binding(1) var<storage, read> mask: array<u32>;
@group(0) @binding(2) var<storage, read_write> heights: array<f32>;
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let col = gid.x;
  let row = gid.y;
  if (col >= params.width || row >= params.height) { return; }
  let idx = row * params.width + col;
  if (mask[idx] == 1u) { heights[idx] = params.target_height; }
}`;
