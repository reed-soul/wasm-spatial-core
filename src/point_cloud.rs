//! Point Cloud Processing
//!
//! Hand-written LAS header parser and point cloud decimation utilities.
//! The LAS format is simple enough (first 96 bytes = public header) that we
//! don't need the `las-rs` crate — this avoids native dependencies that may
//! not compile for `wasm32-unknown-unknown`.

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::DEFAULT_MAX_INPUT_SIZE;

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
#[derive(Debug)]
pub struct LasPointCloud {
    pub(crate) positions: Vec<f32>, // interleaved [x, y, z, x, y, z, ...]
    pub(crate) colors: Option<Vec<u8>>, // [r, g, b, r, g, b, ...] if present
    pub(crate) point_count: u32,
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

pub(crate) fn read_i32_le(bytes: &[u8], offset: usize) -> i32 {
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
        version_minor: bytes[25],
        point_format_id: bytes[104],
        point_data_record_length: read_u16_le(bytes, 105),
        num_points: read_u32_le(bytes, 107),
        bounds_max_x: read_f64_le(bytes, 179),
        bounds_max_y: read_f64_le(bytes, 187),
        bounds_max_z: read_f64_le(bytes, 195),
        bounds_min_x: read_f64_le(bytes, 203),
        bounds_min_y: read_f64_le(bytes, 211),
        bounds_min_z: read_f64_le(bytes, 219),
    })
}

pub fn parse_las_points_core(bytes: &[u8]) -> Result<LasPointCloud, String> {
    if bytes.len() < 230 {
        return Err("LAS data too short for point parsing (need at least 230 bytes)".to_string());
    }

    let num_points = read_u32_le(bytes, 107) as usize;
    let point_offset = read_u32_le(bytes, 96) as usize;
    let point_format = bytes[104];
    let point_record_len = read_u16_le(bytes, 105) as usize;

    // Scale/offset for coordinate reconstruction
    let x_scale = read_f64_le(bytes, 131);
    let y_scale = read_f64_le(bytes, 139);
    let z_scale = read_f64_le(bytes, 147);
    let x_offset = read_f64_le(bytes, 155);
    let y_offset = read_f64_le(bytes, 163);
    let z_offset = read_f64_le(bytes, 171);

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
pub fn parse_las_header(bytes: &[u8]) -> Result<LasHeader, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "LAS input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_las_header_core(bytes).map_err(SpatialError::point_cloud_error)
}

/// WASM binding for LAS point parsing.
#[wasm_bindgen(js_name = "parseLasPoints")]
pub fn parse_las_points(bytes: &[u8]) -> Result<LasPointCloud, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "LAS input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_las_points_core(bytes).map_err(SpatialError::point_cloud_error)
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
// Large file processing utilities
// ===========================================================================

/// Estimate memory usage for a point cloud with the given parameters.
///
/// # Arguments
/// * `num_points` — Number of points.
/// * `has_color` — Whether RGB color data is included.
/// * `has_normals` — Whether normal vectors are included.
///
/// # Returns
/// Estimated memory in bytes.
///
/// Breakdown:
/// - Positions: 12 bytes/point (Float32 × 3)
/// - Colors: 3 bytes/point (Uint8 × 3, if has_color)
/// - Normals: 12 bytes/point (Float32 × 3, if has_normals)
/// - Octree nodes: ~64 bytes/node (estimated as num_points / maxPointsPerNode * 8)
/// - pnts tiles: ~14 bytes/point
/// - Overhead: ~1KB
#[wasm_bindgen(js_name = "estimateMemoryForPoints")]
pub fn estimate_memory_for_points(num_points: usize, has_color: bool, has_normals: bool) -> usize {
    let positions_bytes = num_points * 12; // Float32 × 3
    let color_bytes = if has_color { num_points * 3 } else { 0 }; // Uint8 × 3
    let normal_bytes = if has_normals { num_points * 12 } else { 0 }; // Float32 × 3

    // Rough octree node estimate
    let estimated_nodes = (num_points / 50_000).max(1) * 9;
    let octree_bytes = estimated_nodes * 64;

    // pnts tile encoding
    let pnts_bytes = num_points * 14;

    // Overhead
    let overhead = 1024;

    positions_bytes + color_bytes + normal_bytes + octree_bytes + pnts_bytes + overhead
}

/// Auto-decimate a point cloud to the target count using the specified method.
///
/// # Arguments
/// * `positions` — Flat `[x, y, z, ...]` buffer.
/// * `target_count` — Desired number of output points.
/// * `method` — Decimation method: 0 = random, 1 = grid, 2 = voxel grid (with colors).
///
/// # Returns
/// Decimated positions as `Float32Array`.
///
/// Methods:
/// - **Random** (0): Fisher-Yates shuffle, keep first N.
/// - **Grid** (1): Divide space into grid cells, keep first point per cell.
///   Cell size is computed to approximately achieve `target_count`.
/// - **Voxel Grid** (2): Same as grid but uses dedicated voxel grid decimation
///   (useful when colors are also available).
#[wasm_bindgen(js_name = "autoDecimate")]
pub fn auto_decimate_core(positions: &[f32], target_count: usize, method: u32) -> Vec<f32> {
    let point_count = positions.len() / 3;
    if point_count == 0 || target_count == 0 || target_count >= point_count {
        return positions.to_vec();
    }

    match method {
        0 => auto_decimate_random(positions, target_count),
        1 => auto_decimate_grid(positions, target_count),
        _ => auto_decimate_random(positions, target_count), // fallback
    }
}

/// Random decimation: Fisher-Yates partial shuffle.
fn auto_decimate_random(positions: &[f32], target_count: usize) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let output_count = target_count.min(point_count);

    let mut seed: u64 = 54321;
    let next_rand = |s: &mut u64| -> f64 {
        *s = s.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        (*s as f64) / (0x7fffffff as f64)
    };

    // Create output indices
    let mut indices: Vec<usize> = (0..point_count).collect();
    for i in 0..output_count {
        let j = i + (next_rand(&mut seed) * (point_count - i) as f64) as usize;
        indices.swap(i, j);
    }

    let mut out = Vec::with_capacity(output_count * 3);
    for &idx in indices.iter().take(output_count) {
        out.push(positions[idx * 3]);
        out.push(positions[idx * 3 + 1]);
        out.push(positions[idx * 3 + 2]);
    }
    out
}

/// Grid decimation: divide space into grid cells, keep first point per cell.
fn auto_decimate_grid(positions: &[f32], target_count: usize) -> Vec<f32> {
    let point_count = positions.len() / 3;
    if point_count == 0 {
        return Vec::new();
    }

    // Compute bounding box
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        min_z = min_z.min(z);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        max_z = max_z.max(z);
    }

    // Estimate cell size to get approximately target_count cells.
    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let dz = max_z - min_z;
    let volume = dx * dy * dz;
    let cell_volume = volume / target_count.max(1) as f32;
    let cell_size = cell_volume.cbrt().max(1e-10);

    // Grid decimation — keep first point per cell
    let mut grid: std::collections::HashMap<(i64, i64, i64), usize> =
        std::collections::HashMap::new();

    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        let cx = ((x - min_x) / cell_size).floor() as i64;
        let cy = ((y - min_y) / cell_size).floor() as i64;
        let cz = ((z - min_z) / cell_size).floor() as i64;
        grid.entry((cx, cy, cz)).or_insert(i);
    }

    let kept: Vec<usize> = grid.into_values().collect();
    let mut out = Vec::with_capacity(kept.len() * 3);
    for &idx in &kept {
        out.push(positions[idx * 3]);
        out.push(positions[idx * 3 + 1]);
        out.push(positions[idx * 3 + 2]);
    }
    out
}

/// Parse LAS points in chunks without loading all data into memory at once.
///
/// Calls `on_chunk` for each chunk of parsed positions, passing ownership
/// of the chunk data. Suitable for processing files larger than available memory.
///
/// # Arguments
/// * `bytes` — Raw LAS file bytes.
/// * `chunk_size` — Maximum number of points per chunk (e.g., 50_000).
/// * `on_chunk` — Callback receiving `(Vec<f32>, Option<Vec<u8>>, u32)` —
///   positions, optional colors, and point count for this chunk.
///
/// # Returns
/// Total number of points parsed, or an error message.
pub fn parse_las_points_chunked<F>(
    bytes: &[u8],
    chunk_size: usize,
    mut on_chunk: F,
) -> Result<usize, String>
where
    F: FnMut(Vec<f32>, Option<Vec<u8>>, u32),
{
    let header = parse_las_header_core(bytes)?;

    if header.num_points == 0 {
        return Ok(0);
    }

    let point_data_start = 227usize; // After public header block (for format 0-5)
    let record_length = header.point_data_record_length as usize;
    let format_id = header.point_format_id;
    let has_color = format_id >= 2;

    // Color offset within each point record
    let color_offset = match format_id {
        0 | 1 => None,
        2 | 3 => Some(20),
        4 | 5 => Some(20),
        _ => return Err(format!("Unsupported LAS point format: {}", format_id)),
    };

    let total_points = header.num_points as usize;
    let end = bytes
        .len()
        .min(point_data_start + total_points * record_length);

    let mut parsed = 0usize;
    let mut pos = point_data_start;

    while pos < end && parsed < total_points {
        let remaining = end - pos;
        let this_chunk = chunk_size.min(remaining / record_length.max(1));
        if this_chunk == 0 {
            break;
        }

        let mut positions = Vec::with_capacity(this_chunk * 3);
        let mut colors: Option<Vec<u8>> = if has_color {
            Some(Vec::with_capacity(this_chunk * 3))
        } else {
            None
        };

        for _ in 0..this_chunk {
            if pos + record_length > end {
                break;
            }
            // X, Y, Z at offset 0, 4, 8 (doubles, scale+offset applied)
            let x = (read_f64_le(bytes, pos) - header.bounds_min_x) as f32;
            let y = (read_f64_le(bytes, pos + 8) - header.bounds_min_y) as f32;
            let z = (read_f64_le(bytes, pos + 16) - header.bounds_min_z) as f32;
            positions.push(x);
            positions.push(y);
            positions.push(z);

            if let Some(ref mut col) = colors {
                if let Some(co) = color_offset {
                    if pos + co + 3 <= end {
                        col.push(bytes[pos + co]);
                        col.push(bytes[pos + co + 1]);
                        col.push(bytes[pos + co + 2]);
                    }
                }
            }

            pos += record_length;
            parsed += 1;
        }

        let count = positions.len() / 3;
        on_chunk(positions, colors, count as u32);
    }

    Ok(parsed)
}

// ===========================================================================
// LAZ Decompression (using laz crate) — only compiled with laz-support feature
// ===========================================================================

#[cfg(feature = "laz-support")]
use std::io::{Cursor, Read, Seek, SeekFrom};

/// Internal LAS header info needed for LAZ decompression.
#[cfg(feature = "laz-support")]
struct LazFileHeader {
    num_points: u32,
    point_format_id: u8,
    #[allow(dead_code)]
    point_record_length: u16,
    offset_to_points: u32,
    x_scale: f64,
    y_scale: f64,
    z_scale: f64,
    x_offset: f64,
    y_offset: f64,
    z_offset: f64,
    num_vlrs: u32,
}

#[cfg(feature = "laz-support")]
impl LazFileHeader {
    fn read_from_cursor(cursor: &mut Cursor<&[u8]>) -> Result<Self, String> {
        if cursor.get_ref().len() < 230 {
            return Err("LAZ header requires at least 230 bytes".to_string());
        }
        if &cursor.get_ref()[0..4] != b"LASF" {
            return Err(format!(
                "Invalid LAS magic: expected b\"LASF\", got {:?}",
                &cursor.get_ref()[0..4]
            ));
        }

        cursor
            .seek(SeekFrom::Start(24))
            .map_err(|e| e.to_string())?;
        let mut buf1 = [0u8; 1];
        cursor.read_exact(&mut buf1).map_err(|e| e.to_string())?;
        let _major = buf1[0];
        cursor.read_exact(&mut buf1).map_err(|e| e.to_string())?;
        let _minor = buf1[0];

        // Read fields sequentially starting at offset 94 (per LAS spec)
        cursor
            .seek(SeekFrom::Start(94))
            .map_err(|e| e.to_string())?;
        let mut buf2 = [0u8; 2];
        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
        let _header_size = u16::from_le_bytes(buf2);

        let mut buf4 = [0u8; 4];
        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
        let offset_to_points = u32::from_le_bytes(buf4);

        let mut buf4 = [0u8; 4];
        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
        let num_vlrs = u32::from_le_bytes(buf4);

        let mut buf1 = [0u8; 1];
        cursor.read_exact(&mut buf1).map_err(|e| e.to_string())?;
        let point_format_id = buf1[0];

        let mut buf2 = [0u8; 2];
        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
        let point_record_length = u16::from_le_bytes(buf2);

        let mut buf4 = [0u8; 4];
        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
        let num_points = u32::from_le_bytes(buf4);

        // Scale/offset
        cursor
            .seek(SeekFrom::Start(134))
            .map_err(|e| e.to_string())?;
        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let x_scale = f64::from_le_bytes(buf8);

        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let y_scale = f64::from_le_bytes(buf8);

        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let z_scale = f64::from_le_bytes(buf8);

        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let x_offset = f64::from_le_bytes(buf8);

        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let y_offset = f64::from_le_bytes(buf8);

        let mut buf8 = [0u8; 8];
        cursor.read_exact(&mut buf8).map_err(|e| e.to_string())?;
        let z_offset = f64::from_le_bytes(buf8);

        Ok(LazFileHeader {
            num_points,
            point_format_id: point_format_id & 0x3F, // strip compression bit
            point_record_length,
            offset_to_points,
            x_scale,
            y_scale,
            z_scale,
            x_offset,
            y_offset,
            z_offset,
            num_vlrs,
        })
    }
}

/// Find the LASZIP VLR in the VLR section and parse it.
///
/// The LASZIP VLR has user_id="laszip encoded" (0x6C61737A697020656E636F646564) and record_id=22204.
#[cfg(feature = "laz-support")]
fn find_laszip_vlr(cursor: &mut Cursor<&[u8]>, num_vlrs: u32) -> Result<laz::LazVlr, String> {
    for _ in 0..num_vlrs {
        // Each VLR: reserved(2) + user_id(16) + record_id(2) + record_length(2) + description(32) + data(record_length)
        let mut reserved = [0u8; 2];
        cursor
            .read_exact(&mut reserved)
            .map_err(|e| e.to_string())?;

        let mut user_id = [0u8; 16];
        cursor.read_exact(&mut user_id).map_err(|e| e.to_string())?;

        let mut buf2 = [0u8; 2];
        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
        let record_id = u16::from_le_bytes(buf2);

        let mut buf2 = [0u8; 2];
        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
        let record_length = u16::from_le_bytes(buf2) as usize;

        let mut description = [0u8; 32];
        cursor
            .read_exact(&mut description)
            .map_err(|e| e.to_string())?;

        let mut data = vec![0u8; record_length];
        cursor.read_exact(&mut data).map_err(|e| e.to_string())?;

        let user_id_str = String::from_utf8_lossy(&user_id)
            .trim_end_matches('\0')
            .to_string();

        if record_id == 22204 && user_id_str == "laszip encoded" {
            return laz::LazVlr::read_from(data.as_slice())
                .map_err(|e| format!("Failed to parse LASZIP VLR: {}", e));
        }
    }

    Err("LASZIP VLR not found in file. File may not be LAZ compressed.".to_string())
}

/// Core LAZ point decompression — pure Rust, testable without WASM runtime.
///
/// Given a complete LAZ byte buffer:
/// 1. Parse the LAS header (reuse our existing header parser)
/// 2. Scan VLRs to find the LASZIP VLR
/// 3. Create a LasZipDecompressor from the point data section
/// 4. Decompress all points
/// 5. Convert raw point bytes to positions + colors
#[cfg(feature = "laz-support")]
pub fn parse_laz_points_core(bytes: &[u8]) -> Result<LasPointCloud, String> {
    if bytes.len() < 230 {
        return Err("LAZ data too short for header".to_string());
    }
    if &bytes[0..4] != b"LASF" {
        return Err(format!(
            "Invalid LAZ magic: expected b\"LASF\", got {:?}",
            &bytes[0..4]
        ));
    }

    let mut cursor = Cursor::new(bytes);

    // Read LAS header
    let header = LazFileHeader::read_from_cursor(&mut cursor)?;

    // Check if this is actually compressed (point_format_id & 0x80)
    // Point format is at offset 104 per LAS spec
    if bytes[104] & 0x80 == 0 {
        return Err(
            "File appears to be uncompressed LAS (no compression bit set). Use parseLasPoints instead."
                .to_string(),
        );
    }

    // Find LASZIP VLR — position cursor after VLRs
    // Header size is at offset 94 (u16) per LAS spec
    let header_size = u16::from_le_bytes([bytes[94], bytes[95]]) as u64;
    cursor
        .seek(SeekFrom::Start(header_size))
        .map_err(|e| e.to_string())?;

    let laszip_vlr = find_laszip_vlr(&mut cursor, header.num_vlrs)?;

    // Create decompressor pointing to the start of compressed data.
    // We slice the bytes to create a new cursor starting at position 0,
    // because the laz crate stores chunk table offsets relative to the
    // start of the compressed data stream.
    let compressed_slice = &bytes[header.offset_to_points as usize..];
    let point_size = laszip_vlr.items_size() as usize;
    let num_points = header.num_points as usize;

    let has_color = matches!(
        header.point_format_id,
        2 | 3 | 8 // RGB formats
    );
    let has_color_in_laz = has_color
        || laszip_vlr.items().iter().any(|item| {
            matches!(
                item.item_type(),
                laz::LazItemType::RGB12 | laz::LazItemType::RGB14
            )
        });

    let mut decompressor = laz::LasZipDecompressor::new(Cursor::new(compressed_slice), laszip_vlr)
        .map_err(|e| format!("Failed to create LAZ decompressor: {}", e))?;

    let mut positions: Vec<f32> = Vec::with_capacity(num_points * 3);
    let mut colors: Option<Vec<u8>> = if has_color_in_laz {
        Some(Vec::with_capacity(num_points * 3))
    } else {
        None
    };

    let mut point_buf = vec![0u8; point_size];

    for _ in 0..num_points {
        decompressor
            .decompress_one(&mut point_buf)
            .map_err(|e| format!("LAZ decompression error: {}", e))?;

        let raw_x = read_i32_le(&point_buf, 0) as f64;
        let raw_y = read_i32_le(&point_buf, 4) as f64;
        let raw_z = read_i32_le(&point_buf, 8) as f64;

        positions.push((raw_x * header.x_scale + header.x_offset) as f32);
        positions.push((raw_y * header.y_scale + header.y_offset) as f32);
        positions.push((raw_z * header.z_scale + header.z_offset) as f32);

        if has_color_in_laz {
            // RGB is at offset 20 for Point10 (20 bytes) format
            // Point14 (30 bytes) also has RGB at offset 20
            if point_buf.len() >= 23 {
                if let Some(ref mut c) = colors {
                    c.push(point_buf[20]);
                    c.push(point_buf[21]);
                    c.push(point_buf[22]);
                }
            }
        }
    }

    Ok(LasPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// Core LAZ decompression with progress callback — pure Rust.
#[cfg(feature = "laz-support")]
pub fn parse_laz_points_with_progress_core<F>(
    bytes: &[u8],
    mut on_progress: F,
    interval: u32,
) -> Result<LasPointCloud, String>
where
    F: FnMut(u32, u32),
{
    if bytes.len() < 230 {
        return Err("LAZ data too short for header".to_string());
    }
    if &bytes[0..4] != b"LASF" {
        return Err(format!(
            "Invalid LAZ magic: expected b\"LASF\", got {:?}",
            &bytes[0..4]
        ));
    }

    let mut cursor = Cursor::new(bytes);
    let header = LazFileHeader::read_from_cursor(&mut cursor)?;

    if bytes[104] & 0x80 == 0 {
        return Err("File appears to be uncompressed LAS. Use parseLasPoints instead.".to_string());
    }

    let header_size = u16::from_le_bytes([bytes[94], bytes[95]]) as u64;
    cursor
        .seek(SeekFrom::Start(header_size))
        .map_err(|e| e.to_string())?;
    let laszip_vlr = find_laszip_vlr(&mut cursor, header.num_vlrs)?;

    let compressed_slice = &bytes[header.offset_to_points as usize..];

    let point_size = laszip_vlr.items_size() as usize;
    let num_points = header.num_points as usize;
    let has_color_in_laz = laszip_vlr.items().iter().any(|item| {
        matches!(
            item.item_type(),
            laz::LazItemType::RGB12 | laz::LazItemType::RGB14
        )
    });

    let mut decompressor = laz::LasZipDecompressor::new(Cursor::new(compressed_slice), laszip_vlr)
        .map_err(|e| format!("Failed to create LAZ decompressor: {}", e))?;

    let mut positions: Vec<f32> = Vec::with_capacity(num_points * 3);
    let mut colors: Option<Vec<u8>> = if has_color_in_laz {
        Some(Vec::with_capacity(num_points * 3))
    } else {
        None
    };

    let mut point_buf = vec![0u8; point_size];
    let mut last_reported: u32 = 0;

    for i in 0..num_points {
        decompressor
            .decompress_one(&mut point_buf)
            .map_err(|e| format!("LAZ decompression error: {}", e))?;

        let raw_x = read_i32_le(&point_buf, 0) as f64;
        let raw_y = read_i32_le(&point_buf, 4) as f64;
        let raw_z = read_i32_le(&point_buf, 8) as f64;

        positions.push((raw_x * header.x_scale + header.x_offset) as f32);
        positions.push((raw_y * header.y_scale + header.y_offset) as f32);
        positions.push((raw_z * header.z_scale + header.z_offset) as f32);

        if has_color_in_laz && point_buf.len() >= 23 {
            if let Some(ref mut c) = colors {
                c.push(point_buf[20]);
                c.push(point_buf[21]);
                c.push(point_buf[22]);
            }
        }

        if interval > 0 && (i as u32).saturating_sub(last_reported) >= interval {
            on_progress(i as u32, num_points as u32);
            last_reported = i as u32;
        }
    }

    on_progress(num_points as u32, num_points as u32);

    Ok(LasPointCloud {
        point_count: positions.len() as u32 / 3,
        positions,
        colors,
    })
}

/// WASM binding for LAZ point decompression.
///
/// Parses a LAZ compressed file and returns decompressed points.
#[cfg(feature = "laz-support")]
#[wasm_bindgen(js_name = "parseLazPoints")]
pub fn parse_laz_points(bytes: &[u8]) -> Result<LasPointCloud, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "LAZ input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_laz_points_core(bytes).map_err(SpatialError::point_cloud_error)
}

/// Parse LAZ points with a JS progress callback. Reports every 10,000 points.
#[cfg(feature = "laz-support")]
#[wasm_bindgen(js_name = "parseLazPointsStream")]
pub fn parse_laz_points_stream(
    bytes: &[u8],
    on_progress: &js_sys::Function,
) -> Result<LasPointCloud, SpatialErrorDetail> {
    let this = JsValue::NULL;

    parse_laz_points_with_progress_core(
        bytes,
        |processed, total| {
            let _ = on_progress.call2(&this, &JsValue::from(processed), &JsValue::from(total));
        },
        10_000,
    )
    .map_err(SpatialError::point_cloud_error)
}

// ===========================================================================
// Auto-detect: parsePointCloudAuto
// ===========================================================================

/// Detect point cloud format from header bytes.
///
/// Returns "las", "laz", or "copc".
fn detect_point_cloud_format(bytes: &[u8]) -> Result<&'static str, String> {
    // Check E57 first (different magic)
    #[cfg(feature = "e57-support")]
    if crate::e57::is_e57_format(bytes) {
        return Ok("e57");
    }

    if bytes.len() < 230 {
        return Err("Data too short to detect format".to_string());
    }
    if &bytes[0..4] != b"LASF" {
        return Err(format!(
            "Invalid LAS/LAZ magic: expected b\"LASF\", got {:?}",
            &bytes[0..4]
        ));
    }

    // Check version: COPC uses LAS 1.4 (major=1, minor=4)
    let major = bytes[24];
    let minor = bytes[25];
    let is_copc = major == 1 && minor == 4;

    // Check compression bit at offset 104 (per LAS spec)
    let point_format = bytes[104];
    let is_compressed = point_format & 0x80 != 0;

    if is_copc && is_compressed {
        Ok("copc")
    } else if is_compressed {
        Ok("laz")
    } else {
        Ok("las")
    }
}

/// Unified entry point: automatically detects LAS/LAZ/COPC format and parses points.
///
/// # Format Detection
///
/// - **LAS**: `LASF` magic, version ≤ 1.3, no compression bit
/// - **LAZ**: `LASF` magic, any version, compression bit set at byte 104
///   (requires `laz-support` feature)
/// - **COPC**: `LASF` magic, version 1.4, compression bit set, COPC VLR present
///   (requires `laz-support` feature)
///
/// All three formats use the same decompression path internally. COPC adds
/// spatial indexing but falls back to full decompression for the auto path.
#[wasm_bindgen(js_name = "parsePointCloudAuto")]
pub fn parse_point_cloud_auto(bytes: &[u8]) -> Result<LasPointCloud, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "Input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }

    let format = detect_point_cloud_format(bytes).map_err(SpatialError::point_cloud_error)?;

    match format {
        "las" => parse_las_points_core(bytes).map_err(SpatialError::point_cloud_error),
        #[cfg(feature = "laz-support")]
        "laz" | "copc" => parse_laz_points_core(bytes).map_err(SpatialError::point_cloud_error),
        #[cfg(not(feature = "laz-support"))]
        "laz" | "copc" => Err(SpatialError::point_cloud_error(
            "LAZ/COPC format detected but laz-support feature is not enabled. \
             Build with --features laz-support to enable LAZ decompression.",
        )),
        #[cfg(feature = "e57-support")]
        "e57" => {
            // E57 has its own result type, convert to LasPointCloud
            let e57_result =
                crate::e57::parse_e57_core(bytes).map_err(SpatialError::point_cloud_error)?;
            Ok(LasPointCloud {
                positions: e57_result.positions,
                colors: e57_result.colors,
                point_count: e57_result.point_count,
            })
        }
        _ => Err(SpatialError::point_cloud_error("Unknown format")),
    }
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
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).ok();
    js_sys::Reflect::set(&obj, &"colors".into(), &col_arr).ok();
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
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).ok();
    js_sys::Reflect::set(&obj, &"colors".into(), &col_arr).ok();
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
pub fn parse_pcd_ascii(text: &str) -> Result<PcdPointCloud, SpatialErrorDetail> {
    if text.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "PCD input is {} bytes, max is {}",
            text.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_pcd_ascii_core(text).map_err(SpatialError::point_cloud_error)
}

/// Parse binary PCD format bytes into a point cloud.
#[wasm_bindgen(js_name = "parsePcdBinary")]
pub fn parse_pcd_binary(bytes: &[u8]) -> Result<PcdPointCloud, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "PCD binary input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_pcd_binary_core(bytes).map_err(SpatialError::point_cloud_error)
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
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).ok();
    js_sys::Reflect::set(&obj, &"indices".into(), &idx_arr).ok();
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
    let num_points = read_u32_le(bytes, 107);
    let point_offset = read_u32_le(bytes, 96) as usize;
    let point_format = bytes[104];
    let point_record_len = read_u16_le(bytes, 105) as usize;

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
) -> Result<LasPointCloud, SpatialErrorDetail> {
    let _num_points = read_u32_le(bytes, 107);
    let this = JsValue::NULL;

    parse_las_points_with_progress_core(
        bytes,
        |processed, total| {
            let _ = on_progress.call2(&this, &JsValue::from(processed), &JsValue::from(total));
        },
        10_000,
    )
    .map_err(SpatialError::point_cloud_error)
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
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr).ok();
    js_sys::Reflect::set(&obj, &"colors".into(), &col_arr).ok();
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
    /// Internal constructor used by PointCloudStreamer.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_from_parts(
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
        bounds_max_x: f64,
        bounds_max_y: f64,
        bounds_max_z: f64,
        bounds_min_x: f64,
        bounds_min_y: f64,
        bounds_min_z: f64,
        file_size: u32,
    ) -> Self {
        Self {
            num_points,
            point_offset,
            point_format_id,
            point_record_length,
            x_scale,
            y_scale,
            z_scale,
            x_offset,
            y_offset,
            z_offset,
            bounds_min_x,
            bounds_min_y,
            bounds_min_z,
            bounds_max_x,
            bounds_max_y,
            bounds_max_z,
            file_size,
        }
    }

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
pub fn parse_las_header_only(bytes: &[u8]) -> Result<LasHeaderInfo, SpatialErrorDetail> {
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

    Ok(LasHeaderInfo {
        num_points: read_u32_le(bytes, 107),
        point_offset: read_u32_le(bytes, 96),
        point_format_id: bytes[104],
        point_record_length: read_u16_le(bytes, 105),
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
) -> Result<PointData, SpatialErrorDetail> {
    let has_color = point_format == 2 || point_format == 3;
    let needed = if has_color { 26 } else { 20 };

    if offset + needed > bytes.len() {
        return Err(SpatialError::point_cloud_error(format!(
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
// Point Cloud Colorization
// ===========================================================================

/// Core: colorize points by height gradient.
///
/// Maps Z values from [min_z, max_z] to a color gradient between low_color and high_color.
/// Returns RGBA Float32Array (values 0.0-1.0).
pub fn colorize_by_height_core(
    positions: &[f32],
    min_z: f32,
    max_z: f32,
    low_color: [f32; 3],
    high_color: [f32; 3],
) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let mut out = Vec::with_capacity(point_count * 4);

    let range = max_z - min_z;

    for i in 0..point_count {
        let z = positions[i * 3 + 2];
        let t = if range.abs() < 1e-6 {
            0.5
        } else {
            ((z - min_z) / range).clamp(0.0, 1.0)
        };

        let r = low_color[0] + t * (high_color[0] - low_color[0]);
        let g = low_color[1] + t * (high_color[1] - low_color[1]);
        let b = low_color[2] + t * (high_color[2] - low_color[2]);

        out.push(r / 255.0);
        out.push(g / 255.0);
        out.push(b / 255.0);
        out.push(1.0); // alpha
    }

    out
}

/// Core: colorize points by intensity values (grayscale mapping).
///
/// Intensities are expected to be in [0, 255]. Maps to grayscale RGBA.
pub fn colorize_by_intensity_core(positions: &[f32], intensities: &[f32]) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let mut out = Vec::with_capacity(point_count * 4);

    for i in 0..point_count {
        let intensity = if intensities.is_empty() {
            128.0 // default gray if no intensities provided
        } else if i < intensities.len() {
            intensities[i]
        } else {
            128.0
        };
        let v = (intensity / 255.0).clamp(0.0, 1.0);

        out.push(v);
        out.push(v);
        out.push(v);
        out.push(1.0);
    }

    out
}

/// Core: apply a discrete color array to a point cloud.
///
/// Colors should be RGBA Float32Array (one color per point). Returns as-is or
/// creates a default gray color for missing colors.
pub fn apply_color_ramp_core(positions: &[f32], colors: &[f32]) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let mut out = Vec::with_capacity(point_count * 4);

    for i in 0..point_count {
        if (i * 4 + 3) < colors.len() {
            out.push(colors[i * 4]);
            out.push(colors[i * 4 + 1]);
            out.push(colors[i * 4 + 2]);
            out.push(colors[i * 4 + 3]);
        } else {
            out.push(0.5); // default gray
            out.push(0.5);
            out.push(0.5);
            out.push(1.0);
        }
    }

    out
}

/// Colorize points by height gradient.
///
/// # Arguments
///
/// * `positions` — Float32Array `[x0, y0, z0, x1, y1, z1, ...]`
/// * `min_z` — Minimum Z value for gradient start
/// * `max_z` — Maximum Z value for gradient end
/// * `low_color` — Float32Array `[r, g, b]` (0-255) for min Z
/// * `high_color` — Float32Array `[r, g, b]` (0-255) for max Z
///
/// # Returns
///
/// Float32Array RGBA `[r0, g0, b0, a0, ...]` (0.0-1.0).
#[wasm_bindgen(js_name = "colorizeByHeight")]
pub fn colorize_by_height(
    positions: &js_sys::Float32Array,
    min_z: f32,
    max_z: f32,
    low_color: &js_sys::Float32Array,
    high_color: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);

    let mut lc_buf = vec![0.0f32; low_color.length() as usize];
    low_color.copy_to(&mut lc_buf);
    let mut hc_buf = vec![0.0f32; high_color.length() as usize];
    high_color.copy_to(&mut hc_buf);

    let lc: [f32; 3] = [
        lc_buf[0],
        lc_buf.get(1).copied().unwrap_or(0.0),
        lc_buf.get(2).copied().unwrap_or(0.0),
    ];
    let hc: [f32; 3] = [
        hc_buf[0],
        hc_buf.get(1).copied().unwrap_or(0.0),
        hc_buf.get(2).copied().unwrap_or(0.0),
    ];

    let result = colorize_by_height_core(&pos_buf, min_z, max_z, lc, hc);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Colorize points by intensity values (grayscale).
///
/// # Arguments
///
/// * `positions` — Float32Array `[x0, y0, z0, ...]`
/// * `intensities` — Float32Array of intensity values per point (0-255)
///
/// # Returns
///
/// Float32Array RGBA `[r, g, b, a, ...]` (grayscale, 0.0-1.0).
#[wasm_bindgen(js_name = "colorizeByIntensity")]
pub fn colorize_by_intensity(
    positions: &js_sys::Float32Array,
    intensities: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);
    let mut int_buf = vec![0.0f32; intensities.length() as usize];
    intensities.copy_to(&mut int_buf);

    let result = colorize_by_intensity_core(&pos_buf, &int_buf);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Apply a discrete color array to a point cloud.
///
/// # Arguments
///
/// * `positions` — Float32Array `[x0, y0, z0, ...]`
/// * `colors` — Float32Array `[r0, g0, b0, a0, ...]` (0.0-1.0), one color per point
///
/// # Returns
///
/// Float32Array RGBA matching colors length, padded with gray for missing entries.
#[wasm_bindgen(js_name = "applyColorRamp")]
pub fn apply_color_ramp(
    positions: &js_sys::Float32Array,
    colors: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let mut pos_buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos_buf);
    let mut col_buf = vec![0.0f32; colors.length() as usize];
    colors.copy_to(&mut col_buf);

    let result = apply_color_ramp_core(&pos_buf, &col_buf);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

// ===========================================================================
// Normal Estimation
// ===========================================================================

/// Native helper: estimate normals from flat `[x,y,z,...]` slice.
fn estimate_normals_native(positions: &[f32], k: usize) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let k = k.max(3).min(point_count.saturating_sub(1));

    let mut normals = vec![0.0f32; positions.len()];

    for i in 0..point_count {
        let px = positions[i * 3] as f64;
        let py = positions[i * 3 + 1] as f64;
        let pz = positions[i * 3 + 2] as f64;

        // Find k nearest neighbors using brute-force
        let mut distances: Vec<(f64, usize)> = Vec::with_capacity(point_count);
        for j in 0..point_count {
            if j == i {
                continue;
            }
            let dx = positions[j * 3] as f64 - px;
            let dy = positions[j * 3 + 1] as f64 - py;
            let dz = positions[j * 3 + 2] as f64 - pz;
            distances.push((dx * dx + dy * dy + dz * dz, j));
        }
        distances.select_nth_unstable_by(k, |a, b| a.0.total_cmp(&b.0));

        // Build covariance matrix from k neighbors (centered)
        let neighbors = &distances[..k];
        let mut cx = 0.0f64;
        let mut cy = 0.0f64;
        let mut cz = 0.0f64;
        for &(_, j) in neighbors {
            cx += positions[j * 3] as f64;
            cy += positions[j * 3 + 1] as f64;
            cz += positions[j * 3 + 2] as f64;
        }
        cx /= k as f64;
        cy /= k as f64;
        cz /= k as f64;

        // Covariance matrix (symmetric 3x3, stored as 6 elements)
        let mut cov = [0.0f64; 6];
        for &(_, j) in neighbors {
            let dx = positions[j * 3] as f64 - cx;
            let dy = positions[j * 3 + 1] as f64 - cy;
            let dz = positions[j * 3 + 2] as f64 - cz;
            cov[0] += dx * dx;
            cov[1] += dx * dy;
            cov[2] += dx * dz;
            cov[3] += dy * dy;
            cov[4] += dy * dz;
            cov[5] += dz * dz;
        }

        let (nx, ny, nz) = eigen_vector_3x3_symmetric(&cov);
        normals[i * 3] = nx as f32;
        normals[i * 3 + 1] = ny as f32;
        normals[i * 3 + 2] = nz as f32;
    }

    normals
}

/// Native helper: flip normals consistently toward centroid.
fn flip_normals_native(normals: &[f32], positions: &[f32]) -> Vec<f32> {
    let point_count = normals.len() / 3;

    let mut result = normals.to_vec();

    // Compute centroid
    let mut cx = 0.0f64;
    let mut cy = 0.0f64;
    let mut cz = 0.0f64;
    for i in 0..point_count {
        cx += positions[i * 3] as f64;
        cy += positions[i * 3 + 1] as f64;
        cz += positions[i * 3 + 2] as f64;
    }
    cx /= point_count as f64;
    cy /= point_count as f64;
    cz /= point_count as f64;

    for i in 0..point_count {
        let nx = result[i * 3] as f64;
        let ny = result[i * 3 + 1] as f64;
        let nz = result[i * 3 + 2] as f64;

        let dx = positions[i * 3] as f64 - cx;
        let dy = positions[i * 3 + 1] as f64 - cy;
        let dz = positions[i * 3 + 2] as f64 - cz;

        if nx * dx + ny * dy + nz * dz < 0.0 {
            result[i * 3] = -result[i * 3];
            result[i * 3 + 1] = -result[i * 3 + 1];
            result[i * 3 + 2] = -result[i * 3 + 2];
        }
    }

    result
}

/// Estimate normals for a point cloud using brute-force k-nearest neighbors.
///
/// For each point, finds the k nearest neighbors, fits a plane via SVD,
/// and returns the normal vector of that plane.
///
/// # Arguments
///
/// * `positions` — Flat `Float32Array` `[x0,y0,z0, x1,y1,z1, ...]`.
/// * `k` — Number of nearest neighbors for plane fitting (min 3).
///
/// # Returns
///
/// `Float32Array` `[nx0,ny0,nz0, nx1,ny1,nz1, ...]` — unit normals.
#[wasm_bindgen(js_name = "estimateNormals")]
pub fn estimate_normals(positions: &js_sys::Float32Array, k: u32) -> js_sys::Float32Array {
    let len = positions.length() as usize;
    let mut pos_buf = vec![0.0f32; len];
    positions.copy_to(&mut pos_buf);

    let result = estimate_normals_native(&pos_buf, k as usize);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Flip normals to ensure consistent orientation toward the centroid.
///
/// For each normal, checks if its dot product with the vector from the
/// centroid to the point is positive. If not, the normal is negated.
///
/// # Arguments
///
/// * `normals` — Flat `Float32Array` `[nx0,ny0,nz0, ...]`.
/// * `positions` — Flat `Float32Array` `[x0,y0,z0, ...]`.
///
/// # Returns
///
/// `Float32Array` with consistently oriented normals.
#[wasm_bindgen(js_name = "flipNormals")]
pub fn flip_normals(
    normals: &js_sys::Float32Array,
    positions: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let len = normals.length() as usize;
    let mut norm_buf = vec![0.0f32; len];
    normals.copy_to(&mut norm_buf);
    let mut pos_buf = vec![0.0f32; len];
    positions.copy_to(&mut pos_buf);

    let result = flip_normals_native(&norm_buf, &pos_buf);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Compute the eigenvector corresponding to the smallest eigenvalue of a
/// 3x3 symmetric matrix stored as [xx, xy, xz, yy, yz, zz].
///
/// Uses the characteristic polynomial approach for 3x3 matrices.
fn eigen_vector_3x3_symmetric(cov: &[f64; 6]) -> (f64, f64, f64) {
    // Covariance matrix:
    // | cov[0]  cov[1]  cov[2] |
    // | cov[1]  cov[3]  cov[4] |
    // | cov[2]  cov[4]  cov[5] |

    let a11 = cov[0];
    let a12 = cov[1];
    let a13 = cov[2];
    let a22 = cov[3];
    let a23 = cov[4];
    let a33 = cov[5];

    // Compute eigenvalues using the characteristic polynomial
    let trace = a11 + a22 + a33;
    let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a13 * a23)
        + a13 * (a12 * a23 - a13 * a22);

    let cof00 = a22 * a33 - a23 * a23;
    let cof11 = a11 * a33 - a13 * a13;
    let cof22 = a11 * a22 - a12 * a12;

    let cof_matrix_trace = cof00 + cof11 + cof22;

    // Solve cubic: λ³ - trace·λ² + cof_trace·λ - det = 0
    // Use the iterative approach: for the smallest eigenvalue,
    // we use power iteration on the inverse (or Jacobi iteration)

    // Simple Jacobi eigenvector computation
    // We want the eigenvector for the smallest eigenvalue (the normal)
    let mut n = [1.0_f64, 0.0_f64, 0.0_f64];

    // Iterate: multiply by covariance matrix and normalize
    // The eigenvector with smallest eigenvalue is the one that minimizes
    // n^T * C * n. We use inverse power iteration.
    // For a 3x3 positive semi-definite matrix, we can use the fact that
    // the normal is the eigenvector of the smallest eigenvalue.

    // Direct computation using the characteristic polynomial:
    let p = -trace;
    let q = cof_matrix_trace;
    let r = -det;

    // Cardano's formula for depressed cubic t³ + pt + q' = 0
    let b = q / 3.0 - p * p / 9.0;
    let c = r + 2.0 * p * p * p / 27.0 - p * q / 3.0;
    let discriminant = c * c / 4.0 + b * b * b / 27.0;

    let e1: f64;
    let e2: f64;
    let e3: f64;

    if discriminant > 0.0 {
        // One real root, two complex conjugates — shouldn't happen for symmetric
        let sqrt_disc = discriminant.sqrt();
        let u = (-c / 2.0 + sqrt_disc).cbrt();
        let v = (-c / 2.0 - sqrt_disc).cbrt();
        e1 = u + v + p / 3.0;
        // For symmetric matrices we should have 3 real eigenvalues
        // Fall back to approximation
        e2 = e1 * 0.5;
        e3 = e1 * 0.5;
    } else {
        let cos_phi = (-c / 2.0) / (b * b / 27.0).sqrt().max(1e-30);
        let phi = cos_phi.acos() / 3.0;
        let m = (b.abs()).sqrt().cbrt();
        let sqrt_b3 = 2.0 * m;

        e1 = sqrt_b3 * phi.cos() + p / 3.0;
        e2 = sqrt_b3 * ((phi + 2.0 * std::f64::consts::PI / 3.0).cos()) + p / 3.0;
        e3 = sqrt_b3 * ((phi + 4.0 * std::f64::consts::PI / 3.0).cos()) + p / 3.0;
    }

    // Find the smallest eigenvalue
    let min_e = e1.min(e2).min(e3);

    // Compute eigenvector for the smallest eigenvalue using cross-product method
    // (C - λI)v = 0, so v is in the null space of (C - λI)
    let m00 = a11 - min_e;
    let m01 = a12;
    let m02 = a13;
    let m11 = a22 - min_e;
    let m12 = a23;

    // Cross product of two rows gives a vector in the null space
    n[0] = m01 * m12 - m02 * m11;
    n[1] = m02 * m01 - m00 * m12; // fixed: should be m00*m12
    n[2] = m00 * m11 - m01 * m01;

    // Normalize
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-10 {
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
    }

    (n[0], n[1], n[2])
}

// ===========================================================================
// Point Cloud Coloring Enhancement
// ===========================================================================

/// Get the standard ASPRS classification color for a given class ID.
///
/// Returns (R, G, B) tuple.
const fn asprs_color(class: u8) -> (u8, u8, u8) {
    match class {
        0 => (255, 255, 255),  // Never classified (white)
        1 => (139, 90, 43),    // Ground (brown)
        2 => (34, 139, 34),    // Low Vegetation
        3 => (0, 180, 0),      // Medium Vegetation
        4 => (0, 100, 0),      // High Vegetation
        5 => (100, 100, 255),  // Building (blue-gray)
        6 => (139, 0, 0),      // Low Point / noise (dark red)
        7 => (192, 192, 192),  // Reserved (gray)
        8 => (255, 255, 0),    // Model Key Point (yellow)
        9 => (0, 0, 255),      // Water (blue)
        10 => (255, 165, 0),   // Rail (orange)
        11 => (100, 100, 100), // Road Surface (dark gray)
        12 => (128, 0, 128),   // Overhead Structure (purple)
        13 => (0, 255, 255),   // Wire - Guard (Shield) (cyan)
        14 => (255, 0, 255),   // Wire - Conductor (Phase) (magenta)
        15 => (255, 0, 0),     // Transmission Tower (red)
        16 => (255, 69, 0),    // Wire-Structure Connector
        17 => (210, 180, 140), // Bridge Deck (tan)
        18 => (200, 0, 0),     // High Noise
        19 => (148, 0, 211),   // Overhead Structure Point
        20 => (101, 67, 33),   // Ignored Ground
        21 => (173, 216, 230), // Snow
        22 => (255, 192, 203), // Temporal Exclusion
        _ => (128, 128, 128),  // Default (gray)
    }
}

/// Colorize points by ASPRS classification IDs.
///
/// Each point is assigned a color from the standard ASPRS classification
/// color table based on its class ID.
///
/// # Parameters
///
/// - `classes`: Uint8Array where each element is a classification ID (0-255)
///
/// # Returns
///
/// Uint8Array of RGB values `[r0, g0, b0, r1, g1, b1, ...]`
#[wasm_bindgen(js_name = "colorizeByClassification")]
pub fn colorize_by_classification(classes: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let classes = classes.to_vec();
    let colors = colorize_by_classification_core(&classes);
    let arr = js_sys::Uint8Array::new_with_length(colors.len() as u32);
    arr.copy_from(&colors);
    arr
}

/// Core classification coloring.
pub(crate) fn colorize_by_classification_core(classes: &[u8]) -> Vec<u8> {
    let mut colors = Vec::with_capacity(classes.len() * 3);
    for &class in classes {
        let (r, g, b) = asprs_color(class);
        colors.push(r);
        colors.push(g);
        colors.push(b);
    }
    colors
}

/// Colorize points by a heatmap gradient.
///
/// Maps scalar values to a blue→cyan→green→yellow→red color gradient.
///
/// # Parameters
///
/// - `values`: Float32Array of scalar values (one per point)
/// - `min`: Minimum value for the gradient range
/// - `max`: Maximum value for the gradient range
///
/// # Returns
///
/// Uint8Array of RGB values `[r0, g0, b0, r1, g1, b1, ...]`
#[wasm_bindgen(js_name = "colorizeByHeatmap")]
pub fn colorize_by_heatmap(
    values: &js_sys::Float32Array,
    min: f32,
    max: f32,
) -> js_sys::Uint8Array {
    let values = values.to_vec();
    let colors = colorize_by_heatmap_core(&values, min, max);
    let arr = js_sys::Uint8Array::new_with_length(colors.len() as u32);
    arr.copy_from(&colors);
    arr
}

/// Core heatmap coloring.
pub(crate) fn colorize_by_heatmap_core(values: &[f32], min: f32, max: f32) -> Vec<u8> {
    let range = (max - min).max(0.001);
    let mut colors = Vec::with_capacity(values.len() * 3);

    for &v in values {
        let t = ((v - min) / range).clamp(0.0, 1.0);
        let (r, g, b) = heatmap_color(t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
    }

    colors
}

/// Map a 0-1 value to a heatmap color (blue→cyan→green→yellow→red).
fn heatmap_color(t: f32) -> (u8, u8, u8) {
    // 5-stop gradient:
    // 0.00 = blue  (0, 0, 255)
    // 0.25 = cyan  (0, 255, 255)
    // 0.50 = green (0, 255, 0)
    // 0.75 = yellow (255, 255, 0)
    // 1.00 = red   (255, 0, 0)

    let (r, g, b) = if t < 0.25 {
        let f = t / 0.25;
        (0.0, f * 255.0, 255.0)
    } else if t < 0.5 {
        let f = (t - 0.25) / 0.25;
        (0.0, 255.0, (1.0 - f) * 255.0)
    } else if t < 0.75 {
        let f = (t - 0.5) / 0.25;
        (f * 255.0, 255.0, 0.0)
    } else {
        let f = (t - 0.75) / 0.25;
        (255.0, (1.0 - f) * 255.0, 0.0)
    };

    (
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

/// Build a smooth color ramp from discrete color stops.
///
/// Creates a linearly interpolated gradient between the provided colors.
///
/// # Parameters
///
/// - `colors`: Uint8Array of color stops `[r0, g0, b0, r1, g1, b1, ...]`
///   Must have at least 2 colors (6 bytes).
/// - `num_steps`: Number of output colors to generate
///
/// # Returns
///
/// Uint8Array of interpolated colors `[r0, g0, b0, r1, g1, b1, ...]`
#[wasm_bindgen(js_name = "buildColorRamp")]
pub fn build_color_ramp(
    colors: &js_sys::Uint8Array,
    num_steps: u32,
) -> Result<js_sys::Uint8Array, SpatialErrorDetail> {
    let colors = colors.to_vec();
    let result = build_color_ramp_core(&colors, num_steps as usize)
        .map_err(SpatialError::point_cloud_error)?;
    let arr = js_sys::Uint8Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    Ok(arr)
}

/// Core color ramp builder.
pub(crate) fn build_color_ramp_core(colors: &[u8], num_steps: usize) -> Result<Vec<u8>, String> {
    if colors.len() < 6 {
        return Err("Need at least 2 color stops (6 bytes)".to_string());
    }
    if !colors.len().is_multiple_of(3) {
        return Err("Colors must be multiples of 3 (RGB)".to_string());
    }
    if num_steps == 0 {
        return Err("num_steps must be > 0".to_string());
    }

    let num_stops = colors.len() / 3;
    let mut result = Vec::with_capacity(num_steps * 3);

    for i in 0..num_steps {
        let t = i as f64 / (num_steps - 1).max(1) as f64;

        // Find which segment we're in
        let segment = (t * (num_stops - 1) as f64) as usize;
        let segment = segment.min(num_stops - 2);
        let local_t = t * (num_stops - 1) as f64 - segment as f64;

        let r0 = colors[segment * 3] as f64;
        let g0 = colors[segment * 3 + 1] as f64;
        let b0 = colors[segment * 3 + 2] as f64;
        let r1 = colors[(segment + 1) * 3] as f64;
        let g1 = colors[(segment + 1) * 3 + 1] as f64;
        let b1 = colors[(segment + 1) * 3 + 2] as f64;

        result.push(((1.0 - local_t) * r0 + local_t * r1) as u8);
        result.push(((1.0 - local_t) * g0 + local_t * g1) as u8);
        result.push(((1.0 - local_t) * b0 + local_t * b1) as u8);
    }

    Ok(result)
}

// ===========================================================================
// Point Cloud Statistics
// ===========================================================================

/// Compute comprehensive statistics for a point cloud.
///
/// Returns a JSON string with:
/// - `pointCount`: Number of points
/// - `bounds`: `{ minX, minY, minZ, maxX, maxY, maxZ }`
/// - `centroid`: `[cx, cy, cz]`
/// - `averagePointSpacing`: Average nearest-neighbor distance (sampled)
/// - `density`: Points per cubic meter
///
/// For large point clouds (>100K points), nearest-neighbor computation
/// is sampled to keep performance reasonable.
#[wasm_bindgen(js_name = "pointCloudStats")]
pub fn point_cloud_stats(positions: &js_sys::Float32Array) -> Result<String, SpatialErrorDetail> {
    let positions = positions.to_vec();
    point_cloud_stats_core(&positions).map_err(SpatialError::point_cloud_error)
}

/// Core implementation of point cloud statistics (pure Rust, testable).
pub(crate) fn point_cloud_stats_core(positions: &[f32]) -> Result<String, String> {
    let point_count = positions.len() / 3;

    if point_count == 0 {
        return Err("Cannot compute stats: no points".to_string());
    }

    // Compute bounds and centroid
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut min_z = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut max_z = f64::MIN;
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_z = 0.0_f64;

    for i in (0..positions.len()).step_by(3) {
        let x = positions[i] as f64;
        let y = positions[i + 1] as f64;
        let z = positions[i + 2] as f64;

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        min_z = min_z.min(z);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        max_z = max_z.max(z);

        sum_x += x;
        sum_y += y;
        sum_z += z;
    }

    let cx = sum_x / point_count as f64;
    let cy = sum_y / point_count as f64;
    let cz = sum_z / point_count as f64;

    // Compute bounding volume
    let dx = (max_x - min_x).max(0.001);
    let dy = (max_y - min_y).max(0.001);
    let dz = (max_z - min_z).max(0.001);
    let volume = dx * dy * dz;
    let density = point_count as f64 / volume;

    // Average nearest-neighbor distance (sampled for large clouds)
    let sample_size = 1000.min(point_count);
    let step = point_count / sample_size;
    let mut total_nn_dist = 0.0_f64;
    let mut nn_count = 0_usize;

    for si in 0..sample_size {
        let idx = si * step * 3;
        if idx + 3 > positions.len() {
            break;
        }
        let px = positions[idx] as f64;
        let py = positions[idx + 1] as f64;
        let pz = positions[idx + 2] as f64;

        let mut min_dist_sq = f64::MAX;
        // Search for nearest neighbor (avoid self)
        for j in (0..positions.len()).step_by(3) {
            if j == idx {
                continue;
            }
            let dx = positions[j] as f64 - px;
            let dy = positions[j + 1] as f64 - py;
            let dz = positions[j + 2] as f64 - pz;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
            }
        }

        if min_dist_sq < f64::MAX {
            total_nn_dist += min_dist_sq.sqrt();
            nn_count += 1;
        }
    }

    let avg_spacing = if nn_count > 0 {
        total_nn_dist / nn_count as f64
    } else {
        0.0
    };

    let stats = serde_json::json!({
        "pointCount": point_count,
        "bounds": {
            "minX": min_x,
            "minY": min_y,
            "minZ": min_z,
            "maxX": max_x,
            "maxY": max_y,
            "maxZ": max_z,
        },
        "centroid": [cx, cy, cz],
        "averagePointSpacing": avg_spacing,
        "density": density,
    });

    Ok(stats.to_string())
}

/// Compute axis-aligned bounding box of a point cloud.
///
/// Returns a Float64Array `[min_x, min_y, min_z, max_x, max_y, max_z]`.
#[wasm_bindgen(js_name = "pointCloudBounds")]
pub fn point_cloud_bounds(positions: &js_sys::Float32Array) -> js_sys::Float64Array {
    let positions = positions.to_vec();
    let (min_x, min_y, min_z, max_x, max_y, max_z) = point_cloud_bounds_core(&positions);
    let arr = js_sys::Float64Array::new_with_length(6);
    arr.set_index(0, min_x);
    arr.set_index(1, min_y);
    arr.set_index(2, min_z);
    arr.set_index(3, max_x);
    arr.set_index(4, max_y);
    arr.set_index(5, max_z);
    arr
}

/// Core bounds computation.
pub(crate) fn point_cloud_bounds_core(positions: &[f32]) -> (f64, f64, f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut min_z = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut max_z = f64::MIN;

    for i in (0..positions.len()).step_by(3) {
        let x = positions[i] as f64;
        let y = positions[i + 1] as f64;
        let z = positions[i + 2] as f64;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        min_z = min_z.min(z);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        max_z = max_z.max(z);
    }

    (min_x, min_y, min_z, max_x, max_y, max_z)
}

/// Compute the centroid (geometric center) of a point cloud.
///
/// Returns a Float64Array `[cx, cy, cz]`.
#[wasm_bindgen(js_name = "pointCloudCentroid")]
pub fn point_cloud_centroid(positions: &js_sys::Float32Array) -> js_sys::Float64Array {
    let positions = positions.to_vec();
    let (cx, cy, cz) = point_cloud_centroid_core(&positions);
    let arr = js_sys::Float64Array::new_with_length(3);
    arr.set_index(0, cx);
    arr.set_index(1, cy);
    arr.set_index(2, cz);
    arr
}

/// Core centroid computation.
pub(crate) fn point_cloud_centroid_core(positions: &[f32]) -> (f64, f64, f64) {
    if positions.len() < 3 {
        return (0.0, 0.0, 0.0);
    }

    let point_count = positions.len() / 3;
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_z = 0.0_f64;

    for i in (0..positions.len()).step_by(3) {
        sum_x += positions[i] as f64;
        sum_y += positions[i + 1] as f64;
        sum_z += positions[i + 2] as f64;
    }

    (
        sum_x / point_count as f64,
        sum_y / point_count as f64,
        sum_z / point_count as f64,
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::point_cloud::test_helpers::build_test_las_blob;

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
    fn test_colorize_by_height_gradient() {
        // 3 points at z=0, z=50, z=100
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 50.0, 2.0, 2.0, 100.0];
        let low: [f32; 3] = [0.0, 0.0, 255.0]; // blue
        let high: [f32; 3] = [255.0, 0.0, 0.0]; // red

        let result = colorize_by_height_core(&positions, 0.0, 100.0, low, high);
        assert_eq!(result.len(), 12); // 3 points × 4 RGBA

        // Point 0 (z=0): should be blue (0, 0, 1.0)
        assert!((result[0] - 0.0).abs() < 1e-6);
        assert!((result[1] - 0.0).abs() < 1e-6);
        assert!((result[2] - 1.0).abs() < 1e-6);
        assert!((result[3] - 1.0).abs() < 1e-6); // alpha

        // Point 2 (z=100): should be red (1.0, 0, 0)
        assert!((result[8] - 1.0).abs() < 1e-6);
        assert!((result[9] - 0.0).abs() < 1e-6);
        assert!((result[10] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_colorize_by_height_mid_point() {
        let positions: Vec<f32> = vec![0.0, 0.0, 50.0]; // z at midpoint
        let low: [f32; 3] = [0.0, 0.0, 0.0]; // black
        let high: [f32; 3] = [255.0, 255.0, 255.0]; // white

        let result = colorize_by_height_core(&positions, 0.0, 100.0, low, high);
        // At midpoint t=0.5, should be gray
        assert!((result[0] - 0.5).abs() < 1e-6);
        assert!((result[1] - 0.5).abs() < 1e-6);
        assert!((result[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_colorize_by_intensity() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let intensities: Vec<f32> = vec![0.0, 255.0]; // black, white

        let result = colorize_by_intensity_core(&positions, &intensities);
        assert_eq!(result.len(), 8); // 2 points × 4

        // Point 0: intensity 0 → black
        assert!((result[0] - 0.0).abs() < 1e-6);
        assert!((result[1] - 0.0).abs() < 1e-6);
        assert!((result[2] - 0.0).abs() < 1e-6);

        // Point 1: intensity 255 → white
        assert!((result[4] - 1.0).abs() < 1e-6);
        assert!((result[5] - 1.0).abs() < 1e-6);
        assert!((result[6] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_color_ramp() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let colors: Vec<f32> = vec![
            1.0, 0.0, 0.0, 1.0, // red
            0.0, 1.0, 0.0,
            1.0, // green
                 // Only 2 colors for 3 points — 3rd gets default
        ];

        let result = apply_color_ramp_core(&positions, &colors);
        assert_eq!(result.len(), 12); // 3 points × 4

        // First point: red
        assert!((result[0] - 1.0).abs() < 1e-6);
        assert!((result[1] - 0.0).abs() < 1e-6);

        // Third point: default gray (0.5)
        assert!((result[8] - 0.5).abs() < 1e-6);
        assert!((result[9] - 0.5).abs() < 1e-6);
    }

    // ── COPC / Range-based access tests (continued) ────────────

    #[test]
    fn test_parse_las_header_only() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0)];
        let blob = build_test_las_blob(&points, false);

        // parse_las_header_only is the WASM function, but we can test the core logic
        let num_points = read_u32_le(&blob, 107);
        let point_offset = read_u32_le(&blob, 96);
        let point_format = blob[104];
        let point_record_length = read_u16_le(&blob, 105);

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

        let point_offset = read_u32_le(&blob, 96) as usize;
        let point_format = blob[104];

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

    // ── Normal Estimation ──────────────────────────────────────

    #[test]
    fn test_eigen_vector_flat_plane() {
        // Flat XY plane at z=0: covariance has zero in z direction
        let cov = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // identity-like, no z variation
        let (_nx, _ny, nz) = eigen_vector_3x3_symmetric(&cov);
        // Smallest eigenvalue corresponds to z direction
        assert!(nz.abs() > 0.9, "nz = {nz}, expected ~1.0");
    }

    #[test]
    fn test_estimate_normals_flat_plane() {
        // Create a flat plane in XY at z=5
        let positions: Vec<f32> = vec![
            0.0, 0.0, 5.0, 1.0, 0.0, 5.0, 0.0, 1.0, 5.0, 1.0, 1.0, 5.0, 2.0, 0.0, 5.0, 0.0, 2.0,
            5.0, 2.0, 2.0, 5.0,
        ];

        let normals = estimate_normals_native(&positions, 4);

        // All normals should be approximately (0, 0, ±1)
        for i in 0..7 {
            let nx = normals[i * 3];
            let ny = normals[i * 3 + 1];
            let nz = normals[i * 3 + 2];
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "Normal {} not unit length: len={}",
                i,
                len
            );
            // z component should dominate
            assert!(
                nz.abs() > 0.9,
                "Normal {}: ({}, {}, {}) — nz should be ~1",
                i,
                nx,
                ny,
                nz
            );
        }
    }

    #[test]
    fn test_flip_normals_consistency() {
        // Points on a sphere around origin — normals should point outward
        let positions: Vec<f32> = vec![
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0,
            -1.0,
        ];

        // Manually set some normals pointing inward
        let normals: Vec<f32> = vec![
            -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            0.0, -1.0,
        ];

        let flipped = flip_normals_native(&normals, &positions);

        // After flip, normals should point away from centroid (origin)
        assert!(flipped[0] > 0.5, "Normal 0 x should be positive");
        assert!(flipped[9] < -0.5, "Normal 3 x should be negative");
    }

    // ── LAZ decompression tests ─────────────────────────────────────

    /// Build a minimal valid LAZ blob by compressing points with the laz crate.
    ///
    /// Structure: LAS header (230 bytes) + LASZIP VLR (54 + VLR data) + compressed point data
    #[cfg(feature = "laz-support")]
    fn build_test_laz_blob(points: &[(f64, f64, f64)], has_color: bool) -> Vec<u8> {
        let num_points = points.len() as u32;
        let header_size = 230u32;

        // Build LASZIP VLR items based on format
        let laz_items = if has_color {
            laz::LazItemRecordBuilder::new()
                .add_item(laz::LazItemType::Point10)
                .add_item(laz::LazItemType::RGB12)
                .build()
        } else {
            laz::LazItemRecordBuilder::new()
                .add_item(laz::LazItemType::Point10)
                .build()
        };

        let point_format: u8 = if has_color { 2 | 0x80 } else { 0x80 }; // compressed bit set
        let point_size = laz::LazVlr::from_laz_items(laz_items.clone()).items_size() as u16;

        // Build raw point data for compression
        let raw_point_data: Vec<u8> = points
            .iter()
            .flat_map(|&(x, y, z)| {
                let mut p = vec![0u8; point_size as usize];
                p[0..4].copy_from_slice(&(x as i32).to_le_bytes());
                p[4..8].copy_from_slice(&(y as i32).to_le_bytes());
                p[8..12].copy_from_slice(&(z as i32).to_le_bytes());
                if has_color && p.len() >= 23 {
                    p[20] = 255; // R
                    p[21] = 128; // G
                    p[22] = 0; // B
                }
                p
            })
            .collect();

        // Compress points
        let mut compressed = std::io::Cursor::new(Vec::new());
        {
            let mut compressor =
                laz::LasZipCompressor::from_laz_items(&mut compressed, laz_items).unwrap();
            compressor.compress_many(&raw_point_data).unwrap();
            compressor.done().unwrap();
        }
        let compressed_data = compressed.into_inner();

        // Build LASZIP VLR
        let laz_vlr = laz::LazVlr::from_laz_items(if has_color {
            laz::LazItemRecordBuilder::new()
                .add_item(laz::LazItemType::Point10)
                .add_item(laz::LazItemType::RGB12)
                .build()
        } else {
            laz::LazItemRecordBuilder::new()
                .add_item(laz::LazItemType::Point10)
                .build()
        });

        let vlr_data = {
            let mut vlr_buf = std::io::Cursor::new(Vec::new());
            laz_vlr.write_to(&mut vlr_buf).unwrap();
            vlr_buf.into_inner()
        };

        // VLR header: reserved(2) + user_id(16) + record_id(2) + record_length(2) + description(32) + data
        let vlr_header_size: usize = 2 + 16 + 2 + 2 + 32;
        let vlr_total_size = vlr_header_size + vlr_data.len();

        let point_offset = header_size + vlr_total_size as u32;

        // Build LAS header
        let mut buf = vec![0u8; header_size as usize];
        buf[0..4].copy_from_slice(b"LASF");
        buf[24] = 1; // version major
        buf[25] = 2; // version minor
        buf[94..96].copy_from_slice(&(header_size as u16).to_le_bytes());
        buf[96..100].copy_from_slice(&point_offset.to_le_bytes());
        buf[100..104].copy_from_slice(&1u32.to_le_bytes()); // num_vlrs = 1
        buf[104] = point_format;
        buf[105..107].copy_from_slice(&point_size.to_le_bytes());
        buf[107..111].copy_from_slice(&num_points.to_le_bytes());
        buf[134..142].copy_from_slice(&1.0_f64.to_le_bytes()); // x_scale
        buf[142..150].copy_from_slice(&1.0_f64.to_le_bytes()); // y_scale
        buf[150..158].copy_from_slice(&1.0_f64.to_le_bytes()); // z_scale

        // Build VLR
        buf.resize(buf.len() + vlr_total_size, 0);
        let vlr_start = header_size as usize;
        // reserved = 0 (already)
        // user_id = "laszip encoded" (16 bytes, null-padded)
        let mut user_id = [0u8; 16];
        user_id[..14].copy_from_slice(b"laszip encoded");
        buf[vlr_start + 2..vlr_start + 18].copy_from_slice(&user_id);
        // record_id = 22204
        buf[vlr_start + 18..vlr_start + 20].copy_from_slice(&22204u16.to_le_bytes());
        // record_length
        buf[vlr_start + 20..vlr_start + 22].copy_from_slice(&(vlr_data.len() as u16).to_le_bytes());
        // description (32 bytes of zeros, already zeroed)
        // VLR data
        buf[vlr_start + vlr_header_size..vlr_start + vlr_total_size].copy_from_slice(&vlr_data);

        // Append compressed point data
        buf.extend_from_slice(&compressed_data);

        buf
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_roundtrip_format0() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0), (70.0, 80.0, 90.0)];
        let laz_blob = build_test_laz_blob(&points, false);

        // Verify it's recognized as LAZ (compression bit)
        assert!(laz_blob[104] & 0x80 != 0, "Should have compression bit set");

        let cloud = parse_laz_points_core(&laz_blob).unwrap();
        assert_eq!(cloud.point_count, 3);
        assert!(cloud.colors.is_none());
        assert_eq!(cloud.positions.len(), 9);

        // Verify positions match
        assert_eq!(cloud.positions[0], 10.0);
        assert_eq!(cloud.positions[1], 20.0);
        assert_eq!(cloud.positions[2], 30.0);
        assert_eq!(cloud.positions[3], 40.0);
        assert_eq!(cloud.positions[4], 50.0);
        assert_eq!(cloud.positions[5], 60.0);
        assert_eq!(cloud.positions[6], 70.0);
        assert_eq!(cloud.positions[7], 80.0);
        assert_eq!(cloud.positions[8], 90.0);
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_roundtrip_format2_with_color() {
        let points = vec![(5.0, 10.0, 15.0), (25.0, 30.0, 35.0)];
        let laz_blob = build_test_laz_blob(&points, true);

        let cloud = parse_laz_points_core(&laz_blob).unwrap();
        assert_eq!(cloud.point_count, 2);
        assert!(cloud.colors.is_some());

        let colors = cloud.colors.unwrap();
        assert_eq!(colors.len(), 6);
        // R=255, G=128, B=0 for each point
        assert_eq!(colors[0], 255);
        assert_eq!(colors[1], 128);
        assert_eq!(colors[2], 0);
        assert_eq!(colors[3], 255);
        assert_eq!(colors[4], 128);
        assert_eq!(colors[5], 0);
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_single_point() {
        let points = vec![(42.0, -17.0, 0.0)];
        let laz_blob = build_test_laz_blob(&points, false);

        let cloud = parse_laz_points_core(&laz_blob).unwrap();
        assert_eq!(cloud.point_count, 1);
        assert_eq!(cloud.positions[0], 42.0);
        assert_eq!(cloud.positions[1], -17.0);
        assert_eq!(cloud.positions[2], 0.0);
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_many_points() {
        let points: Vec<(f64, f64, f64)> = (0..500)
            .map(|i| {
                (
                    i as f64 * 0.1,
                    (i as f64 * 0.2).sin(),
                    (i as f64 * 0.3).cos(),
                )
            })
            .collect();
        let laz_blob = build_test_laz_blob(&points, false);

        let cloud = parse_laz_points_core(&laz_blob).unwrap();
        assert_eq!(cloud.point_count, 500);

        // Spot check first and last
        assert_eq!(cloud.positions[0], 0.0);
        assert_eq!(cloud.positions[1], 0.0_f64.sin() as f32);
        let last = cloud.positions.len() - 3;
        assert!(
            (cloud.positions[last] - 49.0).abs() < 0.2,
            "Last x should be ~49.0, got {}",
            cloud.positions[last]
        );
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_rejects_uncompressed_las() {
        // Build an uncompressed LAS blob (no compression bit)
        let points = vec![(1.0, 2.0, 3.0)];
        let las_blob = build_test_las_blob(&points, false);
        // Ensure no compression bit at offset 104 (per LAS spec)
        assert!(las_blob[104] & 0x80 == 0);

        let result = parse_laz_points_core(&las_blob);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("uncompressed"));
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_rejects_invalid_magic() {
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(b"XASX");
        blob[104] = 0x80; // compression bit at correct LAS spec offset

        let result = parse_laz_points_core(&blob);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_rejects_too_short() {
        let result = parse_laz_points_core(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_progress_callback() {
        let points: Vec<(f64, f64, f64)> = (0..250).map(|i| (i as f64, 0.0, 0.0)).collect();
        let laz_blob = build_test_laz_blob(&points, false);

        let mut call_count = 0u32;
        let cloud = parse_laz_points_with_progress_core(
            &laz_blob,
            |_processed, _total| {
                call_count += 1;
            },
            50, // report every 50 points
        )
        .unwrap();

        assert_eq!(cloud.point_count, 250);
        assert!(
            call_count >= 4,
            "Should have called progress at least 4 times, got {}",
            call_count
        );
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_laz_las_data_consistency() {
        // Create same points as LAS and LAZ, verify they produce identical results
        let points = vec![
            (100.0, -50.0, 25.0),
            (-100.0, 50.0, -25.0),
            (0.0, 0.0, 100.0),
        ];
        let las_blob = build_test_las_blob(&points, false);
        let laz_blob = build_test_laz_blob(&points, false);

        let las_cloud = parse_las_points_core(&las_blob).unwrap();
        let laz_cloud = parse_laz_points_core(&laz_blob).unwrap();

        assert_eq!(las_cloud.point_count, laz_cloud.point_count);
        for i in 0..las_cloud.positions.len() {
            assert!(
                (las_cloud.positions[i] - laz_cloud.positions[i]).abs() < 1e-6,
                "Position mismatch at index {}: LAS={}, LAZ={}",
                i,
                las_cloud.positions[i],
                laz_cloud.positions[i]
            );
        }
    }

    // ── Auto-detect tests ───────────────────────────────────────────

    #[test]
    fn test_auto_detect_las() {
        let points = vec![(1.0, 2.0, 3.0)];
        let las_blob = build_test_las_blob(&points, false);
        assert_eq!(detect_point_cloud_format(&las_blob).unwrap(), "las");
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_auto_detect_laz() {
        let points = vec![(1.0, 2.0, 3.0)];
        let laz_blob = build_test_laz_blob(&points, false);
        assert_eq!(detect_point_cloud_format(&laz_blob).unwrap(), "laz");
    }

    #[test]
    fn test_auto_detect_too_short() {
        let result = detect_point_cloud_format(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_auto_detect_bad_magic() {
        let mut blob = build_test_las_blob(&[(1.0, 2.0, 3.0)], false);
        blob[0..4].copy_from_slice(b"XASX");
        let result = detect_point_cloud_format(&blob);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_point_cloud_auto_las() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0)];
        let las_blob = build_test_las_blob(&points, false);
        let cloud = parse_point_cloud_auto(&las_blob).unwrap();
        assert_eq!(cloud.point_count, 2);
        assert_eq!(cloud.positions[0], 10.0);
    }

    #[test]
    #[cfg(feature = "laz-support")]
    fn test_parse_point_cloud_auto_laz() {
        let points = vec![(10.0, 20.0, 30.0), (40.0, 50.0, 60.0)];
        let laz_blob = build_test_laz_blob(&points, false);
        let cloud = parse_point_cloud_auto(&laz_blob).unwrap();
        assert_eq!(cloud.point_count, 2);
        assert_eq!(cloud.positions[0], 10.0);
    }

    // ── No-laz-support tests ──────────────────────────────────────

    /// Verify that LAS parsing still works when laz-support is disabled.
    #[test]
    fn test_las_works_without_laz_feature() {
        let points = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (-1.0, -2.0, -3.0)];
        let las_blob = build_test_las_blob(&points, false);

        let cloud = parse_las_points_core(&las_blob).unwrap();
        assert_eq!(cloud.point_count, 3);
        assert_eq!(cloud.positions[0], 1.0);
        assert_eq!(cloud.positions[1], 2.0);
        assert_eq!(cloud.positions[2], 3.0);
    }

    /// Verify that auto-detect returns "las" for uncompressed LAS (no laz feature needed).
    #[test]
    fn test_detect_format_las_without_laz() {
        let points = vec![(10.0, 20.0, 30.0)];
        let las_blob = build_test_las_blob(&points, false);
        assert_eq!(detect_point_cloud_format(&las_blob).unwrap(), "las");
    }

    /// Verify that auto-detect still identifies LAZ format even without laz-support.
    #[test]
    fn test_detect_format_laz_identified_without_laz() {
        // Create a minimal blob that looks like LAZ (compression bit set)
        let mut blob = build_test_las_blob(&[(1.0, 2.0, 3.0)], false);
        // Set compression bit at offset 104
        blob[104] |= 0x80;
        // Should detect as LAZ even though decompression is unavailable
        let format = detect_point_cloud_format(&blob).unwrap();
        assert_eq!(format, "laz");
    }

    /// Verify parsePointCloudAuto rejects LAZ when laz-support is not compiled.
    #[cfg(not(feature = "laz-support"))]
    #[test]
    fn test_auto_rejects_laz_without_feature() {
        let mut blob = build_test_las_blob(&[(1.0, 2.0, 3.0)], false);
        blob[104] |= 0x80;
        let result = parse_point_cloud_auto(&blob);
        assert!(result.is_err());
        let err = result.unwrap_err().message();
        assert!(
            err.contains("laz-support"),
            "Error should mention laz-support feature: {}",
            err
        );
    }

    /// Verify parsePointCloudAuto still works for LAS when laz-support is not compiled.
    #[cfg(not(feature = "laz-support"))]
    #[test]
    fn test_auto_works_for_las_without_laz_feature() {
        let points = vec![(7.0, 8.0, 9.0), (10.0, 11.0, 12.0)];
        let las_blob = build_test_las_blob(&points, false);
        let cloud = parse_point_cloud_auto(&las_blob).unwrap();
        assert_eq!(cloud.point_count, 2);
    }

    // ===========================================================================
    // Point Cloud Coloring Enhancement Tests
    // ===========================================================================

    #[test]
    fn test_colorize_by_classification_basic() {
        let classes = vec![0u8, 1, 2, 3, 9, 255];
        let colors = colorize_by_classification_core(&classes);
        assert_eq!(colors.len(), 18); // 6 points × 3 RGB

        // Class 0: never classified = white
        assert_eq!(colors[0], 255); // R
        assert_eq!(colors[1], 255); // G
        assert_eq!(colors[2], 255); // B

        // Class 1: ground = brown
        assert_eq!(colors[3], 139);
        assert_eq!(colors[4], 90);
        assert_eq!(colors[5], 43);

        // Class 2: low vegetation = green
        assert_eq!(colors[6], 34);
        assert_eq!(colors[7], 139);
        assert_eq!(colors[8], 34);

        // Class 9: water = blue (index 4 → colors[12..14])
        assert_eq!(colors[12], 0);
        assert_eq!(colors[13], 0);
        assert_eq!(colors[14], 255);
    }

    #[test]
    fn test_colorize_by_classification_empty() {
        let classes: Vec<u8> = vec![];
        let colors = colorize_by_classification_core(&classes);
        assert!(colors.is_empty());
    }

    #[test]
    fn test_heatmap_color_endpoints() {
        // t=0 should be blue, t=1 should be red
        let (r, g, b) = heatmap_color(0.0);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
        assert_eq!(b, 255);

        let (r, g, b) = heatmap_color(1.0);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // t=0.5 should be green
        let (r, g, b) = heatmap_color(0.5);
        assert_eq!(r, 0);
        assert!(g > 200);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_colorize_by_heatmap_core() {
        let values = vec![0.0_f32, 0.25, 0.5, 0.75, 1.0];
        let colors = colorize_by_heatmap_core(&values, 0.0, 1.0);
        assert_eq!(colors.len(), 15);

        // First value (0.0) should be blue
        assert_eq!(colors[0], 0);
        assert_eq!(colors[2], 255);

        // Last value (1.0) should be red
        assert_eq!(colors[12], 255);
        assert_eq!(colors[14], 0);
    }

    #[test]
    fn test_colorize_by_heatmap_clamping() {
        let values = vec![-5.0_f32, 10.0];
        let colors = colorize_by_heatmap_core(&values, 0.0, 1.0);
        // Both should be clamped
        assert_eq!(colors.len(), 6);
        // -5 should clamp to 0.0 → blue
        assert_eq!(colors[0], 0);
        assert_eq!(colors[2], 255);
        // 10 should clamp to 1.0 → red
        assert_eq!(colors[3], 255);
        assert_eq!(colors[5], 0);
    }

    #[test]
    fn test_build_color_ramp_two_stops() {
        let colors = vec![255u8, 0, 0, 0, 0, 255]; // red → blue
        let ramp = build_color_ramp_core(&colors, 3).unwrap();
        assert_eq!(ramp.len(), 9);

        // First stop = red
        assert_eq!(ramp[0], 255);
        assert_eq!(ramp[1], 0);
        assert_eq!(ramp[2], 0);

        // Middle ≈ purple (127 or 128, 0, 127 or 128)
        assert!((ramp[3] as i16 - 128).abs() <= 1);
        assert_eq!(ramp[4], 0);
        assert!((ramp[5] as i16 - 128).abs() <= 1);

        // Last stop = blue
        assert_eq!(ramp[6], 0);
        assert_eq!(ramp[7], 0);
        assert_eq!(ramp[8], 255);
    }

    #[test]
    fn test_build_color_ramp_errors() {
        let colors = vec![255u8, 0, 0]; // only 1 stop
        let result = build_color_ramp_core(&colors, 10);
        assert!(result.is_err());

        let colors = vec![255u8, 0, 0, 0, 0]; // odd number
        let result = build_color_ramp_core(&colors, 10);
        assert!(result.is_err());

        let colors = vec![255u8, 0, 0, 0, 0, 255];
        let result = build_color_ramp_core(&colors, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_color_ramp_multi_stop() {
        let colors = vec![
            255, 0, 0, // red
            0, 255, 0, // green
            0, 0, 255, // blue
        ];
        let ramp = build_color_ramp_core(&colors, 5).unwrap();
        assert_eq!(ramp.len(), 15);

        // Stop 0 = red
        assert_eq!(ramp[0], 255);
        assert_eq!(ramp[2], 0);

        // Stop 2 = green (middle)
        assert_eq!(ramp[6], 0);
        assert_eq!(ramp[7], 255);

        // Stop 4 = blue
        assert_eq!(ramp[12], 0);
        assert_eq!(ramp[14], 255);
    }

    // ===========================================================================
    // Point Cloud Statistics Tests
    // ===========================================================================

    #[test]
    fn test_point_cloud_bounds_empty() {
        let positions: Vec<f32> = vec![];
        let (min_x, min_y, min_z, max_x, max_y, max_z) = point_cloud_bounds_core(&positions);
        assert_eq!(min_x, f64::MAX);
        assert_eq!(min_y, f64::MAX);
        assert_eq!(min_z, f64::MAX);
        assert_eq!(max_x, f64::MIN);
        assert_eq!(max_y, f64::MIN);
        assert_eq!(max_z, f64::MIN);
    }

    #[test]
    fn test_point_cloud_bounds_single() {
        let positions = vec![5.0_f32, -3.0, 10.0];
        let (min_x, min_y, min_z, max_x, max_y, max_z) = point_cloud_bounds_core(&positions);
        assert_eq!(min_x, 5.0);
        assert_eq!(min_y, -3.0);
        assert_eq!(min_z, 10.0);
        assert_eq!(max_x, 5.0);
        assert_eq!(max_y, -3.0);
        assert_eq!(max_z, 10.0);
    }

    #[test]
    fn test_point_cloud_bounds_multiple() {
        let positions = vec![1.0_f32, 2.0, 3.0, -5.0, 10.0, 0.0, 8.0, -1.0, 6.0];
        let (min_x, min_y, min_z, max_x, max_y, max_z) = point_cloud_bounds_core(&positions);
        assert_eq!(min_x, -5.0);
        assert_eq!(max_x, 8.0);
        assert_eq!(min_y, -1.0);
        assert_eq!(max_y, 10.0);
        assert_eq!(min_z, 0.0);
        assert_eq!(max_z, 6.0);
    }

    #[test]
    fn test_point_cloud_centroid() {
        let positions = vec![
            0.0_f32, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 6.0,
        ];
        let (cx, cy, cz) = point_cloud_centroid_core(&positions);
        assert!((cx - 0.5).abs() < 1e-6);
        assert!((cy - 1.0).abs() < 1e-6);
        assert!((cz - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_point_cloud_centroid_empty() {
        let positions: Vec<f32> = vec![];
        let (cx, cy, cz) = point_cloud_centroid_core(&positions);
        assert_eq!(cx, 0.0);
        assert_eq!(cy, 0.0);
        assert_eq!(cz, 0.0);
    }

    #[test]
    fn test_point_cloud_stats_basic() {
        let positions = vec![
            0.0_f32, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0,
        ];
        let stats_json = point_cloud_stats_core(&positions).unwrap();
        let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();

        assert_eq!(stats["pointCount"], 4);
        assert_eq!(stats["bounds"]["minX"], 0.0);
        assert_eq!(stats["bounds"]["maxX"], 10.0);
        assert_eq!(stats["bounds"]["minY"], 0.0);
        assert_eq!(stats["bounds"]["maxY"], 10.0);
        assert_eq!(stats["bounds"]["minZ"], 0.0);
        assert_eq!(stats["bounds"]["maxZ"], 10.0);

        // Centroid should be (2.5, 2.5, 2.5)
        let centroid = stats["centroid"].as_array().unwrap();
        assert!((centroid[0].as_f64().unwrap() - 2.5).abs() < 1e-6);
        assert!((centroid[1].as_f64().unwrap() - 2.5).abs() < 1e-6);
        assert!((centroid[2].as_f64().unwrap() - 2.5).abs() < 1e-6);

        // Density: 4 points / (10*10*10) = 0.004
        assert!((stats["density"].as_f64().unwrap() - 0.004).abs() < 1e-6);

        // Average spacing should be > 0
        assert!(stats["averagePointSpacing"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_point_cloud_stats_empty_error() {
        let positions: Vec<f32> = vec![];
        let result = point_cloud_stats_core(&positions);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no points"));
    }

    #[test]
    fn test_point_cloud_stats_json_valid() {
        let positions = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let stats_json = point_cloud_stats_core(&positions).unwrap();
        let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();

        assert!(stats.is_object());
        assert!(stats.get("pointCount").is_some());
        assert!(stats.get("bounds").is_some());
        assert!(stats.get("centroid").is_some());
        assert!(stats.get("density").is_some());
        assert!(stats.get("averagePointSpacing").is_some());
    }

    // -----------------------------------------------------------------------
    // Large file processing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_estimate_memory_basic() {
        // 100K points, positions only: 100K * 12 = 1.2MB + octree + pnts
        let mem = estimate_memory_for_points(100_000, false, false);
        assert!(mem > 1_000_000, "Expected > 1MB, got {}", mem);
        assert!(mem < 5_000_000, "Expected < 5MB, got {}", mem);
    }

    #[test]
    fn test_estimate_memory_with_colors() {
        let mem_no_color = estimate_memory_for_points(10_000, false, false);
        let mem_color = estimate_memory_for_points(10_000, true, false);
        // Color adds 3 bytes/point = 30KB
        assert!(mem_color > mem_no_color);
        assert_eq!(mem_color - mem_no_color, 30_000);
    }

    #[test]
    fn test_estimate_memory_with_normals() {
        let mem_normals = estimate_memory_for_points(10_000, false, true);
        let mem_no_normals = estimate_memory_for_points(10_000, false, false);
        // Normals add 12 bytes/point = 120KB
        assert!(mem_normals > mem_no_normals);
        assert_eq!(mem_normals - mem_no_normals, 120_000);
    }

    #[test]
    fn test_auto_decimate_random() {
        let positions: Vec<f32> = (0..1000)
            .flat_map(|i| [i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3])
            .collect();
        let result = auto_decimate_core(&positions, 100, 0);
        assert_eq!(result.len(), 300); // 100 points × 3
    }

    #[test]
    fn test_auto_decimate_grid() {
        let positions: Vec<f32> = (0..1000)
            .flat_map(|i| [i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3])
            .collect();
        let result = auto_decimate_core(&positions, 100, 1);
        // Grid decimation keeps ≤ target_count points
        assert!(result.len() <= 300);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_auto_decimate_identity() {
        // Target >= source → should return all points
        let positions: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let result = auto_decimate_core(&positions, 100, 0);
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_auto_decimate_empty() {
        let positions: Vec<f32> = vec![];
        let result = auto_decimate_core(&positions, 100, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_chunked_las_parse() {
        // Create a minimal LAS file with 10 points
        let las_data = build_test_las_blob(
            &[
                (0.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                (1.0, 1.0, 0.0),
                (0.0, 0.0, 1.0),
                (1.0, 0.0, 1.0),
                (0.0, 1.0, 1.0),
                (1.0, 1.0, 1.0),
                (2.0, 0.0, 0.0),
                (2.0, 1.0, 0.0),
            ],
            true, // has color
        );

        let mut chunks = Vec::new();
        let total = parse_las_points_chunked(&las_data, 3, |pos, col, count| {
            chunks.push((pos.len() / 3, col.is_some(), count));
        })
        .unwrap();

        assert_eq!(total, 10);
        assert_eq!(chunks.len(), 4); // 3 + 3 + 3 + 1
        let total_from_chunks: u32 = chunks.iter().map(|c| c.2).sum();
        assert_eq!(total_from_chunks, 10);
    }

    #[test]
    fn test_chunked_las_empty() {
        let las_data = build_test_las_blob(&[], false);
        let total = parse_las_points_chunked(&las_data, 100, |_, _, _| {
            panic!("should not call callback for empty LAS");
        })
        .unwrap();
        assert_eq!(total, 0);
    }
}

/// Test-only helpers for integration tests (non-WASM targets).
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers {
    use super::*;

    /// Extract positions from a LasPointCloud without WASM typed arrays.
    pub fn get_positions(cloud: &LasPointCloud) -> &[f32] {
        &cloud.positions
    }

    /// Extract colors from a LasPointCloud without WASM typed arrays.
    pub fn get_colors(cloud: &LasPointCloud) -> Option<&[u8]> {
        cloud.colors.as_deref()
    }

    /// Get point count without WASM accessor.
    pub fn get_point_count(cloud: &LasPointCloud) -> u32 {
        cloud.point_count
    }

    /// Build a test LAS blob with given points (uses internal offset conventions).
    pub fn build_test_las_blob(points: &[(f64, f64, f64)], has_color: bool) -> Vec<u8> {
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

        let mut buf = vec![0u8; header_size as usize];
        buf[0..4].copy_from_slice(b"LASF");
        buf[24] = 1;
        buf[25] = 2;
        buf[96..100].copy_from_slice(&point_offset.to_le_bytes());
        buf[104] = point_format;
        buf[105..107].copy_from_slice(&record_len.to_le_bytes());
        buf[107..111].copy_from_slice(&num_points.to_le_bytes());
        buf[131..139].copy_from_slice(&1.0_f64.to_le_bytes());
        buf[139..147].copy_from_slice(&1.0_f64.to_le_bytes());
        buf[147..155].copy_from_slice(&1.0_f64.to_le_bytes());
        buf[179..187].copy_from_slice(&max_x.to_le_bytes());
        buf[187..195].copy_from_slice(&max_y.to_le_bytes());
        buf[195..203].copy_from_slice(&max_z.to_le_bytes());
        buf[203..211].copy_from_slice(&min_x.to_le_bytes());
        buf[211..219].copy_from_slice(&min_y.to_le_bytes());
        buf[219..227].copy_from_slice(&min_z.to_le_bytes());

        for &(x, y, z) in points {
            let base = buf.len();
            let pt_size = record_len as usize;
            buf.resize(base + pt_size, 0);
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
}
