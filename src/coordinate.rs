//! Coordinate system transformations.
//!
//! Provides high-frequency CRS projection conversions between common spatial
//! reference systems (WGS84, GCJ-02, BD-09, Web Mercator, CGCS2000, etc.)
//! directly inside the browser via WASM.
//!
//! ## Design Notes
//!
//! All public APIs are offered in two flavours:
//!
//! 1. **In-place (`&mut [f64]`)** — True zero-copy. The JS caller passes a
//!    view into WASM linear memory; coordinates are mutated in place with no
//!    allocation or copying. This is the recommended hot-path API.
//!
//! 2. **Returning (`&Float64Array → Float64Array`)** — Convenience API that
//!    creates a new output buffer. Incurs two copies (in + out) and is best
//!    for small datasets or when the caller wants to keep the original data.
//!
//! All buffers use a flat `[lng0, lat0, lng1, lat1, …]` layout, directly
//! compatible with WebGL `ARRAY_BUFFER` uploads.

use wasm_bindgen::prelude::*;
use js_sys::Float64Array;

// ===========================================================================
// Constants
// ===========================================================================

/// WGS-84 / GCJ-02 ellipsoid semi-major axis
const A: f64 = 6378245.0;
/// WGS-84 / GCJ-02 eccentricity squared
const EE: f64 = 0.006_693_421_62;
/// WGS-84 Earth radius for Web Mercator
const EARTH_RADIUS: f64 = 6378137.0;
/// BD-09 encryption constant
const X_PI: f64 = std::f64::consts::PI * 3000.0 / 180.0;

// ===========================================================================
// Internal helpers — GCJ-02
// ===========================================================================

fn transform_lat(x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let mut lat = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y
        + 0.2 * x.abs().sqrt();
    lat += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    lat += (20.0 * (y * PI).sin() + 40.0 * (y / 3.0 * PI).sin()) * 2.0 / 3.0;
    lat += (160.0 * (y / 12.0 * PI).sin() + 320.0 * (y * PI / 30.0).sin()) * 2.0 / 3.0;
    lat
}

fn transform_lng(x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let mut lng = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y
        + 0.1 * x.abs().sqrt();
    lng += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    lng += (20.0 * (x * PI).sin() + 40.0 * (x / 3.0 * PI).sin()) * 2.0 / 3.0;
    lng += (150.0 * (x / 12.0 * PI).sin() + 300.0 * (x / 30.0 * PI).sin()) * 2.0 / 3.0;
    lng
}

fn out_of_china(lng: f64, lat: f64) -> bool {
    !(73.66..=135.05).contains(&lng) || !(3.86..=53.55).contains(&lat)
}

// ===========================================================================
// Single-point transforms (internal)
// ===========================================================================

/// WGS-84 → GCJ-02
fn wgs84_to_gcj02_pt(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let mut d_lat = transform_lat(lng - 105.0, lat - 35.0);
    let mut d_lng = transform_lng(lng - 105.0, lat - 35.0);
    let rad_lat = lat.to_radians();
    let magic = 1.0 - EE * rad_lat.sin() * rad_lat.sin();
    let sqrt_magic = magic.sqrt();
    d_lat = (d_lat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrt_magic) * std::f64::consts::PI);
    d_lng = (d_lng * 180.0) / (A / sqrt_magic * rad_lat.cos() * std::f64::consts::PI);
    (lng + d_lng, lat + d_lat)
}

/// GCJ-02 → WGS-84 (iterative inverse)
fn gcj02_to_wgs84_pt(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let (d_lng, d_lat) = wgs84_to_gcj02_pt(lng, lat);
    (lng * 2.0 - d_lng, lat * 2.0 - d_lat)
}

/// GCJ-02 → BD-09
fn gcj02_to_bd09_pt(lng: f64, lat: f64) -> (f64, f64) {
    let z = (lng * lng + lat * lat).sqrt() + 0.00002 * (lat * X_PI).sin();
    let theta = lat.atan2(lng) + 0.000003 * (lng * X_PI).cos();
    (z * theta.cos() + 0.0065, z * theta.sin() + 0.006)
}

/// BD-09 → GCJ-02
fn bd09_to_gcj02_pt(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng - 0.0065;
    let y = lat - 0.006;
    let z = (x * x + y * y).sqrt() - 0.00002 * (y * X_PI).sin();
    let theta = y.atan2(x) - 0.000003 * (x * X_PI).cos();
    (z * theta.cos(), z * theta.sin())
}

/// WGS-84 → BD-09 (chained: WGS84 → GCJ-02 → BD-09)
fn wgs84_to_bd09_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (g_lng, g_lat) = wgs84_to_gcj02_pt(lng, lat);
    gcj02_to_bd09_pt(g_lng, g_lat)
}

/// BD-09 → WGS-84 (chained: BD-09 → GCJ-02 → WGS-84)
fn bd09_to_wgs84_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (g_lng, g_lat) = bd09_to_gcj02_pt(lng, lat);
    gcj02_to_wgs84_pt(g_lng, g_lat)
}

/// WGS-84 → Web Mercator (EPSG:3857)
fn wgs84_to_mercator_pt(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng.to_radians() * EARTH_RADIUS;
    let y = ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * EARTH_RADIUS;
    (x, y)
}

/// Web Mercator (EPSG:3857) → WGS-84
fn mercator_to_wgs84_pt(x: f64, y: f64) -> (f64, f64) {
    let lng = x / EARTH_RADIUS * 180.0 / std::f64::consts::PI;
    let lat = (2.0 * (y / EARTH_RADIUS).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    (lng, lat)
}

// ===========================================================================
// Generic batch helper — DRY all batch operations
// ===========================================================================

/// Apply a point transform function to every `(lng, lat)` pair in a flat slice.
/// This is the true zero-copy workhorse — mutates in place with zero allocation.
#[inline]
fn transform_slice_in_place(coords: &mut [f64], f: fn(f64, f64) -> (f64, f64)) {
    let len = coords.len();
    let mut i = 0;
    while i + 1 < len {
        let (new_x, new_y) = f(coords[i], coords[i + 1]);
        coords[i] = new_x;
        coords[i + 1] = new_y;
        i += 2;
    }
}

/// Apply a point transform function, returning a new `Float64Array`.
/// Incurs two copies (input read + output write) but preserves original data.
fn transform_batch_copy(
    coords: &Float64Array,
    f: fn(f64, f64) -> (f64, f64),
) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0f64; len];
    coords.copy_to(&mut buf);

    transform_slice_in_place(&mut buf, f);

    let result = Float64Array::new_with_length(len as u32);
    result.copy_from(&buf);
    result
}

// ===========================================================================
// PUBLIC WASM API — In-place (zero-copy) operations
// ===========================================================================
//
// These take `&mut [f64]` which wasm-bindgen maps to a direct view into
// WASM linear memory. The JS caller passes a typed array backed by
// wasm memory — no copies occur.

/// **[Zero-Copy]** In-place WGS-84 → GCJ-02.
///
/// Mutates the input `[lng, lat, …]` buffer directly in WASM linear memory.
/// ```js
/// const buf = new Float64Array(wasmMemory.buffer, ptr, len);
/// wasm.batchWgs84ToGcj02InPlace(buf);
/// // buf is now in GCJ-02 — no copy occurred
/// ```
#[wasm_bindgen(js_name = "batchWgs84ToGcj02InPlace")]
pub fn batch_wgs84_to_gcj02_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, wgs84_to_gcj02_pt);
}

/// **[Zero-Copy]** In-place GCJ-02 → WGS-84.
#[wasm_bindgen(js_name = "batchGcj02ToWgs84InPlace")]
pub fn batch_gcj02_to_wgs84_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, gcj02_to_wgs84_pt);
}

/// **[Zero-Copy]** In-place WGS-84 → BD-09.
#[wasm_bindgen(js_name = "batchWgs84ToBd09InPlace")]
pub fn batch_wgs84_to_bd09_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, wgs84_to_bd09_pt);
}

/// **[Zero-Copy]** In-place BD-09 → WGS-84.
#[wasm_bindgen(js_name = "batchBd09ToWgs84InPlace")]
pub fn batch_bd09_to_wgs84_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, bd09_to_wgs84_pt);
}

/// **[Zero-Copy]** In-place GCJ-02 → BD-09.
#[wasm_bindgen(js_name = "batchGcj02ToBd09InPlace")]
pub fn batch_gcj02_to_bd09_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, gcj02_to_bd09_pt);
}

/// **[Zero-Copy]** In-place BD-09 → GCJ-02.
#[wasm_bindgen(js_name = "batchBd09ToGcj02InPlace")]
pub fn batch_bd09_to_gcj02_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, bd09_to_gcj02_pt);
}

/// **[Zero-Copy]** In-place WGS-84 → Web Mercator (EPSG:3857).
#[wasm_bindgen(js_name = "batchWgs84ToMercatorInPlace")]
pub fn batch_wgs84_to_mercator_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, wgs84_to_mercator_pt);
}

/// **[Zero-Copy]** In-place Web Mercator (EPSG:3857) → WGS-84.
#[wasm_bindgen(js_name = "batchMercatorToWgs84InPlace")]
pub fn batch_mercator_to_wgs84_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, mercator_to_wgs84_pt);
}

// ===========================================================================
// PUBLIC WASM API — Copy-based (convenience) operations
// ===========================================================================

/// Batch WGS-84 → GCJ-02. Returns a **new** `Float64Array`.
///
/// For large datasets, prefer the `InPlace` variant to avoid copies.
#[wasm_bindgen(js_name = "batchWgs84ToGcj02")]
pub fn batch_wgs84_to_gcj02(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, wgs84_to_gcj02_pt)
}

/// Batch GCJ-02 → WGS-84. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchGcj02ToWgs84")]
pub fn batch_gcj02_to_wgs84(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, gcj02_to_wgs84_pt)
}

/// Batch WGS-84 → BD-09. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchWgs84ToBd09")]
pub fn batch_wgs84_to_bd09(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, wgs84_to_bd09_pt)
}

/// Batch BD-09 → WGS-84. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchBd09ToWgs84")]
pub fn batch_bd09_to_wgs84(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, bd09_to_wgs84_pt)
}

/// Batch GCJ-02 → BD-09. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchGcj02ToBd09")]
pub fn batch_gcj02_to_bd09(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, gcj02_to_bd09_pt)
}

/// Batch BD-09 → GCJ-02. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchBd09ToGcj02")]
pub fn batch_bd09_to_gcj02(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, bd09_to_gcj02_pt)
}

/// Batch WGS-84 → Web Mercator (EPSG:3857). Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchWgs84ToMercator")]
pub fn batch_wgs84_to_mercator(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, wgs84_to_mercator_pt)
}

/// Batch Web Mercator (EPSG:3857) → WGS-84. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchMercatorToWgs84")]
pub fn batch_mercator_to_wgs84(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, mercator_to_wgs84_pt)
}

// ===========================================================================
// CGCS2000 (China Geodetic Coordinate System 2000)
// ===========================================================================

/// Check if CGCS2000 and WGS-84 are equivalent for the caller's precision.
///
/// CGCS2000 and WGS-84 share virtually identical ellipsoid parameters.
/// The difference is sub-centimetre level (< 0.11 mm at epoch 2000.0).
///
/// For engineering-grade accuracy (> 1 cm), they are interchangeable.
/// This function returns `true`, indicating the identity transform is valid.
///
/// For geodetic-survey-grade work (mm-level), users should apply an
/// epoch-dependent tectonic plate motion model — this is outside the
/// scope of a browser-based library.
#[wasm_bindgen(js_name = "cgcs2000IsWgs84Compatible")]
pub fn cgcs2000_is_wgs84_compatible() -> bool {
    true
}

/// **[Zero-Copy]** In-place "WGS-84 → CGCS2000" — identity transform.
///
/// Provided for API completeness. Since CGCS2000 ≈ WGS-84 (< 1 cm difference),
/// this is a no-op. The buffer is returned unchanged.
///
/// If your pipeline requires an explicit CGCS2000 step, call this to make the
/// intent clear in code without incurring any runtime cost.
#[wasm_bindgen(js_name = "batchWgs84ToCgcs2000InPlace")]
pub fn batch_wgs84_to_cgcs2000_in_place(_coords: &mut [f64]) {
    // Identity transform — CGCS2000 and WGS-84 are equivalent for
    // all practical engineering purposes (< 1 cm difference).
    // This is an intentional no-op for API completeness.
}

/// Batch "WGS-84 → CGCS2000" — identity transform. Returns a copy.
///
/// See [`cgcs2000_is_wgs84_compatible`] for precision details.
#[wasm_bindgen(js_name = "batchWgs84ToCgcs2000")]
pub fn batch_wgs84_to_cgcs2000(coords: &Float64Array) -> Float64Array {
    // Return a copy of the input — CGCS2000 ≈ WGS-84
    let len = coords.length() as usize;
    let mut buf = vec![0f64; len];
    coords.copy_to(&mut buf);
    let result = Float64Array::new_with_length(len as u32);
    result.copy_from(&buf);
    result
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── WGS-84 ↔ GCJ-02 ──────────────────────────────────────

    #[test]
    fn test_wgs84_to_gcj02_in_china() {
        let (lng, lat) = wgs84_to_gcj02_pt(116.404, 39.915);
        assert!((lng - 116.410).abs() < 0.01, "lng = {lng}");
        assert!((lat - 39.917).abs() < 0.01, "lat = {lat}");
    }

    #[test]
    fn test_out_of_china_passthrough() {
        let (lng, lat) = wgs84_to_gcj02_pt(-73.9857, 40.7484);
        assert!((lng - (-73.9857)).abs() < 1e-10);
        assert!((lat - 40.7484).abs() < 1e-10);
    }

    #[test]
    fn test_roundtrip_gcj02() {
        let (g_lng, g_lat) = wgs84_to_gcj02_pt(116.404, 39.915);
        let (w_lng, w_lat) = gcj02_to_wgs84_pt(g_lng, g_lat);
        assert!((w_lng - 116.404).abs() < 0.001);
        assert!((w_lat - 39.915).abs() < 0.001);
    }

    // ── BD-09 ─────────────────────────────────────────────────

    #[test]
    fn test_gcj02_to_bd09() {
        let (lng, lat) = gcj02_to_bd09_pt(116.404, 39.915);
        assert!((lng - 116.410).abs() < 0.02, "bd09 lng = {lng}");
        assert!((lat - 39.921).abs() < 0.02, "bd09 lat = {lat}");
    }

    #[test]
    fn test_roundtrip_bd09() {
        let (b_lng, b_lat) = gcj02_to_bd09_pt(116.404, 39.915);
        let (g_lng, g_lat) = bd09_to_gcj02_pt(b_lng, b_lat);
        assert!((g_lng - 116.404).abs() < 0.0001, "g_lng = {g_lng}");
        assert!((g_lat - 39.915).abs() < 0.0001, "g_lat = {g_lat}");
    }

    #[test]
    fn test_wgs84_bd09_roundtrip() {
        let (b_lng, b_lat) = wgs84_to_bd09_pt(116.404, 39.915);
        let (w_lng, w_lat) = bd09_to_wgs84_pt(b_lng, b_lat);
        assert!((w_lng - 116.404).abs() < 0.002, "w_lng = {w_lng}");
        assert!((w_lat - 39.915).abs() < 0.002, "w_lat = {w_lat}");
    }

    // ── Mercator ──────────────────────────────────────────────

    #[test]
    fn test_wgs84_to_mercator() {
        let (x, _y) = wgs84_to_mercator_pt(0.0, 0.0);
        assert!(x.abs() < 1e-6, "origin x = {x}");
    }

    #[test]
    fn test_roundtrip_mercator() {
        let (x, y) = wgs84_to_mercator_pt(116.404, 39.915);
        let (lng, lat) = mercator_to_wgs84_pt(x, y);
        assert!((lng - 116.404).abs() < 1e-8, "lng = {lng}");
        assert!((lat - 39.915).abs() < 1e-8, "lat = {lat}");
    }

    // ── In-place ──────────────────────────────────────────────

    #[test]
    fn test_in_place_wgs84_to_gcj02() {
        let mut coords = vec![116.404, 39.915, -73.9857, 40.7484];
        batch_wgs84_to_gcj02_in_place(&mut coords);

        // Beijing should be offset
        assert!((coords[0] - 116.410).abs() < 0.01);
        assert!((coords[1] - 39.917).abs() < 0.01);

        // NYC should pass through unchanged
        assert!((coords[2] - (-73.9857)).abs() < 1e-10);
        assert!((coords[3] - 40.7484).abs() < 1e-10);
    }

    // ── CGCS2000 ──────────────────────────────────────────────

    #[test]
    fn test_cgcs2000_identity() {
        assert!(cgcs2000_is_wgs84_compatible());

        let mut coords = vec![116.404, 39.915];
        let original = coords.clone();
        batch_wgs84_to_cgcs2000_in_place(&mut coords);
        assert_eq!(coords, original, "CGCS2000 should be identity");
    }
}
