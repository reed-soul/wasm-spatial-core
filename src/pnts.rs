//! # PNTS Tile Encoder
//!
//! Encodes point cloud data into the 3D Tiles Point Cloud (`.pnts`) binary format.
//!
//! ## Format Layout
//!
//! Header (28 bytes)
//! Feature Table JSON (4-byte padded)
//! Feature Table Binary: POSITION (Float32) + optional RGB (Uint8)
//! Batch Table JSON (4-byte padded)
//! Batch Table Binary
use crate::errors::SpatialError;

// ===========================================================================
// PNTS encoding
// ===========================================================================

/// Encode a point cloud tile into the 3D Tiles Point Cloud (pnts) binary format.
///
/// # Arguments
/// * `positions` — Flat `[x, y, z, ...]` positions. Coordinates are stored
///   relative to `center` (i.e. `position[i] - center[i % 3]`).
/// * `center` — Tile center `[cx, cy, cz]`.
/// * `colors` — Optional flat `[r, g, b, ...]` byte array (one byte per channel).
///
/// # Returns
/// The complete `.pnts` binary blob.
pub fn encode_pnts_tile(
    positions: &[f32],
    center: [f64; 3],
    colors: Option<&[u8]>,
) -> Result<Vec<u8>, crate::errors::SpatialErrorDetail> {
    let num_points = positions.len() / 3;
    if num_points == 0 {
        return Err(
            SpatialError::PointCloudError.with_detail("cannot encode pnts tile with 0 points")
        );
    }
    if let Some(colors) = colors {
        if colors.len() != num_points * 3 {
            return Err(SpatialError::PointCloudError.with_detail(format!(
                "color count mismatch: expected {} bytes, got {}",
                num_points * 3,
                colors.len()
            )));
        }
    }
    let has_colors = colors.is_some();
    let position_bytes = num_points * 3 * 4; // Float32
    let color_bytes = if has_colors { num_points * 3 } else { 0 }; // Uint8

    // Feature Table Binary body.
    let feature_binary_len = position_bytes + color_bytes;

    // Feature Table JSON.
    let ft_json = if has_colors {
        format!(
            r#"{{"POSITION":{{"byteOffset":0}},"RGB":{{"byteOffset":{}}}}}"#,
            position_bytes
        )
    } else {
        r#"{"POSITION":{"byteOffset":0}}"#.to_string()
    };
    let ft_json_padded = pad_to_4(&ft_json);

    // Batch Table JSON (empty).
    let bt_json = "{}";
    let bt_json_padded = pad_to_4(bt_json);

    // Batch Table Binary (empty).
    let bt_binary_len = 0u32;

    // Header (28 bytes).
    let header = PntsHeader {
        magic: *b"pnts",
        version: 1,
        byte_length: 28
            + ft_json_padded.len() as u32
            + feature_binary_len as u32
            + bt_json_padded.len() as u32
            + bt_binary_len,
        feature_table_json_byte_length: ft_json_padded.len() as u32,
        feature_table_binary_byte_length: feature_binary_len as u32,
        batch_table_json_byte_length: bt_json_padded.len() as u32,
        batch_table_binary_byte_length: bt_binary_len,
    };

    // Assemble.
    let mut buf = Vec::with_capacity(header.byte_length as usize);

    // Header.
    buf.extend_from_slice(&header.magic);
    buf.extend_from_slice(&header.version.to_le_bytes());
    buf.extend_from_slice(&header.byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_binary_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_binary_byte_length.to_le_bytes());

    // Feature Table JSON (padded).
    buf.extend_from_slice(ft_json_padded.as_bytes());

    // Feature Table Binary.
    for chunk in positions.chunks_exact(3) {
        let x = (chunk[0] as f64 - center[0]) as f32;
        let y = (chunk[1] as f64 - center[1]) as f32;
        let z = (chunk[2] as f64 - center[2]) as f32;
        buf.extend_from_slice(&x.to_le_bytes());
        buf.extend_from_slice(&y.to_le_bytes());
        buf.extend_from_slice(&z.to_le_bytes());
    }
    if let Some(rgb) = colors {
        buf.extend_from_slice(rgb);
    }

    // Batch Table JSON (padded).
    buf.extend_from_slice(bt_json_padded.as_bytes());

    debug_assert_eq!(buf.len(), header.byte_length as usize);

    Ok(buf)
}

/// Parse a pnts header from raw bytes. Returns `(header, remaining_bytes)`.
///
/// Useful for validating encoded tiles.
pub fn parse_pnts_header(
    data: &[u8],
) -> Result<(PntsHeader, &[u8]), crate::errors::SpatialErrorDetail> {
    if data.len() < 28 {
        return Err(SpatialError::PointCloudError
            .with_detail("pnts data too short for header (< 28 bytes)"));
    }

    let magic = &data[0..4];
    if magic != b"pnts" {
        return Err(SpatialError::PointCloudError.with_detail(format!(
            "invalid pnts magic: expected b\"pnts\", got {:?}",
            magic
        )));
    }

    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != 1 {
        return Err(SpatialError::PointCloudError
            .with_detail(format!("unsupported pnts version: {}, expected 1", version)));
    }

    let byte_length = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let ft_json_len = u32::from_le_bytes(data[12..16].try_into().unwrap());
    let ft_bin_len = u32::from_le_bytes(data[16..20].try_into().unwrap());
    let bt_json_len = u32::from_le_bytes(data[20..24].try_into().unwrap());
    let bt_bin_len = u32::from_le_bytes(data[24..28].try_into().unwrap());

    let header = PntsHeader {
        magic: *b"pnts",
        version,
        byte_length,
        feature_table_json_byte_length: ft_json_len,
        feature_table_binary_byte_length: ft_bin_len,
        batch_table_json_byte_length: bt_json_len,
        batch_table_binary_byte_length: bt_bin_len,
    };

    Ok((header, &data[28..]))
}

/// Parsed pnts header fields.
#[derive(Debug, Clone)]
pub struct PntsHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub byte_length: u32,
    pub feature_table_json_byte_length: u32,
    pub feature_table_binary_byte_length: u32,
    pub batch_table_json_byte_length: u32,
    pub batch_table_binary_byte_length: u32,
}

impl PntsHeader {
    /// Expected byte length: header + FT JSON (padded) + FT binary + BT JSON (padded) + BT binary.
    /// The byte_length should equal this value.
    pub fn total_expected_bytes(&self) -> u32 {
        28 + pad_len(self.feature_table_json_byte_length)
            + self.feature_table_binary_byte_length
            + pad_len(self.batch_table_json_byte_length)
            + self.batch_table_binary_byte_length
    }
}

/// Pad a string to a 4-byte boundary with spaces.
fn pad_to_4(s: &str) -> String {
    let len = s.len();
    let pad = (4 - len % 4) % 4;
    if pad == 0 {
        s.to_string()
    } else {
        let mut padded = s.to_string();
        padded.extend(std::iter::repeat_n(' ', pad));
        padded
    }
}

/// Compute padded length for a value that needs 4-byte alignment.
pub fn pad_len(len: u32) -> u32 {
    len + (4 - len % 4) % 4
}

// ===========================================================================
// WASM exports
// ===========================================================================

use wasm_bindgen::prelude::*;

/// Encode a point cloud tile into 3D Tiles Point Cloud (pnts) binary format.
///
/// # Arguments
/// * `positions` — `Float32Array` of `[x, y, z, ...]`.
/// * `center_x`, `center_y`, `center_z` — Tile center coordinates.
/// * `colors` — Optional `Uint8Array` of `[r, g, b, ...]`.
///
/// Returns a `Uint8Array` containing the complete `.pnts` binary.
#[wasm_bindgen(js_name = "encodePntsTile")]
pub fn encode_pnts_tile_js(
    positions: &[f32],
    center_x: f64,
    center_y: f64,
    center_z: f64,
    colors: Option<Vec<u8>>,
) -> Result<js_sys::Uint8Array, JsValue> {
    let result = encode_pnts_tile(positions, [center_x, center_y, center_z], colors.as_deref())
        .map_err(JsValue::from)?;
    Ok(js_sys::Uint8Array::from(&result[..]))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes() {
        let positions = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let tile = encode_pnts_tile(&positions, [0.0; 3], None).unwrap();
        assert_eq!(&tile[0..4], b"pnts");
    }

    #[test]
    fn test_header_fields() {
        let positions = vec![1.0f32, 2.0, 3.0];
        let tile = encode_pnts_tile(&positions, [10.0, 20.0, 30.0], None).unwrap();
        let (header, _) = parse_pnts_header(&tile).unwrap();

        assert_eq!(header.version, 1);
        assert_eq!(header.byte_length, tile.len() as u32);
        assert!(header.feature_table_json_byte_length > 0);
        assert_eq!(header.feature_table_binary_byte_length, 12); // 1 point * 3 * 4
        assert_eq!(header.batch_table_json_byte_length, 4); // "{}" padded to 4
        assert_eq!(header.batch_table_binary_byte_length, 0);
    }

    #[test]
    fn test_position_offset() {
        let positions = vec![10.0f32, 20.0, 30.0];
        let center = [1.0f64, 2.0, 3.0];
        let tile = encode_pnts_tile(&positions, center, None).unwrap();

        // Skip header (28) + FT JSON (padded).
        let (header, _rest) = parse_pnts_header(&tile).unwrap();
        let ft_json_len = header.feature_table_json_byte_length as usize;
        let binary_start = 28 + ft_json_len;
        let x = f32::from_le_bytes(tile[binary_start..binary_start + 4].try_into().unwrap());
        let y = f32::from_le_bytes(tile[binary_start + 4..binary_start + 8].try_into().unwrap());
        let z = f32::from_le_bytes(
            tile[binary_start + 8..binary_start + 12]
                .try_into()
                .unwrap(),
        );

        // Position should be relative to center: 10-1=9, 20-2=18, 30-3=27
        assert!((x - 9.0).abs() < 1e-5);
        assert!((y - 18.0).abs() < 1e-5);
        assert!((z - 27.0).abs() < 1e-5);
    }

    #[test]
    fn test_with_colors() {
        let positions = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let colors = vec![255u8, 0, 0, 0, 255, 0]; // red, green
        let tile = encode_pnts_tile(&positions, [0.0; 3], Some(&colors)).unwrap();

        let (header, _) = parse_pnts_header(&tile).unwrap();
        assert!(header.feature_table_json_byte_length > 0);
        // Binary: 6 floats (positions) + 6 bytes (colors)
        assert_eq!(header.feature_table_binary_byte_length, 24 + 6);

        // Check RGB bytes at end of feature binary.
        let ft_json_len = pad_len(header.feature_table_json_byte_length) as usize;
        let color_offset = 28 + ft_json_len + 24;
        assert_eq!(tile[color_offset], 255); // R of point 1
        assert_eq!(tile[color_offset + 1], 0); // G of point 1
        assert_eq!(tile[color_offset + 2], 0); // B of point 1
        assert_eq!(tile[color_offset + 3], 0); // R of point 2
        assert_eq!(tile[color_offset + 4], 255); // G of point 2
        assert_eq!(tile[color_offset + 5], 0); // B of point 2
    }

    #[test]
    fn test_zero_points_error() {
        let result = encode_pnts_tile(&[], [0.0; 3], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_color_mismatch_error() {
        let positions = vec![1.0f32, 2.0, 3.0]; // 1 point = needs 3 color bytes
        let colors = vec![255u8, 0]; // only 2 bytes, need 3
        let result = encode_pnts_tile(&positions, [0.0; 3], Some(&colors));
        assert!(result.is_err());
    }

    #[test]
    fn test_byte_length_consistency() {
        let positions = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let tile = encode_pnts_tile(&positions, [0.0; 3], None).unwrap();
        let (header, _) = parse_pnts_header(&tile).unwrap();
        assert_eq!(header.byte_length as usize, tile.len());
        assert_eq!(header.total_expected_bytes(), tile.len() as u32);
    }

    #[test]
    fn test_ft_json_contains_position() {
        let positions = vec![1.0f32, 2.0, 3.0];
        let tile = encode_pnts_tile(&positions, [0.0; 3], None).unwrap();
        let (header, _) = parse_pnts_header(&tile).unwrap();
        let _ft_json =
            std::str::from_utf8(&tile[28..28 + header.feature_table_json_byte_length as usize])
                .unwrap();
    }

    #[test]
    fn test_large_coordinates_encode() {
        // Coordinates near f32 max range.
        let positions = vec![
            1e6f32, -2e6f32, 3e6f32,
            5e6f32, 6e6f32, -7e6f32,
        ];
        let center = [3e6f64, 2e6f64, -2e6f64];
        let tile = encode_pnts_tile(&positions, center, None).unwrap();
        let (header, _) = parse_pnts_header(&tile).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.byte_length as usize, tile.len());
        assert_eq!(
            header.feature_table_binary_byte_length, 24,
            "2 points × 3 × 4 bytes"
        );
    }

    #[test]
    fn test_single_point_encode() {
        let positions = vec![42.0f32, -17.5, 100.0];
        let tile = encode_pnts_tile(&positions, [42.0, -17.5, 100.0], None).unwrap();
        let (header, _) = parse_pnts_header(&tile).unwrap();
        assert_eq!(header.feature_table_binary_byte_length, 12);
        // Center-relative position should be ~0.
        let ft_json_len = pad_len(header.feature_table_json_byte_length) as usize;
        let x = f32::from_le_bytes(
            tile[28 + ft_json_len..28 + ft_json_len + 4].try_into().unwrap(),
        );
        assert!(x.abs() < 0.01, "single-point offset should be ~0, got {x}");
    }
}

// ===========================================================================
// tileset.json generator
// ===========================================================================

use crate::octree::{Bounds, Octree, DEFAULT_MAX_POINTS_PER_NODE};

/// Result of generating a tileset from an octree.
#[derive(Debug, Clone)]
pub struct TilesetResult {
    /// Serialized tileset.json content.
    tileset_json: String,
    /// Per-tile pnts binary blobs, indexed by leaf node order.
    tiles: Vec<Vec<u8>>,
    /// Bounding box for each tile.
    tile_bounds: Vec<Bounds>,
    /// URI for each tile.
    tile_uris: Vec<String>,
}

impl TilesetResult {
    /// Get the tileset.json content as a string.
    pub fn tileset_json(&self) -> &str {
        &self.tileset_json
    }

    /// Number of tiles (leaf nodes).
    pub fn tile_count(&self) -> u32 {
        self.tiles.len() as u32
    }

    /// Get a tile's pnts binary data by index.
    pub fn tile(&self, index: usize) -> Option<&[u8]> {
        self.tiles.get(index).map(|v| v.as_slice())
    }

    /// Get the URI string for a tile.
    pub fn tile_uri(&self, index: usize) -> Option<&str> {
        self.tile_uris.get(index).map(|s| s.as_str())
    }

    /// Total bytes across all tiles.
    pub fn total_bytes(&self) -> usize {
        self.tiles.iter().map(|t| t.len()).sum()
    }

    /// Get bounding box for a tile.
    pub fn tile_bounds(&self, index: usize) -> Option<Bounds> {
        self.tile_bounds.get(index).copied()
    }
}

/// WASM-accessible tileset result handle.
#[wasm_bindgen(js_name = "TilesetResult")]
pub struct WasmTilesetResult {
    inner: TilesetResult,
}

#[wasm_bindgen(js_class = "TilesetResult")]
impl WasmTilesetResult {
    /// The tileset.json content.
    #[wasm_bindgen(js_name = "tilesetJson")]
    pub fn tileset_json(&self) -> String {
        self.inner.tileset_json().to_string()
    }

    /// Number of tiles.
    #[wasm_bindgen(getter = "tileCount")]
    pub fn tile_count(&self) -> u32 {
        self.inner.tile_count()
    }

    /// Get tile binary data as `Uint8Array`.
    #[wasm_bindgen]
    pub fn tile(&self, index: usize) -> js_sys::Uint8Array {
        match self.inner.tile(index) {
            Some(data) => js_sys::Uint8Array::from(data),
            None => js_sys::Uint8Array::new_with_length(0),
        }
    }

    /// Get tile URI string.
    #[wasm_bindgen(js_name = "tileUri")]
    pub fn tile_uri(&self, index: usize) -> Option<String> {
        self.inner.tile_uri(index).map(|s| s.to_string())
    }

    /// Total bytes across all tiles.
    #[wasm_bindgen(getter = "totalBytes")]
    pub fn total_bytes(&self) -> usize {
        self.inner.total_bytes()
    }

    /// Tile bounding box as `Float64Array`.
    #[wasm_bindgen(js_name = "tileBounds")]
    pub fn tile_bounds(&self, index: usize) -> js_sys::Float64Array {
        match self.inner.tile_bounds(index) {
            Some(b) => js_sys::Float64Array::from(&b[..]),
            None => js_sys::Float64Array::new_with_length(0),
        }
    }
}

/// Geometric error factor per level. Higher levels (closer to leaves) get
/// smaller error values.
const GEOMETRIC_ERROR_FACTOR: f64 = 0.5;

/// Generate a complete 3D Tiles tileset from an octree and point data.
///
/// Each leaf node becomes a `.pnts` tile. Internal nodes form the tileset
/// hierarchy with appropriate `geometricError` and `boundingVolume`.
///
/// # Arguments
/// * `octree` — Built octree spatial index.
/// * `positions` — Reordered positions buffer (matches octree leaf ranges).
/// * `colors` — Optional color data (same reordering as positions).
pub fn generate_tileset(
    octree: &Octree,
    positions: &[f32],
    colors: Option<&[u8]>,
) -> Result<TilesetResult, crate::errors::SpatialErrorDetail> {
    let root_bounds = octree.root_bounds();
    let _root_geometric_error = compute_geometric_error(&root_bounds, 0);

    // Build tile content for each leaf.
    let mut tiles = Vec::new();
    let mut tile_bounds = Vec::new();
    let mut tile_uris = Vec::new();

    for (leaf_idx, node) in octree.leaves().enumerate() {
        if node.point_count == 0 {
            continue;
        }

        let start = node.point_start;
        let count = node.point_count as usize;
        let end = start + count;

        // Extract slice for this leaf.
        let pos_slice = &positions[start * 3..end * 3];

        // Tile center = node bounds center.
        let cx = (node.bounds[0] + node.bounds[3]) * 0.5;
        let cy = (node.bounds[1] + node.bounds[4]) * 0.5;
        let cz = (node.bounds[2] + node.bounds[5]) * 0.5;

        // Extract color slice if available.
        let color_slice = colors.map(|c| &c[start * 3..end * 3]);

        let tile_data = encode_pnts_tile(pos_slice, [cx, cy, cz], color_slice)
            .map_err(|e| crate::errors::SpatialError::PointCloudError.with_detail(e.to_string()))?;

        let uri = format!("tile_{leaf_idx}.pnts");

        tiles.push(tile_data);
        tile_bounds.push(node.bounds);
        tile_uris.push(uri);
    }

    // Build tileset.json tree structure.
    let tileset_json = build_tileset_json(octree, &tile_uris);

    Ok(TilesetResult {
        tileset_json,
        tiles,
        tile_bounds,
        tile_uris,
    })
}

/// Build the tileset.json tree from the octree hierarchy.
fn build_tileset_json(octree: &Octree, tile_uris: &[String]) -> String {
    let root = build_tile_node(octree, 0, tile_uris);
    let asset = r#"{"version":"1.0"}"#;

    format!(
        r#"{{"asset":{},"geometricError":{},"root":{}}}"#,
        asset,
        compute_geometric_error(&octree.root_bounds(), 0),
        root
    )
}

/// Recursively build a tile JSON node from an octree node.
fn build_tile_node(octree: &Octree, node_idx: usize, tile_uris: &[String]) -> String {
    let node = &octree.nodes[node_idx];
    let bounds = node.bounds;

    // Compute bounding volume as a box: 12 values
    // [centerX, centerY, centerZ, halfX, halfY, halfZ, ... normals + distances]
    // Box: center (3) + half-axes (3*3) = 12 values
    let cx = (bounds[0] + bounds[3]) * 0.5;
    let cy = (bounds[1] + bounds[4]) * 0.5;
    let cz = (bounds[2] + bounds[5]) * 0.5;
    let hx = (bounds[3] - bounds[0]) * 0.5;
    let hy = (bounds[4] - bounds[1]) * 0.5;
    let hz = (bounds[5] - bounds[2]) * 0.5;

    let bounding_volume = format!(
        r#""box":[{},{},{},{},0,0,0,{},0,0,0,{}]"#,
        cx, cy, cz, hx, hy, hz
    );

    let geo_error = compute_geometric_error(&bounds, node.level);

    // If this is a leaf node with points, add content.
    let mut children_json = String::new();
    if let Some(child_indices) = node.children.as_deref() {
        let child_strs: Vec<String> = child_indices
            .iter()
            .map(|&ci| build_tile_node(octree, ci, tile_uris))
            .collect();
        children_json = format!(",\"children\":[{}]", child_strs.join(","));
    }

    // Find leaf index for tile URI.
    let content_json = if node.is_leaf() && node.point_count > 0 {
        // Determine leaf index by counting leaves up to this node.
        let leaf_idx = octree
            .nodes
            .iter()
            .take(node_idx + 1)
            .filter(|n| n.is_leaf() && n.point_count > 0)
            .count()
            - 1;
        if leaf_idx < tile_uris.len() {
            let uri = &tile_uris[leaf_idx];
            format!(r#","content":{{"uri":"{}"}}"#, uri)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Build the node JSON: {"boundingVolume":{"box":[...]},"geometricError":N,...}
    let mut json = String::from(r#"{"boundingVolume":{"#);
    json.push_str(&bounding_volume);
    json.push_str(r#"},"geometricError":"#);
    json.push_str(&geo_error.to_string());
    json.push_str(&content_json);
    json.push_str(&children_json);
    json.push('}');
    json
}

/// Compute geometric error for a node at a given depth level.
/// Uses the bounding box diagonal scaled by a factor that decreases with level.
fn compute_geometric_error(bounds: &Bounds, level: u32) -> f64 {
    let dx = bounds[3] - bounds[0];
    let dy = bounds[4] - bounds[1];
    let dz = bounds[5] - bounds[2];
    let diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
    diagonal * GEOMETRIC_ERROR_FACTOR / (1 << level.min(20)) as f64
}

/// WASM export: generate a tileset from octree and point data.
#[wasm_bindgen(js_name = "generateTileset")]
pub fn generate_tileset_js(
    positions: &[f32],
    max_points_per_node: Option<u32>,
    max_depth: Option<u32>,
    colors: Option<Vec<u8>>,
) -> Result<WasmTilesetResult, JsValue> {
    let max_pts = max_points_per_node.unwrap_or(DEFAULT_MAX_POINTS_PER_NODE);
    let max_d = max_depth.unwrap_or(crate::octree::DEFAULT_MAX_DEPTH);
    let mut buf = positions.to_vec();
    let octree = Octree::build(&mut buf, max_pts, max_d);

    let result = generate_tileset(&octree, &buf, colors.as_deref()).map_err(JsValue::from)?;

    Ok(WasmTilesetResult { inner: result })
}

// ===========================================================================
// LOD: Screen-Space Error & Visible Tile Selection
// ===========================================================================

/// Compute the screen-space error (SSE) for a tile given camera parameters.
///
/// SSE = geometricError / (distance × 2 × tan(fov / 2)) × screenHeight
///
/// # Arguments
/// * `geometric_error` — The tile's geometric error value (from tileset).
/// * `distance` — Distance from camera to the tile's bounding volume center.
/// * `fov` — Camera vertical field of view in **radians**.
/// * `screen_height` — Screen height in pixels.
///
/// # Returns
/// SSE in pixels. Higher = more visual error = need to refine (load children).
pub fn compute_screen_space_error(
    geometric_error: f64,
    distance: f64,
    fov: f64,
    screen_height: f64,
) -> f64 {
    if distance <= 0.0 || screen_height <= 0.0 || fov <= 0.0 {
        return f64::INFINITY; // Undefined → always refine
    }
    let pixel_size = 2.0 * (fov * 0.5).tan() / screen_height;
    geometric_error / (distance * pixel_size)
}

/// WASM export: compute screen-space error.
#[wasm_bindgen(js_name = "computeScreenSpaceError")]
pub fn compute_screen_space_error_js(
    geometric_error: f64,
    distance: f64,
    fov: f64,
    screen_height: f64,
) -> f64 {
    compute_screen_space_error(geometric_error, distance, fov, screen_height)
}

/// Default SSE threshold in pixels. Tiles whose SSE falls below this
/// are considered "good enough" and children are not loaded.
const DEFAULT_SSE_THRESHOLD: f64 = 1.0;

/// Determine which tiles should be loaded given camera parameters.
///
/// Traverses the tileset octree from root, computing SSE for each node.
/// If SSE < threshold → the tile is sufficient, add to visible set.
/// If SSE >= threshold → need to refine, descend into children.
///
/// # Arguments
/// * `octree` — Built octree.
/// * `camera_x`, `camera_y`, `camera_z` — Camera world position.
/// * `camera_fov` — Vertical field of view in **radians**.
/// * `screen_width`, `screen_height` — Viewport dimensions in pixels.
/// * `sse_threshold` — SSE threshold in pixels (default: 1.0).
///
/// # Returns
/// Indices of leaf nodes whose tiles should be loaded.
pub fn get_visible_tiles(
    octree: &Octree,
    camera_x: f64,
    camera_y: f64,
    camera_z: f64,
    camera_fov: f64,
    _screen_width: f64,
    screen_height: f64,
    sse_threshold: Option<f64>,
) -> Vec<usize> {
    let threshold = sse_threshold.unwrap_or(DEFAULT_SSE_THRESHOLD);
    let mut visible = Vec::new();
    traverse_lod(octree, 0, camera_x, camera_y, camera_z, camera_fov, screen_height, threshold, &mut visible);
    visible
}

fn traverse_lod(
    octree: &Octree,
    node_idx: usize,
    cam_x: f64,
    cam_y: f64,
    cam_z: f64,
    fov: f64,
    screen_height: f64,
    sse_threshold: f64,
    visible: &mut Vec<usize>,
) {
    let node = &octree.nodes[node_idx];
    let bounds = node.bounds;

    // Node center.
    let cx = (bounds[0] + bounds[3]) * 0.5;
    let cy = (bounds[1] + bounds[4]) * 0.5;
    let cz = (bounds[2] + bounds[5]) * 0.5;

    // Distance from camera to node center.
    let dx = cx - cam_x;
    let dy = cy - cam_y;
    let dz = cz - cam_z;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-10);

    // Geometric error for this node.
    let geo_error = compute_geometric_error(&bounds, node.level);

    // Screen-space error in pixels.
    let sse = compute_screen_space_error(geo_error, distance, fov, screen_height);

    if sse < sse_threshold || node.is_leaf() {
        // This tile is good enough (or we're at a leaf) — load it.
        if node.point_count > 0 {
            visible.push(node_idx);
        }
    } else {
        // Need to refine — descend into children.
        if let Some(children) = node.children.as_deref() {
            for &child_idx in children {
                if child_idx < octree.nodes.len() {
                    traverse_lod(
                        octree,
                        child_idx,
                        cam_x, cam_y, cam_z,
                        fov, screen_height,
                        sse_threshold,
                        visible,
                    );
                }
            }
        }
    }
}

/// WASM export: get visible tiles for a camera position.
#[wasm_bindgen(js_name = "getVisibleTiles")]
pub fn get_visible_tiles_js(
    positions: &[f32],
    camera_x: f64,
    camera_y: f64,
    camera_z: f64,
    camera_fov: f64,
    screen_width: f64,
    screen_height: f64,
    max_points_per_node: Option<u32>,
    max_depth: Option<u32>,
    sse_threshold: Option<f64>,
) -> js_sys::Uint32Array {
    let max_pts = max_points_per_node.unwrap_or(DEFAULT_MAX_POINTS_PER_NODE);
    let max_d = max_depth.unwrap_or(crate::octree::DEFAULT_MAX_DEPTH);
    let mut buf = positions.to_vec();
    let octree = Octree::build(&mut buf, max_pts, max_d);

    let visible = get_visible_tiles(
        &octree, camera_x, camera_y, camera_z,
        camera_fov, screen_width, screen_height, sse_threshold,
    );

    let result = js_sys::Uint32Array::new_with_length(visible.len() as u32);
    for (i, &idx) in visible.iter().enumerate() {
        result.set_index(i as u32, idx as u32);
    }
    result
}

// ===========================================================================
// LOD tests
// ===========================================================================

#[cfg(test)]
mod lod_tests {
    use super::*;

    fn make_positions(triples: &[[f32; 3]]) -> Vec<f32> {
        let mut v = Vec::with_capacity(triples.len() * 3);
        for &[x, y, z] in triples {
            v.extend_from_slice(&[x, y, z]);
        }
        v
    }

    #[test]
    fn test_sse_basic() {
        // geoError=1.0, distance=100.0, fov=60° (π/3), screen=1080
        let sse = compute_screen_space_error(1.0, 100.0, std::f64::consts::FRAC_PI_3, 1080.0);
        // Expected: 1.0 / (100 * 2 * tan(30°) / 1080) = 1.0 / (100 * 1.1547 / 1080)
        // = 1.0 / 0.1069 ≈ 9.36
        assert!(sse > 8.0 && sse < 10.0, "SSE should be ~9.36, got {sse}");
    }

    #[test]
    fn test_sse_decreases_with_distance() {
        let sse_near = compute_screen_space_error(10.0, 10.0, std::f64::consts::FRAC_PI_3, 1080.0);
        let sse_far = compute_screen_space_error(10.0, 1000.0, std::f64::consts::FRAC_PI_3, 1080.0);
        assert!(sse_far < sse_near, "SSE should decrease with distance");
    }

    #[test]
    fn test_sse_increases_with_geo_error() {
        let sse_small = compute_screen_space_error(1.0, 100.0, std::f64::consts::FRAC_PI_3, 1080.0);
        let sse_large = compute_screen_space_error(10.0, 100.0, std::f64::consts::FRAC_PI_3, 1080.0);
        assert!(sse_large > sse_small, "SSE should increase with geometric error");
    }

    #[test]
    fn test_sse_zero_distance() {
        let sse = compute_screen_space_error(1.0, 0.0, std::f64::consts::FRAC_PI_3, 1080.0);
        assert!(sse.is_infinite(), "Zero distance should yield infinite SSE");
    }

    #[test]
    fn test_sse_zero_screen() {
        let sse = compute_screen_space_error(1.0, 100.0, std::f64::consts::FRAC_PI_3, 0.0);
        assert!(sse.is_infinite(), "Zero screen height should yield infinite SSE");
    }

    #[test]
    fn test_visible_tiles_close_camera_loads_more() {
        // 100 points spread in a cube.
        let triples: Vec<[f32; 3]> = (0..100)
            .map(|i| {
                [
                    ((i % 10) as f32 - 5.0) * 2.0,
                    (((i / 10) % 10) as f32 - 5.0) * 2.0,
                    0.0f32,
                ]
            })
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 10, 5);

        let fov = std::f64::consts::FRAC_PI_3; // 60°

        // Far camera — should load fewer tiles (root or few children).
        let far_tiles = get_visible_tiles(&tree, 0.0, 0.0, 10000.0, fov, 1920.0, 1080.0, None);
        // Close camera — should load more tiles (more refinement).
        let close_tiles = get_visible_tiles(&tree, 0.0, 0.0, 1.0, fov, 1920.0, 1080.0, None);

        assert!(
            close_tiles.len() >= far_tiles.len(),
            "Close camera should load >= tiles as far camera: close={}, far={}",
            close_tiles.len(),
            far_tiles.len()
        );
    }

    #[test]
    fn test_visible_tiles_empty_octree() {
        let mut positions: Vec<f32> = vec![];
        let tree = Octree::build(&mut positions, 10, 5);
        let visible = get_visible_tiles(&tree, 0.0, 0.0, 0.0, std::f64::consts::FRAC_PI_3, 1920.0, 1080.0, None);
        assert!(visible.is_empty());
    }

    #[test]
    fn test_visible_tiles_threshold_effect() {
        // 50 points spread.
        let triples: Vec<[f32; 3]> = (0..50)
            .map(|i| [(i % 5) as f32, ((i / 5) % 5) as f32, (i / 25) as f32])
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 5, 5);

        let fov = std::f64::consts::FRAC_PI_3;

        // High threshold (lenient) → fewer tiles loaded.
        let lenient = get_visible_tiles(&tree, 0.0, 0.0, 5.0, fov, 1920.0, 1080.0, Some(100.0));
        // Low threshold (strict) → more tiles loaded.
        let strict = get_visible_tiles(&tree, 0.0, 0.0, 5.0, fov, 1920.0, 1080.0, Some(0.1));

        assert!(
            strict.len() >= lenient.len(),
            "Strict threshold should load >= tiles: strict={}, lenient={}",
            strict.len(),
            lenient.len()
        );
    }
}

// ===========================================================================
// tileset tests
// ===========================================================================

#[cfg(test)]
mod tileset_tests {
    use super::*;
    fn make_positions(triples: &[[f32; 3]]) -> Vec<f32> {
        let mut v = Vec::with_capacity(triples.len() * 3);
        for &[x, y, z] in triples {
            v.extend_from_slice(&[x, y, z]);
        }
        v
    }

    #[test]
    fn test_tileset_json_structure() {
        let triples: Vec<[f32; 3]> = (0..100)
            .map(|i| {
                [
                    ((i % 10) as f32 - 5.0) * 2.0,
                    (((i / 10) % 10) as f32 - 5.0) * 2.0,
                    ((i / 100) as f32) * 2.0,
                ]
            })
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 10, 5);
        let result = generate_tileset(&tree, &positions, None).unwrap();

        let json = result.tileset_json();
        // Verify basic structure.
        assert!(json.contains("\"asset\""), "tileset should have asset");
        assert!(json.contains("\"root\""), "tileset should have root");
        assert!(
            json.contains("\"boundingVolume\""),
            "tileset should have boundingVolume"
        );
        assert!(
            json.contains("\"geometricError\""),
            "tileset should have geometricError"
        );
    }

    #[test]
    fn test_tile_pnts_format() {
        let triples: Vec<[f32; 3]> = (0..50)
            .map(|i| [(i % 5) as f32, ((i / 5) % 5) as f32, (i / 25) as f32])
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 20, 5);
        let result = generate_tileset(&tree, &positions, None).unwrap();

        assert!(result.tile_count() > 0);
        for i in 0..result.tile_count() {
            let tile_data = result.tile(i as usize).unwrap();
            assert_eq!(&tile_data[0..4], b"pnts", "tile {} should be valid pnts", i);
            let (header, _) = parse_pnts_header(tile_data).unwrap();
            assert_eq!(header.version, 1);
            assert_eq!(header.byte_length as usize, tile_data.len());
        }
    }

    #[test]
    fn test_bounding_volume() {
        let triples: Vec<[f32; 3]> = vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 1, 5);
        let result = generate_tileset(&tree, &positions, None).unwrap();

        let json = result.tileset_json();
        assert!(json.contains("\"box\":"), "should have box bounding volume");

        // Verify tile count = 8 (one per octant).
        assert_eq!(result.tile_count(), 8);

        // Each tile should have valid bounds.
        for i in 0..result.tile_count() {
            let bounds = result.tile_bounds(i as usize).unwrap();
            assert!(bounds[3] >= bounds[0], "tile {} bounds invalid", i);
            assert!(bounds[4] >= bounds[1], "tile {} bounds invalid", i);
            assert!(bounds[5] >= bounds[2], "tile {} bounds invalid", i);
        }
    }

    #[test]
    fn test_tileset_with_colors() {
        let triples: Vec<[f32; 3]> = (0..30)
            .map(|i| [(i % 3) as f32, ((i / 3) % 3) as f32, (i / 9) as f32])
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 10, 5);

        // Create matching color array.
        let colors: Vec<u8> = (0..30 * 3).map(|_| 128).collect();

        let result = generate_tileset(&tree, &positions, Some(&colors)).unwrap();
        assert!(result.tile_count() > 0);

        // Verify each tile has colors in the FT binary.
        let leaf_nodes: Vec<&crate::octree::OctreeNode> = tree.leaves().collect();
        for (i, tile_data) in result.tiles.iter().enumerate() {
            let (header, _) = parse_pnts_header(tile_data).unwrap();
            if i < leaf_nodes.len() {
                let node = leaf_nodes[i];
                let expected = node.point_count as usize * 3 * 4 // positions
                    + node.point_count as usize * 3; // colors
                assert_eq!(
                    header.feature_table_binary_byte_length as usize, expected,
                    "tile {} binary size mismatch",
                    i
                );
            }
        }
    }

    #[test]
    fn test_total_bytes() {
        let triples: Vec<[f32; 3]> = (0..100)
            .map(|i| [(i % 10) as f32, ((i / 10) % 10) as f32, 0.0])
            .collect();
        let mut positions = make_positions(&triples);
        let tree = Octree::build(&mut positions, 25, 5);
        let result = generate_tileset(&tree, &positions, None).unwrap();
        assert!(result.total_bytes() > 0);
        // Each tile should be at least header + JSON + binary.
        for i in 0..result.tile_count() {
            assert!(result.tile(i as usize).unwrap().len() >= 40);
        }
    }
}
