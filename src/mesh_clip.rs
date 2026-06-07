//! Plane clip with attribute interpolation (Wave 5.3).

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::spatial_ir::MeshChunk;

const PLANE_EPS: f32 = 1e-5;

/// Half-space clip plane: keep vertices where `dot(normal, p) >= distance`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipPlane {
    pub normal: [f32; 3],
    pub distance: f32,
}

impl ClipPlane {
    pub fn new(normal: [f32; 3], distance: f32) -> Result<Self, SpatialErrorDetail> {
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len < 1e-12 {
            return Err(SpatialError::InvalidInput.with_detail("clip plane normal is zero"));
        }
        Ok(Self {
            normal: [normal[0] / len, normal[1] / len, normal[2] / len],
            distance,
        })
    }

    pub fn signed_distance(&self, x: f32, y: f32, z: f32) -> f32 {
        self.normal[0] * x + self.normal[1] * y + self.normal[2] * z - self.distance
    }

    pub fn on_plane(&self, x: f32, y: f32, z: f32) -> bool {
        self.signed_distance(x, y, z).abs() <= PLANE_EPS
    }
}

#[derive(Debug, Clone)]
struct VertexAttr {
    position: [f32; 3],
    normal: Option<[f32; 3]>,
    texcoord: Option<[f32; 2]>,
}

struct ClipBuilder {
    positions: Vec<f32>,
    normals: Option<Vec<f32>>,
    texcoords: Option<Vec<f32>>,
    indices: Vec<u32>,
    has_normals: bool,
    has_texcoords: bool,
    weld_map: std::collections::HashMap<(i64, i64, i64), u32>,
}

impl ClipBuilder {
    fn from_mesh(mesh: &MeshChunk) -> Self {
        Self {
            positions: mesh.positions.clone(),
            normals: mesh.normals.clone(),
            texcoords: mesh.texcoords.clone(),
            indices: Vec::new(),
            has_normals: mesh.normals.is_some(),
            has_texcoords: mesh.texcoords.is_some(),
            weld_map: std::collections::HashMap::new(),
        }
    }

    fn vertex_attr(&self, idx: u32) -> VertexAttr {
        let base = idx as usize * 3;
        let position = [
            self.positions[base],
            self.positions[base + 1],
            self.positions[base + 2],
        ];
        let normal = self
            .normals
            .as_ref()
            .map(|n| [n[base], n[base + 1], n[base + 2]]);
        let texcoord = self.texcoords.as_ref().map(|t| {
            let tb = idx as usize * 2;
            [t[tb], t[tb + 1]]
        });
        VertexAttr {
            position,
            normal,
            texcoord,
        }
    }

    fn vertex_key(v: &VertexAttr) -> (i64, i64, i64) {
        let scale = 1_000_000.0f32;
        (
            (v.position[0] * scale).round() as i64,
            (v.position[1] * scale).round() as i64,
            (v.position[2] * scale).round() as i64,
        )
    }

    fn add_vertex(&mut self, v: &VertexAttr) -> u32 {
        if let Some(&idx) = self.weld_map.get(&Self::vertex_key(v)) {
            return idx;
        }
        let idx = (self.positions.len() / 3) as u32;
        self.positions.extend_from_slice(&v.position);
        if self.has_normals {
            let n = v.normal.unwrap_or([0.0, 1.0, 0.0]);
            self.normals
                .get_or_insert_with(Vec::new)
                .extend_from_slice(&n);
        }
        if self.has_texcoords {
            let t = v.texcoord.unwrap_or([0.0, 0.0]);
            self.texcoords
                .get_or_insert_with(Vec::new)
                .extend_from_slice(&t);
        }
        self.weld_map.insert(Self::vertex_key(v), idx);
        idx
    }

    fn add_triangle(&mut self, a: &VertexAttr, b: &VertexAttr, c: &VertexAttr) {
        let i0 = self.add_vertex(a);
        let i1 = self.add_vertex(b);
        let i2 = self.add_vertex(c);
        self.indices.extend_from_slice(&[i0, i1, i2]);
    }

    fn into_mesh(self, source: &MeshChunk) -> MeshChunk {
        let mut chunk = MeshChunk {
            metadata: source.metadata.clone(),
            positions: self.positions,
            indices: self.indices,
            normals: self.normals,
            texcoords: self.texcoords,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        chunk.metadata.bump_version();
        chunk.refresh_metadata();
        chunk
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn lerp_vertex(v0: &VertexAttr, v1: &VertexAttr, t: f32) -> VertexAttr {
    VertexAttr {
        position: [
            lerp_f32(v0.position[0], v1.position[0], t),
            lerp_f32(v0.position[1], v1.position[1], t),
            lerp_f32(v0.position[2], v1.position[2], t),
        ],
        normal: match (v0.normal, v1.normal) {
            (Some(a), Some(b)) => Some([
                lerp_f32(a[0], b[0], t),
                lerp_f32(a[1], b[1], t),
                lerp_f32(a[2], b[2], t),
            ]),
            _ => None,
        },
        texcoord: match (v0.texcoord, v1.texcoord) {
            (Some(a), Some(b)) => Some([lerp_f32(a[0], b[0], t), lerp_f32(a[1], b[1], t)]),
            _ => None,
        },
    }
}

fn clip_polygon_to_plane(vertices: &[VertexAttr], plane: &ClipPlane) -> Vec<VertexAttr> {
    if vertices.is_empty() {
        return Vec::new();
    }

    let n = vertices.len();
    let mut output = Vec::new();

    for i in 0..n {
        let curr = &vertices[i];
        let prev = &vertices[(i + n - 1) % n];
        let d_curr = plane.signed_distance(curr.position[0], curr.position[1], curr.position[2]);
        let d_prev = plane.signed_distance(prev.position[0], prev.position[1], prev.position[2]);
        let curr_in = d_curr >= -PLANE_EPS;
        let prev_in = d_prev >= -PLANE_EPS;

        if curr_in {
            if !prev_in {
                let t = d_prev / (d_prev - d_curr);
                output.push(lerp_vertex(prev, curr, t));
            }
            output.push(curr.clone());
        } else if prev_in {
            let t = d_prev / (d_prev - d_curr);
            output.push(lerp_vertex(prev, curr, t));
        }
    }

    output
}

fn fan_triangulate(vertices: &[VertexAttr], builder: &mut ClipBuilder) {
    if vertices.len() < 3 {
        return;
    }
    for i in 1..vertices.len() - 1 {
        builder.add_triangle(&vertices[0], &vertices[i], &vertices[i + 1]);
    }
}

fn clip_triangle(
    plane: &ClipPlane,
    source: &ClipBuilder,
    output: &mut ClipBuilder,
    i0: u32,
    i1: u32,
    i2: u32,
) {
    let tri = [
        source.vertex_attr(i0),
        source.vertex_attr(i1),
        source.vertex_attr(i2),
    ];
    let clipped = clip_polygon_to_plane(&tri, plane);
    fan_triangulate(&clipped, output);
}

/// Clip a triangle mesh to the positive half-space of `plane`.
pub fn clip_mesh_by_plane(
    mesh: &MeshChunk,
    plane: &ClipPlane,
) -> Result<MeshChunk, SpatialErrorDetail> {
    if mesh.mode != MeshChunk::MODE_TRIANGLES || mesh.indices.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail("clip requires indexed triangle mesh"));
    }

    let mut builder = ClipBuilder::from_mesh(mesh);
    builder.positions.clear();
    builder.normals = if builder.has_normals {
        Some(Vec::new())
    } else {
        None
    };
    builder.texcoords = if builder.has_texcoords {
        Some(Vec::new())
    } else {
        None
    };

    let source = ClipBuilder::from_mesh(mesh);
    for tri in mesh.indices.chunks_exact(3) {
        clip_triangle(plane, &source, &mut builder, tri[0], tri[1], tri[2]);
    }

    if builder.indices.is_empty() {
        return Err(SpatialError::GeometryError.with_detail("plane clip removed entire mesh"));
    }

    Ok(builder.into_mesh(mesh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial_ir::ChunkMeta;

    fn triangle_with_uv() -> MeshChunk {
        let mut mesh = MeshChunk {
            metadata: ChunkMeta::new("tri"),
            positions: vec![
                0.0, 0.0, 0.0, //
                2.0, 0.0, 1.0, //
                0.0, 2.0, 1.0, //
            ],
            indices: vec![0, 1, 2],
            normals: Some(vec![
                0.0, 0.0, 1.0, //
                0.0, 0.0, 1.0, //
                0.0, 0.0, 1.0, //
            ]),
            texcoords: Some(vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0]),
            mode: MeshChunk::MODE_TRIANGLES,
        };
        mesh.refresh_metadata();
        mesh
    }

    #[test]
    fn test_single_triangle_clip_creates_plane_vertices() {
        let mesh = triangle_with_uv();
        let plane = ClipPlane::new([0.0, 0.0, 1.0], 0.5).unwrap();
        let clipped = clip_mesh_by_plane(&mesh, &plane).unwrap();

        assert!(clipped.vertex_count() > 3);
        assert_eq!(clipped.indices.len() % 3, 0);

        for chunk in clipped.positions.chunks_exact(3) {
            assert!(plane.signed_distance(chunk[0], chunk[1], chunk[2]) >= -PLANE_EPS);
        }
    }

    #[test]
    fn test_clip_interpolates_uv_and_normal() {
        let mesh = triangle_with_uv();
        let plane = ClipPlane::new([0.0, 0.0, 1.0], 0.5).unwrap();
        let clipped = clip_mesh_by_plane(&mesh, &plane).unwrap();

        let tex = clipped.texcoords.as_ref().unwrap();
        let norms = clipped.normals.as_ref().unwrap();
        assert_eq!(tex.len(), clipped.vertex_count() * 2);
        assert_eq!(norms.len(), clipped.vertex_count() * 3);

        for chunk in tex.chunks_exact(2) {
            assert!((0.0..=1.0).contains(&chunk[0]));
            assert!((0.0..=1.0).contains(&chunk[1]));
        }
    }

    #[test]
    fn test_clip_fully_outside_returns_error() {
        let mesh = triangle_with_uv();
        let plane = ClipPlane::new([0.0, 0.0, 1.0], 2.0).unwrap();
        let err = clip_mesh_by_plane(&mesh, &plane).unwrap_err();
        assert_eq!(err.code(), SpatialError::GeometryError.code());
    }
}
