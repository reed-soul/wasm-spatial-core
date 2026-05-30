//! Point Cloud Processing
//!
//! Hand-written LAS header parser and point cloud decimation utilities.
//! The LAS format is simple enough (first 96 bytes = public header) that we
//! don't need the `las-rs` crate — this avoids native dependencies that may
//! not compile for `wasm32-unknown-unknown`.

use wasm_bindgen::prelude::*;

// ===========================================================================
// LAS Header Format (Public Header Block, first 227 bytes for full header)
// ===========================================================================

/// Parsed LAS file header.
#[wasm_bindgen]
pub struct LasHeader {
    version_major: u8,
    version_minor: u8,
    point_format_id: u8,
    point_data_record_length: u16,
    num_points: u32,
    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_min_z: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
    bounds_max_z: f64,
}

#[wasm_bindgen]
impl LasHeader {
    #[wasm_bindgen(getter, js_name = "versionMajor")]
    pub fn version_major(&self) -> u8 {
        self.version_major
    }

    #[wasm_bindgen(getter, js_name = "versionMinor")]
    pub fn version_minor(&self) -> u8 {
        self.version_minor
    }

    #[wasm_bindgen(getter, js_name = "pointFormatId")]
    pub fn point_format_id(&self) -> u8 {
        self.point_format_id
    }

    #[wasm_bindgen(getter, js_name = "pointDataRecordLength")]
    pub fn point_data_record_length(&self) -> u16 {
        self.point_data_record_length
    }

    #[wasm_bindgen(getter, js_name = "numPoints")]
    pub fn num_points(&self) -> u32 {
        self.num_points
    }

    #[wasm_bindgen(getter, js_name = "boundsMinX")]
    pub fn bounds_min_x(&self) -> f64 {
        self.bounds_min_x
    }

    #[wasm_bindgen(getter, js_name = "boundsMinY")]
    pub fn bounds_min_y(&self) -> f64 {
        self.bounds_min_y
    }

    #[wasm_bindgen(getter, js_name = "boundsMinZ")]
    pub fn bounds_min_z(&self) -> f64 {
        self.bounds_min_z
    }

    #[wasm_bindgen(getter, js_name = "boundsMaxX")]
    pub fn bounds_max_x(&self) -> f64 {
        self.bounds_max_x
    }

    #[wasm_bindgen(getter, js_name = "boundsMaxY")]
    pub fn bounds_max_y(&self) -> f64 {
        self.bounds_max_y
    }

    #[wasm_bindgen(getter, js_name = "boundsMaxZ")]
    pub fn bounds_max_z(&self) -> f64 {
        self.bounds_max_z
    }

    /// Human-readable version string like "1.2".
    #[wasm_bindgen(js_name = "versionString")]
    pub fn version_string(&self) -> String {
        format!("{}.{}", self.version_major, self.version_minor)
    }
}

/// Parsed LAS point cloud data.
#[wasm_bindgen]
pub struct LasPointCloud {
    positions: Vec<f32>,     // interleaved [x, y, z, x, y, z, ...]
    colors: Option<Vec<u8>>, // [r, g, b, r, g, b, ...] if present
    point_count: u32,
}

#[wasm_bindgen]
impl LasPointCloud {
    /// Interleaved XYZ positions as Float32Array `[x0, y0, z0, x1, y1, z1, ...]`.
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        let arr = js_sys::Float32Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    /// RGB colors as Uint8Array `[r0, g0, b0, r1, g1, b1, ...]`, or `null` if not present.
    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> Option<js_sys::Uint8Array> {
        self.colors.as_ref().map(|c| {
            let arr = js_sys::Uint8Array::new_with_length(c.len() as u32);
            arr.copy_from(c);
            arr
        })
    }

    /// Number of points in the cloud.
    #[wasm_bindgen(getter, js_name = "pointCount")]
    pub fn point_count(&self) -> u32 {
        self.point_count
    }
}

// ===========================================================================
// Internal byte readers
// ===========================================================================

fn read_f64_le(bytes: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ===========================================================================
// LAS parsing
// ===========================================================================

// ===========================================================================
// LAS parsing core (pure Rust — testable without WASM runtime)
// ===========================================================================

fn parse_las_header_core(bytes: &[u8]) -> Result<LasHeader, String> {
    if bytes.len() < 230 {
        return Err("LAS header requires at least 230 bytes".to_string());
    }

    if &bytes[0..4] != b"LASF" {
        return Err(format!(
            "Invalid LAS magic: expected b\"LASF\", got {:?}",
            &bytes[0..4]
        ));
    }

    Ok(LasHeader {
        version_major: bytes[24],
        version_minor: bytes[26],
        point_format_id: bytes[106],
        point_data_record_length: read_u16_le(bytes, 108),
        num_points: read_u32_le(bytes, 110),
        bounds_max_x: read_f64_le(bytes, 182),
        bounds_max_y: read_f64_le(bytes, 190),
        bounds_max_z: read_f64_le(bytes, 198),
        bounds_min_x: read_f64_le(bytes, 206),
        bounds_min_y: read_f64_le(bytes, 214),
        bounds_min_z: read_f64_le(bytes, 222),
    })
}

fn parse_las_points_core(bytes: &[u8]) -> Result<LasPointCloud, String> {
    let num_points = read_u32_le(bytes, 110) as usize;
    let point_offset = read_u32_le(bytes, 98) as usize;
    let point_format = bytes[106];
    let point_record_len = read_u16_le(bytes, 108) as usize;

    // Scale/offset for coordinate reconstruction
    let x_scale = read_f64_le(bytes, 134);
    let y_scale = read_f64_le(bytes, 142);
    let z_scale = read_f64_le(bytes, 150);
    let x_offset = read_f64_le(bytes, 158);
    let y_offset = read_f64_le(bytes, 166);
    let z_offset = read_f64_le(bytes, 174);

    // Format 0: 20 bytes, Format 2: 26 bytes (with RGB)
    let has_color = point_format == 2 || point_format == 3;
    let expected_record_len = if has_color { 26 } else { 20 };

    if point_record_len < expected_record_len {
        return Err(format!(
            "Point record length {} too small for format {} (need {})",
            point_record_len, point_format, expected_record_len
        ));
    }

    let mut positions: Vec<f32> = Vec::with_capacity(num_points * 3);
    let mut colors: Option<Vec<u8>> = if has_color {
        Some(Vec::with_capacity(num_points * 3))
    } else {
        None
    };

    for i in 0..num_points {
        let base = point_offset + i * point_record_len;
        if base + expected_record_len > bytes.len() {
            break; // truncated file
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

/// WASM binding for LAS header parsing.
#[wasm_bindgen(js_name = "parseLasHeader")]
pub fn parse_las_header(bytes: &[u8]) -> Result<LasHeader, JsValue> {
    parse_las_header_core(bytes).map_err(|e| JsValue::from_str(&e))
}

/// WASM binding for LAS point parsing.
#[wasm_bindgen(js_name = "parseLasPoints")]
pub fn parse_las_points(bytes: &[u8]) -> Result<LasPointCloud, JsValue> {
    parse_las_points_core(bytes).map_err(|e| JsValue::from_str(&e))
}

// ===========================================================================
// Decimation (pure Rust core — testable without WASM runtime)
// ===========================================================================

fn voxel_grid_decimate_core(
    positions: &[f32],
    colors: &[u8],
    cell_size: f32,
) -> (Vec<f32>, Vec<u8>) {
    let point_count = positions.len() / 3;
    let has_colors = !colors.is_empty();

    if point_count == 0 || cell_size <= 0.0 {
        return (Vec::new(), Vec::new());
    }

    let mut grid: std::collections::HashMap<(i64, i64, i64), usize> =
        std::collections::HashMap::new();

    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        let cx = (x / cell_size).floor() as i64;
        let cy = (y / cell_size).floor() as i64;
        let cz = (z / cell_size).floor() as i64;
        grid.entry((cx, cy, cz)).or_insert(i);
    }

    let kept: Vec<usize> = grid.into_values().collect();
    let mut out_pos = Vec::with_capacity(kept.len() * 3);
    let mut out_col: Vec<u8> = Vec::with_capacity(kept.len() * 3);

    for &idx in &kept {
        out_pos.push(positions[idx * 3]);
        out_pos.push(positions[idx * 3 + 1]);
        out_pos.push(positions[idx * 3 + 2]);
        if has_colors {
            out_col.push(colors[idx * 3]);
            out_col.push(colors[idx * 3 + 1]);
            out_col.push(colors[idx * 3 + 2]);
        }
    }

    (out_pos, out_col)
}

fn random_decimate_core(
    positions: &[f32],
    colors: &[u8],
    target_count: usize,
) -> (Vec<f32>, Vec<u8>) {
    let point_count = positions.len() / 3;
    let has_colors = !colors.is_empty();
    let output_count = target_count.min(point_count);

    let mut seed: u64 = 12345;
    let next_rand = |s: &mut u64| -> f64 {
        *s = s.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        (*s as f64) / (0x7fffffff as f64)
    };

    let mut indices: Vec<usize> = (0..point_count).collect();
    for i in 0..output_count {
        let j = i + (next_rand(&mut seed) * (point_count - i) as f64) as usize;
        indices.swap(i, j);
    }

    let mut out_pos = Vec::with_capacity(output_count * 3);
    let mut out_col: Vec<u8> = Vec::with_capacity(output_count * 3);

    for &idx in indices.iter().take(output_count) {
        out_pos.push(positions[idx * 3]);
        out_pos.push(positions[idx * 3 + 1]);
        out_pos.push(positions[idx * 3 + 2]);
        if has_colors {
            out_col.push(colors[idx * 3]);
            out_col.push(colors[idx * 3 + 1]);
            out_col.push(colors[idx * 3 + 2]);
        }
    }

    (out_pos, out_col)
}

// ===========================================================================
// Decimation WASM API (thin wrappers)
// ===========================================================================

/// Voxel grid decimation: divide space into `cell_size` cubes, keep one point per cell.
#[wasm_bindgen(js_name = "decimateVoxelGrid")]
pub fn decimate_voxel_grid(
    positions: &js_sys::Float32Array,
    colors: &js_sys::Uint8Array,
    cell_size: f32,
) -> js_sys::Object {
    let pos_len = positions.length() as usize;
    let col_len = colors.length() as usize;

    let mut pos_buf = vec![0.0f32; pos_len];
    positions.copy_to(&mut pos_buf);
    let mut col_buf = vec![0u8; col_len];
    colors.copy_to(&mut col_buf);

    let (out_pos, out_col) = voxel_grid_decimate_core(&pos_buf, &col_buf, cell_size);

    let pos_arr = js_sys::Float32Array::new_with_length(out_pos.len() as u32);
    pos_arr.copy_from(&out_pos);
    let col_arr = js_sys::Uint8Array::new_with_length(out_col.len() as u32);
    col_arr.copy_from(&out_col);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).unwrap();
    js_sys::Reflect::set(&obj, &"colors".into(), &col_arr).unwrap();
    obj
}

/// Random decimation to a target point count.
#[wasm_bindgen(js_name = "decimateRandom")]
pub fn decimate_random(
    positions: &js_sys::Float32Array,
    colors: &js_sys::Uint8Array,
    target_count: u32,
) -> js_sys::Object {
    let pos_len = positions.length() as usize;
    let col_len = colors.length() as usize;

    let mut pos_buf = vec![0.0f32; pos_len];
    positions.copy_to(&mut pos_buf);
    let mut col_buf = vec![0u8; col_len];
    colors.copy_to(&mut col_buf);

    let (out_pos, out_col) = random_decimate_core(&pos_buf, &col_buf, target_count as usize);

    let pos_arr = js_sys::Float32Array::new_with_length(out_pos.len() as u32);
    pos_arr.copy_from(&out_pos);
    let col_arr = js_sys::Uint8Array::new_with_length(out_col.len() as u32);
    col_arr.copy_from(&out_col);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).unwrap();
    js_sys::Reflect::set(&obj, &"colors".into(), &col_arr).unwrap();
    obj
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid LAS 1.2 header (227 bytes) with format 0 or 2,
    /// followed by point data.
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
        buf[96..98].copy_from_slice(&(header_size as u16).to_le_bytes());
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
                buf[base + 20] = 255; // R
                buf[base + 21] = 128; // G
                buf[base + 22] = 0; // B
            }
        }

        buf
    }

    // ── LAS header tests ────────────────────────────────────────────

    #[test]
    fn test_parse_las_header() {
        let blob = build_test_las_blob(&[(10.0, 20.0, 30.0)], false);
        let header = parse_las_header_core(&blob).unwrap();

        assert_eq!(header.version_major, 1);
        assert_eq!(header.version_minor, 2);
        assert_eq!(header.num_points, 1);
        assert_eq!(header.point_format_id, 0);
        assert_eq!(header.point_data_record_length, 20);
        assert_eq!(header.bounds_min_x, 10.0);
        assert_eq!(header.bounds_max_x, 10.0);
    }

    #[test]
    fn test_parse_las_header_version_string() {
        let blob = build_test_las_blob(&[(0.0, 0.0, 0.0)], false);
        let header = parse_las_header_core(&blob).unwrap();
        assert_eq!(header.version_string(), "1.2");
    }

    #[test]
    fn test_parse_las_header_invalid_magic() {
        let mut blob = build_test_las_blob(&[(0.0, 0.0, 0.0)], false);
        blob[0] = b'X';
        let result = parse_las_header_core(&blob);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_las_header_too_short() {
        let result = parse_las_header_core(&[0u8; 10]);
        assert!(result.is_err());
    }

    // ── LAS point parsing tests ─────────────────────────────────────

    #[test]
    fn test_parse_las_points_format0() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0), (70.0, 80.0, 90.0)];
        let blob = build_test_las_blob(&points, false);
        let cloud = parse_las_points_core(&blob).unwrap();

        assert_eq!(cloud.point_count, 3);
        assert_eq!(cloud.positions.len(), 9);
        assert!(cloud.colors.is_none());

        assert_eq!(cloud.positions[0], 10.0);
        assert_eq!(cloud.positions[1], 20.0);
        assert_eq!(cloud.positions[2], 30.0);
        assert_eq!(cloud.positions[6], 70.0);
        assert_eq!(cloud.positions[7], 80.0);
        assert_eq!(cloud.positions[8], 90.0);
    }

    #[test]
    fn test_parse_las_points_format2_with_color() {
        let points = vec![(5.0, 10.0, 15.0), (25.0, 30.0, 35.0)];
        let blob = build_test_las_blob(&points, true);
        let cloud = parse_las_points_core(&blob).unwrap();

        assert_eq!(cloud.point_count, 2);
        assert_eq!(cloud.positions.len(), 6);
        assert!(cloud.colors.is_some());

        let colors = cloud.colors.unwrap();
        assert_eq!(colors.len(), 6);
        assert_eq!(colors[0], 255);
        assert_eq!(colors[1], 128);
        assert_eq!(colors[2], 0);
    }

    #[test]
    fn test_parse_las_points_empty() {
        let blob = build_test_las_blob(&[], false);
        let cloud = parse_las_points_core(&blob).unwrap();
        assert_eq!(cloud.point_count, 0);
        assert!(cloud.positions.is_empty());
    }

    // ── Voxel grid decimation tests ─────────────────────────────────

    #[test]
    fn test_voxel_grid_decimation() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 2.0, 2.0,
            0.0, 0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 2.0, 2.0,
        ];
        let colors: Vec<u8> = (0..27).collect();

        let (out_pos, out_col) = voxel_grid_decimate_core(&positions, &colors, 2.0);
        let kept = out_pos.len() / 3;
        assert!(kept <= 9, "Kept {} points, expected <= 9", kept);
        assert!(kept >= 4, "Kept {} points, expected >= 4", kept);
        assert_eq!(out_col.len(), kept * 3);
    }

    #[test]
    fn test_voxel_grid_empty() {
        let (out_pos, out_col) = voxel_grid_decimate_core(&[], &[], 1.0);
        assert!(out_pos.is_empty());
        assert!(out_col.is_empty());
    }

    #[test]
    fn test_voxel_grid_no_colors() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 3.0, 3.0, 3.0];
        let (out_pos, out_col) = voxel_grid_decimate_core(&positions, &[], 1.0);
        assert_eq!(out_pos.len() / 3, 3); // all in different cells
        assert!(out_col.is_empty());
    }

    #[test]
    fn test_voxel_grid_all_same_cell() {
        let positions: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let (out_pos, _) = voxel_grid_decimate_core(&positions, &[], 10.0);
        assert_eq!(out_pos.len() / 3, 1); // all in one cell
    }

    // ── Random decimation tests ───────────────────────────────────────

    #[test]
    fn test_random_decimation() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 4.0,
        ];
        let colors: Vec<u8> = (0..15).collect();

        let (out_pos, out_col) = random_decimate_core(&positions, &colors, 3);
        assert_eq!(out_pos.len(), 9); // 3 points × 3
        assert_eq!(out_col.len(), 9);
    }

    #[test]
    fn test_random_decimation_target_larger() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let (out_pos, _) = random_decimate_core(&positions, &[], 100);
        assert_eq!(out_pos.len(), 6); // keeps all
    }

    #[test]
    fn test_random_decimation_empty() {
        let (out_pos, _) = random_decimate_core(&[], &[], 5);
        assert!(out_pos.is_empty());
    }

    #[test]
    fn test_random_decimation_deterministic() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let (a, _) = random_decimate_core(&positions, &[], 2);
        let (b, _) = random_decimate_core(&positions, &[], 2);
        assert_eq!(a, b, "Same seed should produce same result");
    }
}
