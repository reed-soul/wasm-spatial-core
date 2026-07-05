// heightfield_flatten_v1.wgsl — flatten masked heightfield cells to target elevation
// @version 1.0.1
// @workgroup_size 8, 8
//
// v1.0.1: renamed `target` → `target_height`. `target` is a reserved keyword
// in WGSL, so v1.0.0 failed to compile in Chrome with
// "'target' is a reserved keyword". Caught by tests/webgpu-bench.spec.mjs.

struct FlattenParams {
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
    if (col >= params.width || row >= params.height) {
        return;
    }

    let idx = row * params.width + col;
    if (mask[idx] == 1u) {
        heights[idx] = params.target_height;
    }
}
