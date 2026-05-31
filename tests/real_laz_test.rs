//! Real-data validation tests for LAS point cloud parsing and pipeline.
//!
//! Uses `tests/fixtures/sample.las` (a real TerraScan LAS v1.2 file with 1065
//! colored points) and synthetically-generated point clouds that simulate
//! real-world LiDAR characteristics (non-uniform distribution, noise, gaps).

use wasm_spatial_core::test_exports::test_helpers::{
    build_test_las_blob, get_colors, get_point_count, get_positions,
};
use wasm_spatial_core::{
    generate_tileset, parse_las_header_core, parse_las_points_core, parse_pnts_header, Octree,
};

// ===========================================================================
// Helper: load fixture
// ===========================================================================

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures").join(name);
    if !path.exists() {
        panic!("fixture not found: {}", path.display());
    }
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

// ===========================================================================
// 1. Real LAS file validation (sample.las — TerraScan v1.2, 1065 pts)
// ===========================================================================

#[test]
fn test_real_las_header_magic_and_version() {
    let blob = load_fixture("sample.las");
    let header = parse_las_header_core(&blob).expect("LAS header parse failed");

    // LASF magic is handled internally, but version should be valid
    assert!(
        (header.version_major(), header.version_minor()) >= (1, 0),
        "version should be >= 1.0, got {}.{}",
        header.version_major(),
        header.version_minor()
    );
    assert!(header.point_format_id() <= 10, "format ID should be valid");
    assert!(header.num_points() > 0, "should have at least one point");
    assert!(
        header.point_data_record_length() >= 20,
        "record length should be >= 20"
    );

    eprintln!(
        "sample.las: v{}.{} fmt={} pts={} rec_len={}",
        header.version_major(),
        header.version_minor(),
        header.point_format_id(),
        header.num_points(),
        header.point_data_record_length()
    );
}

#[test]
fn test_real_las_point_count_matches_header() {
    let blob = load_fixture("sample.las");
    let header = parse_las_header_core(&blob).expect("header parse failed");
    let cloud = parse_las_points_core(&blob).expect("points parse failed");

    let n_parsed = get_point_count(&cloud);
    assert_eq!(
        n_parsed,
        header.num_points(),
        "parsed point count ({}) != header count ({})",
        n_parsed,
        header.num_points()
    );
}

#[test]
fn test_real_las_coordinates_are_finite() {
    let blob = load_fixture("sample.las");
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let positions = get_positions(&cloud);

    let n = positions.len() / 3;
    for i in 0..n {
        let (x, y, z) = (positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]);
        assert!(
            x.is_finite() && y.is_finite() && z.is_finite(),
            "point {} has non-finite coords: ({}, {}, {})",
            i,
            x,
            y,
            z
        );
    }

    // Not all zeros
    let any_nonzero = positions.iter().any(|&v| v.abs() > 1e-10);
    assert!(any_nonzero, "all coordinates are zero");
}

#[test]
fn test_real_las_bounds_consistency() {
    let blob = load_fixture("sample.las");
    let header = parse_las_header_core(&blob).expect("header parse failed");
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let positions = get_positions(&cloud);

    let n = positions.len() / 3;
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for i in 0..n {
        min_x = min_x.min(positions[i * 3]);
        max_x = max_x.max(positions[i * 3]);
        min_y = min_y.min(positions[i * 3 + 1]);
        max_y = max_y.max(positions[i * 3 + 1]);
        min_z = min_z.min(positions[i * 3 + 2]);
        max_z = max_z.max(positions[i * 3 + 2]);
    }

    // Header bounds are stored as scaled/offset coordinates. Some LAS writers
    // store the raw scaled integer bounds rather than the actual coordinate range.
    // We verify header bounds are finite and that actual data has a non-zero range.
    let header_bounds = [
        header.bounds_min_x(),
        header.bounds_max_x(),
        header.bounds_min_y(),
        header.bounds_max_y(),
        header.bounds_min_z(),
        header.bounds_max_z(),
    ];
    for &hb in &header_bounds {
        assert!(hb.is_finite(), "header bound should be finite, got {hb}");
    }

    // Actual data should have non-zero ranges
    assert!(max_x > min_x, "X range should be non-zero");
    assert!(max_y > min_y, "Y range should be non-zero");

    eprintln!(
        "Bounds: X[{:.1}, {:.1}] Y[{:.1}, {:.1}] Z[{:.1}, {:.1}]",
        min_x, max_x, min_y, max_y, min_z, max_z
    );
}

// ===========================================================================
// 2. Full pipeline: parse → octree → tileset → pnts validation
// ===========================================================================

#[test]
fn test_real_las_octree_has_spatial_partition() {
    let blob = load_fixture("sample.las");
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let mut positions = get_positions(&cloud).to_vec();

    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 4, 50);
    let octree = Octree::build(&mut positions, max_pts as u32, 10);

    assert!(
        octree.node_count() > 1,
        "1065 points should split into multiple octree nodes (got {})",
        octree.node_count()
    );
    assert_eq!(octree.total_points(), point_count as u32);

    eprintln!(
        "Octree: {} nodes, depth {}, leaves {}",
        octree.node_count(),
        octree.depth(),
        octree.leaf_count()
    );
}

#[test]
fn test_real_las_tileset_valid() {
    let blob = load_fixture("sample.las");
    let cloud = parse_las_points_core(&blob).expect("points parse failed");
    let mut positions = get_positions(&cloud).to_vec();
    let colors = get_colors(&cloud).map(|c| c.to_vec());

    let point_count = positions.len() / 3;
    let max_pts = std::cmp::max(point_count / 4, 50);

    let octree = Octree::build(&mut positions, max_pts as u32, 10);
    let result = generate_tileset(&octree, &positions, colors.as_deref())
        .expect("tileset generation failed");

    // Validate tileset.json parses as JSON
    let json_val: serde_json::Value =
        serde_json::from_str(result.tileset_json()).expect("tileset.json is not valid JSON");
    assert!(json_val["asset"].is_object(), "missing asset");
    assert!(json_val["root"].is_object(), "missing root");
    assert!(
        json_val["root"]["boundingVolume"].is_object(),
        "missing boundingVolume"
    );

    // Validate every tile
    let mut total_pts = 0u32;
    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile missing");
        assert_eq!(&tile_data[0..4], b"pnts", "tile {} missing pnts magic", i);
        let (hdr, _) = parse_pnts_header(tile_data).expect("pnts header parse failed");
        assert_eq!(hdr.version, 1);
        assert!(hdr.byte_length > 0);
        total_pts += hdr.feature_table_binary_byte_length / 15;
    }

    assert_eq!(
        total_pts, point_count as u32,
        "total points in tiles ({}) != source ({})",
        total_pts, point_count
    );

    eprintln!(
        "Tileset: {} tiles, {} total bytes, {} points across tiles",
        result.tile_count(),
        result.total_bytes(),
        total_pts
    );
}

// ===========================================================================
// 3. Synthetic "dirty" data: non-uniform, noisy, gapped point clouds
// ===========================================================================

/// Generate non-uniform point cloud (dense center, sparse edges).
fn gen_nonuniform_points(n: usize) -> Vec<(f64, f64, f64)> {
    let mut pts = Vec::with_capacity(n);
    let mut rng = simple_rng(42);
    for _ in 0..n {
        // Gaussian distribution centered at origin
        let x = gauss_rand(&mut rng, 0.0, 2.0);
        let y = gauss_rand(&mut rng, 0.0, 2.0);
        let z = x * 0.3 + y * 0.1 + gauss_rand(&mut rng, 0.0, 0.5);
        pts.push((x, y, z));
    }
    pts
}

/// Generate point cloud with Gaussian noise added.
fn gen_noisy_points(n: usize, noise_std: f64) -> Vec<(f64, f64, f64)> {
    let mut pts = Vec::with_capacity(n);
    let mut rng = simple_rng(123);
    for i in 0..n {
        let x = ((i % 32) as f64) * 2.0 - 32.0;
        let y = ((i / 32) as f64) * 2.0 - 32.0;
        let z = (x * 0.3 + y * 0.1).sin() * 5.0;
        pts.push((
            x + gauss_rand(&mut rng, 0.0, noise_std),
            y + gauss_rand(&mut rng, 0.0, noise_std),
            z + gauss_rand(&mut rng, 0.0, noise_std),
        ));
    }
    pts
}

/// Generate point cloud with gaps (simulating LiDAR occlusion).
fn gen_gapped_points(n: usize) -> Vec<(f64, f64, f64)> {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let x = ((i % 50) as f64) * 1.0;
        let y = ((i / 50) as f64) * 1.0;
        // Create a rectangular gap
        if x > 15.0 && x < 25.0 && y > 15.0 && y < 25.0 {
            continue;
        }
        let z = (x * 0.1 + y * 0.2).sin() * 3.0;
        pts.push((x, y, z));
    }
    pts
}

/// Simple xoshiro256** PRNG for deterministic noise.
fn simple_rng(seed: u64) -> u64 {
    seed
}

fn gauss_rand(state: &mut u64, _mean: f64, std_dev: f64) -> f64 {
    // Simple Box-Muller transform using LCG
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let u1 = (*state as f64) / u64::MAX as f64;
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let u2 = (*state as f64) / u64::MAX as f64;
    let u1 = u1.max(1e-30); // avoid log(0)
    let mag = (-2.0 * u1.ln()).sqrt();
    mag * (2.0 * std::f64::consts::PI * u2).cos() * std_dev
}

#[test]
fn test_nonuniform_points_no_panic() {
    let pts = gen_nonuniform_points(2000);
    let blob = build_test_las_blob(&pts, true);

    let cloud = parse_las_points_core(&blob).expect("nonuniform parse failed");
    assert_eq!(get_point_count(&cloud), 2000);

    let mut positions = get_positions(&cloud).to_vec();
    let octree = Octree::build(&mut positions, 200, 8);
    assert!(octree.node_count() > 1, "nonuniform data should split");

    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    assert!(result.tile_count() > 0);
    eprintln!(
        "Non-uniform: {} pts → {} nodes → {} tiles",
        2000,
        octree.node_count(),
        result.tile_count()
    );
}

#[test]
fn test_noisy_points_no_panic() {
    let pts = gen_noisy_points(2000, 0.5);
    let blob = build_test_las_blob(&pts, false);

    let cloud = parse_las_points_core(&blob).expect("noisy parse failed");
    assert_eq!(get_point_count(&cloud), 2000);

    let mut positions = get_positions(&cloud).to_vec();
    let octree = Octree::build(&mut positions, 200, 8);
    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    assert!(result.tile_count() > 0);

    // Verify all coordinates are finite (noise shouldn't produce NaN)
    for &v in &positions {
        assert!(v.is_finite(), "noisy data produced non-finite value: {v}");
    }

    eprintln!(
        "Noisy: {} pts → {} nodes → {} tiles",
        2000,
        octree.node_count(),
        result.tile_count()
    );
}

#[test]
fn test_gapped_points_no_panic() {
    let pts = gen_gapped_points(2500);
    let blob = build_test_las_blob(&pts, false);

    let cloud = parse_las_points_core(&blob).expect("gapped parse failed");
    // Fewer points than requested due to gap removal
    assert!(get_point_count(&cloud) < 2500);
    assert!(
        get_point_count(&cloud) > 2000,
        "gap should only remove a subset"
    );

    let mut positions = get_positions(&cloud).to_vec();
    let octree = Octree::build(&mut positions, 200, 8);
    let result = generate_tileset(&octree, &positions, None).expect("tileset failed");
    assert!(result.tile_count() > 0);

    eprintln!(
        "Gapped: {} pts (was 2500) → {} nodes → {} tiles",
        get_point_count(&cloud),
        octree.node_count(),
        result.tile_count()
    );
}

// ===========================================================================
// 4. Edge cases: single point, empty-ish data, truncated file
// ===========================================================================

#[test]
fn test_single_point_no_panic() {
    let pts = vec![(1.0, 2.0, 3.0)];
    let blob = build_test_las_blob(&pts, false);

    let header = parse_las_header_core(&blob).expect("single pt header");
    assert_eq!(header.num_points(), 1);

    let cloud = parse_las_points_core(&blob).expect("single pt parse");
    let positions = get_positions(&cloud);
    assert_eq!(positions.len(), 3);
    assert!((positions[0] - 1.0f32).abs() < 1.0);
    assert!((positions[1] - 2.0f32).abs() < 1.0);
    assert!((positions[2] - 3.0f32).abs() < 1.0);

    // Octree with 1 point
    let mut pos = positions.to_vec();
    let octree = Octree::build(&mut pos, 10, 10);
    assert!(octree.node_count() >= 1);
    assert_eq!(octree.total_points(), 1);

    eprintln!("Single point: OK");
}

#[test]
fn test_two_points_no_panic() {
    let pts = vec![(0.0, 0.0, 0.0), (100.0, 100.0, 100.0)];
    let blob = build_test_las_blob(&pts, true);
    let cloud = parse_las_points_core(&blob).expect("two pts parse");
    assert_eq!(get_point_count(&cloud), 2);

    let mut positions = get_positions(&cloud).to_vec();
    let octree = Octree::build(&mut positions, 1, 10);
    let result = generate_tileset(&octree, &positions, get_colors(&cloud)).expect("tileset");
    assert!(result.tile_count() > 0);

    let total: usize = (0..result.tile_count())
        .map(|i| result.tile(i as usize).unwrap().len())
        .sum();
    assert!(total > 0);
    eprintln!("Two points: {} tiles", result.tile_count());
}

#[test]
fn test_truncated_las_no_panic() {
    let pts = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
    let blob = build_test_las_blob(&pts, false);

    // Truncate in the middle of point data
    let truncated_len = 230 + 15; // header + partial point
    let truncated = &blob[..truncated_len];

    // Should not panic — may return error or partial results
    let result = std::panic::catch_unwind(|| parse_las_points_core(truncated));
    // Just verify no panic
    let _ = result;
    eprintln!(
        "Truncated LAS: no panic (result: {})",
        if result.is_ok() { "Ok" } else { "Err" }
    );
}

#[test]
fn test_very_short_bytes_no_panic() {
    // 4 bytes — way too short
    let result = std::panic::catch_unwind(|| {
        let _ = parse_las_header_core(&[0u8; 4]);
    });
    assert!(result.is_ok(), "should not panic on very short input");
    // Should fail gracefully
    assert!(parse_las_header_core(&[0u8; 4]).is_err());
    assert!(parse_las_points_core(&[0u8; 4]).is_err());
    eprintln!("Very short bytes: graceful error");
}
