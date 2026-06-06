// heightfield_flatten_v1.wgsl — flatten masked heightfield cells to target elevation
// @version 1.0.0
// @workgroup_size 8, 8

struct FlattenParams {
    width: u32,
    height: u32,
    target: f32,
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
        heights[idx] = params.target;
    }
}
