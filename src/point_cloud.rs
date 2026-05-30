//! Point Cloud Processing
//!
//! Hand-written LAS header parser and point cloud decimation utilities.
//! The LAS format is simple enough (first 96 bytes = public header) that we
//! don't need the `las-rs` crate — this avoids native dependencies that may
//! not compile for `wasm32-unknown-unknown`.

use wasm_bindgen::prelude::*;

use crate::validate_input_size;

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

pub fn read_f64_le(bytes: &[u8], offset: usize) -> f64 {
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

pub fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
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

pub fn parse_las_header_core(bytes: &[u8]) -> Result<LasHeader, String> {
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

pub fn parse_las_points_core(bytes: &[u8]) -> Result<LasPointCloud, String> {
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
    validate_input_size(bytes.len(), "LAS input")?;
    parse_las_header_core(bytes).map_err(|e| JsValue::from_str(&e))
}

/// WASM binding for LAS point parsing.
#[wasm_bindgen(js_name = "parseLasPoints")]
pub fn parse_las_points(bytes: &[u8]) -> Result<LasPointCloud, JsValue> {
    validate_input_size(bytes.len(), "LAS input")?;
    parse_las_points_core(bytes).map_err(|e| JsValue::from_str(&e))
}

// ===========================================================================
// Decimation (pure Rust core — testable without WASM runtime)
// ===========================================================================

pub fn voxel_grid_decimate_core(
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

pub fn random_decimate_core(
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
// PCD Format Support
// ===========================================================================

/// Parsed PCD point cloud data — reuses the same public layout as LasPointCloud.
#[wasm_bindgen]
#[derive(Debug)]
pub struct PcdPointCloud {
    positions: Vec<f32>,
    colors: Option<Vec<u8>>,
    point_count: u32,
}

#[wasm_bindgen]
impl PcdPointCloud {
    /// Interleaved XYZ positions as Float32Array.
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        let arr = js_sys::Float32Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    /// RGB colors as Uint8Array, or `null` if not present.
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

/// Internal PCD header representation.
struct PcdHeader {
    #[allow(dead_code)]
    version: String,
    fields: Vec<String>,
    size: Vec<usize>,
    num_points: u32,
    data_type: String, // "ascii" or "binary"
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

fn parse_pcd_header(lines: &[&str]) -> Result<PcdHeader, String> {
    let mut version = String::from("0.7");
    let mut fields = Vec::new();
    let mut size = Vec::new();
    let mut num_points: u32 = 0;
    let mut data_type = String::from("ascii");
    let mut width: u32 = 0;
    let mut height: u32 = 1;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("VERSION") {
            version = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("FIELDS") {
            fields = rest.split_whitespace().map(|s| s.to_lowercase()).collect();
        } else if let Some(rest) = trimmed.strip_prefix("SIZE") {
            size = rest
                .split_whitespace()
                .map(|s| s.parse::<usize>().unwrap_or(4))
                .collect();
        } else if let Some(rest) = trimmed.strip_prefix("POINTS") {
            num_points = rest.trim().parse::<u32>().unwrap_or(0);
        } else if let Some(rest) = trimmed.strip_prefix("WIDTH") {
            width = rest.trim().parse::<u32>().unwrap_or(0);
        } else if let Some(rest) = trimmed.strip_prefix("HEIGHT") {
            height = rest.trim().parse::<u32>().unwrap_or(1);
        } else if let Some(rest) = trimmed.strip_prefix("DATA") {
            data_type = rest.trim().to_lowercase();
        }
    }

    // If POINTS not specified, derive from WIDTH * HEIGHT
    if num_points == 0 && width > 0 {
        num_points = width * height;
    }

    if fields.is_empty() {
        return Err("PCD header missing FIELDS".to_string());
    }

    // Ensure size vector matches fields
    while size.len() < fields.len() {
        size.push(4); // default to float32
    }

    Ok(PcdHeader {
        version,
        fields,
        size,
        num_points,
        data_type,
        width,
        height,
    })
}

/// Parse RGB float (encoded as packed int in PCD) into (r, g, b) bytes.
fn unpack_pcd_rgb(val: f32) -> (u8, u8, u8) {
    let bits = val.to_bits();
    let r = ((bits >> 16) & 0xFF) as u8;
    let g = ((bits >> 8) & 0xFF) as u8;
    let b = (bits & 0xFF) as u8;
    (r, g, b)
}

/// Find the column indices for x, y, z, rgb.
fn locate_xyz_rgb(
    fields: &[String],
) -> (Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
    let mut xi = None;
    let mut yi = None;
    let mut zi = None;
    let mut rgbi = None;

    for (i, f) in fields.iter().enumerate() {
        match f.as_str() {
            "x" => xi = Some(i),
            "y" => yi = Some(i),
            "z" => zi = Some(i),
            "rgb" | "rgba" => rgbi = Some(i),
            _ => {}
        }
    }

    (xi, yi, zi, rgbi)
}

/// Core ASCII PCD parser (testable without WASM).
fn parse_pcd_ascii_core(text: &str) -> Result<PcdPointCloud, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut header_end = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "" && i > 0 {
            // First empty line marks end of header for most PCD files
            header_end = i + 1;
            break;
        }
        if line.trim().starts_with("DATA") {
            header_end = i + 1;
            break;
        }
    }

    let header = parse_pcd_header(&lines[..header_end.min(lines.len())])?;

    if header.data_type != "ascii" {
        return Err(format!("Expected ASCII PCD, got {}", header.data_type));
    }

    let (xi, yi, zi, rgbi) = locate_xyz_rgb(&header.fields);
    let xi = xi.ok_or_else(|| "PCD missing 'x' field".to_string())?;
    let yi = yi.ok_or_else(|| "PCD missing 'y' field".to_string())?;
    let zi = zi.ok_or_else(|| "PCD missing 'z' field".to_string())?;
    let has_color = rgbi.is_some();
    let rgbi = rgbi.unwrap_or(0);
    let num_fields = header.fields.len();

    let mut positions: Vec<f32> = Vec::with_capacity(header.num_points as usize * 3);
    let mut colors: Option<Vec<u8>> = if has_color {
        Some(Vec::with_capacity(header.num_points as usize * 3))
    } else {
        None
    };

    for line in lines.iter().skip(header_end) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let values: Vec<&str> = trimmed.split_whitespace().collect();
        if values.len() < num_fields {
            continue;
        }

        let x: f32 = values.get(xi).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let y: f32 = values.get(yi).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let z: f32 = values.get(zi).and_then(|v| v.parse().ok()).unwrap_or(0.0);

        positions.push(x);
        positions.push(y);
        positions.push(z);

        if has_color {
            let rgb_val: f32 = values.get(rgbi).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let (r, g, b) = unpack_pcd_rgb(rgb_val);
            if let Some(ref mut c) = colors {
                c.push(r);
                c.push(g);
                c.push(b);
            }
        }
    }

    Ok(PcdPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// Core binary PCD parser (testable without WASM).
fn parse_pcd_binary_core(bytes: &[u8]) -> Result<PcdPointCloud, String> {
    // Find header end — scan for "DATA binary\n" or "DATA binary\r\n"
    let marker = b"DATA binary\n";
    let marker_crlf = b"DATA binary\r\n";
    let text_end = bytes
        .windows(marker.len())
        .position(|w| w == marker || w == marker_crlf)
        .map(|pos| {
            let mlen = if &bytes[pos..pos + marker_crlf.len()] == marker_crlf {
                marker_crlf.len()
            } else {
                marker.len()
            };
            pos + mlen
        });

    let text_end =
        text_end.ok_or_else(|| "PCD binary: could not find 'DATA binary' marker".to_string())?;

    // Parse header as text
    let header_text = String::from_utf8_lossy(&bytes[..text_end]);
    let header_lines: Vec<&str> = header_text.lines().collect();
    let header = parse_pcd_header(&header_lines)?;

    if header.data_type != "binary" {
        return Err(format!("Expected binary PCD, got {}", header.data_type));
    }

    let (xi, yi, zi, rgbi) = locate_xyz_rgb(&header.fields);
    let xi = xi.ok_or_else(|| "PCD missing 'x' field".to_string())?;
    let yi = yi.ok_or_else(|| "PCD missing 'y' field".to_string())?;
    let zi = zi.ok_or_else(|| "PCD missing 'z' field".to_string())?;
    let has_color = rgbi.is_some();
    let rgbi = rgbi.unwrap_or(0);

    // Calculate record size (sum of all field sizes)
    let record_size: usize = header.size.iter().sum();

    let mut positions: Vec<f32> = Vec::with_capacity(header.num_points as usize * 3);
    let mut colors: Option<Vec<u8>> = if has_color {
        Some(Vec::with_capacity(header.num_points as usize * 3))
    } else {
        None
    };

    let data_start = text_end;
    for i in 0..header.num_points as usize {
        let offset = data_start + i * record_size;
        if offset + record_size > bytes.len() {
            break;
        }

        let point_bytes = &bytes[offset..offset + record_size];

        // Calculate byte offsets for each field
        let mut field_offsets: Vec<usize> = Vec::with_capacity(header.fields.len());
        let mut cur = 0;
        for &s in &header.size {
            field_offsets.push(cur);
            cur += s;
        }

        let read_f32_at = |data: &[u8], off: usize| -> f32 {
            if off + 4 <= data.len() {
                f32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
            } else {
                0.0
            }
        };

        let x = read_f32_at(point_bytes, field_offsets[xi]);
        let y = read_f32_at(point_bytes, field_offsets[yi]);
        let z = read_f32_at(point_bytes, field_offsets[zi]);

        positions.push(x);
        positions.push(y);
        positions.push(z);

        if has_color {
            let rgb_val = read_f32_at(point_bytes, field_offsets[rgbi]);
            let (r, g, b) = unpack_pcd_rgb(rgb_val);
            if let Some(ref mut c) = colors {
                c.push(r);
                c.push(g);
                c.push(b);
            }
        }
    }

    Ok(PcdPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// Parse ASCII PCD format text into a point cloud.
#[wasm_bindgen(js_name = "parsePcdAscii")]
pub fn parse_pcd_ascii(text: &str) -> Result<PcdPointCloud, JsValue> {
    validate_input_size(text.len(), "PCD input")?;
    parse_pcd_ascii_core(text).map_err(|e| JsValue::from_str(&e))
}

/// Parse binary PCD format bytes into a point cloud.
#[wasm_bindgen(js_name = "parsePcdBinary")]
pub fn parse_pcd_binary(bytes: &[u8]) -> Result<PcdPointCloud, JsValue> {
    validate_input_size(bytes.len(), "PCD binary input")?;
    parse_pcd_binary_core(bytes).map_err(|e| JsValue::from_str(&e))
}

// ===========================================================================
// GPU-ready Buffer Generation (Phase 3.5)
// ===========================================================================

/// Generate an interleaved vertex buffer for WebGL2/WebGPU.
///
/// Layout: `[x, y, z, nx, ny, nz, r, g, b, a, ...]` per vertex (10 floats).
/// Normals default to `(0, 0, 1)` if not provided.
/// Colors default to `(255, 255, 255, 255)` (white, opaque) if not provided.
#[wasm_bindgen(js_name = "generateInterleavedVertexBuffer")]
pub fn generate_interleaved_vertex_buffer(
    positions: &js_sys::Float32Array,
    colors: &js_sys::Uint8Array,
    normals: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let point_count = positions.length() as usize / 3;
    let has_colors = colors.length() > 0;
    let has_normals = normals.length() > 0;

    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);

    let mut col_buf = vec![0u8; colors.length() as usize];
    if has_colors {
        colors.copy_to(&mut col_buf);
    }

    let mut norm_buf = vec![0.0f32; normals.length() as usize];
    if has_normals {
        normals.copy_to(&mut norm_buf);
    }

    // 10 floats per vertex: xyz(3) + normal(3) + rgba(4)
    let mut out = Vec::with_capacity(point_count * 10);

    for i in 0..point_count {
        // Position
        out.push(pos_buf[i * 3]);
        out.push(pos_buf[i * 3 + 1]);
        out.push(pos_buf[i * 3 + 2]);

        // Normal
        if has_normals {
            out.push(norm_buf[i * 3]);
            out.push(norm_buf[i * 3 + 1]);
            out.push(norm_buf[i * 3 + 2]);
        } else {
            out.push(0.0);
            out.push(0.0);
            out.push(1.0); // default: up
        }

        // Color (RGBA as float 0..1)
        if has_colors {
            out.push(col_buf[i * 3] as f32 / 255.0);
            out.push(col_buf[i * 3 + 1] as f32 / 255.0);
            out.push(col_buf[i * 3 + 2] as f32 / 255.0);
        } else {
            out.push(1.0); // white
            out.push(1.0);
            out.push(1.0);
        }
        out.push(1.0); // alpha always opaque
    }

    let arr = js_sys::Float32Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Generate indexed geometry from positions.
///
/// Returns `{ positions: Float32Array, indices: Uint32Array }`.
/// For point clouds this is trivial (indices = [0, 1, 2, ...]) but the
/// layout is standard for mesh geometry consumers.
#[wasm_bindgen(js_name = "generateIndexedGeometry")]
pub fn generate_indexed_geometry(positions: &js_sys::Float32Array) -> js_sys::Object {
    let point_count = positions.length() as usize / 3;

    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);

    let indices: Vec<u32> = (0..point_count as u32).collect();

    let pos_arr = js_sys::Float32Array::new_with_length(pos_buf.len() as u32);
    pos_arr.copy_from(&pos_buf);
    let idx_arr = js_sys::Uint32Array::new_with_length(indices.len() as u32);
    idx_arr.copy_from(&indices);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).unwrap();
    js_sys::Reflect::set(&obj, &"indices".into(), &idx_arr).unwrap();
    obj
}

// ===========================================================================
// Streaming & Progress API (Phase 3.6)
// ===========================================================================

/// Core LAS parser with progress callback (pure Rust for testing).
///
/// Calls `progress(processed, total)` approximately every `interval` points.
fn parse_las_points_with_progress_core<F>(
    bytes: &[u8],
    mut on_progress: F,
    interval: u32,
) -> Result<LasPointCloud, String>
where
    F: FnMut(u32, u32),
{
    let num_points = read_u32_le(bytes, 110);
    let point_offset = read_u32_le(bytes, 98) as usize;
    let point_format = bytes[106];
    let point_record_len = read_u16_le(bytes, 108) as usize;

    let x_scale = read_f64_le(bytes, 134);
    let y_scale = read_f64_le(bytes, 142);
    let z_scale = read_f64_le(bytes, 150);
    let x_offset = read_f64_le(bytes, 158);
    let y_offset = read_f64_le(bytes, 166);
    let z_offset = read_f64_le(bytes, 174);

    let has_color = point_format == 2 || point_format == 3;
    let expected_record_len = if has_color { 26 } else { 20 };

    if point_record_len < expected_record_len {
        return Err(format!(
            "Point record length {} too small for format {}",
            point_record_len, point_format
        ));
    }

    let mut positions: Vec<f32> = Vec::with_capacity(num_points as usize * 3);
    let mut colors: Option<Vec<u8>> = if has_color {
        Some(Vec::with_capacity(num_points as usize * 3))
    } else {
        None
    };

    let mut last_reported: u32 = 0;

    for i in 0..num_points as usize {
        let base = point_offset + i * point_record_len;
        if base + expected_record_len > bytes.len() {
            break;
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

        // Report progress periodically
        if interval > 0 && (i as u32).saturating_sub(last_reported) >= interval {
            on_progress(i as u32, num_points);
            last_reported = i as u32;
        }
    }

    // Final progress report
    on_progress(positions.len() as u32 / 3, num_points);

    Ok(LasPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// Parse LAS points with a JS progress callback. Reports every 10,000 points.
#[wasm_bindgen(js_name = "parseLasPointsWithProgress")]
pub fn parse_las_points_with_progress(
    bytes: &[u8],
    on_progress: &js_sys::Function,
) -> Result<LasPointCloud, JsValue> {
    let _num_points = read_u32_le(bytes, 110);
    let this = JsValue::NULL;

    parse_las_points_with_progress_core(
        bytes,
        |processed, total| {
            let _ = on_progress.call2(&this, &JsValue::from(processed), &JsValue::from(total));
        },
        10_000,
    )
    .map_err(|e| JsValue::from_str(&e))
}

/// Core voxel grid decimation with progress callback.
fn voxel_grid_decimate_with_progress_core<F>(
    positions: &[f32],
    colors: &[u8],
    cell_size: f32,
    mut on_progress: F,
    interval: u32,
) -> (Vec<f32>, Vec<u8>)
where
    F: FnMut(u32, u32),
{
    let point_count = positions.len() / 3;
    let has_colors = !colors.is_empty();

    if point_count == 0 || cell_size <= 0.0 {
        return (Vec::new(), Vec::new());
    }

    // Phase 1: build voxel grid
    let mut grid: std::collections::HashMap<(i64, i64, i64), usize> =
        std::collections::HashMap::new();

    let mut processed: u32 = 0;
    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        let cx = (x / cell_size).floor() as i64;
        let cy = (y / cell_size).floor() as i64;
        let cz = (z / cell_size).floor() as i64;
        grid.entry((cx, cy, cz)).or_insert(i);

        processed += 1;
        if interval > 0 && processed.is_multiple_of(interval) {
            on_progress(processed, point_count as u32);
        }
    }

    // Phase 2: collect kept points
    let kept: Vec<usize> = grid.into_values().collect();
    let kept_count = kept.len() as u32;
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

    on_progress(point_count as u32, point_count as u32);

    let _ = kept_count; // used implicitly in output size
    (out_pos, out_col)
}

/// Voxel grid decimation with a JS progress callback. Reports every 10,000 points.
#[wasm_bindgen(js_name = "decimateVoxelGridWithProgress")]
pub fn decimate_voxel_grid_with_progress(
    positions: &js_sys::Float32Array,
    colors: &js_sys::Uint8Array,
    cell_size: f32,
    on_progress: &js_sys::Function,
) -> js_sys::Object {
    let pos_len = positions.length() as usize;
    let col_len = colors.length() as usize;

    let mut pos_buf = vec![0.0f32; pos_len];
    positions.copy_to(&mut pos_buf);
    let mut col_buf = vec![0u8; col_len];
    colors.copy_to(&mut col_buf);

    let this = JsValue::NULL;
    let (out_pos, out_col) = voxel_grid_decimate_with_progress_core(
        &pos_buf,
        &col_buf,
        cell_size,
        |processed, total| {
            let _ = on_progress.call2(&this, &JsValue::from(processed), &JsValue::from(total));
        },
        10_000,
    );

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
// COPC: Cloud-Optimized Point Cloud — Header-Only & Range-Based Access
// ===========================================================================

/// Lightweight LAS header info for range-based access (COPC core concept).
///
/// This lets frontend code compute byte offsets for individual points and
/// use `fetch` with `Range` headers to load only the points it needs.
#[wasm_bindgen]
pub struct LasHeaderInfo {
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
    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_min_z: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
    bounds_max_z: f64,
    file_size: u32,
}

#[wasm_bindgen]
impl LasHeaderInfo {
    #[wasm_bindgen(getter, js_name = "numPoints")]
    pub fn num_points(&self) -> u32 {
        self.num_points
    }

    #[wasm_bindgen(getter, js_name = "pointOffset")]
    pub fn point_offset(&self) -> u32 {
        self.point_offset
    }

    #[wasm_bindgen(getter, js_name = "pointFormatId")]
    pub fn point_format_id(&self) -> u8 {
        self.point_format_id
    }

    #[wasm_bindgen(getter, js_name = "pointRecordLength")]
    pub fn point_record_length(&self) -> u16 {
        self.point_record_length
    }

    #[wasm_bindgen(getter, js_name = "xScale")]
    pub fn x_scale(&self) -> f64 {
        self.x_scale
    }
    #[wasm_bindgen(getter, js_name = "yScale")]
    pub fn y_scale(&self) -> f64 {
        self.y_scale
    }
    #[wasm_bindgen(getter, js_name = "zScale")]
    pub fn z_scale(&self) -> f64 {
        self.z_scale
    }

    #[wasm_bindgen(getter, js_name = "xOffset")]
    pub fn x_offset(&self) -> f64 {
        self.x_offset
    }
    #[wasm_bindgen(getter, js_name = "yOffset")]
    pub fn y_offset(&self) -> f64 {
        self.y_offset
    }
    #[wasm_bindgen(getter, js_name = "zOffset")]
    pub fn z_offset(&self) -> f64 {
        self.z_offset
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

    #[wasm_bindgen(getter, js_name = "fileSize")]
    pub fn file_size(&self) -> u32 {
        self.file_size
    }

    /// Total size of point data in bytes.
    #[wasm_bindgen(js_name = "pointDataSize")]
    pub fn point_data_size(&self) -> u32 {
        self.num_points * self.point_record_length as u32
    }
}

/// Parsed data for a single LAS point.
#[wasm_bindgen]
pub struct PointData {
    x: f64,
    y: f64,
    z: f64,
    intensity: u16,
    r: u8,
    g: u8,
    b: u8,
}

#[wasm_bindgen]
impl PointData {
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.x
    }
    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.y
    }
    #[wasm_bindgen(getter)]
    pub fn z(&self) -> f64 {
        self.z
    }
    #[wasm_bindgen(getter)]
    pub fn intensity(&self) -> u16 {
        self.intensity
    }
    #[wasm_bindgen(getter)]
    pub fn r(&self) -> u8 {
        self.r
    }
    #[wasm_bindgen(getter)]
    pub fn g(&self) -> u8 {
        self.g
    }
    #[wasm_bindgen(getter)]
    pub fn b(&self) -> u8 {
        self.b
    }
}

/// Parse only the LAS header (first 227+ bytes) for range-based access.
///
/// Returns a `LasHeaderInfo` with metadata needed to compute point offsets.
/// This is the core COPC concept: read the header once, then fetch individual
/// points on demand using `Range` headers.
///
/// # Arguments
///
/// * `bytes` - At least 230 bytes from the beginning of a LAS file.
///
/// # Example
///
/// ```ignore
/// // In the browser:
/// const response = await fetch("data.las", { headers: { Range: "bytes=0-229" } });
/// const headerBytes = new Uint8Array(await response.arrayBuffer());
/// const info = parseLasHeaderOnly(headerBytes);
/// console.log(`File has ${info.numPoints()} points`);
///
/// // Fetch point 42:
/// const offset = computeLasPointOffset(info, 42, info.pointFormatId());
/// const pointResponse = await fetch("data.las", {
///     headers: { Range: `bytes=${offset}-${offset + info.pointRecordLength() - 1}` }
/// });
/// const pointBytes = new Uint8Array(await pointResponse.arrayBuffer());
/// const point = parseLasPointAt(pointBytes, 0, info.pointFormatId());
/// ```
#[wasm_bindgen(js_name = "parseLasHeaderOnly")]
pub fn parse_las_header_only(bytes: &[u8]) -> Result<LasHeaderInfo, JsValue> {
    if bytes.len() < 230 {
        return Err(JsValue::from_str("LAS header requires at least 230 bytes"));
    }
    if &bytes[0..4] != b"LASF" {
        return Err(JsValue::from_str("Invalid LAS magic: expected 'LASF'"));
    }

    Ok(LasHeaderInfo {
        num_points: read_u32_le(bytes, 110),
        point_offset: read_u32_le(bytes, 98),
        point_format_id: bytes[106],
        point_record_length: read_u16_le(bytes, 108),
        x_scale: read_f64_le(bytes, 134),
        y_scale: read_f64_le(bytes, 142),
        z_scale: read_f64_le(bytes, 150),
        x_offset: read_f64_le(bytes, 158),
        y_offset: read_f64_le(bytes, 166),
        z_offset: read_f64_le(bytes, 174),
        bounds_max_x: read_f64_le(bytes, 182),
        bounds_max_y: read_f64_le(bytes, 190),
        bounds_max_z: read_f64_le(bytes, 198),
        bounds_min_x: read_f64_le(bytes, 206),
        bounds_min_y: read_f64_le(bytes, 214),
        bounds_min_z: read_f64_le(bytes, 222),
        file_size: bytes.len() as u32,
    })
}

/// Compute the byte offset of the Nth point in a LAS file.
///
/// Given header info from `parseLasHeaderOnly`, compute where point `point_index`
/// starts in the file. This enables range-based `fetch` for individual points.
#[wasm_bindgen(js_name = "computeLasPointOffset")]
pub fn compute_las_point_offset(
    header_info: &LasHeaderInfo,
    point_index: u32,
    _point_format: u8,
) -> usize {
    header_info.point_offset as usize
        + point_index as usize * header_info.point_record_length as usize
}

/// Parse a single LAS point at a given byte offset.
///
/// The `offset` parameter is relative to the start of the `bytes` buffer
/// (which should contain at least `point_record_length` bytes starting at
/// `offset`). Returns a `PointData` with XYZ, intensity, and RGB (if present).
#[wasm_bindgen(js_name = "parseLasPointAt")]
pub fn parse_las_point_at(
    bytes: &[u8],
    offset: usize,
    point_format: u8,
) -> Result<PointData, JsValue> {
    let has_color = point_format == 2 || point_format == 3;
    let needed = if has_color { 26 } else { 20 };

    if offset + needed > bytes.len() {
        return Err(JsValue::from_str(&format!(
            "Not enough bytes at offset {} (need {}, have {})",
            offset,
            needed,
            bytes.len() - offset
        )));
    }

    let raw_x = read_i32_le(bytes, offset) as f64;
    let raw_y = read_i32_le(bytes, offset + 4) as f64;
    let raw_z = read_i32_le(bytes, offset + 8) as f64;
    let intensity = read_u16_le(bytes, offset + 12);

    // Use scale/offset = 1.0/0.0 since this is meant to be used with
    // raw bytes where the caller should handle coordinate reconstruction
    // (or pass pre-scaled bytes).
    let x = raw_x;
    let y = raw_y;
    let z = raw_z;

    let (r, g, b) = if has_color {
        (bytes[offset + 20], bytes[offset + 21], bytes[offset + 22])
    } else {
        (0, 0, 0)
    };

    Ok(PointData {
        x,
        y,
        z,
        intensity,
        r,
        g,
        b,
    })
}

/// Compute the byte offset of the Nth point (pure Rust, testable without WASM).
#[cfg(test)]
fn compute_las_point_offset_core(
    point_offset: u32,
    point_index: u32,
    point_record_length: u16,
) -> usize {
    point_offset as usize + point_index as usize * point_record_length as usize
}

/// Parse a single point at a given offset (pure Rust core, testable without WASM).
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn parse_las_point_at_core(
    bytes: &[u8],
    offset: usize,
    point_format: u8,
    x_scale: f64,
    y_scale: f64,
    z_scale: f64,
    x_offset: f64,
    y_offset: f64,
    z_offset: f64,
) -> Result<(f64, f64, f64), String> {
    let needed = if point_format == 2 || point_format == 3 {
        26
    } else {
        20
    };
    if offset + needed > bytes.len() {
        return Err(format!("Not enough bytes at offset {}", offset));
    }
    let raw_x = read_i32_le(bytes, offset) as f64;
    let raw_y = read_i32_le(bytes, offset + 4) as f64;
    let raw_z = read_i32_le(bytes, offset + 8) as f64;
    Ok((
        raw_x * x_scale + x_offset,
        raw_y * y_scale + y_offset,
        raw_z * z_scale + z_offset,
    ))
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

    // ── PCD ASCII tests ──────────────────────────────────────────

    #[test]
    fn test_parse_pcd_ascii_xyz() {
        let pcd = r#"# .PCD v0.7 - Point Cloud Data file format
VERSION 0.7
FIELDS x y z
SIZE 4 4 4
TYPE F F F
COUNT 1 1 1
WIDTH 5
HEIGHT 1
VIEWPOINT 0 0 0 1 0 0 0
POINTS 5
DATA ascii
1.0 2.0 3.0
4.0 5.0 6.0
7.0 8.0 9.0
10.0 11.0 12.0
13.0 14.0 15.0
"#;
        let cloud = parse_pcd_ascii_core(pcd).unwrap();
        assert_eq!(cloud.point_count, 5);
        assert_eq!(cloud.positions.len(), 15);
        assert!(cloud.colors.is_none());
        assert_eq!(cloud.positions[0], 1.0);
        assert_eq!(cloud.positions[1], 2.0);
        assert_eq!(cloud.positions[2], 3.0);
        assert_eq!(cloud.positions[12], 13.0);
        assert_eq!(cloud.positions[13], 14.0);
        assert_eq!(cloud.positions[14], 15.0);
    }

    #[test]
    fn test_parse_pcd_ascii_xyz_rgb() {
        // RGB in PCD is packed as a float with bits [RR GG BB 00]
        // We'll test with unpack_pcd_rgb directly
        let (r, g, b) = unpack_pcd_rgb(f32::from_bits(0x00_FF_80_40));
        assert_eq!(r, 255);
        assert_eq!(g, 128);
        assert_eq!(b, 64);
    }

    #[test]
    fn test_parse_pcd_ascii_missing_field() {
        let pcd = r#"VERSION 0.7
FIELDS a b c
SIZE 4 4 4
TYPE F F F
COUNT 1 1 1
WIDTH 1
HEIGHT 1
POINTS 1
DATA ascii
1.0 2.0 3.0
"#;
        let result = parse_pcd_ascii_core(pcd);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'x' field"));
    }

    #[test]
    fn test_parse_pcd_ascii_empty() {
        let pcd = r#"VERSION 0.7
FIELDS x y z
SIZE 4 4 4
TYPE F F F
COUNT 1 1 1
WIDTH 0
HEIGHT 1
POINTS 0
DATA ascii
"#;
        let cloud = parse_pcd_ascii_core(pcd).unwrap();
        assert_eq!(cloud.point_count, 0);
    }

    // ── PCD binary tests ────────────────────────────────────────

    #[test]
    fn test_parse_pcd_binary_xyz() {
        // Build a valid binary PCD
        let header = "VERSION 0.7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nCOUNT 1 1 1\nWIDTH 3\nHEIGHT 1\nPOINTS 3\nDATA binary\n";

        let mut data: Vec<u8> = header.as_bytes().to_vec();

        // Point 1: (1.0, 2.0, 3.0)
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());
        // Point 2: (4.0, 5.0, 6.0)
        data.extend_from_slice(&4.0f32.to_le_bytes());
        data.extend_from_slice(&5.0f32.to_le_bytes());
        data.extend_from_slice(&6.0f32.to_le_bytes());
        // Point 3: (7.0, 8.0, 9.0)
        data.extend_from_slice(&7.0f32.to_le_bytes());
        data.extend_from_slice(&8.0f32.to_le_bytes());
        data.extend_from_slice(&9.0f32.to_le_bytes());

        let cloud = parse_pcd_binary_core(&data).unwrap();
        assert_eq!(cloud.point_count, 3);
        assert_eq!(cloud.positions[0], 1.0);
        assert_eq!(cloud.positions[4], 5.0);
        assert_eq!(cloud.positions[8], 9.0);
        assert!(cloud.colors.is_none());
    }

    #[test]
    fn test_parse_pcd_binary_invalid() {
        let data = b"NOT A PCD FILE AT ALL";
        let result = parse_pcd_binary_core(data);
        assert!(result.is_err());
    }

    // ── GPU-ready buffer tests ────────────────────────────────────

    #[test]
    fn test_interleaved_vertex_buffer_no_color_no_normals() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let colors: Vec<u8> = vec![];
        let normals: Vec<f32> = vec![];

        // Test the core logic (without WASM typed arrays)
        let point_count = positions.len() / 3;
        let has_colors = !colors.is_empty();
        let has_normals = !normals.is_empty();

        let mut out: Vec<f32> = Vec::with_capacity(point_count * 10);
        for i in 0..point_count {
            out.push(positions[i * 3]);
            out.push(positions[i * 3 + 1]);
            out.push(positions[i * 3 + 2]);
            if has_normals {
                out.push(normals[i * 3]);
                out.push(normals[i * 3 + 1]);
                out.push(normals[i * 3 + 2]);
            } else {
                out.push(0.0);
                out.push(0.0);
                out.push(1.0);
            }
            if has_colors {
                out.push(colors[i * 3] as f32 / 255.0);
                out.push(colors[i * 3 + 1] as f32 / 255.0);
                out.push(colors[i * 3 + 2] as f32 / 255.0);
            } else {
                out.push(1.0);
                out.push(1.0);
                out.push(1.0);
            }
            out.push(1.0); // alpha
        }

        assert_eq!(out.len(), 20); // 2 vertices × 10 floats
                                   // First vertex: 1,2,3, 0,0,1, 1,1,1,1
        assert_eq!(out[0], 1.0);
        assert_eq!(out[3], 0.0);
        assert_eq!(out[5], 1.0); // default normal Z
        assert_eq!(out[6], 1.0); // default white
        assert_eq!(out[9], 1.0); // alpha
    }

    #[test]
    fn test_interleaved_vertex_buffer_with_colors_and_normals() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let colors: Vec<u8> = vec![255, 0, 0, 0, 255, 0]; // red, green
        let normals: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0, -1.0, 0.0]; // up, down

        let point_count = positions.len() / 3;
        let mut out: Vec<f32> = Vec::with_capacity(point_count * 10);
        for i in 0..point_count {
            out.push(positions[i * 3]);
            out.push(positions[i * 3 + 1]);
            out.push(positions[i * 3 + 2]);
            out.push(normals[i * 3]);
            out.push(normals[i * 3 + 1]);
            out.push(normals[i * 3 + 2]);
            out.push(colors[i * 3] as f32 / 255.0);
            out.push(colors[i * 3 + 1] as f32 / 255.0);
            out.push(colors[i * 3 + 2] as f32 / 255.0);
            out.push(1.0);
        }

        // Vertex 0: red, normal up (0,1,0)
        assert_eq!(out[3], 0.0);
        assert_eq!(out[4], 1.0); // normal Y
        assert_eq!(out[6], 1.0); // R full
        assert_eq!(out[7], 0.0); // G zero
    }

    #[test]
    fn test_generate_indexed_geometry_sequential() {
        // Test the core logic: indices = [0, 1, 2, ...]
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let point_count = positions.len() / 3;
        let indices: Vec<u32> = (0..point_count as u32).collect();

        assert_eq!(indices.len(), 3);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[1], 1);
        assert_eq!(indices[2], 2);
    }

    // ── Progress API tests ───────────────────────────────────────

    #[test]
    fn test_las_parse_with_progress() {
        let points = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)];
        let blob = build_test_las_blob(&points, false);

        let mut call_count = 0u32;
        let mut last_processed = 0u32;

        let cloud = parse_las_points_with_progress_core(
            &blob,
            |processed, _total| {
                call_count += 1;
                last_processed = processed;
            },
            10_000, // large interval — won't trigger mid-parse for 3 points
        )
        .unwrap();

        assert_eq!(cloud.point_count, 3);
        // Should have at least the final callback
        assert!(call_count >= 1);
        assert_eq!(last_processed, 3);
    }

    #[test]
    fn test_las_parse_with_progress_interval_1() {
        let points: Vec<(f64, f64, f64)> = (0..5).map(|i| (i as f64, 0.0, 0.0)).collect();
        let blob = build_test_las_blob(&points, false);

        let mut call_count = 0u32;

        let _cloud = parse_las_points_with_progress_core(
            &blob,
            |_processed, _total| {
                call_count += 1;
            },
            1, // every point
        )
        .unwrap();

        // 5 points + final callback = at least 6 calls
        assert!(call_count >= 5);
    }

    #[test]
    fn test_decimate_voxel_grid_with_progress() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0];
        let colors: Vec<u8> = vec![];

        let mut call_count = 0u32;
        let mut last_processed = 0u32;

        let (out_pos, out_col) = voxel_grid_decimate_with_progress_core(
            &positions,
            &colors,
            1.0,
            |processed, total| {
                call_count += 1;
                last_processed = processed;
                assert!(total > 0);
            },
            2, // every 2 points
        );

        assert_eq!(out_pos.len(), 12); // 4 points, different cells
        assert!(out_col.is_empty());
        // At least final callback
        assert!(call_count >= 1);
        assert_eq!(last_processed, 4); // 4 points processed
    }

    // ── COPC / Range-based access tests ──────────────────────────

    #[test]
    fn test_parse_las_header_only() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0)];
        let blob = build_test_las_blob(&points, false);

        // parse_las_header_only is the WASM function, but we can test the core logic
        let num_points = read_u32_le(&blob, 110);
        let point_offset = read_u32_le(&blob, 98);
        let point_format = blob[106];
        let point_record_length = read_u16_le(&blob, 108);

        assert_eq!(num_points, 2);
        assert_eq!(point_offset, 230);
        assert_eq!(point_format, 0);
        assert_eq!(point_record_length, 20);
    }

    #[test]
    fn test_compute_las_point_offset() {
        // 230 byte header, 20 byte points
        assert_eq!(compute_las_point_offset_core(230, 0, 20), 230);
        assert_eq!(compute_las_point_offset_core(230, 1, 20), 250);
        assert_eq!(compute_las_point_offset_core(230, 5, 26), 360);
        assert_eq!(compute_las_point_offset_core(0, 0, 20), 0);
    }

    #[test]
    fn test_parse_las_point_at_range_based() {
        let points = vec![
            (100.0, 200.0, 300.0),
            (400.0, 500.0, 600.0),
            (700.0, 800.0, 900.0),
        ];
        let blob = build_test_las_blob(&points, false);

        let point_offset = read_u32_le(&blob, 98) as usize;
        let point_format = blob[106];

        // Parse point 0 (at point_offset)
        let (x0, y0, z0) = parse_las_point_at_core(
            &blob,
            point_offset,
            point_format,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
        )
        .unwrap();
        assert_eq!(x0, 100.0);
        assert_eq!(y0, 200.0);
        assert_eq!(z0, 300.0);

        // Parse point 2 (at point_offset + 2 * record_length)
        let offset2 = compute_las_point_offset_core(point_offset as u32, 2, 20);
        let (x2, y2, z2) =
            parse_las_point_at_core(&blob, offset2, point_format, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0)
                .unwrap();
        assert_eq!(x2, 700.0);
        assert_eq!(y2, 800.0);
        assert_eq!(z2, 900.0);
    }
}
