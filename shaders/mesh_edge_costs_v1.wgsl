// mesh_edge_costs_v1.wgsl — evaluate QEM collapse cost per edge
// @version 1.1.0
// @workgroup_size 256

struct EdgeCostParams {
    edge_count: u32,
    vertex_count: u32,
}

@group(0) @binding(0) var<uniform> params: EdgeCostParams;
@group(0) @binding(1) var<storage, read> positions: array<f32>;
@group(0) @binding(2) var<storage, read> quadrics: array<f32>;
@group(0) @binding(3) var<storage, read> edges: array<u32>;
@group(0) @binding(4) var<storage, read_write> costs: array<f32>;

fn load_quadric(vertex: u32) -> array<f32, 10> {
    var q: array<f32, 10>;
    let base = vertex * 10u;
    for (var i = 0u; i < 10u; i = i + 1u) {
        q[i] = quadrics[base + i];
    }
    return q;
}

fn add_quadric(a: array<f32, 10>, b: array<f32, 10>) -> array<f32, 10> {
    var out: array<f32, 10>;
    for (var i = 0u; i < 10u; i = i + 1u) {
        out[i] = a[i] + b[i];
    }
    return out;
}

fn eval_quadric(q: array<f32, 10>, p: vec3<f32>) -> f32 {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    return q[0] * x * x
        + q[4] * y * y
        + q[7] * z * z
        + 2.0 * (q[1] * x * y + q[2] * x * z + q[5] * y * z)
        + 2.0 * (q[3] * x + q[6] * y + q[8] * z)
        + q[9];
}

fn optimal_position(q: array<f32, 10>) -> vec3<f32> {
    let m0 = q[0];
    let m1 = q[1];
    let m2 = q[2];
    let m3 = q[3];
    let m4 = q[4];
    let m5 = q[5];
    let m6 = q[6];
    let m7 = q[7];
    let m8 = q[8];

    let det = m0 * (m4 * m7 - m5 * m5)
        - m1 * (m1 * m7 - m2 * m5)
        + m2 * (m1 * m5 - m2 * m4);

    if (abs(det) < 1e-12) {
        return vec3<f32>(0.0);
    }

    let x = (-m3 * (m4 * m7 - m5 * m5) + m6 * (m1 * m7 - m2 * m5) - m8 * (m1 * m5 - m2 * m4)) / det;
    let y = (m0 * (-m6 * m7 + m5 * m8) - m1 * (-m3 * m7 + m2 * m8) + m2 * (-m3 * m5 + m2 * m6)) / det;
    let z = (m0 * (m4 * m8 - m5 * m6) - m1 * (m1 * m8 - m2 * m6) + m2 * (m1 * m6 - m2 * m4)) / det;
    return vec3<f32>(x, y, z);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let edge_idx = gid.x;
    if (edge_idx >= params.edge_count) {
        return;
    }

    let ebase = edge_idx * 2u;
    let a = edges[ebase];
    let b = edges[ebase + 1u];
    if (a >= params.vertex_count || b >= params.vertex_count) {
        costs[edge_idx] = 1e30;
        return;
    }

    let qa = load_quadric(a);
    let qb = load_quadric(b);
    let q = add_quadric(qa, qb);

    var pos = optimal_position(q);
    if (all(pos == vec3<f32>(0.0))) {
        let pa = vec3<f32>(
            positions[a * 3u],
            positions[a * 3u + 1u],
            positions[a * 3u + 2u],
        );
        let pb = vec3<f32>(
            positions[b * 3u],
            positions[b * 3u + 1u],
            positions[b * 3u + 2u],
        );
        pos = (pa + pb) * 0.5;
    }

    let cost = max(eval_quadric(q, pos), 0.0);
    costs[edge_idx] = cost;
}
