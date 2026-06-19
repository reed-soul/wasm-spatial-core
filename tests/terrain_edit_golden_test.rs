//! W3.7 — golden raster fixtures for terrain mask deformation.
//!
//! Fixture binaries under `tests/fixtures/terrain_edit/` are checked into CI.

use wasm_spatial_core::{
    excavate_inside, fill_inside, flatten_polygon, rasterize_polygon_mask, CutMode, TerrainGrid,
};

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = std::path::Path::new("tests/fixtures/terrain_edit").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("missing fixture {}: {e}", path.display()))
}

fn fixture_f32_le(name: &str) -> Vec<f32> {
    let bytes = fixture_bytes(name);
    assert_eq!(
        bytes.len() % 4,
        0,
        "f32 fixture {name} length must be multiple of 4"
    );
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

const BOUNDS_4: [f64; 4] = [0.0, 0.0, 3.0, 3.0];
const POLYGON_4: [f64; 8] = [0.5, 0.5, 2.5, 0.5, 2.5, 2.5, 0.5, 2.5];

#[test]
fn test_golden_mask_4x4_fixture() {
    let expected = fixture_bytes("mask_4x4_square.bin");
    assert_eq!(expected.len(), 16);

    let mask = rasterize_polygon_mask(4, 4, &BOUNDS_4, &POLYGON_4).expect("rasterize 4x4 mask");
    assert_eq!(mask, expected);
}

#[test]
fn test_golden_flatten_4x4_fixture() {
    let expected = fixture_f32_le("heights_flatten_4x4.bin");
    assert_eq!(expected.len(), 16);

    let mut grid = TerrainGrid::new(vec![10.0; 16], 4, 4, BOUNDS_4).unwrap();
    flatten_polygon(&mut grid, &POLYGON_4, 0.0, 0).unwrap();

    for (got, want) in grid.heights.iter().zip(expected.iter()) {
        assert!(
            (got - want).abs() < 1e-5,
            "flatten mismatch: got {got}, want {want}"
        );
    }
}

#[test]
fn test_golden_excavate_32x32_fixture() {
    let spots = fixture_f32_le("excavate_32x32_spots.bin");
    assert_eq!(spots.len(), 2);
    let (want_center, want_corner) = (spots[0], spots[1]);

    let bounds = [0.0, 0.0, 1.0, 1.0];
    let polygon = vec![0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75];
    let mask = fixture_bytes("mask_32x32_square.bin");
    assert_eq!(mask.len(), 32 * 32);

    let live_mask = rasterize_polygon_mask(32, 32, &bounds, &polygon).unwrap();
    assert_eq!(live_mask, mask);

    let mut heights = vec![20.0f32; 32 * 32];
    excavate_inside(&mut heights, &mask, CutMode::ByDepth(3.0)).unwrap();

    let center = 16 * 32 + 16;
    assert!((heights[center] - want_center).abs() < 1e-5);
    assert!((heights[0] - want_corner).abs() < 1e-5);
}

#[test]
fn test_golden_fill_2x2() {
    // Inline golden — fill only raises cells below target.
    let mut heights = vec![5.0, 15.0, 8.0, 12.0];
    let mask = vec![1u8; 4];
    fill_inside(&mut heights, &mask, 10.0).unwrap();

    let expected = [10.0, 15.0, 10.0, 12.0];
    for (got, want) in heights.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-5);
    }
}
