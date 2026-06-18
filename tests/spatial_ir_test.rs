//! Integration tests for Spatial IR + GLB ingest (Wave 2).
//!
//! Run with: `cargo test --features mesh-ingest spatial_ir`

use wasm_spatial_core::{
    batch_enu_to_wgs84_core, batch_wgs84_to_enu_core, compute_svd_alignment_core, parse_glb_core,
    Aabb, ChunkMeta, EnuFrame, MeshChunk, PointCloudChunk, SpatialChunk,
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
        texcoords: None,
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

#[test]
fn test_point_cloud_export_to_pnts() {
    let mut pc = PointCloudChunk {
        metadata: ChunkMeta::new("las"),
        positions: vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0],
        colors: None,
        normals: None,
    };
    pc.refresh_metadata();

    let export = pc.export_to_pnts("subset.pnts").unwrap();
    assert_eq!(&export.pnts[0..4], b"pnts");
    assert!(export.tileset_json.contains("subset.pnts"));
}

#[test]
fn test_enu_roundtrip_1km() {
    let frame = EnuFrame::from_anchor(116.391, 39.907, 50.0);
    let enu: [f64; 6] = [1000.0, 0.0, 0.0, 0.0, 1000.0, 0.0];
    let wgs = batch_enu_to_wgs84_core(&enu, &frame);
    let back = batch_wgs84_to_enu_core(&wgs, &frame);
    for i in 0..2 {
        let dx = (enu[i * 3] - back[i * 3]).abs();
        let dy = (enu[i * 3 + 1] - back[i * 3 + 1]).abs();
        let dz = (enu[i * 3 + 2] - back[i * 3 + 2]).abs();
        let err = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(err < 1e-3, "ENU round-trip error {err} m");
    }
}

#[test]
fn test_svd_alignment_photogrammetry_to_enu() {
    // Simulated workflow: photo-local control points → surveyed WGS84 → ENU targets.
    let frame = EnuFrame::from_anchor(116.391, 39.907, 50.0);

    let photo_local = [
        0.0, 0.0, 0.0, //
        100.0, 0.0, 0.0, //
        0.0, 100.0, 0.0, //
        0.0, 0.0, 50.0, //
    ];

    // Ground-truth similarity: 1.02 scale, small Z rotation, translation in ENU.
    let theta = 0.05_f64;
    let (c, s) = (theta.cos(), theta.sin());
    let scale = 1.02;
    let t = [12.0, -8.0, 3.0];

    let mut survey_wgs = Vec::with_capacity(12);
    for chunk in photo_local.chunks_exact(3) {
        let lx = chunk[0];
        let ly = chunk[1];
        let lz = chunk[2];
        let ex = scale * (c * lx - s * ly) + t[0];
        let ey = scale * (s * lx + c * ly) + t[1];
        let ez = scale * lz + t[2];
        let wgs = frame.enu_to_wgs84(ex, ey, ez);
        survey_wgs.extend_from_slice(&wgs);
    }

    let target_enu = batch_wgs84_to_enu_core(&survey_wgs, &frame);
    let result = compute_svd_alignment_core(&photo_local, &target_enu, true).unwrap();

    assert!((result.transform.scale - scale).abs() < 1e-6);
    assert!(result.rms_error < 1e-6);
}
