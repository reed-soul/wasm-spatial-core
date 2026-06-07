//! Cap planar boundary loops after clip (Wave 5.4).

use std::collections::HashMap;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::mesh_clip::ClipPlane;
use crate::spatial_ir::MeshChunk;

/// Add cap triangles for open boundary loops lying on `plane`.
pub fn cap_mesh_holes(
    mesh: &MeshChunk,
    plane: &ClipPlane,
) -> Result<MeshChunk, SpatialErrorDetail> {
    let loops = find_plane_boundary_loops(mesh, plane)?;
    if loops.is_empty() {
        return Ok(mesh.clone());
    }

    let (u_axis, v_axis) = plane_basis(plane.normal);
    let positions = mesh.positions.clone();
    let mut normals = mesh.normals.clone();
    let texcoords = mesh.texcoords.clone();
    let mut indices = mesh.indices.clone();

    for loop_indices in loops {
        let cap_tris = triangulate_loop(&loop_indices, &positions, plane, u_axis, v_axis)?;
        indices.extend_from_slice(&cap_tris);

        for &vi in &loop_indices {
            if normals.is_none() {
                normals = Some(vec![0.0; positions.len()]);
            }
            let n = normals.as_mut().unwrap();
            if n.len() < positions.len() {
                n.resize(positions.len(), 0.0);
            }
            let base = vi as usize * 3;
            n[base] = plane.normal[0];
            n[base + 1] = plane.normal[1];
            n[base + 2] = plane.normal[2];
        }
    }

    let mut chunk = MeshChunk {
        metadata: mesh.metadata.clone(),
        positions,
        indices,
        normals,
        texcoords,
        mode: mesh.mode,
    };
    chunk.metadata.bump_version();
    chunk.refresh_metadata();
    Ok(chunk)
}

/// Clip by plane then cap the open boundary (W5.3 + W5.4).
pub fn clip_and_cap_mesh(
    mesh: &MeshChunk,
    plane: &ClipPlane,
) -> Result<MeshChunk, SpatialErrorDetail> {
    let clipped = crate::mesh_clip::clip_mesh_by_plane(mesh, plane)?;
    cap_mesh_holes(&clipped, plane)
}

fn plane_basis(normal: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let n = normal;
    let ref_axis = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = cross(n, ref_axis);
    let u_len = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let u = [u[0] / u_len, u[1] / u_len, u[2] / u_len];
    let v = cross(n, u);
    (u, v)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn find_plane_boundary_loops(
    mesh: &MeshChunk,
    plane: &ClipPlane,
) -> Result<Vec<Vec<u32>>, SpatialErrorDetail> {
    let mut edge_use: HashMap<(u32, u32), u32> = HashMap::new();

    for tri in mesh.indices.chunks_exact(3) {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            *edge_use.entry(edge_key(a, b)).or_insert(0) += 1;
        }
    }

    let mut boundary: HashMap<u32, Vec<u32>> = HashMap::new();
    for ((a, b), count) in edge_use {
        if count != 1 {
            continue;
        }
        if !vertex_on_plane(mesh, plane, a) || !vertex_on_plane(mesh, plane, b) {
            continue;
        }
        boundary.entry(a).or_default().push(b);
        boundary.entry(b).or_default().push(a);
    }

    let mut visited = std::collections::HashSet::new();
    let mut loops = Vec::new();

    for &start in boundary.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut loop_indices = Vec::new();
        let mut current = start;
        let mut prev = start;
        loop {
            visited.insert(current);
            loop_indices.push(current);
            let neighbors = boundary.get(&current).cloned().unwrap_or_default();
            let next = neighbors
                .iter()
                .find(|&&n| n != prev)
                .copied()
                .or_else(|| neighbors.first().copied());
            let Some(next) = next else {
                break;
            };
            if next == start && loop_indices.len() > 2 {
                break;
            }
            prev = current;
            current = next;
            if visited.contains(&current) && current != start {
                break;
            }
        }
        if loop_indices.len() >= 3 {
            loops.push(loop_indices);
        }
    }

    Ok(loops)
}

fn vertex_on_plane(mesh: &MeshChunk, plane: &ClipPlane, idx: u32) -> bool {
    let base = idx as usize * 3;
    plane.on_plane(
        mesh.positions[base],
        mesh.positions[base + 1],
        mesh.positions[base + 2],
    )
}

fn triangulate_loop(
    loop_indices: &[u32],
    positions: &[f32],
    _plane: &ClipPlane,
    u_axis: [f32; 3],
    v_axis: [f32; 3],
) -> Result<Vec<u32>, SpatialErrorDetail> {
    let mut flat = Vec::with_capacity(loop_indices.len() * 2);
    for &idx in loop_indices {
        let base = idx as usize * 3;
        let p = [positions[base], positions[base + 1], positions[base + 2]];
        let rel = [
            p[0] * u_axis[0] + p[1] * u_axis[1] + p[2] * u_axis[2],
            p[0] * v_axis[0] + p[1] * v_axis[1] + p[2] * v_axis[2],
        ];
        flat.push(rel[0]);
        flat.push(rel[1]);
    }

    let triangulated = earcutr::earcut(&flat, &[], 2)
        .map_err(|_| SpatialError::GeometryError.with_detail("earcut failed"))?;

    if triangulated.len() < 3 {
        return Err(SpatialError::GeometryError.with_detail("cap triangulation empty"));
    }

    let mut out = Vec::with_capacity(triangulated.len());
    for idx in triangulated {
        out.push(loop_indices[idx]);
    }
    Ok(out)
}

/// Euler characteristic V - E + F for a triangle mesh (closed manifold check helper).
pub fn euler_characteristic(mesh: &MeshChunk) -> i32 {
    let v = mesh.vertex_count() as i32;
    let f = (mesh.indices.len() / 3) as i32;
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    for tri in mesh.indices.chunks_exact(3) {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            *edges.entry(edge_key(a, b)).or_insert(0) += 1;
        }
    }
    let e = edges.len() as i32;
    v - e + f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_clip::clip_mesh_by_plane;
    use crate::spatial_ir::ChunkMeta;

    fn unit_cube_mesh() -> MeshChunk {
        let positions = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 2, 6, 7, 2, 7, 3, 0, 3, 7, 0, 7,
            4, 1, 5, 6, 1, 6, 2,
        ];
        let mut mesh = MeshChunk {
            metadata: ChunkMeta::new("cube"),
            positions,
            indices,
            normals: None,
            texcoords: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        mesh.refresh_metadata();
        mesh
    }

    #[test]
    fn test_box_clip_and_cap_is_watertight() {
        let mesh = unit_cube_mesh();
        let plane = ClipPlane::new([0.0, 0.0, 1.0], 0.5).unwrap();
        let capped = clip_and_cap_mesh(&mesh, &plane).unwrap();

        assert!(capped.indices.len() > mesh.indices.len() / 2);
        assert_eq!(euler_characteristic(&capped), 2);
    }

    #[test]
    fn test_cap_adds_triangles_on_plane() {
        let mesh = unit_cube_mesh();
        let plane = ClipPlane::new([0.0, 0.0, 1.0], 0.5).unwrap();
        let clipped = clip_mesh_by_plane(&mesh, &plane).unwrap();
        let capped = cap_mesh_holes(&clipped, &plane).unwrap();
        assert!(capped.indices.len() > clipped.indices.len());
    }
}
