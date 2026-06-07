//! Mesh geometry edit — OBB classification and split (Wave 5.1–5.2).

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::spatial_ir::MeshChunk;

const OBB_HALF: f32 = 0.5;
const OBB_EPS: f32 = 1e-5;

/// Triangle index triples classified relative to an oriented bounding box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObbTriangleClassification {
    pub inside: Vec<[u32; 3]>,
    pub outside: Vec<[u32; 3]>,
}

/// Whether `mesh-edit` is enabled in this build.
#[wasm_bindgen(js_name = "supportsMeshEdit")]
pub fn supports_mesh_edit() -> bool {
    true
}

/// Classify triangles as inside/outside an OBB (phase 1 — boundary triangles appear in both sets).
///
/// `obb` is a column-major 4×4 matrix mapping the unit cube `[-0.5, 0.5]³` to world space.
pub fn classify_triangles_by_obb(
    positions: &[f32],
    indices: &[u32],
    obb: &[f32; 16],
) -> Result<ObbTriangleClassification, SpatialErrorDetail> {
    let inv = invert_mat4(obb)
        .ok_or_else(|| SpatialError::GeometryError.with_detail("OBB matrix is not invertible"))?;

    let mut inside = Vec::new();
    let mut outside = Vec::new();

    for tri in indices.chunks_exact(3) {
        let i0 = tri[0];
        let i1 = tri[1];
        let i2 = tri[2];

        let v0 = vertex_in_obb(positions, i0, &inv);
        let v1 = vertex_in_obb(positions, i1, &inv);
        let v2 = vertex_in_obb(positions, i2, &inv);

        let all_inside = v0 && v1 && v2;
        let all_outside = !v0 && !v1 && !v2;

        if all_inside {
            inside.push([i0, i1, i2]);
        } else if all_outside {
            outside.push([i0, i1, i2]);
        } else {
            inside.push([i0, i1, i2]);
            outside.push([i0, i1, i2]);
        }
    }

    Ok(ObbTriangleClassification { inside, outside })
}

/// Split a mesh into inside/outside submeshes relative to an OBB (no plane interpolation).
pub fn split_mesh_by_obb(
    mesh: &MeshChunk,
    obb: &[f32; 16],
) -> Result<(MeshChunk, MeshChunk), SpatialErrorDetail> {
    if mesh.mode != MeshChunk::MODE_TRIANGLES || mesh.indices.is_empty() {
        return Err(SpatialError::InvalidInput.with_detail("split requires indexed triangle mesh"));
    }

    let classified = classify_triangles_by_obb(&mesh.positions, &mesh.indices, obb)?;

    if classified.inside.is_empty() || classified.outside.is_empty() {
        return Err(SpatialError::GeometryError.with_detail("OBB split produced an empty submesh"));
    }

    Ok((
        mesh.build_subset(&classified.inside),
        mesh.build_subset(&classified.outside),
    ))
}

fn vertex_in_obb(positions: &[f32], idx: u32, inv_obb: &[f32; 16]) -> bool {
    let base = idx as usize * 3;
    if base + 2 >= positions.len() {
        return false;
    }

    let (lx, ly, lz) = transform_point(
        inv_obb,
        positions[base],
        positions[base + 1],
        positions[base + 2],
    );
    point_in_unit_cube(lx, ly, lz)
}

fn point_in_unit_cube(x: f32, y: f32, z: f32) -> bool {
    let range = (-OBB_HALF - OBB_EPS)..=(OBB_HALF + OBB_EPS);
    range.contains(&x) && range.contains(&y) && range.contains(&z)
}

fn transform_point(m: &[f32; 16], x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let ox = m[0] * x + m[4] * y + m[8] * z + m[12];
    let oy = m[1] * x + m[5] * y + m[9] * z + m[13];
    let oz = m[2] * x + m[6] * y + m[10] * z + m[14];
    (ox, oy, oz)
}

/// Invert a column-major 4×4 matrix (affine or general).
fn invert_mat4(m: &[f32; 16]) -> Option<[f32; 16]> {
    let mut inv = [0.0f32; 16];

    inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
        + m[9] * m[7] * m[14]
        + m[13] * m[6] * m[11]
        - m[13] * m[7] * m[10];
    inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
        - m[8] * m[7] * m[14]
        - m[12] * m[6] * m[11]
        + m[12] * m[7] * m[10];
    inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
        + m[8] * m[7] * m[13]
        + m[12] * m[5] * m[11]
        - m[12] * m[7] * m[9];
    inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
        - m[8] * m[6] * m[13]
        - m[12] * m[5] * m[10]
        + m[12] * m[6] * m[9];
    inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
        - m[9] * m[3] * m[14]
        - m[13] * m[2] * m[11]
        + m[13] * m[3] * m[10];
    inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
        + m[8] * m[3] * m[14]
        + m[12] * m[2] * m[11]
        - m[12] * m[3] * m[10];
    inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
        - m[8] * m[3] * m[13]
        - m[12] * m[1] * m[11]
        + m[12] * m[3] * m[9];
    inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
        + m[8] * m[2] * m[13]
        + m[12] * m[1] * m[10]
        - m[12] * m[2] * m[9];
    inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
        + m[5] * m[3] * m[14]
        + m[13] * m[2] * m[7]
        - m[13] * m[3] * m[6];
    inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
        - m[4] * m[3] * m[14]
        - m[12] * m[2] * m[7]
        + m[12] * m[3] * m[6];
    inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
        + m[4] * m[3] * m[13]
        + m[12] * m[1] * m[7]
        - m[12] * m[3] * m[5];
    inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
        - m[4] * m[2] * m[13]
        - m[12] * m[1] * m[6]
        + m[12] * m[2] * m[5];
    inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
        - m[5] * m[3] * m[10]
        - m[9] * m[2] * m[7]
        + m[9] * m[3] * m[6];
    inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
        + m[4] * m[3] * m[10]
        + m[8] * m[2] * m[7]
        - m[8] * m[3] * m[6];
    inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
        - m[4] * m[3] * m[9]
        - m[8] * m[1] * m[7]
        + m[8] * m[3] * m[5];
    inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
        + m[4] * m[2] * m[9]
        + m[8] * m[1] * m[6]
        - m[8] * m[2] * m[5];

    let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
    if det.abs() < 1e-12 {
        return None;
    }

    let inv_det = 1.0 / det;
    for v in &mut inv {
        *v *= inv_det;
    }

    Some(inv)
}

/// WASM-visible mesh split result.
#[wasm_bindgen]
pub struct WasmMeshSplit {
    inside: MeshChunk,
    outside: MeshChunk,
}

#[wasm_bindgen]
impl WasmMeshSplit {
    #[wasm_bindgen(getter, js_name = "inside")]
    pub fn inside_chunk(&self) -> WasmMeshChunk {
        WasmMeshChunk::from_chunk(self.inside.clone())
    }

    #[wasm_bindgen(getter, js_name = "outside")]
    pub fn outside_chunk(&self) -> WasmMeshChunk {
        WasmMeshChunk::from_chunk(self.outside.clone())
    }

    #[wasm_bindgen(js_name = "insideGlb")]
    pub fn inside_glb(&self) -> js_sys::Uint8Array {
        let bytes = self.inside.to_glb_bytes();
        let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        arr.copy_from(&bytes);
        arr
    }

    #[wasm_bindgen(js_name = "outsideGlb")]
    pub fn outside_glb(&self) -> js_sys::Uint8Array {
        let bytes = self.outside.to_glb_bytes();
        let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        arr.copy_from(&bytes);
        arr
    }
}

use crate::spatial_ir::WasmMeshChunk;

/// Split a [`WasmMeshChunk`] by OBB and return inside/outside submeshes.
#[wasm_bindgen(js_name = "splitMeshByObb")]
pub fn split_mesh_by_obb_js(
    mesh: &WasmMeshChunk,
    obb: &js_sys::Float32Array,
) -> Result<WasmMeshSplit, JsValue> {
    if obb.length() < 16 {
        return Err(SpatialError::InvalidInput
            .with_detail("obb must be a column-major 4×4 matrix (16 floats)")
            .into());
    }

    let mut matrix = [0.0f32; 16];
    for i in 0..16u32 {
        matrix[i as usize] = obb.get_index(i);
    }

    let (inside, outside) = split_mesh_by_obb(mesh.inner(), &matrix).map_err(JsValue::from)?;

    Ok(WasmMeshSplit { inside, outside })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial_ir::ChunkMeta;

    fn unit_cube_obb_matrix() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.5, 0.5, 0.5, 1.0, //
        ]
    }

    fn unit_cube_mesh() -> MeshChunk {
        let positions = vec![
            0.0, 0.0, 0.0, //
            1.0, 0.0, 0.0, //
            1.0, 1.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0, //
            1.0, 0.0, 1.0, //
            1.0, 1.0, 1.0, //
            0.0, 1.0, 1.0, //
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3, // bottom
            4, 6, 5, 4, 7, 6, // top
            0, 4, 5, 0, 5, 1, // front
            2, 6, 7, 2, 7, 3, // back
            0, 3, 7, 0, 7, 4, // left
            1, 5, 6, 1, 6, 2, // right
        ];
        let mut mesh = MeshChunk {
            metadata: ChunkMeta::new("test"),
            positions,
            indices,
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        mesh.refresh_metadata();
        mesh
    }

    fn regular_tetrahedron_mesh() -> MeshChunk {
        let s = 1.0f32;
        let positions = vec![
            0.0,
            0.0,
            0.0, //
            s,
            0.0,
            0.0, //
            s / 2.0,
            s * 0.8660254,
            0.0, //
            s / 2.0,
            s * 0.2886751,
            s * 0.8164966, //
        ];
        let indices = vec![0, 1, 2, 0, 1, 3, 1, 2, 3, 0, 2, 3];
        let mut mesh = MeshChunk {
            metadata: ChunkMeta::new("tet"),
            positions,
            indices,
            normals: None,
            mode: MeshChunk::MODE_TRIANGLES,
        };
        mesh.refresh_metadata();
        mesh
    }

    fn obb_containing_tetrahedron() -> [f32; 16] {
        [
            1.2, 0.0, 0.0, 0.0, //
            0.0, 1.2, 0.0, 0.0, //
            0.0, 0.0, 1.2, 0.0, //
            0.5, 0.4, 0.4, 1.0, //
        ]
    }

    #[test]
    fn test_unit_cube_obb_classify() {
        let mesh = unit_cube_mesh();
        let obb = unit_cube_obb_matrix();
        let classified = classify_triangles_by_obb(&mesh.positions, &mesh.indices, &obb).unwrap();

        assert_eq!(classified.inside.len(), 12);
        assert!(classified.outside.is_empty());
    }

    #[test]
    fn test_tetrahedron_obb_classify_all_inside() {
        let mesh = regular_tetrahedron_mesh();
        let obb = obb_containing_tetrahedron();
        let classified = classify_triangles_by_obb(&mesh.positions, &mesh.indices, &obb).unwrap();

        assert_eq!(classified.inside.len(), 4);
        assert!(classified.outside.is_empty());
    }

    #[test]
    fn test_tetrahedron_obb_classify_all_outside() {
        let mesh = regular_tetrahedron_mesh();
        let obb = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            10.0, 10.0, 10.0, 1.0, //
        ];
        let classified = classify_triangles_by_obb(&mesh.positions, &mesh.indices, &obb).unwrap();

        assert!(classified.inside.is_empty());
        assert_eq!(classified.outside.len(), 4);
    }

    #[test]
    fn test_split_mesh_by_obb_triangle_count() {
        let mesh = unit_cube_mesh();
        let obb = [
            0.5, 0.0, 0.0, 0.0, //
            0.0, 0.5, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.25, 0.25, 0.5, 1.0, //
        ];

        let (inside, outside) = split_mesh_by_obb(&mesh, &obb).unwrap();
        let inside_tris = inside.indices.len() / 3;
        let outside_tris = outside.indices.len() / 3;
        let original_tris = mesh.indices.len() / 3;

        assert!(inside_tris > 0);
        assert!(outside_tris > 0);
        assert!(inside_tris + outside_tris >= original_tris);
    }

    #[test]
    fn test_split_exports_glb_bytes() {
        let mesh = unit_cube_mesh();
        let obb = [
            0.5, 0.0, 0.0, 0.0, //
            0.0, 0.5, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.25, 0.25, 0.5, 1.0, //
        ];
        let (inside, outside) = split_mesh_by_obb(&mesh, &obb).unwrap();

        let inside_glb = inside.to_glb_bytes();
        let outside_glb = outside.to_glb_bytes();
        assert!(inside_glb.len() > 12);
        assert!(outside_glb.len() > 12);
    }
}
