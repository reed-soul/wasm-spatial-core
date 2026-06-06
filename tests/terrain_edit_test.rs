//! Integration tests for terrain deformation (Wave 3).

use wasm_spatial_core::{
    encode_deformed_terrain_tileset, flatten_inside, flatten_polygon, rasterize_polygon_mask,
    CutMode, TerrainGrid,
};

#[test]
fn test_terrain_edit_pipeline() {
    let bounds = [0.0, 0.0, 1.0, 1.0];
    let polygon = vec![0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75];
    let mut grid = TerrainGrid::new(vec![20.0; 64], 8, 8, bounds).unwrap();

    let mask = rasterize_polygon_mask(8, 8, &bounds, &polygon).unwrap();
    flatten_inside(&mut grid.heights, &mask, 5.0).unwrap();

    let result = encode_deformed_terrain_tileset(&grid, 1).unwrap();
    let json: serde_json::Value = serde_json::from_str(result.tileset_json_str()).unwrap();
    assert!(json.get("root").is_some());
}

#[test]
fn test_golden_flatten_4x4() {
    // Golden heights: 4×4 grid, flatten center 2×2 region to 0
    let bounds = [0.0, 0.0, 3.0, 3.0];
    let heights = vec![
        10.0, 10.0, 10.0, 10.0, //
        10.0, 10.0, 10.0, 10.0, //
        10.0, 10.0, 10.0, 10.0, //
        10.0, 10.0, 10.0, 10.0, //
    ];
    // Polygon covers cell centers at (1,1) and (2,1) — indices 5 and 6
    let polygon = vec![0.5, 0.5, 2.5, 0.5, 2.5, 2.5, 0.5, 2.5];
    let mut grid = TerrainGrid::new(heights, 4, 4, bounds).unwrap();
    flatten_polygon(&mut grid, &polygon, 0.0, 0).unwrap();

    // Golden: corners unchanged, interior flattened
    assert!((grid.heights[0] - 10.0).abs() < 1e-5);
    assert!((grid.heights[5] - 0.0).abs() < 1e-5);
    assert!((grid.heights[6] - 0.0).abs() < 1e-5);
    assert!((grid.heights[15] - 10.0).abs() < 1e-5);
}

#[test]
fn test_golden_excavate_depth() {
    let bounds = [0.0, 0.0, 1.0, 1.0];
    let polygon = vec![0.2, 0.2, 0.8, 0.2, 0.8, 0.8, 0.2, 0.8];
    let mut grid = TerrainGrid::new(vec![30.0; 16], 4, 4, bounds).unwrap();
    let mask = rasterize_polygon_mask(4, 4, &bounds, &polygon).unwrap();
    wasm_spatial_core::excavate_inside(&mut grid.heights, &mask, CutMode::ByDepth(5.0)).unwrap();

    assert!((grid.heights[0] - 30.0).abs() < 1e-5);
    assert!((grid.heights[5] - 25.0).abs() < 1e-5);
}
