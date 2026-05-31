//! Real external GeoTIFF validation tests.
//!
//! Tests with downloaded real GeoTIFF files (not synthetic).

use wasm_spatial_core::parse_geotiff_core;

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures").join(name);
    if !path.exists() {
        eprintln!("Skipping: fixture not found: {}", path.display());
        return Vec::new();
    }
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

// ===========================================================================
// External GeoTIFF: rasterio RGBA.byte.tif (791×718, RGB)
// Source: https://github.com/rasterio/rasterio/main/tests/data/RGBA.byte.tif
// ===========================================================================

#[test]
fn test_external_rgba_geotiff_parses() {
    let blob = load_fixture("rgba_791x718.tif");
    if blob.is_empty() {
        return; // skip if not downloaded
    }

    // This is an RGB TIFF, not elevation data, but it should parse structurally
    let result = parse_geotiff_core(&blob);
    assert!(result.is_ok(), "RGBA TIFF should parse: {:?}", result.err());

    let info = result.unwrap();
    assert_eq!(info.width(), 791);
    assert_eq!(info.height(), 718);

    eprintln!(
        "rgba_791x718.tif: {}x{}, {} elevation values",
        info.width(),
        info.height(),
        info.elevations().len()
    );
}

#[test]
fn test_external_rgba_geotiff_all_finite() {
    let blob = load_fixture("rgba_791x718.tif");
    if blob.is_empty() {
        return;
    }

    let info = parse_geotiff_core(&blob).unwrap();
    for &v in info.elevations() {
        assert!(v.is_finite(), "non-finite elevation in external TIFF: {v}");
    }

    let min = info
        .elevations()
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let max = info
        .elevations()
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    eprintln!("rgba_791x718.tif: elevation range [{:.1}, {:.1}]", min, max);
}

#[test]
fn test_external_rgba_geotiff_crs() {
    let blob = load_fixture("rgba_791x718.tif");
    if blob.is_empty() {
        return;
    }

    let info = parse_geotiff_core(&blob).unwrap();
    let crs = info.crs();
    // CRS may be empty if no GeoKeys, or contain WGS84 info
    if !crs.is_empty() {
        let crs_val: serde_json::Value =
            serde_json::from_str(&crs).expect("CRS should be valid JSON");
        assert!(crs_val.is_object());
        eprintln!("CRS: {}", crs);
    } else {
        eprintln!("No CRS data (file may lack GeoKeys)");
    }
}
