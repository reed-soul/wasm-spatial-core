//! # Streaming Point Cloud Loader
//!
//! Supports range-based loading of LAS point clouds. Parse only the header
//! first (227+ bytes), then fetch points on demand using byte offsets.
//!
//! For uncompressed LAS files, any point can be accessed with:
//!   `offset = point_data_offset + point_index * point_record_length`
//!
//! For COPC (Cloud Optimized Point Cloud), the chunk table in VLR allows
//! indexed access to compressed chunks.

use wasm_bindgen::prelude::*;

use crate::point_cloud::{
    read_f64_le, read_i32_le, read_u16_le, read_u32_le, LasPointCloud,
};
use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::DEFAULT_MAX_INPUT_SIZE;

// ===========================================================================
// PointCloudStreamer — WASM class
// ===========================================================================

/// Streaming point cloud loader.
///
/// Parse a LAS header first, then read points or regions on demand without
/// loading the entire file into memory.
///
/// # Example (JS)
///
/// ```ignore
/// const streamer = new PointCloudStreamer();
/// const header = streamer.parseHeader(headerBytes);
/// console.log(`File has ${header.numPoints()} points`);
///
/// // Read points 100..200:
/// const region = streamer.readRegion(fullBytes, headerBytes, 100, 100);
/// ```
#[wasm_bindgen]
pub struct PointCloudStreamer {
    /// Cached header info from the last parseHeader call.
    num_points: u32,
    point_offset: u32,
    point_format_id: u8,
    point_record_length: u16,
    x_scale: f64,
    y_scale: f64,
    z_scale: f64,
    x_offset: f64,
    y_offset: f64,
    z_offset: f64,
    /// Whether the header has been parsed.
    initialized: bool,
}

#[wasm_bindgen]
impl PointCloudStreamer {
    /// Create a new streamer instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        PointCloudStreamer {
            num_points: 0,
            point_offset: 0,
            point_format_id: 0,
            point_record_length: 0,
            x_scale: 1.0,
            y_scale: 1.0,
            z_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            initialized: false,
        }
    }

    /// Parse a LAS header from the first 230+ bytes of a file.
    ///
    /// Stores metadata internally for subsequent `totalPoints()` and
    /// `readRegion()` calls. Returns the same `LasHeaderInfo` that
    /// `parseLasHeaderOnly` would produce.
    ///
    /// # Arguments
    ///
    /// * `bytes` — At least 230 bytes from the start of a LAS file.
    #[wasm_bindgen(js_name = "parseHeader")]
    pub fn parse_header(&mut self, bytes: &[u8]) -> Result<crate::point_cloud::LasHeaderInfo, SpatialErrorDetail> {
        if bytes.len() < 230 {
            return Err(SpatialError::point_cloud_error(
                "LAS header requires at least 230 bytes",
            ));
        }
        if &bytes[0..4] != b"LASF" {
            return Err(SpatialError::point_cloud_error(
                "Invalid LAS magic: expected 'LASF'",
            ));
        }

        self.num_points = read_u32_le(bytes, 110);
        self.point_offset = read_u32_le(bytes, 98);
        self.point_format_id = bytes[106];
        self.point_record_length = read_u16_le(bytes, 108);
        self.x_scale = read_f64_le(bytes, 134);
        self.y_scale = read_f64_le(bytes, 142);
        self.z_scale = read_f64_le(bytes, 150);
        self.x_offset = read_f64_le(bytes, 158);
        self.y_offset = read_f64_le(bytes, 166);
        self.z_offset = read_f64_le(bytes, 174);
        self.initialized = true;

        Ok(crate::point_cloud::LasHeaderInfo::new_from_parts(
            self.num_points,
            self.point_offset,
            self.point_format_id,
            self.point_record_length,
            self.x_scale,
            self.y_scale,
            self.z_scale,
            self.x_offset,
            self.y_offset,
            self.z_offset,
            read_f64_le(bytes, 182), // bounds_max_x
            read_f64_le(bytes, 190),
            read_f64_le(bytes, 198),
            read_f64_le(bytes, 206), // bounds_min_x
            read_f64_le(bytes, 214),
            read_f64_le(bytes, 222),
            bytes.len() as u32,
        ))
    }

    /// Return the total number of points from the last parsed header.
    ///
    /// Returns 0 if no header has been parsed yet.
    #[wasm_bindgen(js_name = "totalPoints")]
    pub fn total_points(&self) -> u32 {
        self.num_points
    }

    /// Parse all points from a complete LAS byte buffer.
    ///
    /// This is a convenience method that combines header parsing with
    /// full point extraction. For large files, prefer `readRegion()`.
    ///
    /// # Arguments
    ///
    /// * `bytes` — Full LAS file bytes (header + point data).
    /// * `header_bytes` — First 230+ bytes (header portion).
    #[wasm_bindgen(js_name = "readPoints")]
    pub fn read_points(
        &mut self,
        bytes: &[u8],
        header_bytes: &[u8],
    ) -> Result<LasPointCloud, SpatialErrorDetail> {
        // Parse header if not done yet
        if !self.initialized {
            self.parse_header(header_bytes)?;
        }

        if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
            return Err(SpatialError::point_cloud_error(format!(
                "Input too large: {} bytes (max {})",
                bytes.len(),
                DEFAULT_MAX_INPUT_SIZE
            )));
        }

        parse_las_region_core(
            bytes,
            self.point_offset,
            self.point_format_id,
            self.point_record_length,
            self.x_scale,
            self.y_scale,
            self.z_scale,
            self.x_offset,
            self.y_offset,
            self.z_offset,
            0,
            self.num_points as usize,
        )
        .map_err(SpatialError::point_cloud_error)
    }

    /// Read a specific region of points from a LAS file.
    ///
    /// For uncompressed LAS, computes exact byte offsets:
    ///   `offset = point_data_offset + point_index * point_record_length`
    ///
    /// # Arguments
    ///
    /// * `bytes` — LAS file bytes (header + at least the requested points).
    /// * `header_bytes` — First 230+ bytes (header portion), used to
    ///   initialize the streamer if not already done.
    /// * `start_index` — First point index to read (0-based).
    /// * `count` — Number of points to read.
    #[wasm_bindgen(js_name = "readRegion")]
    pub fn read_region(
        &mut self,
        bytes: &[u8],
        header_bytes: &[u8],
        start_index: u32,
        count: u32,
    ) -> Result<LasPointCloud, SpatialErrorDetail> {
        if !self.initialized {
            self.parse_header(header_bytes)?;
        }

        let si = start_index as usize;
        let cnt = count as usize;

        if si >= self.num_points as usize {
            return Err(SpatialError::point_cloud_error(format!(
                "start_index {} >= total_points {}",
                si, self.num_points
            )));
        }

        let actual_count = cnt.min((self.num_points as usize).saturating_sub(si));
        if actual_count == 0 {
            return Err(SpatialError::point_cloud_error("count is 0"));
        }

        parse_las_region_core(
            bytes,
            self.point_offset,
            self.point_format_id,
            self.point_record_length,
            self.x_scale,
            self.y_scale,
            self.z_scale,
            self.x_offset,
            self.y_offset,
            self.z_offset,
            si,
            actual_count,
        )
        .map_err(SpatialError::point_cloud_error)
    }
}

// ===========================================================================
// Region-based parser (pure Rust core)
// ===========================================================================

/// Parse a contiguous range of points from a LAS byte buffer.
///
/// This is the core function behind `PointCloudStreamer::read_region` and
/// `PointCloudStreamer::read_points`. It only reads the requested point range
/// without iterating over the full file.
pub fn parse_las_region_core(
    bytes: &[u8],
    point_offset: u32,
    point_format_id: u8,
    point_record_length: u16,
    x_scale: f64,
    y_scale: f64,
    z_scale: f64,
    x_offset: f64,
    y_offset: f64,
    z_offset: f64,
    start_index: usize,
    count: usize,
) -> Result<LasPointCloud, String> {
    let has_color = point_format_id == 2 || point_format_id == 3;
    let expected_record_len: usize = if has_color { 26 } else { 20 };

    if (point_record_length as usize) < expected_record_len {
        return Err(format!(
            "Point record length {} too small for format {} (need {})",
            point_record_length, point_format_id, expected_record_len
        ));
    }

    let record_len = point_record_length as usize;
    let mut positions: Vec<f32> = Vec::with_capacity(count * 3);
    let mut colors: Option<Vec<u8>> = if has_color {
        Some(Vec::with_capacity(count * 3))
    } else {
        None
    };

    for i in 0..count {
        let idx = start_index + i;
        let base = point_offset as usize + idx * record_len;

        if base + expected_record_len > bytes.len() {
            break; // truncated or range request past end
        }

        let raw_x = read_i32_le(bytes, base) as f64;
        let raw_y = read_i32_le(bytes, base + 4) as f64;
        let raw_z = read_i32_le(bytes, base + 8) as f64;

        positions.push((raw_x * x_scale + x_offset) as f32);
        positions.push((raw_y * y_scale + y_offset) as f32);
        positions.push((raw_z * z_scale + z_offset) as f32);

        if has_color {
            if let Some(ref mut c) = colors {
                c.push(bytes[base + 20]);
                c.push(bytes[base + 21]);
                c.push(bytes[base + 22]);
            }
        }
    }

    Ok(LasPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// Compute the byte offset and byte length for a range of LAS points.
///
/// Returns `(start_byte, byte_count)` for use with `fetch` + `Range` headers.
pub fn compute_region_byte_range(
    point_offset: u32,
    point_record_length: u16,
    start_index: u32,
    count: u32,
) -> (usize, usize) {
    let start = point_offset as usize + start_index as usize * point_record_length as usize;
    let len = count as usize * point_record_length as usize;
    (start, len)
}

/// WASM export: compute byte range for a region of points.
#[wasm_bindgen(js_name = "computeRegionByteRange")]
pub fn compute_region_byte_range_js(
    point_offset: u32,
    point_record_length: u16,
    start_index: u32,
    count: u32,
) -> js_sys::Object {
    let (start, len) = compute_region_byte_range(point_offset, point_record_length, start_index, count);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"startByte".into(), &JsValue::from(start as f64)).ok();
    js_sys::Reflect::set(&obj, &"byteLength".into(), &JsValue::from(len as f64)).ok();
    obj
}

// ===========================================================================
// LAZ Status
// ===========================================================================

/// Check if LAZ (compressed LAS) is supported.
///
/// Currently returns `false`. LAZ support requires the `laz-rs` crate which
/// has native dependencies that may not compile to `wasm32-unknown-unknown`.
#[wasm_bindgen(js_name = "supportsLaz")]
pub fn supports_laz() -> bool {
    false
}

/// Get the current LAZ support status as a human-readable string.
#[wasm_bindgen(js_name = "lazStatus")]
pub fn laz_status() -> String {
    String::from(
        "LAZ support is not yet available. laz-rs depends on native C code \
        (LazPerf) that does not compile to wasm32. Planned approaches: \
        (1) port LazPerf to pure Rust, (2) WASI-based LAZ decompression, \
        (3) server-side LAZ→LAS conversion with client-side streaming.",
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid LAS 1.2 blob for testing.
    fn build_test_las_blob(points: &[(f64, f64, f64)], has_color: bool) -> Vec<u8> {
        let num_points = points.len() as u32;
        let header_size = 230u32;
        let point_offset = header_size;
        let point_format: u8 = if has_color { 2 } else { 0 };
        let record_len: u16 = if has_color { 26 } else { 20 };

        let (mut min_x, mut min_y, mut min_z) = (f64::MAX, f64::MAX, f64::MAX);
        let (mut max_x, mut max_y, mut max_z) = (f64::MIN, f64::MIN, f64::MIN);
        for &(x, y, z) in points {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            max_z = max_z.max(z);
        }

        let x_scale = 1.0_f64;
        let y_scale = 1.0_f64;
        let z_scale = 1.0_f64;

        let mut buf = vec![0u8; header_size as usize];
        buf[0..4].copy_from_slice(b"LASF");
        buf[24] = 1; // version major
        buf[26] = 2; // version minor
        buf[98..102].copy_from_slice(&point_offset.to_le_bytes());
        buf[106] = point_format;
        buf[108..110].copy_from_slice(&record_len.to_le_bytes());
        buf[110..114].copy_from_slice(&num_points.to_le_bytes());
        buf[134..142].copy_from_slice(&x_scale.to_le_bytes());
        buf[142..150].copy_from_slice(&y_scale.to_le_bytes());
        buf[150..158].copy_from_slice(&z_scale.to_le_bytes());
        buf[182..190].copy_from_slice(&max_x.to_le_bytes());
        buf[190..198].copy_from_slice(&max_y.to_le_bytes());
        buf[198..206].copy_from_slice(&max_z.to_le_bytes());
        buf[206..214].copy_from_slice(&min_x.to_le_bytes());
        buf[214..222].copy_from_slice(&min_y.to_le_bytes());
        buf[222..230].copy_from_slice(&min_z.to_le_bytes());

        for &(x, y, z) in points {
            let base = buf.len();
            let point_size = record_len as usize;
            buf.resize(base + point_size, 0);
            buf[base..base + 4].copy_from_slice(&(x as i32).to_le_bytes());
            buf[base + 4..base + 8].copy_from_slice(&(y as i32).to_le_bytes());
            buf[base + 8..base + 12].copy_from_slice(&(z as i32).to_le_bytes());
            if has_color {
                buf[base + 20] = 255;
                buf[base + 21] = 128;
                buf[base + 22] = 0;
            }
        }

        buf
    }

    #[test]
    fn test_streamer_parse_header() {
        let blob = build_test_las_blob(&[(10.0, 20.0, 30.0), (40.0, 50.0, 60.0)], false);
        let mut streamer = PointCloudStreamer::new();
        let header = streamer.parse_header(&blob).unwrap();

        assert_eq!(header.num_points(), 2);
        assert_eq!(streamer.total_points(), 2);
        assert!(streamer.initialized);
    }

    #[test]
    fn test_streamer_read_all_points() {
        let points = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)];
        let blob = build_test_las_blob(&points, false);
        let header_bytes = blob[..230].to_vec();
        let mut streamer = PointCloudStreamer::new();

        let cloud = streamer.read_points(&blob, &header_bytes).unwrap();
        assert_eq!(cloud.point_count, 3);
        assert_eq!(cloud.positions.len(), 9);
    }

    #[test]
    fn test_streamer_read_region() {
        let points: Vec<(f64, f64, f64)> = (0..10).map(|i| (i as f64, i as f64, i as f64)).collect();
        let blob = build_test_las_blob(&points, false);
        let header_bytes = blob[..230].to_vec();
        let mut streamer = PointCloudStreamer::new();

        // Read points 3..7 (4 points)
        let cloud = streamer.read_region(&blob, &header_bytes, 3, 4).unwrap();
        assert_eq!(cloud.point_count, 4);
        // First point should be (3, 3, 3)
        assert_eq!(cloud.positions[0], 3.0);
        assert_eq!(cloud.positions[1], 3.0);
        assert_eq!(cloud.positions[2], 3.0);
        // Last point should be (6, 6, 6)
        assert_eq!(cloud.positions[9], 6.0);
    }

    #[test]
    fn test_streamer_region_past_end() {
        let points: Vec<(f64, f64, f64)> = (0..5).map(|i| (i as f64, 0.0, 0.0)).collect();
        let blob = build_test_las_blob(&points, false);
        let header_bytes = blob[..230].to_vec();
        let mut streamer = PointCloudStreamer::new();

        // Request more points than exist — should clamp
        let cloud = streamer.read_region(&blob, &header_bytes, 3, 100).unwrap();
        assert_eq!(cloud.point_count, 2); // only 2 points available from index 3
    }

    #[test]
    fn test_compute_region_byte_range() {
        // 230 byte header, 20 byte points
        let (start, len) = compute_region_byte_range(230, 20, 5, 3);
        assert_eq!(start, 230 + 5 * 20); // 330
        assert_eq!(len, 3 * 20); // 60
    }

    #[test]
    fn test_laz_status() {
        assert!(!supports_laz());
        let status = laz_status();
        assert!(status.contains("not yet available"));
    }

    #[test]
    fn test_streamer_not_initialized() {
        let streamer = PointCloudStreamer::new();
        assert_eq!(streamer.total_points(), 0);
    }

    #[test]
    fn test_streamer_region_start_past_end() {
        let points = vec![(1.0, 2.0, 3.0)];
        let blob = build_test_las_blob(&points, false);
        let header_bytes = blob[..230].to_vec();
        let mut streamer = PointCloudStreamer::new();

        let result = streamer.read_region(&blob, &header_bytes, 100, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_streamer_with_colors() {
        let points = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)];
        let blob = build_test_las_blob(&points, true);
        let header_bytes = blob[..230].to_vec();
        let mut streamer = PointCloudStreamer::new();

        let cloud = streamer.read_points(&blob, &header_bytes).unwrap();
        assert!(cloud.colors.is_some());
        let colors = cloud.colors.unwrap();
        assert_eq!(colors.len(), 9); // 3 points × 3 channels
    }

    #[test]
    fn test_parse_las_region_core_direct() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0), (70.0, 80.0, 90.0)];
        let blob = build_test_las_blob(&points, false);

        let cloud = parse_las_region_core(
            &blob,
            230, // point_offset
            0,   // point_format_id (no color)
            20,  // point_record_length
            1.0, 1.0, 1.0, // scale
            0.0, 0.0, 0.0, // offset
            1,   // start_index
            1,   // count
        )
        .unwrap();

        assert_eq!(cloud.point_count, 1);
        assert_eq!(cloud.positions[0], 40.0);
        assert_eq!(cloud.positions[1], 50.0);
        assert_eq!(cloud.positions[2], 60.0);
    }
}
