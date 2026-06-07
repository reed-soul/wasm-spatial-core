//! Garland–Heckbert QEM mesh decimation (Wave 5.5).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::spatial_ir::{ChunkMeta, MeshChunk};

const QEM_EPS: f64 = 1e-12;

/// Symmetric 4×4 quadric stored as upper-triangular 10 doubles.
#[derive(Debug, Clone, Copy, Default)]
struct Quadric {
    m: [f64; 10],
}

impl Quadric {
    fn from_plane(a: f64, b: f64, c: f64, d: f64) -> Self {
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

    fn add(&mut self, other: &Self) {
        for i in 0..10 {
            self.m[i] += other.m[i];
        }
    }

    fn evaluate(&self, x: f64, y: f64, z: f64) -> f64 {
        let [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9] = self.m;
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        m0 * xx
            + m4 * yy
            + m7 * zz
            + 2.0 * (m1 * x * y + m2 * x * z + m5 * y * z)
            + 2.0 * (m3 * x + m6 * y + m8 * z)
            + m9
    }

    fn optimal_position(&self) -> Option<[f64; 3]> {
        let [m0, m1, m2, m3, m4, m5, m6, m7, m8, _m9] = self.m;
        let a = [[m0, m1, m2], [m1, m4, m5], [m2, m5, m7]];
        let b = [-m3, -m6, -m8];
        solve_3x3(a, b)
    }
}

fn solve_3x3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
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

fn determinant3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge(u32, u32);

impl Edge {
    fn new(a: u32, b: u32) -> Self {
        if a < b {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }
}

#[derive(Debug, Clone)]
struct CollapseCandidate {
    cost: f64,
    edge: Edge,
    position: [f64; 3],
}

impl PartialEq for CollapseCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for CollapseCandidate {}

impl PartialOrd for CollapseCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CollapseCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

/// QEM simplification result with optional max geometric error estimate.
#[derive(Debug, Clone)]
pub struct QemResult {
    pub mesh: MeshChunk,
    pub max_error: f64,
    pub triangles_before: usize,
    pub triangles_after: usize,
}

/// Decimate a triangle mesh toward `target_triangles` using quadric error metrics.
pub fn simplify_mesh_qem(
    mesh: &MeshChunk,
    target_triangles: usize,
) -> Result<QemResult, SpatialErrorDetail> {
    if mesh.mode != MeshChunk::MODE_TRIANGLES || mesh.indices.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail("QEM requires indexed triangle mesh"));
    }
    if target_triangles == 0 {
        return Err(SpatialError::InvalidInput.with_detail("target_triangles must be > 0"));
    }

    let triangles_before = mesh.indices.len() / 3;
    if triangles_before <= target_triangles {
        return Ok(QemResult {
            mesh: mesh.clone(),
            max_error: 0.0,
            triangles_before,
            triangles_after: triangles_before,
        });
    }

    let mut positions: Vec<[f64; 3]> = mesh
        .positions
        .chunks_exact(3)
        .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64])
        .collect();
    let mut indices = mesh.indices.clone();
    let mut quadrics = vec![Quadric::default(); positions.len()];
    let mut deleted = vec![false; positions.len()];
    let mut max_error = 0.0f64;

    rebuild_quadrics(&positions, &indices, &mut quadrics);

    loop {
        let tri_count = indices.len() / 3;
        if tri_count <= target_triangles {
            break;
        }

        let mut heap = build_collapse_heap(&positions, &indices, &quadrics, &deleted);
        let Some(candidate) = heap.pop() else {
            break;
        };

        if !edge_exists(&indices, candidate.edge, &deleted) {
            continue;
        }

        let Edge(a, b) = candidate.edge;
        if deleted[a as usize] || deleted[b as usize] {
            continue;
        }

        max_error = max_error.max(candidate.cost.sqrt());

        positions[a as usize] = candidate.position;
        let b_q = quadrics[b as usize];
        quadrics[a as usize].add(&b_q);
        deleted[b as usize] = true;

        for idx in indices.iter_mut() {
            if *idx == b {
                *idx = a;
            }
        }

        remove_degenerate_triangles(&mut indices, &positions);
        if indices.len() / 3 <= target_triangles {
            break;
        }
    }

    compact_mesh(&mut positions, &mut indices, &deleted);
    remove_degenerate_triangles(&mut indices, &positions);

    let mut out_positions = Vec::with_capacity(positions.len() * 3);
    for p in &positions {
        out_positions.push(p[0] as f32);
        out_positions.push(p[1] as f32);
        out_positions.push(p[2] as f32);
    }

    let mut out = MeshChunk {
        metadata: mesh.metadata.clone(),
        positions: out_positions,
        indices,
        normals: None,
        texcoords: None,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    out.metadata.bump_version();
    out.refresh_metadata();

    Ok(QemResult {
        triangles_after: out.indices.len() / 3,
        mesh: out,
        max_error,
        triangles_before,
    })
}

fn rebuild_quadrics(positions: &[[f64; 3]], indices: &[u32], quadrics: &mut [Quadric]) {
    for q in quadrics.iter_mut() {
        *q = Quadric::default();
    }
    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = positions[i0];
        let p1 = positions[i1];
        let p2 = positions[i2];
        let Some(plane) = triangle_plane(p0, p1, p2) else {
            continue;
        };
        let q = Quadric::from_plane(plane[0], plane[1], plane[2], plane[3]);
        quadrics[i0].add(&q);
        quadrics[i1].add(&q);
        quadrics[i2].add(&q);
    }
}

fn triangle_plane(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 4]> {
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

fn build_collapse_heap(
    positions: &[[f64; 3]],
    indices: &[u32],
    quadrics: &[Quadric],
    deleted: &[bool],
) -> BinaryHeap<CollapseCandidate> {
    let mut edges = HashSet::new();
    for tri in indices.chunks_exact(3) {
        edges.insert(Edge::new(tri[0], tri[1]));
        edges.insert(Edge::new(tri[1], tri[2]));
        edges.insert(Edge::new(tri[2], tri[0]));
    }

    let mut heap = BinaryHeap::new();
    for edge in edges {
        let Edge(a, b) = edge;
        if deleted[a as usize] || deleted[b as usize] {
            continue;
        }
        if let Some(candidate) = collapse_cost(positions, quadrics, edge) {
            heap.push(candidate);
        }
    }
    heap
}

fn collapse_cost(
    positions: &[[f64; 3]],
    quadrics: &[Quadric],
    edge: Edge,
) -> Option<CollapseCandidate> {
    let Edge(a, b) = edge;
    let mut q = quadrics[a as usize];
    q.add(&quadrics[b as usize]);

    let pos = q
        .optimal_position()
        .unwrap_or(midpoint(positions[a as usize], positions[b as usize]));

    let cost = q.evaluate(pos[0], pos[1], pos[2]).max(0.0);
    Some(CollapseCandidate {
        cost,
        edge,
        position: pos,
    })
}

fn midpoint(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

fn edge_exists(indices: &[u32], edge: Edge, deleted: &[bool]) -> bool {
    let Edge(a, b) = edge;
    if deleted[a as usize] || deleted[b as usize] {
        return false;
    }
    indices.chunks_exact(3).any(|tri| {
        let set = [tri[0], tri[1], tri[2]];
        set.contains(&a) && set.contains(&b)
    })
}

fn remove_degenerate_triangles(indices: &mut Vec<u32>, positions: &[[f64; 3]]) {
    let mut out = Vec::with_capacity(indices.len());
    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 == i1 || i1 == i2 || i0 == i2 {
            continue;
        }
        if triangle_area(positions[i0], positions[i1], positions[i2]) > QEM_EPS {
            out.extend_from_slice(tri);
        }
    }
    *indices = out;
}

fn triangle_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
}

fn compact_mesh(positions: &mut Vec<[f64; 3]>, indices: &mut [u32], deleted: &[bool]) {
    let mut remap = vec![u32::MAX; deleted.len()];
    let mut compact = Vec::new();
    for (i, &is_deleted) in deleted.iter().enumerate() {
        if !is_deleted {
            remap[i] = compact.len() as u32;
            compact.push(positions[i]);
        }
    }
    *positions = compact;
    for idx in indices.iter_mut() {
        *idx = remap[*idx as usize];
    }
}

/// Generate a dense grid mesh for benchmarks (~2 * n² triangles).
pub fn grid_mesh(n: usize) -> MeshChunk {
    let mut positions = Vec::with_capacity(n * n * 3);
    for y in 0..n {
        for x in 0..n {
            positions.push(x as f32);
            positions.push(y as f32);
            positions.push((x as f32 * 0.1).sin() + (y as f32 * 0.1).cos());
        }
    }

    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let i0 = (y * n + x) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + n as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let mut mesh = MeshChunk {
        metadata: ChunkMeta::new("grid"),
        positions,
        indices,
        normals: None,
        texcoords: None,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    mesh.refresh_metadata();
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qem_reduces_triangle_count() {
        let mesh = grid_mesh(64);
        let before = mesh.indices.len() / 3;
        let result = simplify_mesh_qem(&mesh, before / 10).unwrap();
        assert!(result.triangles_after <= before / 10 + 2);
        assert!(result.max_error >= 0.0);
    }

    #[test]
    fn test_qem_preserves_non_empty_mesh() {
        let mesh = grid_mesh(16);
        let result = simplify_mesh_qem(&mesh, 50).unwrap();
        assert!(result.triangles_after >= 50);
        assert!(!result.mesh.positions.is_empty());
    }
}
