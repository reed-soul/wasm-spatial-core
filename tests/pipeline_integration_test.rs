//! End-to-end pipeline integration tests across all supported formats.
//!
//! Exercises complete pipelines:
//!   LAS: parse → header → points → octree → tileset → pnts
//!   GeoTIFF: parse → elevation → quantized-mesh → terrain tileset
//!   PLY: parse → validate
//!   OBJ: parse → validate
//!
//! Also tests cross-format pipelines and edge cases.

use wasm_spatial_core::test_exports::test_helpers::{
    build_test_las_blob, get_colors, get_point_count, get_positions,
};
use wasm_spatial_core::{
    apply_color_ramp_core, contour_lines_core, encode_quantized_mesh_core,
    encode_terrain_tileset_core, generate_tileset, hillshade_core, parse_geotiff_core,
    parse_las_header_core, parse_las_points_core, parse_obj_vertices_core, parse_ply_core,
    parse_pnts_header, ColorRamp, Octree,
};

// ===========================================================================
// Helpers
// ===========================================================================

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures").join(name);
    if !path.exists() {
        panic!("fixture not found: {}", path.display());
    }
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn load_fixture_text(name: &str) -> String {
    let bytes = load_fixture(name);
    String::from_utf8(bytes).expect("fixture is not valid UTF-8")
}

// ===========================================================================
// 1. Complete LAS pipeline
// ===========================================================================

#[test]
fn test_las_full_pipeline() {
    let blob = load_fixture("sample.las");
    let header = parse_las_header_core(&blob).expect("LAS header parse failed");
    assert!(header.num_points() > 0);

    let cloud = parse_las_points_core(&blob).expect("LAS points parse failed");
    assert_eq!(get_point_count(&cloud), header.num_points());

    let mut positions = get_positions(&cloud).to_vec();
    let colors = get_colors(&cloud).map(|c| c.to_vec());
    let n = positions.len() / 3;

    // Build octree
    let max_pts = std::cmp::max(n / 4, 50);
    let octree = Octree::build(&mut positions, max_pts as u32, 10);
    assert!(octree.node_count() > 1);
    assert_eq!(octree.total_points(), n as u32);

    // Generate tileset
    let result = generate_tileset(&octree, &positions, colors.as_deref())
        .expect("tileset generation failed");
    assert!(result.tile_count() > 0);

    // Validate tileset.json
    let json_val: serde_json::Value =
        serde_json::from_str(result.tileset_json()).expect("invalid JSON");
    assert!(json_val["asset"].is_object());
    assert!(json_val["root"].is_object());

    // Validate all pnts tiles
    let mut total_pts = 0u32;
    for i in 0..result.tile_count() {
        let tile = result.tile(i as usize).expect("tile missing");
        assert_eq!(&tile[0..4], b"pnts");
        let (hdr, _) = parse_pnts_header(tile).expect("pnts parse");
        assert_eq!(hdr.version, 1);
        total_pts += hdr.feature_table_binary_byte_length / 15;
    }
    assert_eq!(total_pts, n as u32);

    eprintln!(
        "LAS pipeline: {} pts → {} nodes → {} tiles → {} pts in tiles",
        n,
        octree.node_count(),
        result.tile_count(),
        total_pts
    );
}

// ===========================================================================
// 2. Complete GeoTIFF pipeline
// ===========================================================================

#[test]
fn test_geotiff_full_pipeline() {
    let blob = load_fixture("terrain_64x64.tif");
    let info = parse_geotiff_core(&blob).expect("GeoTIFF parse failed");

    assert_eq!(info.width(), 64);
    assert_eq!(info.height(), 64);
    let elevations = info.elevations();
    assert_eq!(elevations.len(), 64 * 64);

    // Quantized mesh
    let bounds = [0.0, 0.0, 1.0, 1.0];
    let center = [0.5, 0.5, 500.0];
    let mesh = encode_quantized_mesh_core(elevations, 64, 64, &bounds, &center)
        .expect("quantized-mesh failed");
    assert!(mesh.len() > 38);

    // Terrain tileset
    let tileset = encode_terrain_tileset_core(elevations, 64, 64, info.bounds_slice(), &center, 5)
        .expect("terrain tileset failed");
    let json: serde_json::Value = serde_json::from_str(tileset.tileset_json_str()).unwrap();
    assert!(json["asset"].is_object());

    // Color ramp
    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min) as f64;
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    let rgba = apply_color_ramp_core(elevations, min_z, max_z, ColorRamp::Terrain);
    assert_eq!(rgba.len(), elevations.len() * 4);

    // Hillshade
    let shading = hillshade_core(elevations, 64, 64, 315.0, 45.0);
    assert_eq!(shading.len(), elevations.len());

    // Contour lines
    let contours = contour_lines_core(elevations, 64, 64, (max_z - min_z) / 10.0);
    assert!(!contours.is_empty());

    eprintln!(
        "GeoTIFF pipeline: 64x64 → mesh {}B → tileset → ramp {}B → hillshade {}B → {} contours",
        mesh.len(),
        rgba.len(),
        shading.len(),
        contours.len()
    );
}

// ===========================================================================
// 3. PLY pipeline
// ===========================================================================

#[test]
fn test_ply_mesh_parse() {
    let blob = load_fixture("bunny_color.ply");
    let result = parse_ply_core(&blob).expect("PLY mesh parse failed");

    let positions = result.positions_core();
    assert!(
        positions.len() >= 3 * 100,
        "bunny should have at least 100 vertices, got {}",
        positions.len() / 3
    );

    // All coordinates finite
    for &v in positions {
        assert!(v.is_finite(), "PLY vertex has non-finite coord: {v}");
    }

    // Has colors
    assert!(result.has_colors());
    let colors = result.colors_core().expect("should have colors");
    assert_eq!(colors.len(), positions.len() / 3 * 3, "RGB per vertex");

    eprintln!(
        "PLY mesh: {} vertices, {} faces, has colors",
        positions.len() / 3,
        result.face_count()
    );
}

#[test]
fn test_ply_pointcloud_parse() {
    let blob = load_fixture("pointcloud_5k.ply");
    let result = parse_ply_core(&blob).expect("PLY point cloud parse failed");

    let positions = result.positions_core();
    assert!(
        positions.len() >= 3 * 1000,
        "point cloud should have at least 1000 points, got {}",
        positions.len() / 3
    );

    // Has colors
    assert!(result.has_colors());

    // Convert to point cloud and build octree
    let mut pos_buf = positions.to_vec();
    let octree = Octree::build(&mut pos_buf, 500, 10);
    assert!(
        octree.node_count() > 1,
        "5K points should split into multiple nodes"
    );

    eprintln!(
        "PLY pointcloud: {} points → {} octree nodes",
        positions.len() / 3,
        octree.node_count()
    );
}

#[test]
fn test_ply_to_octree_tileset_pipeline() {
    let blob = load_fixture("pointcloud_5k.ply");
    let result = parse_ply_core(&blob).expect("PLY parse failed");

    let mut positions = result.positions_core().to_vec();
    let colors = result.colors_core().map(|c| c.to_vec());
    let n = positions.len() / 3;

    let octree = Octree::build(&mut positions, 500, 10);
    let tileset =
        generate_tileset(&octree, &positions, colors.as_deref()).expect("PLY → tileset failed");

    assert!(tileset.tile_count() > 0);

    for i in 0..tileset.tile_count() {
        let tile = tileset.tile(i as usize).unwrap();
        assert_eq!(&tile[0..4], b"pnts");
    }

    eprintln!(
        "PLY → octree → tileset: {} pts → {} tiles",
        n,
        tileset.tile_count()
    );
}

// ===========================================================================
// 4. OBJ pipeline
// ===========================================================================

#[test]
fn test_obj_parse() {
    let text = load_fixture_text("cube.obj");
    let vertices = parse_obj_vertices_core(&text);

    // Cube has 8 vertices
    assert!(
        vertices.len() >= 8 * 3,
        "cube should have at least 8 vertices, got {}",
        vertices.len() / 3
    );

    // All finite
    for &v in &vertices {
        assert!(v.is_finite(), "OBJ vertex non-finite: {v}");
    }

    // Not all zeros
    let any_nonzero = vertices.iter().any(|&v| v.abs() > 0.1);
    assert!(any_nonzero, "OBJ vertices should not all be zero");

    eprintln!(
        "OBJ cube: {} vertices ({})",
        vertices.len() / 3,
        vertices.len()
    );
}

#[test]
fn test_obj_to_pointcloud_octree() {
    let text = load_fixture_text("cube.obj");
    let mut positions = parse_obj_vertices_core(&text);

    // Build octree from mesh vertices
    let octree = Octree::build(&mut positions, 4, 10);
    assert!(octree.node_count() >= 1);

    // Note: cube only has 8 vertices so it may not split
    let tileset = generate_tileset(&octree, &positions, None).expect("OBJ → tileset failed");
    assert!(tileset.tile_count() >= 1);

    eprintln!(
        "OBJ → point cloud → octree: {} verts → {} nodes → {} tiles",
        positions.len() / 3,
        octree.node_count(),
        tileset.tile_count()
    );
}

// ===========================================================================
// 5. Cross-format pipeline: GeoTIFF → terrain → mesh verification
// ===========================================================================

#[test]
fn test_geotiff_256_full_pipeline() {
    let blob = load_fixture("terrain_256x256.tif");
    let info = parse_geotiff_core(&blob).expect("256x256 parse failed");

    assert_eq!(info.width(), 256);
    assert_eq!(info.height(), 256);

    let elevations = info.elevations();
    let bounds = *info.bounds_slice();
    let center = [
        bounds[0] + (bounds[2] - bounds[0]) / 2.0,
        bounds[1] + (bounds[3] - bounds[1]) / 2.0,
        500.0,
    ];

    // Full pipeline
    let mesh = encode_quantized_mesh_core(elevations, 256, 256, info.bounds_slice(), &center)
        .expect("256 quantized-mesh failed");

    let tileset =
        encode_terrain_tileset_core(elevations, 256, 256, info.bounds_slice(), &center, 5)
            .expect("256 terrain tileset failed");

    let min_z = elevations.iter().copied().fold(f32::INFINITY, f32::min) as f64;
    let max_z = elevations.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;

    let rgba = apply_color_ramp_core(elevations, min_z, max_z, ColorRamp::Heat);
    let shading = hillshade_core(elevations, 256, 256, 315.0, 45.0);

    assert!(mesh.len() > 1000);
    assert!(rgba.len() == 256 * 256 * 4);
    assert!(shading.len() == 256 * 256);

    eprintln!(
        "256x256 pipeline: mesh {}B, tile JSON {}B, ramp {}B, shade {}B",
        mesh.len(),
        tileset.tileset_json_str().len(),
        rgba.len(),
        shading.len()
    );
}

// ===========================================================================
// 6. Boundary / stress tests
// ===========================================================================

#[test]
#[ignore] // Requires ~10GB RAM — run manually: cargo test test_10m_synthetic_performance -- --ignored --nocapture
fn test_10m_synthetic_performance() {
    let n = 10_000_000;
    let mut positions = Vec::with_capacity(n * 3);

    for i in 0..n {
        let x = ((i % 3162) as f32) * 1.0;
        let y = ((i / 3162) as f32) * 1.0;
        let z = ((i * 7) % 1000) as f32 * 0.01;
        positions.push(x);
        positions.push(y);
        positions.push(z);
    }

    let start = std::time::Instant::now();
    let octree = Octree::build(&mut positions, 10000, 10);
    let build_time = start.elapsed();

    assert_eq!(octree.total_points(), n as u32);

    eprintln!(
        "10M points: octree build = {:.2}s, {} nodes",
        build_time.as_secs_f64(),
        octree.node_count()
    );

    // Should be fast (< 5s)
    assert!(
        build_time.as_secs() < 5,
        "10M point octree took too long: {:.2}s",
        build_time.as_secs_f64()
    );
}

#[test]
fn test_single_point_boundary() {
    let pts = vec![(50.0, 60.0, 70.0)];
    let blob = build_test_las_blob(&pts, false);

    // Parse
    let cloud = parse_las_points_core(&blob).expect("single pt");
    assert_eq!(get_point_count(&cloud), 1);

    // Octree
    let mut pos = get_positions(&cloud).to_vec();
    let octree = Octree::build(&mut pos, 10, 10);
    assert!(octree.node_count() >= 1);
    assert_eq!(octree.total_points(), 1);

    // Tileset
    let result = generate_tileset(&octree, &pos, None).expect("single pt tileset");
    assert!(result.tile_count() >= 1);
}

#[test]
fn test_empty_points() {
    // Build blob with 0 points
    let pts: Vec<(f64, f64, f64)> = vec![];
    let blob = build_test_las_blob(&pts, false);

    let cloud = parse_las_points_core(&blob).expect("empty parse");
    assert_eq!(get_point_count(&cloud), 0);
    assert!(get_positions(&cloud).is_empty());

    // Octree with empty positions
    let mut empty: Vec<f32> = vec![];
    let octree = Octree::build(&mut empty, 10, 10);
    assert_eq!(octree.total_points(), 0);

    eprintln!("Empty points: OK");
}

#[test]
fn test_truncated_file_no_panic() {
    let pts = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)];
    let blob = build_test_las_blob(&pts, false);

    // Truncate at various points
    for truncate_at in [0, 10, 100, 230, 235, 250] {
        let truncated = if truncate_at < blob.len() {
            &blob[..truncate_at]
        } else {
            continue;
        };

        let result = std::panic::catch_unwind(|| parse_las_points_core(truncated));
        assert!(
            result.is_ok(),
            "panic on truncated input at {} bytes",
            truncate_at
        );
    }

    eprintln!("Truncated file: no panic at any truncation point");
}

#[test]
fn test_corrupt_header_no_panic() {
    // Completely garbage data
    let garbage = vec![0xDEu8; 500];
    let result = std::panic::catch_unwind(|| {
        let _ = parse_las_header_core(&garbage);
    });
    assert!(result.is_ok(), "garbage header should not panic");

    let result2 = std::panic::catch_unwind(|| {
        let _ = parse_las_points_core(&garbage);
    });
    assert!(result2.is_ok(), "garbage data should not panic");

    // Should return errors, not panic
    assert!(parse_las_header_core(&garbage).is_err());

    eprintln!("Corrupt header: graceful error");
}

// ===========================================================================
// Sample LAS header/points/tileset (kept from original test)
// ===========================================================================

#[test]
fn test_sample_las_header() {
    let blob = load_fixture("sample.las");
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

    assert!((header.version_major(), header.version_minor()) >= (1, 0),);
    assert!(header.num_points() > 0);
    assert!(header.point_data_record_length() >= 20);
}

#[test]
fn test_sample_las_points() {
    let blob = load_fixture("sample.las");
    let header = parse_las_header_core(&blob).expect("header parse failed");
    let cloud = parse_las_points_core(&blob).expect("LAS points parse failed");

    let positions = get_positions(&cloud);
    let n_points = positions.len() / 3;

    assert_eq!(
        n_points,
        header.num_points() as usize,
        "point count mismatch"
    );
    assert!(n_points > 0);

    for i in 0..n_points {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        assert!(
            x.is_finite() && y.is_finite() && z.is_finite(),
            "point {} non-finite",
            i
        );
    }
}

#[test]
fn test_sample_las_octree() {
    let blob = load_fixture("sample.las");
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
    assert!(octree.node_count() >= 1);
}

#[test]
fn test_sample_las_tileset() {
    let blob = load_fixture("sample.las");
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

    assert!(result.tile_count() > 0);
    assert!(result.total_bytes() > 0);

    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).expect("tile data missing");
        assert_eq!(&tile_data[0..4], b"pnts", "tile {} magic", i);
        let (header, _) = parse_pnts_header(tile_data).expect("pnts parse failed");
        assert_eq!(header.version, 1, "tile {} version", i);
        assert!(header.byte_length > 0, "tile {} byte_length", i);
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

    assert!(octree.node_count() > 1);
    assert!(result.tile_count() > 1);

    for i in 0..result.tile_count() {
        let tile_data = result.tile(i as usize).unwrap();
        assert_eq!(&tile_data[0..4], b"pnts");
        let (header, _) = parse_pnts_header(tile_data).unwrap();
        assert_eq!(header.version, 1);
    }
}
