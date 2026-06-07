//! Shared Garland–Heckbert quadric math (CPU + GPU parity reference).

pub const QEM_EPS: f64 = 1e-12;
pub const QUADRIC_FLOAT_COUNT: usize = 10;

/// Symmetric 4×4 quadric stored as upper-triangular 10 doubles.
#[derive(Debug, Clone, Copy, Default)]
pub struct Quadric {
    pub m: [f64; 10],
}

impl Quadric {
    pub fn from_plane(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            m: [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ],
        }
    }

    pub fn add(&mut self, other: &Self) {
        for i in 0..10 {
            self.m[i] += other.m[i];
        }
    }

    pub fn add_assign_slice(&mut self, other: &[f64; 10]) {
        for (dst, &src) in self.m.iter_mut().zip(other.iter()) {
            *dst += src;
        }
    }

    pub fn evaluate(&self, x: f64, y: f64, z: f64) -> f64 {
        let [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9] = self.m;
        m0 * x * x
            + m4 * y * y
            + m7 * z * z
            + 2.0 * (m1 * x * y + m2 * x * z + m5 * y * z)
            + 2.0 * (m3 * x + m6 * y + m8 * z)
            + m9
    }

    pub fn optimal_position(&self) -> Option<[f64; 3]> {
        let [m0, m1, m2, m3, m4, m5, m6, m7, m8, _m9] = self.m;
        let a = [[m0, m1, m2], [m1, m4, m5], [m2, m5, m7]];
        let b = [-m3, -m6, -m8];
        solve_3x3(a, b)
    }

    pub fn from_f32_slice(slice: &[f32]) -> Self {
        let mut m = [0.0f64; 10];
        for (dst, &src) in m.iter_mut().zip(slice.iter().take(10)) {
            *dst = src as f64;
        }
        Self { m }
    }
}

pub fn solve_3x3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = determinant3(a);
    if det.abs() < QEM_EPS {
        return None;
    }
    Some([
        determinant3([
            [b[0], a[0][1], a[0][2]],
            [b[1], a[1][1], a[1][2]],
            [b[2], a[2][1], a[2][2]],
        ]) / det,
        determinant3([
            [a[0][0], b[0], a[0][2]],
            [a[1][0], b[1], a[1][2]],
            [a[2][0], b[2], a[2][2]],
        ]) / det,
        determinant3([
            [a[0][0], a[0][1], b[0]],
            [a[1][0], a[1][1], b[1]],
            [a[2][0], a[2][1], b[2]],
        ]) / det,
    ])
}

pub fn determinant3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

pub fn triangle_plane(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 4]> {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < QEM_EPS {
        return None;
    }
    let n = [n[0] / len, n[1] / len, n[2] / len];
    let d = -(n[0] * a[0] + n[1] * a[1] + n[2] * a[2]);
    Some([n[0], n[1], n[2], d])
}

/// Accumulate per-vertex quadrics from indexed triangles (CPU reference for GPU parity).
pub fn accumulate_vertex_quadrics(positions: &[[f64; 3]], indices: &[u32]) -> Vec<[f64; 10]> {
    let mut quadrics = vec![[0.0f64; 10]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let Some(plane) = triangle_plane(positions[i0], positions[i1], positions[i2]) else {
            continue;
        };
        let q = Quadric::from_plane(plane[0], plane[1], plane[2], plane[3]);
        for &idx in &[i0, i1, i2] {
            for (dst, &src) in quadrics[idx].iter_mut().zip(q.m.iter()) {
                *dst += src;
            }
        }
    }
    quadrics
}

/// Evaluate collapse cost for one edge given vertex quadrics.
pub fn edge_collapse_cost(
    positions: &[[f64; 3]],
    quadrics: &[[f64; 10]],
    a: u32,
    b: u32,
) -> (f64, [f64; 3]) {
    let mut q = Quadric {
        m: quadrics[a as usize],
    };
    q.add_assign_slice(&quadrics[b as usize]);
    edge_cost_from_merged_quadric(positions, a, b, &q)
}

/// Evaluate collapse cost from live `Quadric` storage (CPU QEM loop).
pub fn edge_collapse_cost_quadrics(
    positions: &[[f64; 3]],
    quadrics: &[Quadric],
    a: u32,
    b: u32,
) -> (f64, [f64; 3]) {
    let mut q = quadrics[a as usize];
    q.add(&quadrics[b as usize]);
    edge_cost_from_merged_quadric(positions, a, b, &q)
}

fn edge_cost_from_merged_quadric(
    positions: &[[f64; 3]],
    a: u32,
    b: u32,
    q: &Quadric,
) -> (f64, [f64; 3]) {
    let pos = q
        .optimal_position()
        .unwrap_or(midpoint(positions[a as usize], positions[b as usize]));
    let cost = q.evaluate(pos[0], pos[1], pos[2]).max(0.0);
    (cost, pos)
}

/// Batch edge costs — flat `edges` as `[a0, b0, a1, b1, …]`.
pub fn evaluate_edge_costs(
    positions: &[[f64; 3]],
    quadrics: &[[f64; 10]],
    edges: &[(u32, u32)],
) -> Vec<f64> {
    edges
        .iter()
        .map(|&(a, b)| edge_collapse_cost(positions, quadrics, a, b).0)
        .collect()
}

pub fn midpoint(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

pub fn build_unique_edges(indices: &[u32]) -> Vec<(u32, u32)> {
    use std::collections::HashSet;
    let mut edges = HashSet::new();
    for tri in indices.chunks_exact(3) {
        edges.insert(normalize_edge(tri[0], tri[1]));
        edges.insert(normalize_edge(tri[1], tri[2]));
        edges.insert(normalize_edge(tri[2], tri[0]));
    }
    edges.into_iter().collect()
}

pub fn normalize_edge(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadric_plane_eval_zero_on_plane() {
        let q = Quadric::from_plane(0.0, 0.0, 1.0, -0.5);
        assert!(q.evaluate(1.0, 2.0, 0.5).abs() < 1e-9);
    }
}
