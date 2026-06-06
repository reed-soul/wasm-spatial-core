//! Mesh geometry editing — OBB split, plane clip, QEM decimation (Wave 5).

use std::collections::{BinaryHeap, HashMap};
use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::spatial_ir::{ChunkMeta, MeshChunk, WasmMeshChunk};

// ===========================================================================
// Math helpers
// ===========================================================================

type Mat4 = [f32; 16];
type Vec3 = [f32; 3];

#[inline]
fn mat4_mul_point(m: &Mat4, p: Vec3) -> Vec3 {
    let x = p[0] as f64;
    let y = p[1] as f64;
    let z = p[2] as f64;
    [
        (m[0] as f64 * x + m[4] as f64 * y + m[8] as f64 * z + m[12] as f64) as f32,
        (m[1] as f64 * x + m[5] as f64 * y + m[9] as f64 * z + m[13] as f64) as f32,
        (m[2] as f64 * x + m[6] as f64 * y + m[10] as f64 * z + m[14] as f64) as f32,
    ]
}

/// Invert a 4×4 column-major matrix (returns None if singular).
fn invert_mat4(m: &Mat4) -> Option<Mat4> {
    let a = m.map(|v| v as f64);
    let mut inv = [0.0f64; 16];

    inv[0] = a[5] * a[10] * a[15] - a[5] * a[11] * a[14] - a[9] * a[6] * a[15]
        + a[9] * a[7] * a[14]
        + a[13] * a[6] * a[11]
        - a[13] * a[7] * a[10];
    inv[4] = -a[4] * a[10] * a[15] + a[4] * a[11] * a[14] + a[8] * a[6] * a[15]
        - a[8] * a[7] * a[14]
        - a[12] * a[6] * a[11]
        + a[12] * a[7] * a[10];
    inv[8] = a[4] * a[9] * a[15] - a[4] * a[11] * a[13] - a[8] * a[5] * a[15]
        + a[8] * a[7] * a[13]
        + a[12] * a[5] * a[11]
        - a[12] * a[7] * a[9];
    inv[12] = -a[4] * a[9] * a[14] + a[4] * a[10] * a[13] + a[8] * a[5] * a[14]
        - a[8] * a[6] * a[13]
        - a[12] * a[5] * a[10]
        + a[12] * a[6] * a[9];
    inv[1] = -a[1] * a[10] * a[15] + a[1] * a[11] * a[14] + a[9] * a[2] * a[15]
        - a[9] * a[3] * a[14]
        - a[13] * a[2] * a[11]
        + a[13] * a[3] * a[10];
    inv[5] = a[0] * a[10] * a[15] - a[0] * a[11] * a[14] - a[8] * a[2] * a[15]
        + a[8] * a[3] * a[14]
        + a[12] * a[2] * a[11]
        - a[12] * a[3] * a[10];
    inv[9] = -a[0] * a[9] * a[15] + a[0] * a[11] * a[13] + a[8] * a[1] * a[15]
        - a[8] * a[3] * a[13]
        - a[12] * a[1] * a[11]
        + a[12] * a[3] * a[9];
    inv[13] = a[0] * a[9] * a[14] - a[0] * a[10] * a[13] - a[8] * a[1] * a[14]
        + a[8] * a[2] * a[13]
        + a[12] * a[1] * a[10]
        - a[12] * a[2] * a[9];
    inv[2] = a[1] * a[6] * a[15] - a[1] * a[7] * a[14] - a[5] * a[2] * a[15]
        + a[5] * a[3] * a[14]
        + a[13] * a[2] * a[7]
        - a[13] * a[3] * a[6];
    inv[6] = -a[0] * a[6] * a[15] + a[0] * a[7] * a[14] + a[4] * a[2] * a[15]
        - a[4] * a[3] * a[14]
        - a[12] * a[2] * a[7]
        + a[12] * a[3] * a[6];
    inv[10] = a[0] * a[5] * a[15] - a[0] * a[7] * a[13] - a[4] * a[1] * a[15]
        + a[4] * a[3] * a[13]
        + a[12] * a[1] * a[7]
        - a[12] * a[3] * a[5];
    inv[14] = -a[0] * a[5] * a[14] + a[0] * a[6] * a[13] + a[4] * a[1] * a[14]
        - a[4] * a[2] * a[13]
        - a[12] * a[1] * a[6]
        + a[12] * a[2] * a[5];
    inv[3] = -a[1] * a[6] * a[11] + a[1] * a[7] * a[10] + a[5] * a[2] * a[11]
        - a[5] * a[3] * a[10]
        - a[9] * a[2] * a[7]
        + a[9] * a[3] * a[6];
    inv[7] = a[0] * a[6] * a[11] - a[0] * a[7] * a[10] - a[4] * a[2] * a[11]
        + a[4] * a[3] * a[10]
        + a[8] * a[2] * a[7]
        - a[8] * a[3] * a[6];
    inv[11] = -a[0] * a[5] * a[11] + a[0] * a[7] * a[9] + a[4] * a[1] * a[11]
        - a[4] * a[3] * a[9]
        - a[8] * a[1] * a[7]
        + a[8] * a[3] * a[5];
    inv[15] = a[0] * a[5] * a[10] - a[0] * a[6] * a[9] - a[4] * a[1] * a[10]
        + a[4] * a[2] * a[9]
        + a[8] * a[1] * a[6]
        - a[8] * a[2] * a[5];

    let det = a[0] * inv[0] + a[1] * inv[4] + a[2] * inv[8] + a[3] * inv[12];
    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some(inv.map(|v| (v * inv_det) as f32))
}

fn point_inside_unit_box(p: Vec3) -> bool {
    p[0] >= -0.5 && p[0] <= 0.5 && p[1] >= -0.5 && p[1] <= 0.5 && p[2] >= -0.5 && p[2] <= 0.5
}

fn get_vertex(positions: &[f32], idx: u32) -> Vec3 {
    let b = idx as usize * 3;
    [positions[b], positions[b + 1], positions[b + 2]]
}

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ===========================================================================
// W5.1 — OBB triangle classification
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriClass {
    Inside,
    Outside,
    Straddle,
}

/// Classify each triangle relative to an OBB (unit box [-0.5,0.5]³ transformed by `obb`).
pub fn classify_triangles_obb(
    positions: &[f32],
    indices: &[u32],
    obb: &Mat4,
) -> Result<(Vec<u32>, Vec<u32>), SpatialErrorDetail> {
    if !indices.len().is_multiple_of(3) {
        return Err(SpatialError::InvalidInput.with_detail("indices length must be multiple of 3"));
    }
    let inv = invert_mat4(obb)
        .ok_or_else(|| SpatialError::GeometryError.with_detail("singular OBB matrix"))?;

    let mut inside = Vec::new();
    let mut outside = Vec::new();

    for tri in indices.chunks_exact(3) {
        let v0 = mat4_mul_point(&inv, get_vertex(positions, tri[0]));
        let v1 = mat4_mul_point(&inv, get_vertex(positions, tri[1]));
        let v2 = mat4_mul_point(&inv, get_vertex(positions, tri[2]));

        let i0 = point_inside_unit_box(v0);
        let i1 = point_inside_unit_box(v1);
        let i2 = point_inside_unit_box(v2);

        let class = if i0 && i1 && i2 {
            TriClass::Inside
        } else if !i0 && !i1 && !i2 {
            TriClass::Outside
        } else {
            TriClass::Straddle
        };

        match class {
            TriClass::Inside => inside.extend_from_slice(tri),
            TriClass::Outside => outside.extend_from_slice(tri),
            TriClass::Straddle => {
                inside.extend_from_slice(tri);
                outside.extend_from_slice(tri);
            }
        }
    }

    Ok((inside, outside))
}

// ===========================================================================
// W5.2 — Mesh split (phase 1)
// ===========================================================================

fn extract_submesh(
    mesh: &MeshChunk,
    tri_indices: &[u32],
    label: &str,
) -> Result<MeshChunk, SpatialErrorDetail> {
    if tri_indices.is_empty() {
        return Err(
            SpatialError::GeometryError.with_detail(format!("{label} submesh has no triangles"))
        );
    }

    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut positions = Vec::new();
    let mut normals = mesh.normals.as_ref().map(|_| Vec::new());
    let mut indices = Vec::with_capacity(tri_indices.len());

    for &old in tri_indices {
        let new = *map.entry(old).or_insert_with(|| {
            let ni = (positions.len() / 3) as u32;
            let b = old as usize * 3;
            positions.extend_from_slice(&mesh.positions[b..b + 3]);
            if let (Some(src), Some(dst)) = (mesh.normals.as_ref(), normals.as_mut()) {
                if src.len() >= b + 3 {
                    dst.extend_from_slice(&src[b..b + 3]);
                }
            }
            ni
        });
        indices.push(new);
    }

    let mut chunk = MeshChunk {
        metadata: ChunkMeta::new(label),
        positions,
        indices,
        normals,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    chunk.metadata.bump_version();
    chunk.refresh_metadata();
    Ok(chunk)
}

/// Split mesh into inside/outside submeshes relative to an OBB (phase 1 — boundary duplication OK).
pub fn split_mesh_obb(
    mesh: &MeshChunk,
    obb: &Mat4,
) -> Result<(MeshChunk, MeshChunk), SpatialErrorDetail> {
    if mesh.mode != MeshChunk::MODE_TRIANGLES {
        return Err(SpatialError::InvalidInput.with_detail("split requires triangle mesh"));
    }
    let (inside_idx, outside_idx) = classify_triangles_obb(&mesh.positions, &mesh.indices, obb)?;
    let inside = extract_submesh(mesh, &inside_idx, "inside")?;
    let outside = extract_submesh(mesh, &outside_idx, "outside")?;
    Ok((inside, outside))
}

// ===========================================================================
// W5.3 — Plane clip
// ===========================================================================

/// Clip mesh to the positive half-space of plane `[nx, ny, nz, d]` where `dot(p,n)+d >= 0`.
pub fn clip_mesh_plane(mesh: &MeshChunk, plane: [f32; 4]) -> Result<MeshChunk, SpatialErrorDetail> {
    if mesh.mode != MeshChunk::MODE_TRIANGLES {
        return Err(SpatialError::InvalidInput.with_detail("clip requires triangle mesh"));
    }

    let n = [plane[0], plane[1], plane[2]];
    let d = plane[3];

    let dist = |p: Vec3| p[0] * n[0] + p[1] * n[1] + p[2] * n[2] + d;

    let mut positions = mesh.positions.clone();
    let mut out_normals = mesh.normals.clone().unwrap_or_default();
    let src_normals = mesh.normals.as_deref();
    let has_normals = src_normals.is_some();
    let mut indices: Vec<u32> = Vec::new();

    let mut add_vertex = |p: Vec3, nrm: Option<Vec3>| -> u32 {
        let idx = (positions.len() / 3) as u32;
        positions.extend_from_slice(&p);
        if has_normals {
            let nn = nrm.unwrap_or([0.0, 1.0, 0.0]);
            out_normals.extend_from_slice(&nn);
        }
        idx
    };

    for tri in mesh.indices.chunks_exact(3) {
        let verts: [Vec3; 3] = [
            get_vertex(&mesh.positions, tri[0]),
            get_vertex(&mesh.positions, tri[1]),
            get_vertex(&mesh.positions, tri[2]),
        ];
        let nrms: [Vec3; 3] = if let Some(normals) = src_normals {
            [
                get_vertex(normals, tri[0]),
                get_vertex(normals, tri[1]),
                get_vertex(normals, tri[2]),
            ]
        } else {
            [[0.0; 3]; 3]
        };
        let ds = [dist(verts[0]), dist(verts[1]), dist(verts[2])];
        let inside: Vec<usize> = (0..3).filter(|&i| ds[i] >= 0.0).collect();
        let outside: Vec<usize> = (0..3).filter(|&i| ds[i] < 0.0).collect();

        match (inside.len(), outside.len()) {
            (3, 0) => indices.extend_from_slice(tri),
            (0, 3) => {}
            (2, 1) => {
                let (i0, i1, o) = (inside[0], inside[1], outside[0]);
                let t0 = ds[i0] / (ds[i0] - ds[o]);
                let t1 = ds[i1] / (ds[i1] - ds[o]);
                let p0 = lerp3(verts[i0], verts[o], t0);
                let p1 = lerp3(verts[i1], verts[o], t1);
                let n0 = lerp3(nrms[i0], nrms[o], t0);
                let n1 = lerp3(nrms[i1], nrms[o], t1);
                let a = tri[i0];
                let b = tri[i1];
                let c = add_vertex(p0, Some(n0));
                let d2 = add_vertex(p1, Some(n1));
                indices.extend_from_slice(&[a, b, c, b, d2, c]);
            }
            (1, 2) => {
                let i = inside[0];
                let (o0, o1) = (outside[0], outside[1]);
                let t0 = ds[i] / (ds[i] - ds[o0]);
                let t1 = ds[i] / (ds[i] - ds[o1]);
                let p0 = lerp3(verts[i], verts[o0], t0);
                let p1 = lerp3(verts[i], verts[o1], t1);
                let n0 = lerp3(nrms[i], nrms[o0], t0);
                let n1 = lerp3(nrms[i], nrms[o1], t1);
                let a = tri[i];
                let b = add_vertex(p0, Some(n0));
                let c = add_vertex(p1, Some(n1));
                indices.extend_from_slice(&[a, b, c]);
            }
            _ => {}
        }
    }

    if indices.is_empty() {
        return Err(SpatialError::GeometryError.with_detail("plane clip removed all geometry"));
    }

    let mut chunk = MeshChunk {
        metadata: mesh.metadata.clone(),
        positions,
        indices,
        normals: if has_normals { Some(out_normals) } else { None },
        mode: MeshChunk::MODE_TRIANGLES,
    };
    chunk.metadata.bump_version();
    chunk.refresh_metadata();
    Ok(chunk)
}

// ===========================================================================
// W5.5 — QEM edge-collapse decimation
// ===========================================================================

#[derive(Clone)]
struct Quadric {
    // Symmetric 4×4 matrix stored as 10 doubles: m00,m01,m02,m03,m11,m12,m13,m22,m23,m33
    data: [f64; 10],
}

fn quadric_sym_index(r: usize, c: usize) -> usize {
    let (r, c) = if r <= c { (r, c) } else { (c, r) };
    match (r, c) {
        (0, 0) => 0,
        (0, 1) => 1,
        (0, 2) => 2,
        (0, 3) => 3,
        (1, 1) => 4,
        (1, 2) => 5,
        (1, 3) => 6,
        (2, 2) => 7,
        (2, 3) => 8,
        (3, 3) => 9,
        _ => 0,
    }
}

impl Quadric {
    fn zero() -> Self {
        Self { data: [0.0; 10] }
    }

    fn from_plane(a: f64, b: f64, c: f64, d: f64) -> Self {
        let v = [a, b, c, d];
        let mut data = [0.0; 10];
        for r in 0..4 {
            for c in 0..4 {
                data[quadric_sym_index(r, c)] += v[r] * v[c];
            }
        }
        Self { data }
    }

    fn add(&mut self, other: &Quadric) {
        for i in 0..10 {
            self.data[i] += other.data[i];
        }
    }

    fn eval(&self, x: f64, y: f64, z: f64) -> f64 {
        let w = 1.0;
        let m = &self.data;
        x * x * m[0]
            + 2.0 * x * y * m[1]
            + 2.0 * x * z * m[2]
            + 2.0 * x * w * m[3]
            + y * y * m[4]
            + 2.0 * y * z * m[5]
            + 2.0 * y * w * m[6]
            + z * z * m[7]
            + 2.0 * z * w * m[8]
            + w * w * m[9]
    }
}

fn qem_edge_cost(positions: &[f32], quadrics: &[Quadric], v0: u32, v1: u32) -> u64 {
    let mut data = [0.0f64; 10];
    for (i, slot) in data.iter_mut().enumerate() {
        *slot = quadrics[v0 as usize].data[i] + quadrics[v1 as usize].data[i];
    }
    let q = Quadric { data };
    let p0 = get_vertex(positions, v0);
    let p1 = get_vertex(positions, v1);
    let mx = (p0[0] + p1[0]) as f64 * 0.5;
    let my = (p0[1] + p1[1]) as f64 * 0.5;
    let mz = (p0[2] + p1[2]) as f64 * 0.5;
    let cost = q.eval(mx, my, mz);
    if cost.is_finite() && cost >= 0.0 {
        (cost * 1_000_000.0) as u64
    } else {
        u64::MAX
    }
}

fn face_plane(p0: Vec3, p1: Vec3, p2: Vec3) -> (f64, f64, f64, f64) {
    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-12 {
        return (0.0, 0.0, 1.0, 0.0);
    }
    let a = (nx / len) as f64;
    let b = (ny / len) as f64;
    let c = (nz / len) as f64;
    let d = -(a * p0[0] as f64 + b * p0[1] as f64 + c * p0[2] as f64);
    (a, b, c, d)
}

/// Garland–Heckbert QEM simplification to approximately `target_triangle_count` triangles.
pub fn simplify_mesh_qem(
    positions: &[f32],
    indices: &[u32],
    target_triangle_count: u32,
) -> Result<(Vec<f32>, Vec<u32>), SpatialErrorDetail> {
    if indices.len() < 3 || !indices.len().is_multiple_of(3) {
        return Err(SpatialError::InvalidInput.with_detail("mesh must have triangles"));
    }
    let target = target_triangle_count.max(1) as usize * 3;

    let mut positions: Vec<f32> = positions.to_vec();
    let mut indices: Vec<u32> = indices.to_vec();
    let vertex_count = positions.len() / 3;

    let mut quadrics = vec![Quadric::zero(); vertex_count];
    for tri in indices.chunks_exact(3) {
        let p0 = get_vertex(&positions, tri[0]);
        let p1 = get_vertex(&positions, tri[1]);
        let p2 = get_vertex(&positions, tri[2]);
        let (a, b, c, d) = face_plane(p0, p1, p2);
        let q = Quadric::from_plane(a, b, c, d);
        quadrics[tri[0] as usize].add(&q);
        quadrics[tri[1] as usize].add(&q);
        quadrics[tri[2] as usize].add(&q);
    }

    let mut valid = vec![true; vertex_count];
    let mut edge_heap: BinaryHeap<(u64, u32, u32)> = BinaryHeap::new();

    for tri in indices.chunks_exact(3) {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            edge_heap.push((qem_edge_cost(&positions, &quadrics, lo, hi), lo, hi));
        }
    }

    while indices.len() > target {
        let Some((_, v0, v1)) = edge_heap.pop() else {
            break;
        };
        if !valid[v0 as usize] || !valid[v1 as usize] {
            continue;
        }

        let p0 = get_vertex(&positions, v0);
        let p1 = get_vertex(&positions, v1);
        let mid = [
            (p0[0] + p1[0]) * 0.5,
            (p0[1] + p1[1]) * 0.5,
            (p0[2] + p1[2]) * 0.5,
        ];
        let b0 = v0 as usize * 3;
        positions[b0] = mid[0];
        positions[b0 + 1] = mid[1];
        positions[b0 + 2] = mid[2];
        let q1 = quadrics[v1 as usize].clone();
        quadrics[v0 as usize].add(&q1);
        valid[v1 as usize] = false;

        for idx in indices.iter_mut() {
            if *idx == v1 {
                *idx = v0;
            }
        }

        // Remove degenerate triangles
        indices = indices
            .chunks_exact(3)
            .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
            .flat_map(|t| t.iter().copied())
            .collect();

        if indices.len() <= target {
            break;
        }

        for tri in indices.chunks_exact(3) {
            if tri.contains(&v0) {
                for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                    if valid[a as usize] && valid[b as usize] {
                        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                        edge_heap.push((qem_edge_cost(&positions, &quadrics, lo, hi), lo, hi));
                    }
                }
            }
        }
    }

    // Compact vertices
    let mut remap = vec![u32::MAX; vertex_count];
    let mut compact_pos = Vec::new();
    for (i, &v) in valid.iter().enumerate() {
        if v {
            remap[i] = (compact_pos.len() / 3) as u32;
            let b = i * 3;
            compact_pos.extend_from_slice(&positions[b..b + 3]);
        }
    }
    let compact_idx: Vec<u32> = indices
        .iter()
        .filter_map(|&i| {
            let r = remap[i as usize];
            if r != u32::MAX {
                Some(r)
            } else {
                None
            }
        })
        .collect();

    Ok((compact_pos, compact_idx))
}

impl MeshChunk {
    /// QEM decimation to target triangle count.
    pub fn simplify_qem(
        &self,
        target_triangle_count: u32,
    ) -> Result<MeshChunk, SpatialErrorDetail> {
        let (positions, indices) =
            simplify_mesh_qem(&self.positions, &self.indices, target_triangle_count)?;
        let mut chunk = MeshChunk {
            metadata: self.metadata.clone(),
            positions,
            indices,
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        Ok(chunk)
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Result of OBB mesh split.
#[wasm_bindgen]
pub struct WasmMeshSplitResult {
    inside: MeshChunk,
    outside: MeshChunk,
}

#[wasm_bindgen]
impl WasmMeshSplitResult {
    #[wasm_bindgen(getter)]
    pub fn inside(&self) -> WasmMeshChunk {
        WasmMeshChunk::from_chunk(self.inside.clone())
    }

    #[wasm_bindgen(getter)]
    pub fn outside(&self) -> WasmMeshChunk {
        WasmMeshChunk::from_chunk(self.outside.clone())
    }
}

/// Split mesh by OBB (column-major Mat4, unit box [-0.5,0.5]³).
#[wasm_bindgen(js_name = "splitMeshObb")]
pub fn split_mesh_obb_js(
    mesh: &WasmMeshChunk,
    obb: &js_sys::Float32Array,
) -> Result<WasmMeshSplitResult, JsValue> {
    if obb.length() < 16 {
        return Err(SpatialError::InvalidInput
            .with_detail("obb must be 16 floats (column-major Mat4)")
            .into());
    }
    let mut m = [0.0f32; 16];
    obb.copy_to(&mut m);
    let (inside, outside) = split_mesh_obb(mesh.inner(), &m)?;
    Ok(WasmMeshSplitResult { inside, outside })
}

/// Clip mesh to positive half-space of plane `[nx, ny, nz, d]`.
#[wasm_bindgen(js_name = "clipMeshPlane")]
pub fn clip_mesh_plane_js(
    mesh: &WasmMeshChunk,
    plane: &js_sys::Float32Array,
) -> Result<WasmMeshChunk, JsValue> {
    if plane.length() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("plane must be [nx, ny, nz, d]")
            .into());
    }
    let p = [
        plane.get_index(0),
        plane.get_index(1),
        plane.get_index(2),
        plane.get_index(3),
    ];
    clip_mesh_plane(mesh.inner(), p)
        .map(WasmMeshChunk::from_chunk)
        .map_err(Into::into)
}

/// QEM simplify mesh to target triangle count.
#[wasm_bindgen(js_name = "simplifyMeshQem")]
pub fn simplify_mesh_qem_js(
    mesh: &WasmMeshChunk,
    target_triangle_count: u32,
) -> Result<WasmMeshChunk, JsValue> {
    mesh.inner()
        .simplify_qem(target_triangle_count)
        .map(WasmMeshChunk::from_chunk)
        .map_err(Into::into)
}

/// Whether mesh edit (Wave 5) is available.
#[wasm_bindgen(js_name = "supportsMeshEdit")]
pub fn supports_mesh_edit() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_mesh() -> MeshChunk {
        let positions = vec![
            -0.5, -0.5, -0.5, //
            0.5, -0.5, -0.5, //
            0.5, 0.5, -0.5, //
            -0.5, 0.5, -0.5, //
            -0.5, -0.5, 0.5, //
            0.5, -0.5, 0.5, //
            0.5, 0.5, 0.5, //
            -0.5, 0.5, 0.5, //
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3, //
            4, 6, 5, 4, 7, 6, //
            0, 4, 5, 0, 5, 1, //
            2, 6, 7, 2, 7, 3, //
            0, 3, 7, 0, 7, 4, //
            1, 5, 6, 1, 6, 2, //
        ];
        let mut m = MeshChunk {
            metadata: ChunkMeta::new("cube"),
            positions,
            indices,
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        m.refresh_metadata();
        m
    }

    /// OBB shifted so roughly half the unit cube lies inside.
    fn split_obb() -> Mat4 {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[12] = 0.5;
        m[15] = 1.0;
        m
    }

    fn tetrahedron_mesh() -> MeshChunk {
        let positions = vec![
            0.0, 0.0, 0.0, //
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0, //
        ];
        let indices = vec![0, 1, 2, 0, 2, 3, 0, 3, 1, 1, 3, 2];
        let mut m = MeshChunk {
            metadata: ChunkMeta::new("tet"),
            positions,
            indices,
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        m.refresh_metadata();
        m
    }

    #[test]
    fn test_classify_unit_cube_obb() {
        let mesh = unit_cube_mesh();
        let obb = split_obb();
        let (inside, outside) =
            classify_triangles_obb(&mesh.positions, &mesh.indices, &obb).unwrap();
        assert!(!inside.is_empty());
        assert!(!outside.is_empty());
        assert!(inside.len() + outside.len() >= mesh.indices.len());
    }

    #[test]
    fn test_tetrahedron_classify() {
        let mesh = tetrahedron_mesh();
        let mut obb = split_obb();
        obb[12] = 0.25;
        obb[13] = 0.25;
        obb[14] = 0.25;
        let (inside, outside) =
            classify_triangles_obb(&mesh.positions, &mesh.indices, &obb).unwrap();
        assert!(!inside.is_empty());
        assert!(!outside.is_empty());
        assert!(inside.len() + outside.len() >= mesh.indices.len());
    }

    #[test]
    fn test_split_mesh_obb() {
        let mesh = unit_cube_mesh();
        let (inside, outside) = split_mesh_obb(&mesh, &split_obb()).unwrap();
        assert!(inside.indices.len() >= 3);
        assert!(outside.indices.len() >= 3);
    }

    #[test]
    fn test_clip_single_triangle() {
        let mesh = MeshChunk {
            metadata: ChunkMeta::new("tri"),
            positions: vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        let clipped = clip_mesh_plane(&mesh, [1.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(clipped.indices.len() >= 3);
        assert!(clipped.vertex_count() >= 3);
    }

    #[test]
    fn test_qem_reduces_triangle_count() {
        let mesh = unit_cube_mesh();
        let original = mesh.indices.len() / 3;
        let (pos, idx) = simplify_mesh_qem(&mesh.positions, &mesh.indices, 4).unwrap();
        assert!(idx.len() / 3 <= 4);
        assert!(!pos.is_empty());
        assert!(idx.len() / 3 < original);
    }
}
