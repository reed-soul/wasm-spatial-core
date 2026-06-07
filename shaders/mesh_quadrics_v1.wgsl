// mesh_quadrics_v1.wgsl — accumulate per-vertex quadrics from triangles
// @version 1.1.0
// @workgroup_size 256

struct QuadricParams {
    tri_count: u32,
    vertex_count: u32,
}

@group(0) @binding(0) var<uniform> params: QuadricParams;
@group(0) @binding(1) var<storage, read> positions: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
// vertex_count × 10 atomic u32 slots (bitcast f32)
@group(0) @binding(3) var<storage, read_write> quadrics: array<atomic<u32>>;

fn atomic_add_f32(ptr: ptr<storage, atomic<u32>>, val: f32) {
    loop {
        let old_bits = atomicLoad(ptr);
        let new_bits = bitcast<u32>(bitcast<f32>(old_bits) + val);
        let result = atomicCompareExchangeWeak(ptr, old_bits, new_bits);
        if result.exchanged {
            break;
        }
    }
}

fn add_quadric_to_vertex(vertex: u32, q: array<f32, 10>) {
    let base = vertex * 10u;
    for (var i = 0u; i < 10u; i = i + 1u) {
        atomic_add_f32(&quadrics[base + i], q[i]);
    }
}

fn plane_quadric(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> array<f32, 10> {
    let ab = p1 - p0;
    let ac = p2 - p0;
    let n = cross(ab, ac);
    let len = length(n);
    var q: array<f32, 10>;
    if (len < 1e-12) {
        for (var i = 0u; i < 10u; i = i + 1u) {
            q[i] = 0.0;
        }
        return q;
    }
    let nn = n / len;
    let d = -dot(nn, p0);
    let a = nn.x;
    let b = nn.y;
    let c = nn.z;
    q[0] = a * a;
    q[1] = a * b;
    q[2] = a * c;
    q[3] = a * d;
    q[4] = b * b;
    q[5] = b * c;
    q[6] = b * d;
    q[7] = c * c;
    q[8] = c * d;
    q[9] = d * d;
    return q;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tri = gid.x;
    if (tri >= params.tri_count) {
        return;
    }

    let base = tri * 3u;
    let i0 = indices[base];
    let i1 = indices[base + 1u];
    let i2 = indices[base + 2u];

    if (i0 >= params.vertex_count || i1 >= params.vertex_count || i2 >= params.vertex_count) {
        return;
    }

    let p0 = vec3<f32>(
        positions[i0 * 3u],
        positions[i0 * 3u + 1u],
        positions[i0 * 3u + 2u],
    );
    let p1 = vec3<f32>(
        positions[i1 * 3u],
        positions[i1 * 3u + 1u],
        positions[i1 * 3u + 2u],
    );
    let p2 = vec3<f32>(
        positions[i2 * 3u],
        positions[i2 * 3u + 1u],
        positions[i2 * 3u + 2u],
    );

    let q = plane_quadric(p0, p1, p2);
    add_quadric_to_vertex(i0, q);
    add_quadric_to_vertex(i1, q);
    add_quadric_to_vertex(i2, q);
}
