//! Garland–Heckbert QEM mesh decimation (Wave 5.5).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::mesh_qem_math::{
    accumulate_vertex_quadrics, edge_collapse_cost_quadrics, Quadric, QEM_EPS,
};
use crate::spatial_ir::{ChunkMeta, MeshChunk};

const UV_SEAM_EPS: f32 = 1e-4;
/// Rebuild the collapse heap after this many consecutive stale pops.
const QEM_STALE_POP_REBUILD: usize = 256;

/// QEM simplification options (Wave 5.6 seam preservation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QemOptions {
    /// When true and the mesh has UVs, refuse collapses across UV seams.
    pub preserve_uv_seams: bool,
}

impl Default for QemOptions {
    fn default() -> Self {
        Self {
            preserve_uv_seams: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Edge(u32, u32);

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
    simplify_mesh_qem_with_options(mesh, target_triangles, &QemOptions::default())
}

/// Decimate with explicit QEM options (UV seam preservation, etc.).
pub fn simplify_mesh_qem_with_options(
    mesh: &MeshChunk,
    target_triangles: usize,
    options: &QemOptions,
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
    let mut texcoords: Option<Vec<[f32; 2]>> = mesh
        .texcoords
        .as_ref()
        .map(|t| t.chunks_exact(2).map(|c| [c[0], c[1]]).collect::<Vec<_>>());
    let mut indices = mesh.indices.clone();
    let mut quadrics = vec![Quadric::default(); positions.len()];
    let mut deleted = vec![false; positions.len()];
    let mut max_error = 0.0f64;

    let seam_edges = if options.preserve_uv_seams {
        let mut seams = texcoords
            .as_ref()
            .map(|uv| detect_uv_seam_edges(&indices, uv))
            .unwrap_or_default();
        if let Some(uv) = texcoords.as_ref() {
            seams.extend(find_coincident_uv_seam_edges(&positions, uv));
        }
        seams
    } else {
        HashSet::new()
    };

    rebuild_quadrics(&positions, &indices, &mut quadrics);

    let mut heap: Option<BinaryHeap<CollapseCandidate>> = None;
    let mut stale_pops = 0usize;

    loop {
        let tri_count = indices.len() / 3;
        if tri_count <= target_triangles {
            break;
        }

        let need_rebuild =
            heap.as_ref().is_none_or(|h| h.is_empty()) || stale_pops >= QEM_STALE_POP_REBUILD;
        if need_rebuild {
            heap = Some(build_collapse_heap(
                &positions,
                texcoords.as_deref(),
                &indices,
                &quadrics,
                &deleted,
                &seam_edges,
                options.preserve_uv_seams,
            ));
            stale_pops = 0;
        }

        let heap_mut = heap.as_mut().expect("heap initialized before pop");
        let Some(candidate) = heap_mut.pop() else {
            break;
        };

        if !edge_exists(&indices, candidate.edge, &deleted) {
            stale_pops += 1;
            continue;
        }

        let Edge(a, b) = candidate.edge;
        if deleted[a as usize] || deleted[b as usize] {
            stale_pops += 1;
            continue;
        }
        if is_uv_seam_collapse(
            candidate.edge,
            &positions,
            texcoords.as_deref(),
            &seam_edges,
            options.preserve_uv_seams,
        ) {
            stale_pops += 1;
            continue;
        }

        // Recompute cost — stale heap entries must not drive collapses.
        let Some(fresh) = collapse_cost(&positions, &quadrics, candidate.edge) else {
            stale_pops += 1;
            continue;
        };
        if (fresh.cost - candidate.cost).abs() > QEM_EPS {
            heap_mut.push(fresh);
            stale_pops += 1;
            continue;
        }

        stale_pops = 0;
        max_error = max_error.max(fresh.cost.sqrt());

        positions[a as usize] = fresh.position;
        let b_q = quadrics[b as usize];
        quadrics[a as usize].add(&b_q);
        deleted[b as usize] = true;

        for idx in indices.iter_mut() {
            if *idx == b {
                *idx = a;
            }
        }

        remove_degenerate_triangles(&mut indices, &positions);

        for edge in incident_edges(a, &indices) {
            push_edge_cost_if_valid(
                heap_mut,
                edge,
                &positions,
                texcoords.as_deref(),
                &quadrics,
                &deleted,
                &seam_edges,
                options.preserve_uv_seams,
            );
        }

        if indices.len() / 3 <= target_triangles {
            break;
        }
    }

    compact_mesh(&mut positions, &mut indices, &deleted);
    if let Some(uv) = texcoords.as_mut() {
        compact_texcoords(uv, &deleted);
    }
    remove_degenerate_triangles(&mut indices, &positions);

    let mut out_positions = Vec::with_capacity(positions.len() * 3);
    for p in &positions {
        out_positions.push(p[0] as f32);
        out_positions.push(p[1] as f32);
        out_positions.push(p[2] as f32);
    }

    let out_texcoords = texcoords.map(|uv| {
        let mut flat = Vec::with_capacity(uv.len() * 2);
        for t in uv {
            flat.push(t[0]);
            flat.push(t[1]);
        }
        flat
    });

    let mut out = MeshChunk {
        metadata: mesh.metadata.clone(),
        positions: out_positions,
        indices,
        normals: None,
        texcoords: out_texcoords,
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
    let accumulated = accumulate_vertex_quadrics(positions, indices);
    for (q, m) in quadrics.iter_mut().zip(accumulated.iter()) {
        q.m = *m;
    }
}

fn texcoord_at(texcoords: &[[f32; 2]], idx: u32) -> [f32; 2] {
    texcoords[idx as usize]
}

fn uv_equal(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() <= UV_SEAM_EPS && (a[1] - b[1]).abs() <= UV_SEAM_EPS
}

/// Edges where adjacent triangles disagree on endpoint UVs, or endpoint UVs differ.
pub(crate) fn detect_uv_seam_edges(indices: &[u32], texcoords: &[[f32; 2]]) -> HashSet<Edge> {
    let mut recorded: std::collections::HashMap<Edge, ([f32; 2], [f32; 2])> =
        std::collections::HashMap::new();
    let mut seams = HashSet::new();

    for tri in indices.chunks_exact(3) {
        for &(va, vb) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let edge = Edge::new(va, vb);
            let (low, high) = if va < vb { (va, vb) } else { (vb, va) };
            let uv_pair = (texcoord_at(texcoords, low), texcoord_at(texcoords, high));

            match recorded.get(&edge) {
                Some(prev) if !uv_equal(prev.0, uv_pair.0) || !uv_equal(prev.1, uv_pair.1) => {
                    seams.insert(edge);
                }
                None => {
                    recorded.insert(edge, uv_pair);
                }
                _ => {}
            }
        }
    }

    seams
}

/// Forbid collapses between geometrically coincident vertices with different UVs.
fn find_coincident_uv_seam_edges(positions: &[[f64; 3]], texcoords: &[[f32; 2]]) -> HashSet<Edge> {
    use std::collections::HashMap;

    let inv = 1.0 / POS_SEAM_EPS;
    let mut buckets: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
    for (i, pos) in positions.iter().enumerate() {
        let key = (
            (pos[0] * inv).round() as i64,
            (pos[1] * inv).round() as i64,
            (pos[2] * inv).round() as i64,
        );
        buckets.entry(key).or_default().push(i as u32);
    }

    let mut seams = HashSet::new();
    for members in buckets.values() {
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let (vi, vj) = (members[i], members[j]);
                if !uv_equal(texcoords[vi as usize], texcoords[vj as usize]) {
                    seams.insert(Edge::new(vi, vj));
                }
            }
        }
    }
    seams
}

const POS_SEAM_EPS: f64 = 1e-5;

fn positions_coincident(positions: &[[f64; 3]], a: u32, b: u32) -> bool {
    let pa = positions[a as usize];
    let pb = positions[b as usize];
    let dx = pa[0] - pb[0];
    let dy = pa[1] - pb[1];
    let dz = pa[2] - pb[2];
    dx * dx + dy * dy + dz * dz <= POS_SEAM_EPS * POS_SEAM_EPS
}

fn is_uv_seam_collapse(
    edge: Edge,
    positions: &[[f64; 3]],
    texcoords: Option<&[[f32; 2]]>,
    seam_edges: &HashSet<Edge>,
    preserve: bool,
) -> bool {
    if !preserve {
        return false;
    }
    if seam_edges.contains(&edge) {
        return true;
    }
    let Some(uv) = texcoords else {
        return false;
    };
    let Edge(a, b) = edge;
    positions_coincident(positions, a, b) && !uv_equal(uv[a as usize], uv[b as usize])
}

fn build_collapse_heap(
    positions: &[[f64; 3]],
    texcoords: Option<&[[f32; 2]]>,
    indices: &[u32],
    quadrics: &[Quadric],
    deleted: &[bool],
    seam_edges: &HashSet<Edge>,
    preserve_uv_seams: bool,
) -> BinaryHeap<CollapseCandidate> {
    let mut edges = HashSet::new();
    for tri in indices.chunks_exact(3) {
        edges.insert(Edge::new(tri[0], tri[1]));
        edges.insert(Edge::new(tri[1], tri[2]));
        edges.insert(Edge::new(tri[2], tri[0]));
    }

    let mut heap = BinaryHeap::new();
    for edge in edges {
        push_edge_cost_if_valid(
            &mut heap,
            edge,
            positions,
            texcoords,
            quadrics,
            deleted,
            seam_edges,
            preserve_uv_seams,
        );
    }
    heap
}

#[allow(clippy::too_many_arguments)]
fn push_edge_cost_if_valid(
    heap: &mut BinaryHeap<CollapseCandidate>,
    edge: Edge,
    positions: &[[f64; 3]],
    texcoords: Option<&[[f32; 2]]>,
    quadrics: &[Quadric],
    deleted: &[bool],
    seam_edges: &HashSet<Edge>,
    preserve_uv_seams: bool,
) {
    let Edge(a, b) = edge;
    if deleted[a as usize] || deleted[b as usize] {
        return;
    }
    if is_uv_seam_collapse(edge, positions, texcoords, seam_edges, preserve_uv_seams) {
        return;
    }
    if let Some(candidate) = collapse_cost(positions, quadrics, edge) {
        heap.push(candidate);
    }
}

fn compact_texcoords(texcoords: &mut Vec<[f32; 2]>, deleted: &[bool]) {
    let mut remap = vec![u32::MAX; deleted.len()];
    let mut compact = Vec::new();
    for (i, &is_deleted) in deleted.iter().enumerate() {
        if !is_deleted {
            remap[i] = compact.len() as u32;
            compact.push(texcoords[i]);
        }
    }
    *texcoords = compact;
}

fn collapse_cost(
    positions: &[[f64; 3]],
    quadrics: &[Quadric],
    edge: Edge,
) -> Option<CollapseCandidate> {
    let Edge(a, b) = edge;
    let (cost, pos) = edge_collapse_cost_quadrics(positions, quadrics, a, b);
    Some(CollapseCandidate {
        cost,
        edge,
        position: pos,
    })
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

/// Edges incident on `vertex` in the current triangle list.
fn incident_edges(vertex: u32, indices: &[u32]) -> HashSet<Edge> {
    let mut edges = HashSet::new();
    for tri in indices.chunks_exact(3) {
        if tri.contains(&vertex) {
            for &(va, vb) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                if va == vertex || vb == vertex {
                    edges.insert(Edge::new(va, vb));
                }
            }
        }
    }
    edges
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
fn textured_seam_fixture() -> MeshChunk {
    let mut mesh = MeshChunk {
        metadata: ChunkMeta::new("seam"),
        positions: vec![
            0.0, 0.0, 0.0, //
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            1.0, 0.0, 0.0, //
            2.0, 0.0, 0.0, //
            1.0, 1.0, 0.0, //
        ],
        indices: vec![0, 1, 2, 3, 4, 5],
        normals: None,
        texcoords: Some(vec![
            0.0, 0.0, //
            0.49, 0.0, // left seam side
            0.0, 1.0, //
            0.51, 0.0, // right seam side (same position as v1, different UV)
            1.0, 0.0, //
            0.5, 1.0, //
        ]),
        mode: MeshChunk::MODE_TRIANGLES,
    };
    mesh.refresh_metadata();
    mesh
}

#[cfg(test)]
fn count_coincident_uv_pairs(mesh: &MeshChunk) -> usize {
    let Some(texcoords) = mesh.texcoords.as_ref() else {
        return 0;
    };
    let n = mesh.vertex_count();
    let mut count = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            let bi = i * 3;
            let bj = j * 3;
            let pi = [
                mesh.positions[bi] as f64,
                mesh.positions[bi + 1] as f64,
                mesh.positions[bi + 2] as f64,
            ];
            let pj = [
                mesh.positions[bj] as f64,
                mesh.positions[bj + 1] as f64,
                mesh.positions[bj + 2] as f64,
            ];
            let dx = pi[0] - pj[0];
            let dy = pi[1] - pj[1];
            let dz = pi[2] - pj[2];
            if dx * dx + dy * dy + dz * dz > POS_SEAM_EPS * POS_SEAM_EPS {
                continue;
            }
            let ui = [texcoords[i * 2], texcoords[i * 2 + 1]];
            let uj = [texcoords[j * 2], texcoords[j * 2 + 1]];
            if !uv_equal(ui, uj) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_seam_collapse_blocked_on_coincident_vertices() {
        let mesh = textured_seam_fixture();
        let positions: Vec<[f64; 3]> = mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64])
            .collect();
        let uv: Vec<[f32; 2]> = mesh
            .texcoords
            .as_ref()
            .unwrap()
            .chunks_exact(2)
            .map(|c| [c[0], c[1]])
            .collect();
        let seams = detect_uv_seam_edges(&mesh.indices, &uv);
        assert!(is_uv_seam_collapse(
            Edge::new(1, 3),
            &positions,
            Some(&uv),
            &seams,
            true,
        ));
    }

    #[test]
    fn test_qem_preserves_uv_seam_vertices() {
        let mesh = textured_seam_fixture();
        let pairs_before = count_coincident_uv_pairs(&mesh);
        assert!(pairs_before > 0);

        let preserved = simplify_mesh_qem_with_options(
            &mesh,
            2,
            &QemOptions {
                preserve_uv_seams: true,
            },
        )
        .unwrap();
        assert_eq!(count_coincident_uv_pairs(&preserved.mesh), pairs_before);
        assert_eq!(preserved.mesh.vertex_count(), mesh.vertex_count());
    }

    #[test]
    fn test_coincident_uv_seam_edges_detected() {
        let mesh = textured_seam_fixture();
        let positions: Vec<[f64; 3]> = mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64])
            .collect();
        let uv: Vec<[f32; 2]> = mesh
            .texcoords
            .as_ref()
            .unwrap()
            .chunks_exact(2)
            .map(|c| [c[0], c[1]])
            .collect();
        let seams = find_coincident_uv_seam_edges(&positions, &uv);
        assert!(seams.contains(&Edge::new(1, 3)));
    }

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
