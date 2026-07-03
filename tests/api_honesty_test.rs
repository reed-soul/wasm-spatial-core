//! API honesty / capability contract tests (W0).

#[test]
fn test_get_input_limits_json_fields() {
    let json = wasm_spatial_core::get_input_limits();
    assert!(json.contains("\"maxInputBytes\""));
    assert!(json.contains("\"copcSpatialQuery\""));
    assert!(json.contains("\"geotiffElevationFormats\""));
    assert!(json.contains("\"crsArbitraryEpsg\": false"));
    assert!(json.contains("\"octreeChunkBuilder\": true"));
}

#[test]
fn test_get_supported_crs_has_capabilities() {
    let json = wasm_spatial_core::get_supported_crs();
    assert!(json.contains("\"capabilities\""));
    assert!(json.contains("\"identity\""));
    assert!(json.contains("UTM"));
}

#[test]
fn test_crs_info_unknown_requires_proj() {
    let json = wasm_spatial_core::crs_info("EPSG:32650");
    assert!(json.contains("\"supported\":false"));
    assert!(json.contains("use-external-PROJ"));
}

#[test]
fn test_crs_info_cgcs2000_identity() {
    let json = wasm_spatial_core::crs_info("EPSG:4490");
    assert!(json.contains("\"capabilities\":[\"identity\"]"));
}

#[test]
fn test_estimate_job_bytes_new_ops() {
    use wasm_spatial_core::{estimate_job_bytes, JobOp};

    assert!(
        estimate_job_bytes(JobOp::GeotiffTerrainTileset { pixels: 256 * 256 })
            > estimate_job_bytes(JobOp::GeotiffParse { pixels: 256 * 256 })
    );
    assert!(
        estimate_job_bytes(JobOp::CopcRegion {
            point_count: 50_000,
            has_color: true,
        }) > 0
    );
    assert!(
        estimate_job_bytes(JobOp::OctreeChunkBuild {
            point_count: 50_000,
            has_color: true,
        }) < estimate_job_bytes(JobOp::OctreeBuild {
            point_count: 50_000,
        })
    );
}

#[test]
fn test_suggest_crs_heuristic_matches_best() {
    let a = wasm_spatial_core::suggest_crs_heuristic(116.0, 39.0, 117.0, 40.0);
    let b = wasm_spatial_core::best_crs_for_region(116.0, 39.0, 117.0, 40.0);
    assert_eq!(a, b);
}
