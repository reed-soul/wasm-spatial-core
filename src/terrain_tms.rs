//! TMS-layout quantized-mesh terrain pyramid + `layer.json` generator.
//!
//! Produces the artifacts a Cesium `CesiumTerrainProvider` (the *real* consumer
//! of quantized-mesh-1.0 tiles) can load: a `layer.json` describing the layer
//! plus a TMS `{z}/{x}/{y}.terrain` tile tree.
//!
//! This is distinct from `geotiff::encode_terrain_tileset_core`, which emits a
//! 3D Tiles 1.1 `tileset.json` of flat children — that layout is consumed by
//! `Cesium3DTileset`, which does NOT natively render quantized-mesh. The path
//! here is the one used by Cesium's terrain globe, and is what W3.6 acceptance
//! ("Cesium can actually load these tiles") is gated on.
//!
//! Spec references:
//! - https://github.com/CesiumGS/quantized-mesh (format + `layer.json` fields)
//! - `CesiumTerrainProvider` source (de-facto authority on `available` array
//!   semantics: each element corresponds to a zoom level, entries are
//!   `[startX, endX, startY, endY]` inclusive ranges).

use wasm_bindgen::prelude::*;

use crate::quantized_mesh::encode_quantized_mesh;

/// One TMS tile: relative path (`{z}/{x}/{y}.terrain`) + spec-conformant bytes.
#[derive(Clone, Debug)]
pub struct TmsTile {
    /// Relative path under the layer root, e.g. `"0/0/0.terrain"` or `"1/0/1.terrain"`.
    pub path: String,
    /// Spec-conformant quantized-mesh-1.0 byte stream.
    pub bytes: Vec<u8>,
}

/// Result of building a TMS terrain pyramid: the `layer.json` payload + all tiles.
#[derive(Clone, Debug)]
pub struct TmsPyramidResult {
    /// `layer.json` contents, ready to write to disk verbatim.
    pub layer_json: String,
    /// All tiles in the pyramid (zoom 0 root + zoom 1 quadrants).
    pub tiles: Vec<TmsTile>,
}

/// ECEF center passed straight through to the per-tile encoder.
///
/// The bounds-midpoint + mean-height centroid is what the existing demo and
/// `geotiff::encode_terrain_tileset_core` use; we follow the same convention
/// so tiles from either path look identical to Cesium.
fn bounds_center_ecef(bounds: &[f64; 4], heights: &[f32]) -> [f64; 3] {
    let mid_lng = (bounds[0] + bounds[2]) * 0.5;
    let mid_lat = (bounds[1] + bounds[3]) * 0.5;
    let mean_h = if heights.is_empty() {
        0.0
    } else {
        heights.iter().map(|&h| h as f64).sum::<f64>() / heights.len() as f64
    };
    let (x, y, z) = crate::cesium_adapter::wgs84_to_cartesian3_single(mid_lng, mid_lat, mean_h);
    [x, y, z]
}

/// Split a row-major `width × height` grid into 4 quadrants (NW / NE / SW / SE)
/// by taking every other sample (matches the 2× nearest-neighbour downsample
/// used by `geotiff::encode_terrain_tileset_core`).
///
/// Returns `(sub_heights, sub_w, sub_h, sub_bounds)` for each quadrant in the
/// order `[NW, NE, SW, SE]`. TMS y increases southward — so `y=0` is the
/// **northern** half. Each sub-grid covers the corresponding quarter bounds.
fn split_grid_quadrants(
    heights: &[f32],
    width: usize,
    height: usize,
    bounds: &[f64; 4],
) -> [(Vec<f32>, usize, usize, [f64; 4]); 4] {
    // Half-open split: index < mid is the low (west / north) half.
    let mid_x = width.div_ceil(2);
    let mid_y = height.div_ceil(2);
    let sub_w = mid_x;
    let sub_h = mid_y;

    let min_lng = bounds[0];
    let max_lng = bounds[2];
    let min_lat = bounds[1];
    let max_lat = bounds[3];
    let mid_lng = (min_lng + max_lng) * 0.5;
    let mid_lat = (min_lat + max_lat) * 0.5;

    // Quadrant bounds (lng/lat).
    // NW: x∈[0, mid_x), y∈[0, mid_y)   →  lng [min, mid), lat [mid, max)
    // NE: x∈[mid_x, w), y∈[0, mid_y)   →  lng [mid, max), lat [mid, max)
    // SW: x∈[0, mid_x), y∈[mid_y, h)   →  lng [min, mid), lat [min, mid)
    // SE: x∈[mid_x, w), y∈[mid_y, h)   →  lng [mid, max), lat [min, mid)
    let q_bounds: [[f64; 4]; 4] = [
        [min_lng, mid_lat, mid_lng, max_lat], // NW
        [mid_lng, mid_lat, max_lng, max_lat], // NE
        [min_lng, min_lat, mid_lng, mid_lat], // SW
        [mid_lng, min_lat, max_lng, mid_lat], // SE
    ];
    let x_offsets = [0_usize, mid_x, 0, mid_x];
    let y_offsets = [0_usize, 0, mid_y, mid_y];

    let mut out: [(Vec<f32>, usize, usize, [f64; 4]); 4] = [
        (Vec::new(), sub_w, sub_h, q_bounds[0]),
        (Vec::new(), sub_w, sub_h, q_bounds[1]),
        (Vec::new(), sub_w, sub_h, q_bounds[2]),
        (Vec::new(), sub_w, sub_h, q_bounds[3]),
    ];

    for q in 0..4 {
        let xo = x_offsets[q];
        let yo = y_offsets[q];
        let cells = sub_w * sub_h;
        out[q].0.reserve(cells);
        for row in 0..sub_h {
            for col in 0..sub_w {
                let sx = xo + col;
                let sy = yo + row;
                // Clamp to source grid in case of an odd-sized dimension —
                // pick the nearest valid sample rather than panicking.
                let sx = sx.min(width - 1);
                let sy = sy.min(height - 1);
                out[q].0.push(heights[sy * width + sx]);
            }
        }
    }
    out
}

/// Build a spec-conformant `layer.json` for the given bounds + zoom range.
///
/// Field semantics (verified against Cesium 1.119 `CesiumTerrainProvider.js`
/// `parseMetadataSuccess`):
/// - `tiles`: array of URL templates. Required (Cesium throws if missing).
///   We emit `"{z}/{x}/{y}.terrain"` matching the on-disk TMS layout.
/// - `scheme`: `"tms"` (Cesium y-flips; our generator already writes TMS y).
/// - `available`: per-zoom list of objects `{startX, endX, startY, endY}`
///   (NOT arrays — Cesium reads `range.startX` etc. as properties). Used to
///   build `TileAvailability`, which gates `sampleTerrainMostDetailed`.
fn build_layer_json(bounds: &[f64; 4], max_zoom: u32) -> String {
    let min_lng = bounds[0];
    let min_lat = bounds[1];
    let max_lng = bounds[2];
    let max_lat = bounds[3];

    // Per-zoom available ranges as OBJECTS (Cesium reads .startX/.endX/...).
    // For our fixture pyramid, zoom z has 2^z × 2^z tiles, all present.
    let available: Vec<serde_json::Value> = (0..=max_zoom)
        .map(|z| {
            let span: i64 = (1_i64 << z) - 1; // inclusive end
            serde_json::json!([{ "startX": 0, "endX": span, "startY": 0, "endY": span }])
        })
        .collect();

    serde_json::json!({
        "tilejson": "1.0.0",
        "format": "quantized-mesh-1.0",
        "version": "1.0.0",
        "name": "wasm-spatial-core generated terrain",
        "description": "W3.6 acceptance test terrain (CesiumTerrainProvider)",
        "attribution": "",
        "projection": "EPSG:4326",
        "scheme": "tms",
        "bounds": [min_lng, min_lat, max_lng, max_lat],
        "minzoom": 0,
        "maxzoom": max_zoom,
        "tiles": ["{z}/{x}/{y}.terrain"],
        "available": available,
    })
    .to_string()
}

/// Generate a TMS quantized-mesh terrain pyramid with a `layer.json`.
///
/// Produces:
/// - zoom 0: one tile covering the full `bounds`, encoded from the full grid.
/// - zoom 1: four quadrant tiles (NW / NE / SW / SE), each encoded from the
///   corresponding sub-grid.
///
/// Higher zoom levels are intentionally not supported — this exists to prove
/// spec compliance for the W3.6 acceptance test, not to be a general terrain
/// tiler. `max_zoom` is clamped to 1.
///
/// # Arguments
/// * `heights` — flat row-major `width × height` elevation array.
/// * `width`, `height` — grid dimensions in samples.
/// * `bounds` — `[min_lng, min_lat, max_lng, max_lat]`.
/// * `center` — optional ECEF sphere-center; if `[0,0,0]` is passed we derive
///   one from the bounds + mean height (consistent with the existing demo).
/// * `max_zoom` — clamped to 1 (the only pyramid depth this generator emits).
pub fn encode_terrain_tms_pyramid(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64; 4],
    center: Option<&[f64; 3]>,
    max_zoom: u32,
) -> Result<TmsPyramidResult, String> {
    if width < 2 || height < 2 {
        return Err("TerrainTms: grid must be at least 2×2".into());
    }
    if heights.len() != (width as usize) * (height as usize) {
        return Err(format!(
            "TerrainTms: heights length {} != width×height {}",
            heights.len(),
            (width as usize) * (height as usize)
        ));
    }
    let w = width as usize;
    let h = height as usize;

    // Clamp max_zoom to 1 — deeper pyramids aren't generated here.
    let max_zoom = max_zoom.min(1);

    // Resolve ECEF center: explicit override, else derive from bounds + heights.
    let derived = bounds_center_ecef(bounds, heights);
    let center: [f64; 3] = match center {
        Some(c) if *c != [0.0, 0.0, 0.0] => *c,
        _ => derived,
    };

    let mut tiles = Vec::new();

    // --- zoom 0: single root tile ---
    let root_bytes = encode_quantized_mesh(heights, width, height, bounds, &center)?;
    tiles.push(TmsTile {
        path: "0/0/0.terrain".to_string(),
        bytes: root_bytes,
    });

    if max_zoom >= 1 {
        // --- zoom 1: four quadrants ---
        // TMS quadrant → (x, y) coordinates:
        //   NW = (0, 0)   NE = (1, 0)
        //   SW = (0, 1)   SE = (1, 1)
        let quads = split_grid_quadrants(heights, w, h, bounds);
        // quads order is [NW, NE, SW, SE]; map to (x, y).
        let tms_xy = [(0_u32, 0_u32), (1, 0), (0, 1), (1, 1)];
        for (i, (sub_heights, sub_w, sub_h, sub_bounds)) in quads.into_iter().enumerate() {
            let bytes = encode_quantized_mesh(
                &sub_heights,
                sub_w as u32,
                sub_h as u32,
                &sub_bounds,
                &center,
            )?;
            let (x, y) = tms_xy[i];
            tiles.push(TmsTile {
                path: format!("1/{x}/{y}.terrain"),
                bytes,
            });
        }
    }

    let layer_json = build_layer_json(bounds, max_zoom);
    Ok(TmsPyramidResult { layer_json, tiles })
}

// =============================================================================
// WASM bindings — mirrors the TerrainTilesetResult pattern from geotiff.rs.
// Exposed as a separate type so JS callers can distinguish the TMS/layer.json
// pyramid (for CesiumTerrainProvider) from the 3D-Tiles tileset (for
// Cesium3DTileset, which does NOT natively render quantized-mesh).
// =============================================================================

/// WASM-facing wrapper around `TmsPyramidResult`.
///
/// JS accessors mirror `TerrainTilesetResult`: `layerJson`, `tileCount`,
/// `tilePath(i)`, `tile(i)`, `totalBytes`.
#[wasm_bindgen]
pub struct WasmTmsPyramid {
    layer_json: String,
    tiles: Vec<TmsTile>,
}

#[wasm_bindgen]
impl WasmTmsPyramid {
    /// The `layer.json` contents, ready to write to disk verbatim.
    #[wasm_bindgen(getter, js_name = "layerJson")]
    pub fn layer_json(&self) -> String {
        self.layer_json.clone()
    }

    /// Number of tiles in the pyramid (1 + 4 = 5 for the default zoom 0–1 layout).
    #[wasm_bindgen(getter, js_name = "tileCount")]
    pub fn tile_count(&self) -> u32 {
        self.tiles.len() as u32
    }

    /// Relative TMS path (`{z}/{x}/{y}.terrain`) of the tile at `index`.
    #[wasm_bindgen(js_name = "tilePath")]
    pub fn tile_path(&self, index: usize) -> String {
        self.tiles
            .get(index)
            .map(|t| t.path.clone())
            .unwrap_or_default()
    }

    /// Spec-conformant quantized-mesh-1.0 bytes of the tile at `index`.
    #[wasm_bindgen]
    pub fn tile(&self, index: usize) -> js_sys::Uint8Array {
        match self.tiles.get(index) {
            Some(t) => {
                let arr = js_sys::Uint8Array::new_with_length(t.bytes.len() as u32);
                arr.copy_from(&t.bytes);
                arr
            }
            None => js_sys::Uint8Array::new_with_length(0),
        }
    }

    /// Total bytes across all tiles (debug/info accessor).
    #[wasm_bindgen(getter, js_name = "totalBytes")]
    pub fn total_bytes(&self) -> usize {
        self.tiles.iter().map(|t| t.bytes.len()).sum()
    }
}

/// JS entry point — build a TMS quantized-mesh terrain pyramid.
///
/// # Arguments (JS)
/// * `heights: Float32Array` — row-major elevation grid.
/// * `width: u32`, `height: u32` — grid dimensions in samples.
/// * `bounds: Float64Array | number[]` — `[min_lng, min_lat, max_lng, max_lat]`.
///   Accepts a `Float64Array` view or a plain JS array (length 4).
/// * `center: Float64Array | number[]` — ECEF `[x, y, z]`. Pass an empty array
///   (or any length != 3) to auto-derive from bounds + mean height.
/// * `maxZoom: u32` — clamped to 1 (deeper pyramids not generated here).
#[wasm_bindgen(js_name = "encodeTerrainTmsPyramid")]
pub fn encode_terrain_tms_pyramid_js(
    heights: &[f32],
    width: u32,
    height: u32,
    bounds: &[f64],
    center: &[f64],
    max_zoom: u32,
) -> Result<WasmTmsPyramid, JsValue> {
    if bounds.len() != 4 {
        return Err(JsValue::from_str(&format!(
            "encodeTerrainTmsPyramid: bounds must have length 4, got {}",
            bounds.len()
        )));
    }
    // An empty center (length != 3) means "derive it for me". This avoids
    // needing `Option<&[f64]>` in the wasm-bindgen signature (which isn't
    // supported), while keeping the JS ergonomics — callers pass `new
    // Float64Array(0)` or `[]` to opt into auto-derivation.
    let center_arr: Option<[f64; 3]> = if center.len() == 3 {
        Some([center[0], center[1], center[2]])
    } else {
        None
    };
    let bounds_arr = [bounds[0], bounds[1], bounds[2], bounds[3]];
    let result = encode_terrain_tms_pyramid(
        heights,
        width,
        height,
        &bounds_arr,
        center_arr.as_ref(),
        max_zoom,
    )
    .map_err(|e| {
        let err = crate::errors::SpatialError::parse_error(e);
        JsValue::from(err)
    })?;
    Ok(WasmTmsPyramid {
        layer_json: result.layer_json,
        tiles: result.tiles,
    })
}

// =============================================================================
// Tests — pure Rust coverage; the Playwright test covers the JS/Cesium path.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantized_mesh::decode_index_stream;
    use serde_json::Value;

    fn fixture_heights_8x8() -> (Vec<f32>, u32, u32, [f64; 4]) {
        // 8x8 with a gentle ramp — small enough to reason about, large enough
        // that split_grid_quadrants yields non-trivial 4x4 children.
        let heights = (0..64).map(|i| i as f32 * 5.0).collect();
        (heights, 8, 8, [120.0, 30.0, 120.1, 30.1])
    }

    #[test]
    fn test_layer_json_fields() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
        let v: Value = serde_json::from_str(&pyramid.layer_json).expect("layer.json parses");

        assert_eq!(v["tilejson"], "1.0.0");
        assert_eq!(v["format"], "quantized-mesh-1.0");
        assert_eq!(v["version"], "1.0.0");
        assert_eq!(v["minzoom"], 0);
        assert_eq!(v["maxzoom"], 1);
        assert_eq!(v["projection"], "EPSG:4326");
        assert_eq!(
            v["bounds"].as_array().unwrap().len(),
            4,
            "bounds must be a 4-array"
        );
    }

    #[test]
    fn test_layer_json_available_array_includes_all_tiles() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
        let v: Value = serde_json::from_str(&pyramid.layer_json).unwrap();
        let available = v["available"].as_array().expect("available is array");
        assert_eq!(available.len(), 2, "one entry per zoom level (0 and 1)");

        // zoom 0: a single range object covering the lone tile (0,0).
        let z0 = &available[0];
        assert_eq!(
            z0[0],
            serde_json::json!({ "startX": 0, "endX": 0, "startY": 0, "endY": 0 })
        );

        // zoom 1: a single range object covering the 2×2 quadrant tiles.
        let z1 = &available[1];
        assert_eq!(
            z1[0],
            serde_json::json!({ "startX": 0, "endX": 1, "startY": 0, "endY": 1 })
        );
    }

    #[test]
    fn test_layer_json_has_required_tiles_template() {
        // Cesium's CesiumTerrainProvider throws if `tiles` is missing or empty.
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
        let v: Value = serde_json::from_str(&pyramid.layer_json).unwrap();
        let tiles = v["tiles"].as_array().expect("tiles is array");
        assert!(!tiles.is_empty(), "tiles template array must be non-empty");
        assert!(
            tiles[0].as_str().unwrap().contains("{z}")
                && tiles[0].as_str().unwrap().contains("{x}")
                && tiles[0].as_str().unwrap().contains("{y}"),
            "tiles[0] must be a {{z}}/{{x}}/{{y}} template, got: {}",
            tiles[0]
        );
    }

    #[test]
    fn test_tms_pyramid_tile_count_and_paths() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();
        // 1 root + 4 quadrants = 5 tiles.
        assert_eq!(pyramid.tiles.len(), 5);

        let paths: Vec<&str> = pyramid.tiles.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"0/0/0.terrain"), "root tile present");
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let expected = format!("1/{x}/{y}.terrain");
            assert!(
                paths.contains(&expected.as_str()),
                "quadrant tile {expected} present"
            );
        }
    }

    #[test]
    fn test_tms_pyramid_max_zoom_clamped() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        // Caller asks for zoom 6 — generator clamps to 1.
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 6).unwrap();
        assert_eq!(pyramid.tiles.len(), 5, "only zoom 0 + zoom 1 emitted");
    }

    /// Decode the triangle-count field from a quantized-mesh byte stream to
    /// confirm the encoder produced a non-empty index block.
    fn triangle_count(bytes: &[u8]) -> u32 {
        assert!(bytes.len() > 88, "tile must include 88-byte header");
        let vertex_count = u32::from_le_bytes(bytes[88..92].try_into().unwrap());
        // VertexData = 12 bytes (3 count u32s) + vertex_count × 6 bytes (3 u16).
        let idx_off = 88 + 12 + (vertex_count as usize) * 6;
        let tri_count = u32::from_le_bytes(bytes[idx_off..idx_off + 4].try_into().unwrap());
        // Sanity-check the encoded stream decodes to valid indices.
        let mut enc = Vec::with_capacity(tri_count as usize * 3);
        let mut o = idx_off + 4;
        for _ in 0..(tri_count * 3) {
            enc.push(u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()));
            o += 4;
        }
        let decoded = decode_index_stream(&enc);
        assert!(
            decoded.iter().all(|&i| i < vertex_count),
            "decoded indices must be within vertex range"
        );
        tri_count
    }

    #[test]
    fn test_tms_pyramid_tile_bytes_are_valid() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let pyramid = encode_terrain_tms_pyramid(&heights, w, h, &bounds, None, 1).unwrap();

        for tile in &pyramid.tiles {
            assert!(
                tile.bytes.len() > 88,
                "{}: tile must extend past header ({} bytes)",
                tile.path,
                tile.bytes.len()
            );
            let tris = triangle_count(&tile.bytes);
            assert!(tris > 0, "{}: tile must encode ≥1 triangle", tile.path);
        }
    }

    #[test]
    fn test_split_grid_quadrants_bounds_partition() {
        let (heights, w, h, bounds) = fixture_heights_8x8();
        let quads = split_grid_quadrants(&heights, w as usize, h as usize, &bounds);

        // Sub-grids should each be 4×4 (half of 8×8).
        for (i, (_, sw_, sh_, _)) in quads.iter().enumerate() {
            assert_eq!(*sw_, 4, "quadrant {i} width");
            assert_eq!(*sh_, 4, "quadrant {i} height");
        }

        // Bounds partition: union of all four quadrants == original bounds.
        let all = [&quads[0], &quads[1], &quads[2], &quads[3]];
        let min_lng = all
            .iter()
            .map(|(_, _, _, b)| b[0])
            .fold(f64::INFINITY, f64::min);
        let max_lng = all
            .iter()
            .map(|(_, _, _, b)| b[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_lat = all
            .iter()
            .map(|(_, _, _, b)| b[1])
            .fold(f64::INFINITY, f64::min);
        let max_lat = all
            .iter()
            .map(|(_, _, _, b)| b[3])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((min_lng - bounds[0]).abs() < 1e-9);
        assert!((max_lng - bounds[2]).abs() < 1e-9);
        assert!((min_lat - bounds[1]).abs() < 1e-9);
        assert!((max_lat - bounds[3]).abs() < 1e-9);

        // Sample value check: NW quadrant's first cell is source (0,0).
        assert_eq!(quads[0].0[0], heights[0]);
        // SE quadrant's last cell is source (w-1, h-1) — but with 2× downsample
        // we sample at stride 2, so the SE quadrant's (3,3) maps to source
        // (mid_x + 3 clamped, mid_y + 3 clamped) = (7, 7).
        assert_eq!(quads[3].0[15], heights[63]);
    }

    #[test]
    fn test_rejects_bad_grid_dimensions() {
        let r = encode_terrain_tms_pyramid(&[1.0], 1, 1, &[0.0, 0.0, 1.0, 1.0], None, 1);
        assert!(r.is_err());

        // Length mismatch
        let r = encode_terrain_tms_pyramid(&[1.0; 5], 2, 2, &[0.0, 0.0, 1.0, 1.0], None, 1);
        assert!(r.is_err());
    }
}
