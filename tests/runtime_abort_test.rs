//! Runtime abort and memory budget tests (Wave 1.2 + W1.3).

#![cfg(all(feature = "point-cloud", feature = "test-helpers"))]

use wasm_spatial_core::test_exports::test_helpers::build_test_las_blob;
use wasm_spatial_core::{
    estimate_job_bytes, generate_tileset_with_spacing_abort, parse_las_points_with_progress_abort,
    JobOp, Octree, SpatialError,
};

fn large_las_blob(point_count: usize) -> Vec<u8> {
    let points: Vec<(f64, f64, f64)> = (0..point_count)
        .map(|i| (i as f64 * 0.01, 0.0, 0.0))
        .collect();
    build_test_las_blob(&points, false)
}

#[test]
fn test_abort_mid_las_parse() {
    use std::cell::Cell;

    let blob = large_las_blob(50_000);
    let processed = Cell::new(0u32);

    let err = parse_las_points_with_progress_abort(
        &blob,
        |n, _total| {
            processed.set(n);
        },
        1_000,
        || processed.get() >= 5_000,
    )
    .unwrap_err();

    assert_eq!(err.code(), SpatialError::Cancelled.code());
    assert!(processed.get() >= 5_000);
    assert!(processed.get() < 50_000);
}

#[test]
fn test_abort_mid_tileset_generate() {
    let positions: Vec<f32> = (0..900)
        .flat_map(|i| {
            let f = i as f32 * 0.1;
            [f, f * 0.2, f * 0.05]
        })
        .collect();
    let mut positions = positions;
    let tree = Octree::build(&mut positions, 50, 8);
    let mut tiles_done = 0u32;

    let err = generate_tileset_with_spacing_abort(&tree, &positions, None, None, None, || {
        tiles_done += 1;
        tiles_done >= 2
    })
    .unwrap_err();

    assert_eq!(err.code(), SpatialError::Cancelled.code());
}

#[test]
fn test_estimate_job_bytes_within_reasonable_range() {
    let point_count = 100_000u32;
    let est = estimate_job_bytes(JobOp::LasParse {
        point_count,
        has_color: false,
    });
    let blob = large_las_blob(point_count as usize);
    let actual_blob = blob.len();
    assert!(est >= point_count as usize * 12);
    assert!(est <= actual_blob * 2 + 10_000_000);
}
