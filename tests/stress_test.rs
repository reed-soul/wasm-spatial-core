//! End-to-end stress tests for wasm-spatial-core.
//!
//! These tests exercise the library at scale with large datasets.
//! All tests are marked `#[ignore]` by default — run with:
//!
//! ```sh
//! cargo test -- --ignored
//! ```
//!
//! For CI, only the quick tests (non-ignored) run by default.

use wasm_spatial_core::test_exports::{
    geojson_feature_collection_native, polygon_intersection_native, deduplicate_coords_native,
    parse_geojson_coords, count_geojson_features,
};
use wasm_spatial_core::batch_wgs84_to_gcj02_in_place;

use rand::Rng;

// ---------------------------------------------------------------------------
// Large GeoJSON Parse
// ---------------------------------------------------------------------------

/// Generate a GeoJSON FeatureCollection with `n` random Point features.
fn generate_random_geojson_fc(n: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut features = Vec::with_capacity(n);

    for i in 0..n {
        let lng = rng.gen_range(-180.0..180.0);
        let lat = rng.gen_range(-90.0..90.0);
        let feature = serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [lng, lat]
            },
            "properties": {
                "id": i,
                "value": rng.gen_range(0..1000)
            }
        });
        features.push(feature);
    }

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features
    });
    serde_json::to_string(&fc).unwrap()
}

/// Generate a GeoJSON FeatureCollection with `n` random LineString features.
fn generate_random_linestring_fc(n: usize, vertices_per_line: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut features = Vec::with_capacity(n);

    for _ in 0..n {
        let mut coords = Vec::with_capacity(vertices_per_line);
        for _ in 0..vertices_per_line {
            coords.push(vec![
                rng.gen_range(-180.0..180.0),
                rng.gen_range(-90.0..90.0),
            ]);
        }
        let feature = serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": "LineString",
                "coordinates": coords
            },
            "properties": {}
        });
        features.push(feature);
    }

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features
    });
    serde_json::to_string(&fc).unwrap()
}

#[test]
#[ignore]
fn test_large_geojson_parse() {
    let n = 100_000;
    let geojson = generate_random_geojson_fc(n);

    // Count features
    let count = count_geojson_features(&geojson).unwrap();
    assert_eq!(count, n as u32);

    // Parse all coordinates
    let coords = parse_geojson_coords(&geojson).unwrap();
    assert_eq!(coords.length(), n as u32 * 2);
}

#[test]
#[ignore]
fn test_large_linestring_geojson_parse() {
    let n = 10_000;
    let vpl = 100; // vertices per line
    let geojson = generate_random_linestring_fc(n, vpl);

    let count = count_geojson_features(&geojson).unwrap();
    assert_eq!(count, n as u32);

    let coords = parse_geojson_coords(&geojson).unwrap();
    assert_eq!(coords.length(), (n * vpl) as u32 * 2);
}

// ---------------------------------------------------------------------------
// Large Coordinate Transform
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_large_coordinate_transform() {
    let n = 10_000_000;
    let mut coords = Vec::with_capacity(n * 2);

    // Generate random WGS-84 coordinates
    let mut rng = rand::thread_rng();
    for _ in 0..n {
        coords.push(rng.gen_range(-180.0..180.0));
        coords.push(rng.gen_range(-90.0..90.0));
    }

    // Transform in place
    batch_wgs84_to_gcj02_in_place(&mut coords);

    // Verify no NaN or Inf
    for &v in &coords {
        assert!(v.is_finite(), "Found non-finite value after transform");
    }

    // Verify transformed coords are still in valid range
    for chunk in coords.chunks_exact(2) {
        assert!(
            (chunk[0] - chunk[0].clamp(-180.0, 180.0)).abs() < 1e-10,
            "Lng out of range: {}",
            chunk[0]
        );
        assert!(
            (chunk[1] - chunk[1].clamp(-90.0, 90.0)).abs() < 1e-10,
            "Lat out of range: {}",
            chunk[1]
        );
    }
}

// ---------------------------------------------------------------------------
// Large Polygon Boolean Operations
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_large_polygon_intersection() {
    let n = 1_000;
    let mut rng = rand::thread_rng();

    let mut intersect_count = 0;

    for _ in 0..n {
        let cx1 = rng.gen_range(-50.0..50.0);
        let cy1 = rng.gen_range(-50.0..50.0);
        let cx2 = cx1 + rng.gen_range(-2.0..2.0);
        let cy2 = cy1 + rng.gen_range(-2.0..2.0);
        let s = 1.0;

        let ring1 = vec![
            cx1 - s, cy1 - s,
            cx1 + s, cy1 - s,
            cx1 + s, cy1 + s,
            cx1 - s, cy1 + s,
            cx1 - s, cy1 - s,
        ];
        let ring2 = vec![
            cx2 - s, cy2 - s,
            cx2 + s, cy2 - s,
            cx2 + s, cy2 + s,
            cx2 - s, cy2 + s,
            cx2 - s, cy2 - s,
        ];

        let result = polygon_intersection_native(&ring1, &ring2);
        if !result.is_empty() {
            intersect_count += 1;
        }
    }

    // With offset up to 2 and size 2, should have significant overlap
    assert!(intersect_count > n / 4, "Expected at least 25% intersections, got {}/{}", intersect_count, n);
}

// ---------------------------------------------------------------------------
// Large Point Cloud Decimation (Dedup)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_large_point_deduplication() {
    let n = 1_000_000;
    let mut coords = Vec::with_capacity(n * 3); // ~1.5M points
    let mut rng = rand::thread_rng();

    // Generate random points with some duplicates (within tolerance)
    for _ in 0..n {
        let base_x = rng.gen_range(-180.0..180.0);
        let base_y = rng.gen_range(-90.0..90.0);

        // Add original point
        coords.push(base_x);
        coords.push(base_y);

        // Add a near-duplicate (50% of the time)
        if rng.gen_bool(0.5) {
            coords.push(base_x + rng.gen_range(-0.0001..0.0001));
            coords.push(base_y + rng.gen_range(-0.0001..0.0001));
        }
    }

    let result = deduplicate_coords_native(&coords, 0.001);
    let original_pairs = coords.len() / 2;
    let deduped_pairs = result.len() / 2;

    // Should have removed a significant number
    assert!(
        deduped_pairs < original_pairs,
        "Expected deduplication to reduce points: {} -> {}",
        original_pairs,
        deduped_pairs
    );
    // But should keep at least the original unique points
    assert!(deduped_pairs >= n, "Should keep at least original unique points");
}

// ---------------------------------------------------------------------------
// Large GeoJSON Write Roundtrip
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_large_geojson_write_roundtrip() {
    let n = 10_000;

    let mut all_coords = Vec::with_capacity(n * 2);
    let mut types = Vec::with_capacity(n);
    let mut props = Vec::with_capacity(n);

    for i in 0..n {
        all_coords.push(100.0 + i as f64 * 0.01);
        all_coords.push(30.0 + i as f64 * 0.01);
        types.push("Point".to_string());
        props.push(format!(r#"{{"id":{}}}"#, i));
    }

    let coords_flat: Vec<f64> = all_coords;
    let types_str = types.join(",");
    let props_str = props.join("\x01");

    let json = geojson_feature_collection_native(&coords_flat, &types_str, &props_str);

    // Parse back
    let count = count_geojson_features(&json).unwrap();
    assert_eq!(count, n as u32);

    let parsed_coords = parse_geojson_coords(&json).unwrap();
    assert_eq!(parsed_coords.length(), n as u32 * 2);
}
