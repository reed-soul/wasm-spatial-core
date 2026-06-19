//! Terrain deformation on elevation grids (Wave 3).
//!
//! Polygon-masked cut, flatten, fill, and boundary feathering on heightfields,
//! with re-encode to Cesium quantized-mesh tilesets.

use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

use crate::cesium_adapter::wgs84_to_cartesian3_single;
use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::geotiff::{encode_terrain_tileset_core, TerrainTilesetResult};

/// Excavation mode: subtract depth or lower to a target elevation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CutMode {
    /// Lower inside cells by this amount (meters).
    ByDepth(f32),
    /// Set inside cells to this elevation (meters), not below current if shallower cut.
    ToElevation(f32),
}

/// Elevation grid with geographic bounds `[west, south, east, north]`.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainGrid {
    pub heights: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub bounds: [f64; 4],
}

impl TerrainGrid {
    pub fn new(
        heights: Vec<f32>,
        width: u32,
        height: u32,
        bounds: [f64; 4],
    ) -> Result<Self, SpatialErrorDetail> {
        let expected = (width * height) as usize;
        if heights.len() != expected {
            return Err(SpatialError::InvalidInput.with_detail(format!(
                "heights length {} != width×height {}",
                heights.len(),
                expected
            )));
        }
        if width < 1 || height < 1 {
            return Err(SpatialError::InvalidInput.with_detail("grid dimensions must be positive"));
        }
        Ok(Self {
            heights,
            width,
            height,
            bounds,
        })
    }

    pub fn cell_count(&self) -> usize {
        self.heights.len()
    }
}

/// Geographic coordinate of a grid cell center.
pub fn cell_center(bounds: &[f64; 4], width: u32, height: u32, col: u32, row: u32) -> (f64, f64) {
    let west = bounds[0];
    let south = bounds[1];
    let east = bounds[2];
    let north = bounds[3];

    let lng = if width > 1 {
        west + (col as f64 / (width - 1) as f64) * (east - west)
    } else {
        (west + east) * 0.5
    };
    let lat = if height > 1 {
        south + (row as f64 / (height - 1) as f64) * (north - south)
    } else {
        (south + north) * 0.5
    };
    (lng, lat)
}

/// Ray-casting point-in-polygon for a closed ring `[lng, lat, ...]`.
pub fn point_in_ring_core(px: f64, py: f64, ring: &[f64]) -> bool {
    let n = ring.len() / 2;
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let xi = ring[i * 2];
        let yi = ring[i * 2 + 1];
        let xj = ring[j * 2];
        let yj = ring[j * 2 + 1];

        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }

    inside
}

/// Rasterize a polygon mask on an elevation grid (1 = inside, 0 = outside).
pub fn rasterize_polygon_mask(
    width: u32,
    height: u32,
    bounds: &[f64; 4],
    polygon: &[f64],
) -> Result<Vec<u8>, SpatialErrorDetail> {
    if polygon.len() < 6 || !polygon.len().is_multiple_of(2) {
        return Err(SpatialError::InvalidInput
            .with_detail("polygon must have at least 3 vertices as [lng, lat, ...]"));
    }

    let count = (width * height) as usize;
    let mut mask = vec![0u8; count];

    for row in 0..height {
        for col in 0..width {
            let (lng, lat) = cell_center(bounds, width, height, col, row);
            if point_in_ring_core(lng, lat, polygon) {
                mask[(row * width + col) as usize] = 1;
            }
        }
    }

    Ok(mask)
}

fn apply_inside<F>(heights: &mut [f32], mask: &[u8], mut op: F)
where
    F: FnMut(f32) -> f32,
{
    for (h, &m) in heights.iter_mut().zip(mask.iter()) {
        if m == 1 {
            *h = op(*h);
        }
    }
}

/// Excavate (cut) cells where `mask == 1`.
pub fn excavate_inside(
    heights: &mut [f32],
    mask: &[u8],
    mode: CutMode,
) -> Result<(), SpatialErrorDetail> {
    if heights.len() != mask.len() {
        return Err(SpatialError::InvalidInput.with_detail("heights/mask length mismatch"));
    }

    match mode {
        CutMode::ByDepth(depth) => {
            apply_inside(heights, mask, |h| h - depth);
        }
        CutMode::ToElevation(target) => {
            apply_inside(heights, mask, |h| h.min(target));
        }
    }
    Ok(())
}

/// Flatten inside cells to `target` elevation.
pub fn flatten_inside(
    heights: &mut [f32],
    mask: &[u8],
    target: f32,
) -> Result<(), SpatialErrorDetail> {
    if heights.len() != mask.len() {
        return Err(SpatialError::InvalidInput.with_detail("heights/mask length mismatch"));
    }
    apply_inside(heights, mask, |_| target);
    Ok(())
}

/// Fill inside cells — only raises cells below `target`.
pub fn fill_inside(
    heights: &mut [f32],
    mask: &[u8],
    target: f32,
) -> Result<(), SpatialErrorDetail> {
    if heights.len() != mask.len() {
        return Err(SpatialError::InvalidInput.with_detail("heights/mask length mismatch"));
    }
    apply_inside(heights, mask, |h| h.max(target));
    Ok(())
}

/// Compute per-cell blend weights for boundary feathering (0 = original, 1 = fully deformed).
pub fn compute_feather_weights(
    mask: &[u8],
    width: u32,
    height: u32,
    feather_cells: u32,
) -> Result<Vec<f32>, SpatialErrorDetail> {
    let w = width as usize;
    let h = height as usize;
    if mask.len() != w * h {
        return Err(SpatialError::InvalidInput.with_detail("mask size mismatch"));
    }
    if feather_cells == 0 {
        return Ok(mask
            .iter()
            .map(|&m| if m == 1 { 1.0 } else { 0.0 })
            .collect());
    }

    let mut dist = vec![u32::MAX; w * h];
    let mut queue = VecDeque::new();

    for row in 0..h {
        for col in 0..w {
            let idx = row * w + col;
            if mask[idx] == 0 {
                continue;
            }

            let mut touches_outside = false;
            if row == 0 || row + 1 == h || col == 0 || col + 1 == w {
                touches_outside = true;
            } else {
                for (nr, nc) in [
                    (row - 1, col),
                    (row + 1, col),
                    (row, col - 1),
                    (row, col + 1),
                ] {
                    if mask[nr * w + nc] == 0 {
                        touches_outside = true;
                        break;
                    }
                }
            }

            if touches_outside {
                dist[idx] = 0;
                queue.push_back(idx);
            }
        }
    }

    while let Some(idx) = queue.pop_front() {
        let row = idx / w;
        let col = idx % w;
        let next = dist[idx].saturating_add(1);

        for (nr, nc) in [
            (row.wrapping_sub(1), col),
            (row + 1, col),
            (row, col.wrapping_sub(1)),
            (row, col + 1),
        ] {
            if nr >= h || nc >= w {
                continue;
            }
            let nidx = nr * w + nc;
            if mask[nidx] == 0 || dist[nidx] <= next {
                continue;
            }
            dist[nidx] = next;
            queue.push_back(nidx);
        }
    }

    let feather = feather_cells as f32;
    Ok(dist
        .iter()
        .zip(mask.iter())
        .map(|(&d, &m)| {
            if m == 0 {
                0.0
            } else if d == u32::MAX {
                1.0
            } else {
                ((d as f32 + 1.0) / feather).min(1.0)
            }
        })
        .collect())
}

/// Blend `modified` into `original` using feather weights (W3.5).
pub fn feather_blend(
    original: &[f32],
    modified: &[f32],
    weights: &[f32],
) -> Result<Vec<f32>, SpatialErrorDetail> {
    if original.len() != modified.len() || original.len() != weights.len() {
        return Err(SpatialError::InvalidInput.with_detail("blend buffer length mismatch"));
    }

    Ok(original
        .iter()
        .zip(modified.iter())
        .zip(weights.iter())
        .map(|((o, m), &w)| o * (1.0 - w) + m * w)
        .collect())
}

/// Flatten inside polygon with optional boundary feathering.
pub fn flatten_polygon(
    grid: &mut TerrainGrid,
    polygon: &[f64],
    target: f32,
    feather_cells: u32,
) -> Result<(), SpatialErrorDetail> {
    let original = grid.heights.clone();
    let mask = rasterize_polygon_mask(grid.width, grid.height, &grid.bounds, polygon)?;
    flatten_inside(&mut grid.heights, &mask, target)?;
    if feather_cells > 0 {
        let weights = compute_feather_weights(&mask, grid.width, grid.height, feather_cells)?;
        grid.heights = feather_blend(&original, &grid.heights, &weights)?;
    }
    Ok(())
}

/// Re-encode deformed heights as a quantized-mesh terrain tileset (W3.6).
pub fn encode_deformed_terrain_tileset(
    grid: &TerrainGrid,
    max_zoom: u32,
) -> Result<TerrainTilesetResult, SpatialErrorDetail> {
    if grid.width < 2 || grid.height < 2 {
        return Err(SpatialError::TerrainError
            .with_detail("terrain grid must be at least 2×2 for tileset encoding"));
    }

    let center_lng = (grid.bounds[0] + grid.bounds[2]) * 0.5;
    let center_lat = (grid.bounds[1] + grid.bounds[3]) * 0.5;
    let mid_row = grid.height / 2;
    let mid_col = grid.width / 2;
    let mid_idx = (mid_row * grid.width + mid_col) as usize;
    let center_alt = grid.heights.get(mid_idx).copied().unwrap_or(0.0) as f64;
    let (cx, cy, cz) = wgs84_to_cartesian3_single(center_lng, center_lat, center_alt);
    let center = [cx, cy, cz];

    encode_terrain_tileset_core(
        &grid.heights,
        grid.width,
        grid.height,
        &grid.bounds,
        &center,
        max_zoom,
    )
    .map_err(|e| SpatialError::TerrainError.with_detail(e))
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Parse cut mode from JS: `{ mode: "depth", value: 2 }` or `{ mode: "elevation", value: 100 }`.
fn parse_cut_mode(mode: &str, value: f32) -> Result<CutMode, SpatialErrorDetail> {
    match mode {
        "depth" => Ok(CutMode::ByDepth(value)),
        "elevation" => Ok(CutMode::ToElevation(value)),
        other => Err(SpatialError::InvalidInput.with_detail(format!(
            "unknown cut mode '{other}', expected 'depth' or 'elevation'"
        ))),
    }
}

/// Rasterize polygon mask on a terrain grid.
#[wasm_bindgen(js_name = "rasterizeTerrainMask")]
pub fn rasterize_terrain_mask(
    width: u32,
    height: u32,
    bounds: &[f64],
    polygon: &[f64],
) -> Result<js_sys::Uint8Array, JsValue> {
    if bounds.len() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("bounds must be [west, south, east, north]")
            .into());
    }
    let bounds_arr = [bounds[0], bounds[1], bounds[2], bounds[3]];
    let mask = rasterize_polygon_mask(width, height, &bounds_arr, polygon)?;
    Ok(js_sys::Uint8Array::from(&mask[..]))
}

/// Apply a mask-scoped deformation with optional boundary feathering.
fn deform_terrain_in_polygon(
    heights: &mut [f32],
    width: u32,
    height: u32,
    bounds: [f64; 4],
    polygon: &[f64],
    feather_cells: u32,
    mut apply: impl FnMut(&mut [f32], &[u8]) -> Result<(), SpatialErrorDetail>,
) -> Result<(), SpatialErrorDetail> {
    let mask = rasterize_polygon_mask(width, height, &bounds, polygon)?;
    if feather_cells > 0 {
        let original = heights.to_vec();
        apply(heights, &mask)?;
        let weights = compute_feather_weights(&mask, width, height, feather_cells)?;
        let blended = feather_blend(&original, heights, &weights)?;
        heights.copy_from_slice(&blended);
    } else {
        apply(heights, &mask)?;
    }
    Ok(())
}

/// Excavate terrain inside a polygon.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = "excavateTerrain")]
pub fn excavate_terrain(
    heights: &mut [f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    polygon: &[f64],
    mode: &str,
    value: f32,
    feather_cells: u32,
) -> Result<(), JsValue> {
    if bounds.len() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("bounds must be [west, south, east, north]")
            .into());
    }
    let bounds_arr = [bounds[0], bounds[1], bounds[2], bounds[3]];
    let cut = parse_cut_mode(mode, value)?;
    deform_terrain_in_polygon(
        heights,
        width,
        height,
        bounds_arr,
        polygon,
        feather_cells,
        |h, mask| excavate_inside(h, mask, cut),
    )
    .map_err(Into::into)
}

/// Flatten terrain inside a polygon to target elevation.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = "flattenTerrain")]
pub fn flatten_terrain(
    heights: &mut [f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    polygon: &[f64],
    target: f32,
    feather_cells: u32,
) -> Result<(), JsValue> {
    if bounds.len() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("bounds must be [west, south, east, north]")
            .into());
    }
    let bounds_arr = [bounds[0], bounds[1], bounds[2], bounds[3]];
    deform_terrain_in_polygon(
        heights,
        width,
        height,
        bounds_arr,
        polygon,
        feather_cells,
        |h, mask| flatten_inside(h, mask, target),
    )
    .map_err(Into::into)
}

/// Fill terrain inside a polygon (only raises cells below target).
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = "fillTerrain")]
pub fn fill_terrain(
    heights: &mut [f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    polygon: &[f64],
    target: f32,
    feather_cells: u32,
) -> Result<(), JsValue> {
    if bounds.len() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("bounds must be [west, south, east, north]")
            .into());
    }
    let bounds_arr = [bounds[0], bounds[1], bounds[2], bounds[3]];
    deform_terrain_in_polygon(
        heights,
        width,
        height,
        bounds_arr,
        polygon,
        feather_cells,
        |h, mask| fill_inside(h, mask, target),
    )
    .map_err(Into::into)
}

/// Re-encode deformed terrain as a quantized-mesh tileset (WASM).
#[wasm_bindgen(js_name = "encodeDeformedTerrainTileset")]
pub fn encode_deformed_terrain_tileset_js(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    max_zoom: u32,
) -> Result<TerrainTilesetResult, JsValue> {
    if bounds.len() < 4 {
        return Err(SpatialError::InvalidInput
            .with_detail("bounds must be [west, south, east, north]")
            .into());
    }
    let grid = TerrainGrid::new(
        heights.to_vec(),
        width,
        height,
        [bounds[0], bounds[1], bounds[2], bounds[3]],
    )?;
    encode_deformed_terrain_tileset(&grid, max_zoom).map_err(Into::into)
}

/// Whether terrain edit (Wave 3) is available.
#[wasm_bindgen(js_name = "supportsTerrainEdit")]
pub fn supports_terrain_edit() -> bool {
    true
}

#[cfg(feature = "mesh-ingest")]
impl crate::spatial_ir::HeightfieldChunk {
    /// Apply flatten deformation and refresh metadata.
    pub fn flatten_polygon(
        &mut self,
        polygon: &[f64],
        target: f32,
        feather_cells: u32,
    ) -> Result<(), SpatialErrorDetail> {
        let mut grid =
            TerrainGrid::new(self.heights.clone(), self.width, self.height, self.bounds)?;
        flatten_polygon(&mut grid, polygon, target, feather_cells)?;
        self.heights = grid.heights;
        self.metadata.bump_version();
        self.refresh_metadata();
        Ok(())
    }

    /// Re-encode as quantized-mesh tileset after deformation.
    pub fn encode_terrain_tileset(
        &self,
        max_zoom: u32,
    ) -> Result<TerrainTilesetResult, SpatialErrorDetail> {
        let grid = TerrainGrid::new(self.heights.clone(), self.width, self.height, self.bounds)?;
        encode_deformed_terrain_tileset(&grid, max_zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_16x16() -> TerrainGrid {
        let width = 16u32;
        let height = 16u32;
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let heights = vec![10.0f32; (width * height) as usize];
        TerrainGrid::new(heights, width, height, bounds).unwrap()
    }

    fn square_polygon() -> Vec<f64> {
        vec![0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75]
    }

    #[test]
    fn test_rasterize_16x16_square() {
        let mask =
            rasterize_polygon_mask(16, 16, &[0.0, 0.0, 1.0, 1.0], &square_polygon()).unwrap();
        assert_eq!(mask.len(), 256);

        let inside: usize = mask.iter().map(|&v| v as usize).sum();
        assert!(inside > 0 && inside < 256);

        // Center cell must be inside
        let center_idx = 8 * 16 + 8;
        assert_eq!(mask[center_idx], 1);
        // Corner cell outside polygon
        assert_eq!(mask[0], 0);
    }

    #[test]
    fn test_flatten_inside() {
        let mut grid = grid_16x16();
        let mask = rasterize_polygon_mask(grid.width, grid.height, &grid.bounds, &square_polygon())
            .unwrap();
        flatten_inside(&mut grid.heights, &mask, 5.0).unwrap();

        for (h, &m) in grid.heights.iter().zip(mask.iter()) {
            if m == 1 {
                assert!((h - 5.0).abs() < 1e-5);
            } else {
                assert!((h - 10.0).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn test_excavate_by_depth_golden_32x32() {
        let w = 32u32;
        let h = 32u32;
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let mut heights = vec![20.0f32; (w * h) as usize];
        let polygon = square_polygon();
        let mask = rasterize_polygon_mask(w, h, &bounds, &polygon).unwrap();
        excavate_inside(&mut heights, &mask, CutMode::ByDepth(3.0)).unwrap();

        // Golden spot checks (W3.7)
        let center = 16 * 32 + 16;
        let corner = 0;
        assert!((heights[center] - 17.0).abs() < 1e-5);
        assert!((heights[corner] - 20.0).abs() < 1e-5);
    }

    #[test]
    fn test_fill_only_raises() {
        let mut heights = vec![5.0, 15.0, 8.0, 12.0];
        let mask = vec![1, 1, 1, 1];
        fill_inside(&mut heights, &mask, 10.0).unwrap();
        assert!((heights[0] - 10.0).abs() < 1e-5);
        assert!((heights[1] - 15.0).abs() < 1e-5);
        assert!((heights[2] - 10.0).abs() < 1e-5);
        assert!((heights[3] - 12.0).abs() < 1e-5);
    }

    #[test]
    fn test_feather_removes_cliff() {
        let w = 8u32;
        let h = 8u32;
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let polygon = vec![0.3, 0.3, 0.7, 0.3, 0.7, 0.7, 0.3, 0.7];
        let mask = rasterize_polygon_mask(w, h, &bounds, &polygon).unwrap();
        let original = vec![10.0f32; (w * h) as usize];
        let mut modified = original.clone();
        flatten_inside(&mut modified, &mask, 0.0).unwrap();
        let weights = compute_feather_weights(&mask, w, h, 1).unwrap();
        let blended = feather_blend(&original, &modified, &weights).unwrap();

        // A boundary-adjacent inside cell should be between 0 and 10
        let boundary_inside = (4 * w + 2) as usize;
        if mask[boundary_inside] == 1 && weights[boundary_inside] < 1.0 {
            assert!(blended[boundary_inside] > 0.0 && blended[boundary_inside] < 10.0);
        }
    }

    #[test]
    fn test_encode_deformed_tileset_json() {
        let mut grid = TerrainGrid::new(vec![100.0; 16], 4, 4, [116.0, 39.0, 117.0, 40.0]).unwrap();
        flatten_polygon(&mut grid, &square_polygon(), 50.0, 0).unwrap();
        let result = encode_deformed_terrain_tileset(&grid, 2).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(result.tileset_json_str()).unwrap();
        assert!(parsed.get("root").is_some());
        assert!(result.tile_core(0).is_some());
    }

    #[test]
    #[ignore = "performance gate — run with --ignored --release"]
    fn test_flatten_2048_performance() {
        let n = 2048u32;
        let bounds = [0.0, 0.0, 1.0, 1.0];
        let heights = vec![10.0f32; (n * n) as usize];
        let mut grid = TerrainGrid::new(heights, n, n, bounds).unwrap();
        let polygon = square_polygon();
        let start = std::time::Instant::now();
        flatten_polygon(&mut grid, &polygon, 5.0, 0).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 500,
            "2048×2048 flatten took {} ms",
            elapsed.as_millis()
        );
    }
}
