//! Integration tests for Spatial IR + GLB ingest (Wave 2).
//!
//! Run with: `cargo test --features mesh-ingest spatial_ir`

use wasm_spatial_core::{
    parse_glb_core, Aabb, ChunkMeta, MeshChunk, PointCloudChunk, SpatialChunk,
};

fn sample_triangle_mesh() -> MeshChunk {
    let mut mesh = MeshChunk {
        metadata: ChunkMeta::new("test"),
        positions: vec![
            0.0, 0.0, 0.0, //
            2.0, 0.0, 0.0, //
            1.0, 2.0, 0.0, //
            10.0, 10.0, 0.0, //
            12.0, 10.0, 0.0, //
            11.0, 12.0, 0.0, //
        ],
        indices: vec![0, 1, 2, 3, 4, 5],
        normals: None,
        mode: MeshChunk::MODE_TRIANGLES,
    };
    mesh.refresh_metadata();
    mesh
}

#[test]
fn test_spatial_chunk_mesh_pipeline() {
    let mesh = sample_triangle_mesh();
    let chunk = SpatialChunk::Mesh(mesh);

    let region = Aabb {
        min: [-1.0, -1.0, -1.0],
        max: [3.0, 3.0, 1.0],
    };

    if let SpatialChunk::Mesh(m) = chunk {
        let selected = m.select_by_aabb(&region).unwrap();
        assert_eq!(selected.vertex_count(), 3);
        assert_eq!(selected.indices.len(), 3);

        let glb = selected.to_glb_bytes();
        let reparsed = parse_glb_core(&glb).unwrap();
        assert_eq!(reparsed.vertex_count(), 3);
        assert_eq!(reparsed.indices.len(), 3);
    } else {
        panic!("expected mesh chunk");
    }
}

#[test]
fn test_point_cloud_spatial_chunk() {
    let mut pc = PointCloudChunk {
        metadata: ChunkMeta::new("las"),
        positions: vec![0.0, 0.0, 0.0, 100.0, 100.0, 100.0],
        colors: None,
        normals: None,
    };
    pc.refresh_metadata();

    let chunk = SpatialChunk::PointCloud(pc);
    assert!(chunk.estimate_bytes() > 0);
    assert_eq!(chunk.metadata().source_format, Some("las".to_string()));
}
