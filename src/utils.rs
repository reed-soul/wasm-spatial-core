//! Utility functions and coordinate data quality tools.

/// Sets the `console_error_panic_hook` for better error messages in wasm.
///
/// When the `console_error_panic_hook` feature is enabled, we can get much
/// better error messages in the browser console if our code ever panics.
///
/// For more details see:
/// <https://github.com/rustwasm/console_error_panic_hook#readme>
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// ---------------------------------------------------------------------------
// Coordinate Validation & Cleaning
// ---------------------------------------------------------------------------

use wasm_bindgen::prelude::*;

/// Valid coordinate ranges for different CRS.
struct CrsRange {
    lng_min: f64,
    lng_max: f64,
    lat_min: f64,
    lat_max: f64,
}

impl CrsRange {
    fn wgs84() -> Self {
        Self {
            lng_min: -180.0,
            lng_max: 180.0,
            lat_min: -90.0,
            lat_max: 90.0,
        }
    }

    fn gcj02() -> Self {
        Self::wgs84()
    }

    fn bd09() -> Self {
        Self::wgs84()
    }

    fn mercator() -> Self {
        // Web Mercator (EPSG:3857): ~±20,037,508 meters
        Self {
            lng_min: -20_037_508.0,
            lng_max: 20_037_508.0,
            lat_min: -20_037_508.0,
            lat_max: 20_037_508.0,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "WGS84" => Some(Self::wgs84()),
            "GCJ02" => Some(Self::gcj02()),
            "BD09" => Some(Self::bd09()),
            "Mercator" => Some(Self::mercator()),
            _ => None,
        }
    }
}

/// Check if a coordinate pair is valid for the given CRS range.
#[inline]
fn is_coord_valid(lng: f64, lat: f64, range: &CrsRange) -> bool {
    if !lng.is_finite() || !lat.is_finite() {
        return false;
    }
    (lng - lng.clamp(range.lng_min, range.lng_max)).abs() < 1e-10
        && (lat - lat.clamp(range.lat_min, range.lat_max)).abs() < 1e-10
}

// ---------------------------------------------------------------------------
// Native core functions (testable without WASM)
// ---------------------------------------------------------------------------

/// Core validation logic — works on native slices.
pub fn validate_coords_native(coords: &[f64], crs: &str) -> (u32, Vec<u32>) {
    let range = CrsRange::from_name(crs).expect("Unknown CRS");
    let pair_count = coords.len() / 2;
    let mut valid_count = 0u32;
    let mut invalid_indices = Vec::new();

    for i in 0..pair_count {
        if is_coord_valid(coords[i * 2], coords[i * 2 + 1], &range) {
            valid_count += 1;
        } else {
            invalid_indices.push(i as u32);
        }
    }

    (valid_count, invalid_indices)
}

/// Core clean logic — works on native slices.
pub fn clean_coords_native(coords: &[f64], strategy: &str) -> Vec<f64> {
    let range = CrsRange::wgs84();
    let pair_count = coords.len() / 2;

    match strategy {
        "remove" => {
            let mut cleaned = Vec::with_capacity(coords.len());
            for i in 0..pair_count {
                if is_coord_valid(coords[i * 2], coords[i * 2 + 1], &range) {
                    cleaned.push(coords[i * 2]);
                    cleaned.push(coords[i * 2 + 1]);
                }
            }
            cleaned
        }
        "clamp" | "snap" => {
            let mut cleaned = Vec::with_capacity(coords.len());
            for i in 0..pair_count {
                let mut lng = coords[i * 2];
                let mut lat = coords[i * 2 + 1];
                if !is_coord_valid(lng, lat, &range) {
                    if strategy == "snap" && (!lng.is_finite() || !lat.is_finite()) {
                        if !lng.is_finite() {
                            lng = 0.0;
                        }
                        if !lat.is_finite() {
                            lat = 0.0;
                        }
                    }
                    lng = lng.clamp(range.lng_min, range.lng_max);
                    lat = lat.clamp(range.lat_min, range.lat_max);
                }
                cleaned.push(lng);
                cleaned.push(lat);
            }
            cleaned
        }
        _ => coords.to_vec(),
    }
}

/// Core deduplication logic — works on native slices.
pub fn deduplicate_coords_native(coords: &[f64], tolerance: f64) -> Vec<f64> {
    if coords.is_empty() {
        return Vec::new();
    }
    let pair_count = coords.len() / 2;
    let tol_sq = tolerance * tolerance;

    let mut result: Vec<f64> = Vec::with_capacity(coords.len());
    result.push(coords[0]);
    result.push(coords[1]);

    for i in 1..pair_count {
        let x = coords[i * 2];
        let y = coords[i * 2 + 1];
        let mut is_dup = false;
        for j in (0..result.len()).step_by(2) {
            let dx = x - result[j];
            let dy = y - result[j + 1];
            if dx * dx + dy * dy <= tol_sq {
                is_dup = true;
                break;
            }
        }
        if !is_dup {
            result.push(x);
            result.push(y);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// WASM API wrappers
// ---------------------------------------------------------------------------

/// Result of coordinate validation.
#[wasm_bindgen(js_name = "ValidationResult")]
pub struct ValidationResult {
    valid_count: u32,
    invalid_count: u32,
    invalid_indices: js_sys::Uint32Array,
}

#[wasm_bindgen]
impl ValidationResult {
    #[wasm_bindgen(getter)]
    pub fn valid_count(&self) -> u32 {
        self.valid_count
    }

    #[wasm_bindgen(getter)]
    pub fn invalid_count(&self) -> u32 {
        self.invalid_count
    }

    #[wasm_bindgen(getter)]
    pub fn invalid_indices(&self) -> js_sys::Uint32Array {
        self.invalid_indices.clone()
    }
}

/// Validate coordinate values against the expected range for a given CRS.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `crs` — One of: `"WGS84"`, `"GCJ02"`, `"BD09"`, `"Mercator"`
///
/// # Returns
///
/// A `ValidationResult` with valid count, invalid count, and indices of invalid pairs.
#[wasm_bindgen(js_name = "validateCoords")]
pub fn validate_coords(
    coords: &js_sys::Float64Array,
    crs: &str,
) -> Result<ValidationResult, JsValue> {
    let range = CrsRange::from_name(crs)
        .ok_or_else(|| JsValue::from_str(&format!("Unknown CRS: {}", crs)))?;

    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let pair_count = buf.len() / 2;
    let mut invalid_indices = Vec::new();
    let mut valid_count = 0u32;

    for i in 0..pair_count {
        if is_coord_valid(buf[i * 2], buf[i * 2 + 1], &range) {
            valid_count += 1;
        } else {
            invalid_indices.push(i as u32);
        }
    }

    let invalid_arr = js_sys::Uint32Array::new_with_length(invalid_indices.len() as u32);
    invalid_arr.copy_from(&invalid_indices);

    Ok(ValidationResult {
        valid_count,
        invalid_count: invalid_indices.len() as u32,
        invalid_indices: invalid_arr,
    })
}

/// Clean coordinate data by removing, clamping, or snapping invalid values.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `strategy` — One of: `"remove"`, `"clamp"`, `"snap"`
#[wasm_bindgen(js_name = "cleanCoords")]
pub fn clean_coords(
    coords: &js_sys::Float64Array,
    strategy: &str,
) -> Result<js_sys::Float64Array, JsValue> {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let cleaned = clean_coords_native(&buf, strategy);

    let result = js_sys::Float64Array::new_with_length(cleaned.len() as u32);
    result.copy_from(&cleaned);
    Ok(result)
}

/// Deduplicate coordinates within a tolerance.
///
/// Keeps the first occurrence of each coordinate pair within `tolerance` distance.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `tolerance` — Maximum distance (in coordinate units) for two points to be considered duplicates
#[wasm_bindgen(js_name = "deduplicateCoords")]
pub fn deduplicate_coords(coords: &js_sys::Float64Array, tolerance: f64) -> js_sys::Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let result = deduplicate_coords_native(&buf, tolerance);
    let out = js_sys::Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_all_valid() {
        let coords = &[116.4, 39.9, 121.5, 31.2];
        let (valid, invalid) = validate_coords_native(coords, "WGS84");
        assert_eq!(valid, 2);
        assert!(invalid.is_empty());
    }

    #[test]
    fn test_validate_with_invalid() {
        let coords = &[116.4, 39.9, 999.0, -999.0, 121.5, 31.2];
        let (valid, invalid) = validate_coords_native(coords, "WGS84");
        assert_eq!(valid, 2);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], 1); // second pair (index 1)
    }

    #[test]
    fn test_validate_mercator_range() {
        let coords = &[116.4, 39.9];
        let (valid, _) = validate_coords_native(coords, "Mercator");
        assert_eq!(valid, 1);
    }

    #[test]
    fn test_validate_nan() {
        let coords = &[f64::NAN, 39.9, 116.4, f64::NAN, 121.5, 31.2];
        let (valid, invalid) = validate_coords_native(coords, "WGS84");
        assert_eq!(valid, 1);
        assert_eq!(invalid.len(), 2);
    }

    #[test]
    fn test_clean_remove_strategy() {
        let coords = &[116.4, 39.9, 999.0, -999.0, 121.5, 31.2];
        let result = clean_coords_native(coords, "remove");
        assert_eq!(result.len(), 4); // 2 valid pairs
    }

    #[test]
    fn test_clean_clamp_strategy() {
        let coords = &[200.0, 100.0];
        let result = clean_coords_native(coords, "clamp");
        assert!((result[0] - 180.0).abs() < 1e-10);
        assert!((result[1] - 90.0).abs() < 1e-10);
    }

    #[test]
    fn test_clean_snap_nan() {
        let coords = &[f64::NAN, f64::NAN];
        let result = clean_coords_native(coords, "snap");
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_deduplicate_basic() {
        let coords = &[1.0, 2.0, 1.0, 2.0, 3.0, 4.0];
        let result = deduplicate_coords_native(coords, 0.001);
        assert_eq!(result.len(), 4); // 2 unique pairs
    }

    #[test]
    fn test_deduplicate_with_tolerance() {
        let coords = &[1.0, 2.0, 1.001, 2.001, 3.0, 4.0];
        let result = deduplicate_coords_native(coords, 0.01);
        assert_eq!(result.len(), 4); // 2 unique pairs
    }

    #[test]
    fn test_deduplicate_empty() {
        let coords: &[f64] = &[];
        let result = deduplicate_coords_native(coords, 0.001);
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplicate_exact() {
        // Points just outside tolerance should be kept
        let coords = &[0.0, 0.0, 1.0, 0.0, 2.0, 0.0];
        let result = deduplicate_coords_native(coords, 0.5);
        assert_eq!(result.len(), 6); // all 3 pairs unique
    }
}
