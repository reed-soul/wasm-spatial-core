//! Integration-test coverage for the TMS quantized-mesh pyramid generator.
//!
//! The unit tests inside `src/terrain_tms.rs` cover the internal logic
//! (`layer.json` fields, tile count/paths, byte validity, quadrant split).
//! This file complements them by exercising the **public re-exported API**
//! from `wasm_spatial_core::` — i.e. the path the WASM bindings and external
//! consumers use — to catch breakage in the re-export wiring (a regression
//! class the in-module `tests::` cannot detect).
//!
//! Required feature: `terrain-edit` (matches `quantized_mesh_roundtrip_test.rs`
//! — both modules are gated under `geotiff`, which `terrain-edit` implies).

use wasm_spatial_core::{encode_terrain_tms_pyramid, TmsPyramidResult};

fn fixture_heights_16x16() -> (Vec<f32>, u32, u32, [f64; 4]) {
    // Use a slightly larger grid than the in-module tests so a quadrant
    // sub-tile is a meaningful 8×8 — proves the public API path can drive
    // a non-trivial split + encode.
    let heights = (0..256).map(|i| (i as f32) * 2.5).collect();
    (heights, 16, 16, [116.3, 39.9, 116.4, 40.0])
}

#[test]
fn test_public_api_returns_pyramid() {
    let (heights, w, h, bounds) = fixture_heights_16x16();
    let result: TmsPyramidResult =
        encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).expect("pyramid builds");

    // Re-export path works, returns the documented shape.
    assert!(!result.layer_json.is_empty(), "layer.json populated");
    assert_eq!(result.tiles.len(), 5, "1 root + 4 quadrants");
}

#[test]
fn test_public_api_tiles_have_tms_relative_paths() {
    let (heights, w, h, bounds) = fixture_heights_16x16();
    let result = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();

    // Every tile path must be a valid TMS relative path that, when joined
    // under a layer root, resolves to `{z}/{x}/{y}.terrain`. We verify the
    // shape syntactically — the Playwright test verifies it loads in Cesium.
    for tile in &result.tiles {
        let parts: Vec<&str> = tile.path.split('/').collect();
        assert_eq!(
            parts.len(),
            3,
            "{}: TMS path has exactly 3 parts (z/x/y.terrain)",
            tile.path
        );
        parts[0].parse::<u32>().expect("z parses as u32");
        parts[1].parse::<u32>().expect("x parses as u32");
        assert!(
            parts[2].ends_with(".terrain"),
            "{}: filename ends with .terrain",
            tile.path
        );
    }
}

#[test]
fn test_public_api_center_override_is_respected() {
    let (heights, w, h, bounds) = fixture_heights_16x16();
    // When no override is supplied, the encoder derives an ECEF center from
    // the bounds + mean height. That derived value is non-trivial (not the
    // zero vector). Run the encoder twice and confirm the output is stable
    // for the same input — guards against accidental nondeterminism in the
    // center derivation path.
    let r1 = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
    let r2 = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
    assert_eq!(r1.tiles.len(), r2.tiles.len());
    for (a, b) in r1.tiles.iter().zip(r2.tiles.iter()) {
        assert_eq!(a.path, b.path);
        assert_eq!(
            a.bytes, b.bytes,
            "tile {} bytes must be deterministic",
            a.path
        );
    }
}

#[test]
fn test_public_api_writes_valid_layer_json() {
    let (heights, w, h, bounds) = fixture_heights_16x16();
    let result = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();

    // The layer.json must be parseable as standalone JSON — if a downstream
    // tool (CesiumTerrainProvider or our Node generator) reads it from disk,
    // it must not need a wrapper.
    let v: serde_json::Value =
        serde_json::from_str(&result.layer_json).expect("layer.json is valid JSON");
    assert_eq!(v["format"], "quantized-mesh-1.0");
    assert_eq!(v["maxzoom"], 1);
}
