//! End-to-end validation with a 500K-point synthetic LAS file.
//!
//! Run: cargo test --test large_file_e2e --features point-cloud,test-helpers -- --nocapture

use std::time::Instant;
use wasm_spatial_core::test_exports::test_helpers::get_positions;
use wasm_spatial_core::{
    generate_tileset, parse_las_header_core, parse_las_points_core, parse_pnts_header, Octree,
};

fn load_large_las() -> Vec<u8> {
    let path = std::path::Path::new("test-data/large/synthetic_500k.las");
    if !path.exists() {
        panic!("Run `cargo run --example gen_test_las` first");
    }
    std::fs::read(path).unwrap()
}

#[test]
fn test_500k_header_parse() {
    let blob = load_large_las();
    let start = Instant::now();
    let header = parse_las_header_core(&blob).expect("header parse");
    let elapsed = start.elapsed();

    assert_eq!(header.num_points(), 500_000);
    assert!((header.version_major(), header.version_minor()) >= (1, 2));
    eprintln!("✅ Header parsed in {:?}", elapsed);
    eprintln!(
        "   Points: {}, Version: {}.{}",
        header.num_points(),
        header.version_major(),
        header.version_minor()
    );
}

#[test]
fn test_500k_point_parse() {
    let blob = load_large_las();

    let start = Instant::now();
    let cloud = parse_las_points_core(&blob).expect("point parse");
    let elapsed = start.elapsed();

    assert_eq!(cloud.point_count(), 500_000);
    eprintln!("✅ 500K points parsed in {:?}", elapsed);

    let positions = get_positions(&cloud);
    let n = positions.len() / 3;
    for i in 0..n {
        assert!(
            positions[i * 3].is_finite()
                && positions[i * 3 + 1].is_finite()
                && positions[i * 3 + 2].is_finite(),
            "point {} has non-finite coords",
            i
        );
    }
    eprintln!("   All {} positions finite ✓", n);
}

#[test]
fn test_500k_octree() {
    let blob = load_large_las();
    let cloud = parse_las_points_core(&blob).unwrap();
    let mut positions = get_positions(&cloud).to_vec();
    let point_count = positions.len() / 3;

    let start = Instant::now();
    let max_pts = std::cmp::max(point_count / 16, 1000);
    let octree = Octree::build(&mut positions, max_pts as u32, 10);
    let elapsed = start.elapsed();

    eprintln!("✅ Octree built in {:?}", elapsed);
    eprintln!(
        "   Nodes: {}, Depth: {}, Leaves: {}, Points: {}",
        octree.node_count(),
        octree.depth(),
        octree.leaf_count(),
        octree.total_points()
    );
    assert!(octree.node_count() > 1);
    assert_eq!(octree.total_points(), point_count as u32);
}

#[test]
fn test_500k_tileset() {
    let blob = load_large_las();
    let cloud = parse_las_points_core(&blob).unwrap();
    let mut positions = get_positions(&cloud).to_vec();
    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 16, 1000);

    let octree = Octree::build(&mut positions, max_pts as u32, 10);

    let start = Instant::now();
    let result = generate_tileset(&octree, &positions, None).expect("tileset generation failed");
    let elapsed = start.elapsed();

    eprintln!("✅ Tileset generated in {:?}", elapsed);
    eprintln!(
        "   {} tiles, {:.1} KB total",
        result.tile_count(),
        result.total_bytes() as f64 / 1024.0
    );

    // Validate all tiles have pnts magic and valid headers
    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile missing");
        assert_eq!(&tile_data[0..4], b"pnts", "tile {} missing pnts magic", i);
        let (hdr, _) = parse_pnts_header(tile_data).expect("pnts header parse failed");
        assert!(hdr.byte_length > 0);
    }
    assert!(result.tile_count() > 0);

    // Validate tileset.json
    let json_val: serde_json::Value =
        serde_json::from_str(result.tileset_json()).expect("tileset.json not valid JSON");
    assert!(json_val["asset"].is_object());
    assert!(json_val["root"].is_object());
}

#[test]
fn test_500k_full_pipeline() {
    let blob = load_large_las();
    let t0 = Instant::now();

    let cloud = parse_las_points_core(&blob).unwrap();
    let mut positions = get_positions(&cloud).to_vec();
    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 16, 1000);

    let octree = Octree::build(&mut positions, max_pts as u32, 10);
    let result = generate_tileset(&octree, &positions, None).unwrap();

    let total = t0.elapsed();
    eprintln!("✅ Full pipeline (500K points): {:?}", total);
    eprintln!(
        "   {} points → {} tiles → {:.1} MB",
        point_count,
        result.tile_count(),
        result.total_bytes() as f64 / 1_048_576.0
    );
}
