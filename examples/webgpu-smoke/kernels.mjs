// Sync with shaders/transform_points_v1.wgsl and npm/webgpu.ts
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
