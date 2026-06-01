//! Point Cloud Analysis Toolkit
//!
//! Comprehensive point cloud analysis functions: statistics, filtering,
//! transformation, and merging. All functions are designed for both WASM
//! and native usage, with pure-Rust `_core` variants for testing.

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};

// ===========================================================================
// Structs
// ===========================================================================

/// Comprehensive point cloud statistics.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct PointCloudStats {
    point_count: u32,
    bounds_min_x: f64,
    bounds_min_y: f64,
    bounds_min_z: f64,
    bounds_max_x: f64,
    bounds_max_y: f64,
    bounds_max_z: f64,
    centroid_x: f64,
    centroid_y: f64,
    centroid_z: f64,
    avg_spacing: f64,
    density: f64,
    std_dev_x: f64,
    std_dev_y: f64,
    std_dev_z: f64,
    color_mean_r: f32,
    color_mean_g: f32,
    color_mean_b: f32,
    has_color: bool,
}

#[wasm_bindgen]
impl PointCloudStats {
    #[wasm_bindgen(getter, js_name = "pointCount")]
    pub fn point_count(&self) -> u32 {
        self.point_count
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

    #[wasm_bindgen(getter, js_name = "centroidX")]
    pub fn centroid_x(&self) -> f64 {
        self.centroid_x
    }
    #[wasm_bindgen(getter, js_name = "centroidY")]
    pub fn centroid_y(&self) -> f64 {
        self.centroid_y
    }
    #[wasm_bindgen(getter, js_name = "centroidZ")]
    pub fn centroid_z(&self) -> f64 {
        self.centroid_z
    }

    #[wasm_bindgen(getter, js_name = "avgSpacing")]
    pub fn avg_spacing(&self) -> f64 {
        self.avg_spacing
    }

    #[wasm_bindgen(getter)]
    pub fn density(&self) -> f64 {
        self.density
    }

    #[wasm_bindgen(getter, js_name = "stdDevX")]
    pub fn std_dev_x(&self) -> f64 {
        self.std_dev_x
    }
    #[wasm_bindgen(getter, js_name = "stdDevY")]
    pub fn std_dev_y(&self) -> f64 {
        self.std_dev_y
    }
    #[wasm_bindgen(getter, js_name = "stdDevZ")]
    pub fn std_dev_z(&self) -> f64 {
        self.std_dev_z
    }

    #[wasm_bindgen(getter, js_name = "colorMeanR")]
    pub fn color_mean_r(&self) -> f32 {
        self.color_mean_r
    }
    #[wasm_bindgen(getter, js_name = "colorMeanG")]
    pub fn color_mean_g(&self) -> f32 {
        self.color_mean_g
    }
    #[wasm_bindgen(getter, js_name = "colorMeanB")]
    pub fn color_mean_b(&self) -> f32 {
        self.color_mean_b
    }

    #[wasm_bindgen(getter, js_name = "hasColor")]
    pub fn has_color(&self) -> bool {
        self.has_color
    }

    /// Serialize stats to JSON string.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "pointCount": self.point_count,
            "bounds": {
                "minX": self.bounds_min_x, "minY": self.bounds_min_y, "minZ": self.bounds_min_z,
                "maxX": self.bounds_max_x, "maxY": self.bounds_max_y, "maxZ": self.bounds_max_z,
            },
            "centroid": [self.centroid_x, self.centroid_y, self.centroid_z],
            "avgSpacing": self.avg_spacing,
            "density": self.density,
            "stdDev": [self.std_dev_x, self.std_dev_y, self.std_dev_z],
            "colorMean": if self.has_color { Some([self.color_mean_r, self.color_mean_g, self.color_mean_b]) } else { None },
        }).to_string()
    }
}

/// Result of a point cloud filter operation.
#[wasm_bindgen]
pub struct FilteredResult {
    positions: Vec<f32>,
    colors: Option<Vec<u8>>,
    point_count: u32,
}

#[wasm_bindgen]
impl FilteredResult {
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        let arr = js_sys::Float32Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> Option<js_sys::Uint8Array> {
        self.colors.as_ref().map(|c| {
            let arr = js_sys::Uint8Array::new_with_length(c.len() as u32);
            arr.copy_from(c);
            arr
        })
    }

    #[wasm_bindgen(getter, js_name = "pointCount")]
    pub fn point_count(&self) -> u32 {
        self.point_count
    }
}

// ===========================================================================
// Core implementations (pure Rust, no WASM types)
// ===========================================================================

/// Core: compute comprehensive statistics.
pub(crate) fn compute_stats_core(
    positions: &[f32],
    colors: Option<&[u8]>,
) -> Result<PointCloudStats, String> {
    let point_count = positions.len() / 3;
    if point_count == 0 {
        return Err("Cannot compute stats: no points".to_string());
    }

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

    let n = point_count as f64;
    let cx = sum_x / n;
    let cy = sum_y / n;
    let cz = sum_z / n;

    // Std deviation per axis
    let mut var_x = 0.0_f64;
    let mut var_y = 0.0_f64;
    let mut var_z = 0.0_f64;
    for i in (0..positions.len()).step_by(3) {
        let dx = positions[i] as f64 - cx;
        let dy = positions[i + 1] as f64 - cy;
        let dz = positions[i + 2] as f64 - cz;
        var_x += dx * dx;
        var_y += dy * dy;
        var_z += dz * dz;
    }
    let std_x = (var_x / n).sqrt();
    let std_y = (var_y / n).sqrt();
    let std_z = (var_z / n).sqrt();

    // Bounding volume & density
    let dx = (max_x - min_x).max(0.001);
    let dy = (max_y - min_y).max(0.001);
    let dz = (max_z - min_z).max(0.001);
    let volume = dx * dy * dz;
    let density = n / volume;

    // Average nearest-neighbor distance (sampled)
    let sample_size = 500.min(point_count);
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
        for j in (0..positions.len()).step_by(3) {
            if j == idx {
                continue;
            }
            let ddx = positions[j] as f64 - px;
            let ddy = positions[j + 1] as f64 - py;
            let ddz = positions[j + 2] as f64 - pz;
            let dist_sq = ddx * ddx + ddy * ddy + ddz * ddz;
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

    // Color statistics
    let has_color = colors.is_some() && colors.unwrap().len() >= point_count * 3;
    let (mean_r, mean_g, mean_b) = if has_color {
        let c = colors.unwrap();
        let mut sr = 0.0_f32;
        let mut sg = 0.0_f32;
        let mut sb = 0.0_f32;
        for i in 0..point_count {
            sr += c[i * 3] as f32;
            sg += c[i * 3 + 1] as f32;
            sb += c[i * 3 + 2] as f32;
        }
        (
            sr / point_count as f32,
            sg / point_count as f32,
            sb / point_count as f32,
        )
    } else {
        (0.0, 0.0, 0.0)
    };

    Ok(PointCloudStats {
        point_count: point_count as u32,
        bounds_min_x: min_x,
        bounds_min_y: min_y,
        bounds_min_z: min_z,
        bounds_max_x: max_x,
        bounds_max_y: max_y,
        bounds_max_z: max_z,
        centroid_x: cx,
        centroid_y: cy,
        centroid_z: cz,
        avg_spacing,
        density,
        std_dev_x: std_x,
        std_dev_y: std_y,
        std_dev_z: std_z,
        color_mean_r: mean_r,
        color_mean_g: mean_g,
        color_mean_b: mean_b,
        has_color,
    })
}

/// Core: filter points by bounding box.
#[allow(clippy::too_many_arguments)]
pub(crate) fn filter_by_bounds_core(
    positions: &[f32],
    colors: Option<&[u8]>,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
) -> FilteredResult {
    let point_count = positions.len() / 3;
    let mut out_pos: Vec<f32> = Vec::new();
    let mut out_col: Vec<u8> = Vec::new();
    let has_color = colors.is_some() && colors.unwrap().len() >= point_count * 3;

    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        if x >= min_x && x <= max_x && y >= min_y && y <= max_y && z >= min_z && z <= max_z {
            out_pos.push(x);
            out_pos.push(y);
            out_pos.push(z);
            if has_color {
                let c = colors.unwrap();
                out_col.push(c[i * 3]);
                out_col.push(c[i * 3 + 1]);
                out_col.push(c[i * 3 + 2]);
            }
        }
    }

    FilteredResult {
        point_count: (out_pos.len() / 3) as u32,
        positions: out_pos,
        colors: if has_color && !out_col.is_empty() {
            Some(out_col)
        } else {
            None
        },
    }
}

/// Core: filter by ASPRS classification IDs.
///
/// LAS point format stores classification at byte offset 15 in the point record.
/// This function operates on pre-extracted classification values.
pub(crate) fn filter_by_classification_core(
    positions: &[f32],
    colors: Option<&[u8]>,
    classifications: &[u8],
    class_ids: &[u8],
) -> FilteredResult {
    let point_count = positions.len() / 3;
    let has_color = colors.is_some() && colors.unwrap().len() >= point_count * 3;
    let class_set: std::collections::HashSet<u8> = class_ids.iter().copied().collect();

    let mut out_pos: Vec<f32> = Vec::new();
    let mut out_col: Vec<u8> = Vec::new();

    let effective_count = point_count.min(classifications.len());
    for i in 0..effective_count {
        if class_set.contains(&classifications[i]) {
            out_pos.push(positions[i * 3]);
            out_pos.push(positions[i * 3 + 1]);
            out_pos.push(positions[i * 3 + 2]);
            if has_color {
                let c = colors.unwrap();
                out_col.push(c[i * 3]);
                out_col.push(c[i * 3 + 1]);
                out_col.push(c[i * 3 + 2]);
            }
        }
    }

    FilteredResult {
        point_count: (out_pos.len() / 3) as u32,
        positions: out_pos,
        colors: if has_color && !out_col.is_empty() {
            Some(out_col)
        } else {
            None
        },
    }
}

/// Core: apply a 4×4 transformation matrix (column-major, matching WebGL convention).
///
/// Matrix layout (column-major): m0..m3 = col 0, m4..m7 = col 1, etc.
pub(crate) fn transform_points_core(
    positions: &[f32],
    matrix: &[f32], // 16 elements, column-major
) -> Vec<f32> {
    let point_count = positions.len() / 3;
    let mut out = Vec::with_capacity(positions.len());

    for i in 0..point_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];

        // Matrix × vector (assuming w=1)
        let ox = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12];
        let oy = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13];
        let oz = matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14];

        out.push(ox);
        out.push(oy);
        out.push(oz);
    }

    out
}

/// Core: translate point cloud.
pub(crate) fn translate_points_core(positions: &[f32], dx: f32, dy: f32, dz: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(positions.len());
    for i in (0..positions.len()).step_by(3) {
        out.push(positions[i] + dx);
        out.push(positions[i + 1] + dy);
        out.push(positions[i + 2] + dz);
    }
    out
}

/// Core: scale point cloud.
pub(crate) fn scale_points_core(positions: &[f32], sx: f32, sy: f32, sz: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(positions.len());
    for i in (0..positions.len()).step_by(3) {
        out.push(positions[i] * sx);
        out.push(positions[i + 1] * sy);
        out.push(positions[i + 2] * sz);
    }
    out
}

/// Core: rotate point cloud around an arbitrary axis by angle (radians).
///
/// Uses Rodrigues' rotation formula.
pub(crate) fn rotate_points_core(
    positions: &[f32],
    axis: &[f32], // [x, y, z] — must be normalized
    angle: f32,   // radians
) -> Vec<f32> {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let one_minus_cos = 1.0 - cos_a;

    let ax = axis[0];
    let ay = axis[1];
    let az = axis[2];

    let mut out = Vec::with_capacity(positions.len());
    for i in (0..positions.len()).step_by(3) {
        let x = positions[i];
        let y = positions[i + 1];
        let z = positions[i + 2];

        // Rodrigues' rotation
        let ox =
            x * cos_a + (ay * z - az * y) * sin_a + ax * (ax * x + ay * y + az * z) * one_minus_cos;
        let oy =
            y * cos_a + (az * x - ax * z) * sin_a + ay * (ax * x + ay * y + az * z) * one_minus_cos;
        let oz =
            z * cos_a + (ax * y - ay * x) * sin_a + az * (ax * x + ay * y + az * z) * one_minus_cos;

        out.push(ox);
        out.push(oy);
        out.push(oz);
    }
    out
}

/// Core: merge two point clouds.
pub(crate) fn merge_points_core(
    positions_a: &[f32],
    colors_a: Option<&[u8]>,
    positions_b: &[f32],
    colors_b: Option<&[u8]>,
) -> FilteredResult {
    let mut out_pos = Vec::with_capacity(positions_a.len() + positions_b.len());
    out_pos.extend_from_slice(positions_a);
    out_pos.extend_from_slice(positions_b);

    let merged_colors = match (colors_a, colors_b) {
        (Some(a), Some(b)) => {
            let mut c = Vec::with_capacity(a.len() + b.len());
            c.extend_from_slice(a);
            c.extend_from_slice(b);
            Some(c)
        }
        (Some(a), None) => {
            let mut c = Vec::with_capacity(a.len() + (positions_b.len()));
            c.extend_from_slice(a);
            c.resize(c.len() + positions_b.len(), 128); // gray for B points
            Some(c)
        }
        (None, Some(b)) => {
            let mut c = Vec::with_capacity(positions_a.len() + b.len());
            c.resize(positions_a.len(), 128); // gray for A points
            c.extend_from_slice(b);
            Some(c)
        }
        (None, None) => None,
    };

    FilteredResult {
        point_count: (out_pos.len() / 3) as u32,
        positions: out_pos,
        colors: merged_colors,
    }
}

// ===========================================================================
// WASM API functions
// ===========================================================================

/// Compute comprehensive point cloud statistics.
///
/// Returns a `PointCloudStats` object with bounds, centroid, spacing,
/// density, standard deviation per axis, and color distribution.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `colors` — Optional Uint8Array of `[r, g, b, ...]` (pass `null` or omit)
#[wasm_bindgen(js_name = "pointCloudAnalysis")]
pub fn point_cloud_analysis(
    positions: &js_sys::Float32Array,
    colors: JsValue,
) -> Result<PointCloudStats, SpatialErrorDetail> {
    let positions = positions.to_vec();
    let colors = if colors.is_null() || colors.is_undefined() {
        None
    } else {
        let arr = js_sys::Uint8Array::from(colors);
        Some(arr.to_vec())
    };
    compute_stats_core(&positions, colors.as_deref()).map_err(SpatialError::point_cloud_error)
}

/// Filter point cloud by axis-aligned bounding box.
///
/// Keeps only points where `min_x <= x <= max_x`, etc.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `colors` — Optional Uint8Array of `[r, g, b, ...]` (pass `null` or omit)
/// * `minX`, `minY`, `minZ` — Minimum bounds
/// * `maxX`, `maxY`, `maxZ` — Maximum bounds
#[wasm_bindgen(js_name = "filterByBounds")]
#[allow(clippy::too_many_arguments)]
pub fn filter_by_bounds(
    positions: &js_sys::Float32Array,
    colors: JsValue,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
) -> FilteredResult {
    let positions = positions.to_vec();
    let colors = if colors.is_null() || colors.is_undefined() {
        None
    } else {
        let arr = js_sys::Uint8Array::from(colors);
        Some(arr.to_vec())
    };
    filter_by_bounds_core(
        &positions,
        colors.as_deref(),
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    )
}

/// Filter point cloud by ASPRS classification IDs.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `colors` — Optional Uint8Array of `[r, g, b, ...]` (pass `null` or omit)
/// * `classifications` — Uint8Array of per-point classification values
/// * `classIds` — Uint8Array of class IDs to keep (e.g., `[2, 3]` for vegetation)
#[wasm_bindgen(js_name = "filterByClassification")]
pub fn filter_by_classification(
    positions: &js_sys::Float32Array,
    colors: JsValue,
    classifications: &js_sys::Uint8Array,
    class_ids: &js_sys::Uint8Array,
) -> FilteredResult {
    let positions = positions.to_vec();
    let colors = if colors.is_null() || colors.is_undefined() {
        None
    } else {
        let arr = js_sys::Uint8Array::from(colors);
        Some(arr.to_vec())
    };
    let classifications = classifications.to_vec();
    let class_ids = class_ids.to_vec();
    filter_by_classification_core(&positions, colors.as_deref(), &classifications, &class_ids)
}

/// Apply a 4×4 transformation matrix to point positions.
///
/// Matrix is column-major (WebGL/OpenGL convention).
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `matrix` — Float32Array of 16 elements (column-major 4×4)
#[wasm_bindgen(js_name = "transformPointCloud")]
pub fn transform_point_cloud(
    positions: &js_sys::Float32Array,
    matrix: &js_sys::Float32Array,
) -> js_sys::Float32Array {
    let positions = positions.to_vec();
    let matrix = matrix.to_vec();
    let result = transform_points_core(&positions, &matrix);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Translate (move) a point cloud.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `dx`, `dy`, `dz` — Translation offsets
#[wasm_bindgen(js_name = "translatePointCloud")]
pub fn translate_point_cloud(
    positions: &js_sys::Float32Array,
    dx: f32,
    dy: f32,
    dz: f32,
) -> js_sys::Float32Array {
    let positions = positions.to_vec();
    let result = translate_points_core(&positions, dx, dy, dz);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Scale a point cloud.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `sx`, `sy`, `sz` — Scale factors
#[wasm_bindgen(js_name = "scalePointCloud")]
pub fn scale_point_cloud(
    positions: &js_sys::Float32Array,
    sx: f32,
    sy: f32,
    sz: f32,
) -> js_sys::Float32Array {
    let positions = positions.to_vec();
    let result = scale_points_core(&positions, sx, sy, sz);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Rotate a point cloud around an arbitrary axis.
///
/// Uses Rodrigues' rotation formula. The axis vector should be normalized.
///
/// # Arguments
/// * `positions` — Float32Array of `[x, y, z, ...]`
/// * `axis` — Float32Array of `[x, y, z]` (rotation axis, should be normalized)
/// * `angle` — Rotation angle in radians
#[wasm_bindgen(js_name = "rotatePointCloud")]
pub fn rotate_point_cloud(
    positions: &js_sys::Float32Array,
    axis: &js_sys::Float32Array,
    angle: f32,
) -> js_sys::Float32Array {
    let positions = positions.to_vec();
    let axis = axis.to_vec();
    let result = rotate_points_core(&positions, &axis, angle);
    let arr = js_sys::Float32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    arr
}

/// Merge two point clouds into one.
///
/// Colors are merged when both inputs have colors; if only one has colors,
/// the other's points get gray (128, 128, 128).
///
/// # Arguments
/// * `positionsA` — Float32Array for cloud A
/// * `colorsA` — Optional Uint8Array for cloud A (pass `null` or omit)
/// * `positionsB` — Float32Array for cloud B
/// * `colorsB` — Optional Uint8Array for cloud B (pass `null` or omit)
#[wasm_bindgen(js_name = "mergePointClouds")]
pub fn merge_point_clouds(
    positions_a: &js_sys::Float32Array,
    colors_a: JsValue,
    positions_b: &js_sys::Float32Array,
    colors_b: JsValue,
) -> FilteredResult {
    let positions_a = positions_a.to_vec();
    let colors_a = if colors_a.is_null() || colors_a.is_undefined() {
        None
    } else {
        let arr = js_sys::Uint8Array::from(colors_a);
        Some(arr.to_vec())
    };
    let positions_b = positions_b.to_vec();
    let colors_b = if colors_b.is_null() || colors_b.is_undefined() {
        None
    } else {
        let arr = js_sys::Uint8Array::from(colors_b);
        Some(arr.to_vec())
    };
    merge_points_core(
        &positions_a,
        colors_a.as_deref(),
        &positions_b,
        colors_b.as_deref(),
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Statistics tests ────────────────────────────────────────────

    #[test]
    fn test_compute_stats_basic() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 6.0];
        let stats = compute_stats_core(&positions, None).unwrap();

        assert_eq!(stats.point_count, 4);
        assert_eq!(stats.bounds_min_x, 0.0);
        assert_eq!(stats.bounds_max_x, 2.0);
        assert_eq!(stats.bounds_max_y, 4.0);
        assert_eq!(stats.bounds_max_z, 6.0);

        // Centroid: (0.5, 1.0, 1.5)
        assert!((stats.centroid_x - 0.5).abs() < 1e-6);
        assert!((stats.centroid_y - 1.0).abs() < 1e-6);
        assert!((stats.centroid_z - 1.5).abs() < 1e-6);

        // Density: 4 / (2*4*6) = 0.0833...
        assert!((stats.density - 4.0 / 48.0).abs() < 1e-6);

        // Std dev should be positive
        assert!(stats.std_dev_x > 0.0);
        assert!(stats.std_dev_y > 0.0);
        assert!(stats.std_dev_z > 0.0);

        assert!(!stats.has_color);
    }

    #[test]
    fn test_compute_stats_with_colors() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let colors: Vec<u8> = vec![100, 150, 200, 200, 100, 50];
        let stats = compute_stats_core(&positions, Some(&colors)).unwrap();

        assert!(stats.has_color);
        assert!((stats.color_mean_r - 150.0).abs() < 0.01);
        assert!((stats.color_mean_g - 125.0).abs() < 0.01);
        assert!((stats.color_mean_b - 125.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_stats_empty_error() {
        let result = compute_stats_core(&[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no points"));
    }

    #[test]
    fn test_compute_stats_single_point() {
        let positions = vec![5.0_f32, -3.0, 10.0];
        let stats = compute_stats_core(&positions, None).unwrap();
        assert_eq!(stats.point_count, 1);
        assert_eq!(stats.centroid_x, 5.0);
        assert_eq!(stats.std_dev_x, 0.0); // no variance with 1 point
    }

    #[test]
    fn test_compute_stats_to_json() {
        let positions = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let stats = compute_stats_core(&positions, None).unwrap();
        let json_str = stats.to_json();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["pointCount"], 2);
        assert!(json["bounds"].is_object());
        assert!(json["centroid"].is_array());
        assert!(json["colorMean"].is_null());
    }

    // ── Filter by bounds tests ────────────────────────────────────

    #[test]
    fn test_filter_by_bounds_basic() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, // inside
            5.0, 0.0, 0.0, // outside (x > 3)
            1.0, 2.0, 0.0, // inside
            1.0, 5.0, 0.0, // outside (y > 3)
        ];
        let result = filter_by_bounds_core(&positions, None, 0.0, 0.0, 0.0, 3.0, 3.0, 3.0);
        assert_eq!(result.point_count, 2);
        assert_eq!(result.positions[0], 0.0);
        assert_eq!(result.positions[3], 1.0);
        assert!(result.colors.is_none());
    }

    #[test]
    fn test_filter_by_bounds_with_colors() {
        let positions: Vec<f32> = vec![
            1.0, 2.0, 3.0, // inside
            10.0, 2.0, 3.0, // outside
        ];
        let colors: Vec<u8> = vec![255, 0, 0, 0, 255, 0];
        let result = filter_by_bounds_core(&positions, Some(&colors), 0.0, 0.0, 0.0, 5.0, 5.0, 5.0);
        assert_eq!(result.point_count, 1);
        assert!(result.colors.is_some());
        let c = result.colors.unwrap();
        assert_eq!(c[0], 255);
        assert_eq!(c[1], 0);
    }

    #[test]
    fn test_filter_by_bounds_all_inside() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 2.0, 3.0, 4.0];
        let result = filter_by_bounds_core(&positions, None, 0.0, 0.0, 0.0, 100.0, 100.0, 100.0);
        assert_eq!(result.point_count, 2);
    }

    #[test]
    fn test_filter_by_bounds_all_outside() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = filter_by_bounds_core(&positions, None, 10.0, 10.0, 10.0, 20.0, 20.0, 20.0);
        assert_eq!(result.point_count, 0);
    }

    #[test]
    fn test_filter_by_bounds_empty() {
        let positions: Vec<f32> = vec![];
        let result = filter_by_bounds_core(&positions, None, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert_eq!(result.point_count, 0);
    }

    // ── Filter by classification tests ────────────────────────────

    #[test]
    fn test_filter_by_classification_basic() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, // class 0 (never classified)
            1.0, 1.0, 1.0, // class 2 (vegetation)
            2.0, 2.0, 2.0, // class 2 (vegetation)
            3.0, 3.0, 3.0, // class 9 (water)
        ];
        let classes: Vec<u8> = vec![0, 2, 2, 9];
        let class_ids: Vec<u8> = vec![2]; // keep only vegetation

        let result = filter_by_classification_core(&positions, None, &classes, &class_ids);
        assert_eq!(result.point_count, 2);
    }

    #[test]
    fn test_filter_by_classification_multiple_classes() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0];
        let classes: Vec<u8> = vec![0, 2, 9, 3];
        let class_ids: Vec<u8> = vec![2, 3]; // vegetation + buildings

        let result = filter_by_classification_core(&positions, None, &classes, &class_ids);
        assert_eq!(result.point_count, 2);
    }

    #[test]
    fn test_filter_by_classification_empty_ids() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let classes: Vec<u8> = vec![0, 2];
        let class_ids: Vec<u8> = vec![];

        let result = filter_by_classification_core(&positions, None, &classes, &class_ids);
        assert_eq!(result.point_count, 0);
    }

    // ── Transform tests ────────────────────────────────────────────

    #[test]
    fn test_transform_identity() {
        // Identity matrix (column-major)
        let matrix: Vec<f32> = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = transform_points_core(&positions, &matrix);
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
        assert_eq!(result[3], 4.0);
    }

    #[test]
    fn test_transform_translation() {
        // Translation matrix (column-major): translate by (10, 20, 30)
        let matrix: Vec<f32> = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = transform_points_core(&positions, &matrix);
        assert!((result[0] - 11.0).abs() < 1e-5);
        assert!((result[1] - 22.0).abs() < 1e-5);
        assert!((result[2] - 33.0).abs() < 1e-5);
    }

    #[test]
    fn test_transform_scale_matrix() {
        // Scale by 2x (column-major)
        let matrix: Vec<f32> = vec![
            2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = transform_points_core(&positions, &matrix);
        assert!((result[0] - 2.0).abs() < 1e-5);
        assert!((result[1] - 4.0).abs() < 1e-5);
        assert!((result[2] - 6.0).abs() < 1e-5);
    }

    // ── Translate tests ───────────────────────────────────────────

    #[test]
    fn test_translate_basic() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = translate_points_core(&positions, 10.0, -5.0, 0.0);
        assert!((result[0] - 11.0).abs() < 1e-6);
        assert!((result[1] - (-3.0)).abs() < 1e-6);
        assert_eq!(result[2], 3.0);
        assert!((result[3] - 14.0).abs() < 1e-6);
    }

    #[test]
    fn test_translate_empty() {
        let positions: Vec<f32> = vec![];
        let result = translate_points_core(&positions, 1.0, 2.0, 3.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_translate_zero() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = translate_points_core(&positions, 0.0, 0.0, 0.0);
        assert_eq!(result, positions);
    }

    // ── Scale tests ───────────────────────────────────────────────

    #[test]
    fn test_scale_uniform() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = scale_points_core(&positions, 2.0, 2.0, 2.0);
        assert_eq!(result[0], 2.0);
        assert_eq!(result[1], 4.0);
        assert_eq!(result[2], 6.0);
    }

    #[test]
    fn test_scale_non_uniform() {
        let positions: Vec<f32> = vec![1.0, 1.0, 1.0];
        let result = scale_points_core(&positions, 2.0, 3.0, 4.0);
        assert_eq!(result[0], 2.0);
        assert_eq!(result[1], 3.0);
        assert_eq!(result[2], 4.0);
    }

    #[test]
    fn test_scale_zero() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = scale_points_core(&positions, 0.0, 0.0, 0.0);
        assert_eq!(result[0], 0.0);
        assert_eq!(result[1], 0.0);
        assert_eq!(result[2], 0.0);
    }

    // ── Rotate tests ───────────────────────────────────────────────

    #[test]
    fn test_rotate_z_90_degrees() {
        // Rotate (1, 0, 0) around Z axis by 90° → should be (0, 1, 0)
        let positions: Vec<f32> = vec![1.0, 0.0, 0.0];
        let axis: Vec<f32> = vec![0.0, 0.0, 1.0]; // Z axis
        let result = rotate_points_core(&positions, &axis, std::f32::consts::FRAC_PI_2);
        assert!((result[0] - 0.0).abs() < 1e-5);
        assert!((result[1] - 1.0).abs() < 1e-5);
        assert!((result[2] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_rotate_y_180_degrees() {
        // Rotate (1, 0, 0) around Y by 180° → should be (-1, 0, 0)
        let positions: Vec<f32> = vec![1.0, 0.0, 0.0];
        let axis: Vec<f32> = vec![0.0, 1.0, 0.0]; // Y axis
        let result = rotate_points_core(&positions, &axis, std::f32::consts::PI);
        assert!((result[0] - (-1.0)).abs() < 1e-5);
        assert!((result[1] - 0.0).abs() < 1e-5);
        assert!((result[2] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_rotate_zero_angle() {
        let positions: Vec<f32> = vec![1.0, 2.0, 3.0];
        let axis: Vec<f32> = vec![0.0, 0.0, 1.0];
        let result = rotate_points_core(&positions, &axis, 0.0);
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!((result[1] - 2.0).abs() < 1e-5);
        assert!((result[2] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_rotate_preserves_length() {
        let positions: Vec<f32> = vec![3.0, 4.0, 0.0]; // length = 5
        let axis: Vec<f32> = vec![0.0, 0.0, 1.0];
        let result = rotate_points_core(&positions, &axis, 1.23); // arbitrary angle
        let length = (result[0] * result[0] + result[1] * result[1] + result[2] * result[2]).sqrt();
        assert!((length - 5.0).abs() < 1e-5);
    }

    // ── Merge tests ───────────────────────────────────────────────

    #[test]
    fn test_merge_both_with_colors() {
        let pos_a: Vec<f32> = vec![1.0, 2.0, 3.0];
        let col_a: Vec<u8> = vec![255, 0, 0];
        let pos_b: Vec<f32> = vec![4.0, 5.0, 6.0];
        let col_b: Vec<u8> = vec![0, 255, 0];

        let result = merge_points_core(&pos_a, Some(&col_a), &pos_b, Some(&col_b));
        assert_eq!(result.point_count, 2);
        assert_eq!(result.positions.len(), 6);
        let c = result.colors.unwrap();
        assert_eq!(c[0], 255); // A color
        assert_eq!(c[3], 0); // B R
        assert_eq!(c[4], 255); // B G
    }

    #[test]
    fn test_merge_no_colors() {
        let pos_a: Vec<f32> = vec![1.0, 2.0, 3.0];
        let pos_b: Vec<f32> = vec![4.0, 5.0, 6.0];

        let result = merge_points_core(&pos_a, None, &pos_b, None);
        assert_eq!(result.point_count, 2);
        assert!(result.colors.is_none());
    }

    #[test]
    fn test_merge_partial_colors() {
        let pos_a: Vec<f32> = vec![1.0, 2.0, 3.0];
        let col_a: Vec<u8> = vec![255, 0, 0];
        let pos_b: Vec<f32> = vec![4.0, 5.0, 6.0];
        // B has no colors → A's points keep their color, B gets gray

        let result = merge_points_core(&pos_a, Some(&col_a), &pos_b, None);
        assert_eq!(result.point_count, 2);
        let c = result.colors.unwrap();
        assert_eq!(c.len(), 6);
        assert_eq!(c[0], 255); // A
        assert_eq!(c[3], 128); // B gray
        assert_eq!(c[4], 128);
        assert_eq!(c[5], 128);
    }

    #[test]
    fn test_merge_empty_a() {
        let pos_a: Vec<f32> = vec![];
        let pos_b: Vec<f32> = vec![4.0, 5.0, 6.0];
        let result = merge_points_core(&pos_a, None, &pos_b, None);
        assert_eq!(result.point_count, 1);
    }

    #[test]
    fn test_merge_empty_both() {
        let result = merge_points_core(&[], None, &[], None);
        assert_eq!(result.point_count, 0);
    }
}
