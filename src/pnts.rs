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
        return Err(SpatialError::PointCloudError
            .with_detail("cannot encode pnts tile with 0 points"));
    }
    if colors.is_some() && colors.as_ref().unwrap().len() != num_points * 3 {
        return Err(SpatialError::PointCloudError.with_detail(
            format!(
                "color count mismatch: expected {} bytes, got {}",
                num_points * 3,
                colors.unwrap().len()
            ),
        ));
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
        byte_length: 28 + ft_json_padded.len() as u32 + feature_binary_len as u32
            + bt_json_padded.len() as u32 + bt_binary_len,
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
pub fn parse_pnts_header(data: &[u8]) -> Result<(PntsHeader, &[u8]), crate::errors::SpatialErrorDetail> {
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
        return Err(SpatialError::PointCloudError.with_detail(format!(
            "unsupported pnts version: {}, expected 1",
            version
        )));
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
        padded.extend(std::iter::repeat(' ').take(pad));
        padded
    }
}

/// Compute padded length for a value that needs 4-byte alignment.
fn pad_len(len: u32) -> u32 {
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
    let colors_ref = colors.as_deref();
    let result = encode_pnts_tile(positions, [center_x, center_y, center_z], colors_ref)
        .map_err(|e| JsValue::from(e))?;
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
        let y = f32::from_le_bytes(
            tile[binary_start + 4..binary_start + 8].try_into().unwrap(),
        );
        let z = f32::from_le_bytes(
            tile[binary_start + 8..binary_start + 12].try_into().unwrap(),
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
        let ft_json =
            std::str::from_utf8(&tile[28..28 + header.feature_table_json_byte_length as usize])
                .unwrap();
        assert!(ft_json.contains("POSITION"));
    }
}
