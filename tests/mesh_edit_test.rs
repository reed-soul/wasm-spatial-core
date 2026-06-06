//! Integration tests for mesh geometry edit (Wave 5).

use wasm_spatial_core::{clip_mesh_plane, simplify_mesh_qem, split_mesh_obb, ChunkMeta, MeshChunk};

fn unit_cube() -> MeshChunk {
    let mut m = MeshChunk {
        metadata: ChunkMeta::new("cube"),
        positions: vec![
            -0.5, -0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5, -0.5, 0.5, -0.5, -0.5, -0.5, 0.5,
            0.5, -0.5, 0.5, 0.5, 0.5, 0.5, -0.5, 0.5, 0.5,
        ],
        indices: vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 2, 6, 7, 2, 7, 3, 0, 3, 7, 0, 7,
            4, 1, 5, 6, 1, 6, 2,
        ],
        normals: None,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    m.refresh_metadata();
    m
}

#[test]
fn test_split_export_glb() {
    let mesh = unit_cube();
    let mut obb = [0.0f32; 16];
    obb[0] = 1.0;
    obb[5] = 1.0;
    obb[10] = 1.0;
    obb[12] = 0.5;
    obb[15] = 1.0;

    let (inside, outside) = split_mesh_obb(&mesh, &obb).unwrap();
    let inside_glb = inside.to_glb_bytes();
    let outside_glb = outside.to_glb_bytes();
    assert_eq!(&inside_glb[0..4], b"glTF");
    assert_eq!(&outside_glb[0..4], b"glTF");
    assert!(inside.indices.len() + outside.indices.len() >= mesh.indices.len());
}

#[test]
fn test_qem_pipeline() {
    let mesh = unit_cube();
    let (pos, idx) = simplify_mesh_qem(&mesh.positions, &mesh.indices, 6).unwrap();
    assert!(idx.len() / 3 <= 6);
    let simplified = MeshChunk {
        metadata: ChunkMeta::new("simplified"),
        positions: pos,
        indices: idx,
        normals: None,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    let glb = simplified.to_glb_bytes();
    assert_eq!(&glb[0..4], b"glTF");
}

#[test]
fn test_plane_clip_pipeline() {
    let mesh = unit_cube();
    let clipped = clip_mesh_plane(&mesh, [1.0, 0.0, 0.0, 0.0]).unwrap();
    assert!(clipped.vertex_count() >= 3);
    assert!(!clipped.to_glb_bytes().is_empty());
}
