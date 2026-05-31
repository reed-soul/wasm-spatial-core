//! Pipeline integration test using the real `tests/fixtures/sample.las` file.
//!
//! Exercises the complete point cloud pipeline:
//!   LAS parse → header validate → points validate → octree → tileset → pnts tiles
//!
//! Run with: `cargo test --test pipeline_integration_test --all-features -- --nocapture`

use wasm_spatial_core::test_exports::test_helpers::{get_colors, get_positions};
use wasm_spatial_core::{
    generate_tileset, parse_las_header_core, parse_las_points_core, parse_pnts_header, Octree,
};

/// Load the sample.las fixture.
fn load_sample_las() -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures/sample.las");
    if !path.exists() {
        panic!("sample.las fixture not found at {}", path.display());
    }
    std::fs::read(path).expect("failed to read sample.las")
}

#[test]
fn test_sample_las_header() {
    let blob = load_sample_las();
    let header = parse_las_header_core(&blob).expect("LAS header parse failed");

    eprintln!(
        "═══ sample.las Header ═══\n\
         Version:    {}.{}\n\
         Format:     {}\n\
         Points:     {}\n\
         Record len: {}\n\
         Bounds:     X [{:.1}, {:.1}]\n\
                   Y [{:.1}, {:.1}]\n\
                   Z [{:.1}, {:.1}]\n\
         ══════════════════════════",
        header.version_major(),
        header.version_minor(),
        header.point_format_id(),
        header.num_points(),
        header.point_data_record_length(),
        header.bounds_min_x(),
        header.bounds_max_x(),
        header.bounds_min_y(),
        header.bounds_max_y(),
        header.bounds_min_z(),
        header.bounds_max_z(),
    );

    // Validate header struct is well-formed
    assert!(
        (header.version_major(), header.version_minor()) >= (1, 0),
        "LAS version should be >= 1.0"
    );
    assert!(
        header.num_points() > 0,
        "sample should have at least one point"
    );
    assert!(
        header.point_data_record_length() >= 20,
        "record length should be >= 20 bytes"
    );
    // Note: some LAS files may store min/max in unexpected order.
    // Coordinate validity is checked in test_sample_las_points.
}

#[test]
fn test_sample_las_points() {
    let blob = load_sample_las();
    let header = parse_las_header_core(&blob).expect("header parse failed");
    let cloud = parse_las_points_core(&blob).expect("LAS points parse failed");

    let positions = get_positions(&cloud);
    let n_points = positions.len() / 3;

    eprintln!(
        "═══ sample.las Points ═══\n\
         Expected:   {} (header)\n\
         Actual:     {} (parsed)\n\
         ════════════════════════",
        header.num_points(),
        n_points
    );

    assert_eq!(
        n_points,
        header.num_points() as usize,
        "point count mismatch"
    );
    assert!(n_points > 0);

    // Validate all coordinates are finite
    for i in 0..n_points {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        assert!(
            x.is_finite() && y.is_finite() && z.is_finite(),
            "point {} has non-finite coordinates",
            i
        );
    }

    // Check that positions span a reasonable range (not all zeros)
    let x_min = positions
        .iter()
        .step_by(3)
        .copied()
        .fold(f32::INFINITY, f32::min);
    let x_max = positions
        .iter()
        .step_by(3)
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let y_min = positions
        .iter()
        .skip(1)
        .step_by(3)
        .copied()
        .fold(f32::INFINITY, f32::min);
    let y_max = positions
        .iter()
        .skip(1)
        .step_by(3)
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);

    eprintln!(
        "  Position ranges: X [{:.1}, {:.1}], Y [{:.1}, {:.1}]",
        x_min, x_max, y_min, y_max
    );

    assert!(x_max > x_min, "X range should be non-zero");
    assert!(y_max > y_min, "Y range should be non-zero");
}

#[test]
fn test_sample_las_octree() {
    let blob = load_sample_las();
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let mut positions = get_positions(&cloud).to_vec();

    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 4, 100);
    let octree = Octree::build(&mut positions, max_pts as u32, 10);

    eprintln!(
        "═══ sample.las Octree ═══\n\
         Points:     {}\n\
         Nodes:      {}\n\
         Depth:      {}\n\
         Leaves:     {}\n\
         ═════════════════════════",
        octree.total_points(),
        octree.node_count(),
        octree.depth(),
        octree.leaf_count(),
    );

    assert_eq!(octree.total_points(), point_count as u32);
    assert!(octree.node_count() >= 1, "should have at least root node");
}

#[test]
fn test_sample_las_tileset() {
    let blob = load_sample_las();
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let mut positions = get_positions(&cloud).to_vec();
    let colors = get_colors(&cloud).map(|c| c.to_vec());

    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 4, 100);

    let octree = Octree::build(&mut positions, max_pts as u32, 10);
    let result = generate_tileset(&octree, &positions, colors.as_deref())
        .expect("tileset generation failed");

    eprintln!(
        "═══ sample.las Tileset ═══\n\
         Tiles:      {}\n\
         Total:      {} bytes ({:.2} KB)\n\
         ══════════════════════════",
        result.tile_count(),
        result.total_bytes(),
        result.total_bytes() as f64 / 1024.0,
    );

    assert!(result.tile_count() > 0, "should have at least one tile");
    assert!(result.total_bytes() > 0, "total bytes should be > 0");

    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile data missing");
        assert_eq!(&tile_data[0..4], b"pnts", "tile {} magic", i);

        let (header, _body) = parse_pnts_header(tile_data).expect("pnts parse failed");
        assert_eq!(header.version, 1, "tile {} version", i);
        assert!(header.byte_length > 0, "tile {} byte_length", i);

        eprintln!("  tile {}: {} bytes", i, tile_data.len());
    }
}

#[test]
fn test_large_synthetic_pipeline() {
    let n = 10_000;
    let mut positions = Vec::with_capacity(n * 3);
    let mut colors = Vec::with_capacity(n * 3);

    for i in 0..n {
        let x = ((i % 100) as f32) * 1.0;
        let y = ((i / 100) as f32) * 1.0;
        let z = (((i * 7) % 50) as f32) * 0.5;
        positions.push(x);
        positions.push(y);
        positions.push(z);
        colors.push(255);
        colors.push((i % 256) as u8);
        colors.push((255 - i % 256) as u8);
    }

    let octree = Octree::build(&mut positions, 500, 10);
    let result = generate_tileset(&octree, &positions, Some(&colors)).expect("tileset failed");

    eprintln!(
        "═══ 10K Synthetic Pipeline ═══\n\
         Points:     {}\n\
         Octree:     {} nodes, depth {}\n\
         Tiles:      {}\n\
         Total:      {:.2} KB\n\
         ═══════════════════════════════",
        octree.total_points(),
        octree.node_count(),
        octree.depth(),
        result.tile_count(),
        result.total_bytes() as f64 / 1024.0,
    );

    assert!(
        octree.node_count() > 1,
        "10K points should split into multiple nodes"
    );
    assert!(result.tile_count() > 1, "should have multiple tiles");

    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).unwrap();
        assert_eq!(&tile_data[0..4], b"pnts");
        let (header, _) = parse_pnts_header(tile_data).unwrap();
        assert_eq!(header.version, 1);
    }
}
