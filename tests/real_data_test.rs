//! Real LAS file end-to-end validation.
//!
//! Tests the complete pipeline: LAS parsing → Octree → Tileset → validation
//! using synthetically generated large datasets.

#[cfg(all(feature = "point-cloud", feature = "test-helpers"))]
use wasm_spatial_core::{
    generate_tileset, parse_las_header_core, parse_las_points_core, parse_pnts_header,
    test_exports::test_helpers, Octree,
};

// ===========================================================================
// Task 1.1–1.7: Real format LAS end-to-end (1000 points with color)
// ===========================================================================

#[test]
#[cfg(all(feature = "point-cloud", feature = "test-helpers"))]
fn test_real_format_las_end_to_end() {
    // Use the test helper to build a valid blob
    let points: Vec<(f64, f64, f64)> = (0..1000)
        .map(|i| {
            let x = (i % 32) as f64 * 2.0;
            let y = (i / 32) as f64 * 2.0;
            let z = x * 0.3 + y * 0.1 + (i as f64 * 0.01).sin() * 5.0;
            (x, y, z)
        })
        .collect();

    let blob = wasm_spatial_core::test_exports::test_helpers::build_test_las_blob(&points, true);

    // 1. Parse header
    let header = parse_las_header_core(&blob).expect("Failed to parse header");
    eprintln!(
        "Header: v{}.{} format={}, points={}",
        header.version_major(),
        header.version_minor(),
        header.point_format_id(),
        header.num_points(),
    );
    assert_eq!(header.num_points(), 1000);
    assert_eq!(header.version_major(), 1);
    assert_eq!(header.version_minor(), 2);
    assert_eq!(header.point_format_id(), 2);

    // 2. Parse points
    let cloud = parse_las_points_core(&blob).expect("Failed to parse points");
    let positions = test_helpers::get_positions(&cloud).to_vec();
    let colors = test_helpers::get_colors(&cloud).map(|c| c.to_vec());
    assert_eq!(test_helpers::get_point_count(&cloud), 1000);
    assert!(colors.is_some());

    // 3. Build Octree
    let mut pos_buf = positions;
    let octree = Octree::build(&mut pos_buf, 100, 10);
    eprintln!(
        "Octree: {} nodes, {} points",
        octree.node_count(),
        octree.total_points()
    );
    assert!(octree.node_count() > 1);
    assert_eq!(octree.total_points(), 1000);

    // 4. Generate Tileset
    let result =
        generate_tileset(&octree, &pos_buf, colors.as_deref()).expect("Failed to generate tileset");
    eprintln!(
        "Tileset: {} tiles, {} bytes",
        result.tile_count(),
        result.total_bytes(),
    );
    assert!(result.tile_count() > 0);

    // 5. Validate tileset.json
    let json_val: serde_json::Value =
        serde_json::from_str(result.tileset_json()).expect("tileset.json invalid");
    assert!(json_val["asset"].is_object());
    assert!(json_val["root"].is_object());

    // 6. Validate all pnts tiles
    let mut total_points_in_tiles = 0u32;
    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile missing");
        assert_eq!(&tile_data[0..4], b"pnts");
        let (hdr, _) = parse_pnts_header(tile_data).expect("pnts header parse failed");
        assert_eq!(hdr.byte_length as usize, tile_data.len());
        total_points_in_tiles += hdr.feature_table_binary_byte_length / 15;
    }
    assert_eq!(total_points_in_tiles, 1000, "All points should be in tiles");

    eprintln!("✅ Real format LAS end-to-end passed");
}

// ===========================================================================
// Task 1.8: Large synthetic dataset (100K points, scan-line pattern)
// ===========================================================================

#[cfg(all(feature = "point-cloud", feature = "test-helpers"))]
fn generate_scan_line_points(count: usize) -> Vec<f32> {
    let mut positions = Vec::with_capacity(count * 3);
    let rows = (count as f64).sqrt().ceil() as usize;
    let cols = count.div_ceil(rows);

    for r in 0..rows {
        for c in 0..cols {
            if positions.len() / 3 >= count {
                break;
            }
            let x = c as f32 * 0.5;
            let y = r as f32 * 0.5;
            let z = x * 0.3 + y * 0.1 + ((r * 7 + c * 13) % 100) as f32 * 0.01;
            positions.push(x);
            positions.push(y);
            positions.push(z);
        }
    }
    positions
}

#[test]
#[cfg(all(feature = "point-cloud", feature = "test-helpers"))]
fn test_synthetic_100k_points_pipeline() {
    let num_points = 100_000;
    let mut positions = generate_scan_line_points(num_points);
    eprintln!("Generated {} synthetic points", positions.len() / 3);

    let octree = Octree::build(&mut positions, 5000, 12);
    eprintln!("Octree: {} nodes", octree.node_count());
    assert!(octree.node_count() > 1);

    let colors: Vec<u8> = (0..num_points)
        .flat_map(|i| {
            let z = positions[i * 3 + 2];
            let t = (z * 255.0) as u8;
            [t, 128u8, (255 - t)]
        })
        .collect();

    let result =
        generate_tileset(&octree, &positions, Some(&colors)).expect("Failed to generate tileset");
    eprintln!(
        "Tileset: {} tiles, {:.2} MB",
        result.tile_count(),
        result.total_bytes() as f64 / 1_048_576.0,
    );
    assert!(result.tile_count() > 0);

    let _: serde_json::Value = serde_json::from_str(result.tileset_json()).unwrap();

    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile missing");
        assert_eq!(&tile_data[0..4], b"pnts");
        let (hdr, _) = parse_pnts_header(tile_data).unwrap();
        assert_eq!(hdr.byte_length as usize, tile_data.len());
    }

    eprintln!("✅ 100K synthetic pipeline passed");
}
