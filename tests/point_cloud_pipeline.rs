//! End-to-end pipeline test: point cloud → octree → pnts tiles → tileset.json
//!
//! This integration test exercises the complete Phase A pipeline:
//! 1. Generate synthetic point cloud data
//! 2. Build octree spatial index
//! 3. Generate pnts tiles for each leaf
//! 4. Verify tileset.json is valid JSON
//! 5. Verify all tiles are valid pnts

use wasm_spatial_core::{
    generate_tileset, Octree, parse_pnts_header,
};

/// Generate a synthetic point cloud with `n` points uniformly distributed
/// in a cube from `(-size/2, -size/2, -size/2)` to `(size/2, size/2, size/2)`.
fn generate_synthetic_cloud(n: usize, size: f32) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let half = size / 2.0;
    let mut positions = Vec::with_capacity(n * 3);
    for _ in 0..n {
        positions.push(rng.gen_range(-half..half));
        positions.push(rng.gen_range(-half..half));
        positions.push(rng.gen_range(-half..half));
    }
    positions
}

/// Generate random colors for `n` points.
fn generate_random_colors(n: usize) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..n * 3).map(|_| rng.gen_range(0..=255)).collect()
}

#[test]
fn test_end_to_end_pipeline_1000_points() {
    let n = 1000;
    let positions = generate_synthetic_cloud(n, 100.0);
    let colors = generate_random_colors(n);

    // Step 1: Build octree.
    let mut buf = positions.clone();
    let tree = Octree::build(&mut buf, 100, 8);

    assert_eq!(tree.total_points(), n as u32);
    assert!(tree.node_count() > 1, "should have split into multiple nodes");
    assert!(tree.leaf_count() > 1, "should have multiple leaves");

    // Step 2: Generate tileset.
    let result = generate_tileset(&tree, &buf, Some(&colors)).unwrap();

    // Step 3: Verify tileset.json is valid JSON.
    let json_str = result.tileset_json();
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .expect("tileset.json should be valid JSON");

    // Verify top-level keys.
    assert!(parsed.get("asset").is_some(), "tileset should have asset");
    assert!(parsed.get("root").is_some(), "tileset should have root");
    assert!(parsed.get("geometricError").is_some());

    // Step 4: Verify tile count matches.
    assert!(result.tile_count() > 1);
    assert_eq!(result.tile_count() as usize, tree.leaves().filter(|n| n.point_count > 0).count());

    // Step 5: Verify all tiles are valid pnts.
    for i in 0..result.tile_count() as usize {
        let tile_data = result.tile(i).expect("tile should exist");
        assert!(
            tile_data.len() >= 40,
            "tile {} should have at least 40 bytes",
            i
        );
        assert_eq!(
            &tile_data[0..4],
            b"pnts",
            "tile {} should start with pnts magic",
            i
        );
        let (header, _) = parse_pnts_header(tile_data).expect("tile header should parse");
        assert_eq!(header.version, 1);
        assert_eq!(
            header.byte_length as usize,
            tile_data.len(),
            "tile {} byte_length should match actual length",
            i
        );
        // Verify FT binary contains RGB (since we provided colors).
        assert!(
            header.feature_table_binary_byte_length > 0,
            "tile {} should have feature table binary",
            i
        );
    }

    // Step 6: Verify total bytes.
    assert!(result.total_bytes() > 0);

    // Step 7: Verify each tile has a URI.
    for i in 0..result.tile_count() as usize {
        let uri = result.tile_uri(i).expect("tile URI should exist");
        assert!(uri.ends_with(".pnts"), "tile URI should end with .pnts");
    }

    // Step 8: Verify tile bounds are within root bounds.
    let root_bounds = tree.root_bounds();
    for i in 0..result.tile_count() as usize {
        let tile_bounds = result.tile_bounds(i).expect("tile bounds should exist");
        assert!(
            tile_bounds[0] >= root_bounds[0] - 1e-6,
            "tile {} min_x should be >= root min_x",
            i
        );
        assert!(
            tile_bounds[1] >= root_bounds[1] - 1e-6,
            "tile {} min_y should be >= root min_y",
            i
        );
        assert!(
            tile_bounds[2] >= root_bounds[2] - 1e-6,
            "tile {} min_z should be >= root min_z",
            i
        );
        assert!(
            tile_bounds[3] <= root_bounds[3] + 1e-6,
            "tile {} max_x should be <= root max_x",
            i
        );
        assert!(
            tile_bounds[4] <= root_bounds[4] + 1e-6,
            "tile {} max_y should be <= root max_y",
            i
        );
        assert!(
            tile_bounds[5] <= root_bounds[5] + 1e-6,
            "tile {} max_z should be <= root max_z",
            i
        );
    }
}

#[test]
fn test_pipeline_no_colors() {
    let n = 500;
    let positions = generate_synthetic_cloud(n, 50.0);
    let mut buf = positions;
    let tree = Octree::build(&mut buf, 50, 5);
    let result = generate_tileset(&tree, &buf, None).unwrap();

    // Verify tileset.json is valid.
    let json_str = result.tileset_json();
    let _parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("tileset.json should be valid JSON");

    // Verify all tiles have correct pnts format without colors.
    for i in 0..result.tile_count() as usize {
        let tile_data = result.tile(i).unwrap();
        let (header, _) = parse_pnts_header(tile_data).unwrap();
        assert_eq!(header.version, 1);

        // FT binary should be positions only (no RGB).
        let ft_json_str =
            std::str::from_utf8(&tile_data[28..28 + header.feature_table_json_byte_length as usize])
                .unwrap();
        assert!(
            !ft_json_str.contains("RGB"),
            "tile {} should not have RGB in feature table JSON",
            i
        );
    }
}

#[test]
fn test_pipeline_small_dataset() {
    // Edge case: very small dataset (8 points, perfectly aligned).
    let triples: Vec<[f32; 3]> = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    let mut positions: Vec<f32> = Vec::new();
    for &[x, y, z] in &triples {
        positions.extend_from_slice(&[x, y, z]);
    }

    let tree = Octree::build(&mut positions, 1, 21);
    let result = generate_tileset(&tree, &positions, None).unwrap();

    // Should produce 8 tiles (one per octant).
    assert_eq!(result.tile_count(), 8);

    // tileset.json should be parseable.
    let _: serde_json::Value =
        serde_json::from_str(result.tileset_json()).expect("valid JSON");
}
