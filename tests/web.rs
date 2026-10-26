//! Browser / Node wasm-bindgen smoke tests (one per exported module group).
//!
//! Native integration tests in this directory are `cfg(not(target_arch =
//! "wasm32"))` so `wasm-pack test` can compile the suite. This file is the
//! wasm32 harness.
//!
//! Run: `wasm-pack test --node --release --test web`
//! Enable engine features the same way CI does:
//! `wasm-pack test --node --release --features point-cloud,geotiff,laz-support,mesh-ingest,terrain-edit,mesh-edit,webgpu --test web`

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

use wasm_spatial_core::test_exports::parse_geojson_coords;
use wasm_spatial_core::{suggest_crs_heuristic, version};

#[wasm_bindgen_test]
fn test_version_in_browser() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}

#[wasm_bindgen_test]
fn test_geojson_parse_point() {
    let coords = parse_geojson_coords(r#"{"type":"Point","coordinates":[116.404,39.915]}"#)
        .expect("valid Point GeoJSON");
    assert_eq!(coords.length(), 2);
}

#[wasm_bindgen_test]
fn test_invalid_geojson_is_parse_error() {
    let err = parse_geojson_coords("not-json").unwrap_err();
    assert_eq!(err.code(), "PARSE_ERROR");
}

#[wasm_bindgen_test]
fn test_crs_heuristic_china() {
    let json = suggest_crs_heuristic(116.0, 39.0, 117.0, 40.0);
    assert!(
        json.contains("GCJ-02"),
        "China bbox should suggest GCJ-02, got {json}"
    );
}

#[wasm_bindgen_test]
fn test_geotiff_capability_flag() {
    let enabled = wasm_spatial_core::supports_geotiff();
    #[cfg(feature = "geotiff")]
    assert!(enabled);
    #[cfg(not(feature = "geotiff"))]
    assert!(!enabled);
}

#[cfg(feature = "point-cloud")]
#[wasm_bindgen_test]
fn test_octree_smoke() {
    let mut positions = vec![0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 0.0, 0.0];
    let tree = wasm_spatial_core::Octree::build(&mut positions, 1000, 8);
    assert!(tree.node_count() >= 1);
    assert_eq!(tree.total_points(), 3);
}

#[cfg(feature = "mesh-ingest")]
#[wasm_bindgen_test]
fn test_mesh_ingest_enabled() {
    assert!(wasm_spatial_core::supports_mesh_ingest());
}

#[cfg(feature = "terrain-edit")]
#[wasm_bindgen_test]
fn test_terrain_edit_enabled() {
    assert!(wasm_spatial_core::supports_terrain_edit());
}

#[cfg(feature = "mesh-edit")]
#[wasm_bindgen_test]
fn test_mesh_edit_enabled() {
    assert!(wasm_spatial_core::supports_mesh_edit());
}

#[cfg(feature = "webgpu")]
#[wasm_bindgen_test]
fn test_webgpu_layout_constants() {
    assert!(wasm_spatial_core::supports_webgpu());
    assert_eq!(wasm_spatial_core::POSITION_STRIDE_BYTES, 12);
    assert!(!wasm_spatial_core::webgpu_shader_version().is_empty());
}
