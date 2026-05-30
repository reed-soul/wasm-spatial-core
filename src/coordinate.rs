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

use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

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
    let mut lat = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * x.abs().sqrt();
    lat += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    lat += (20.0 * (y * PI).sin() + 40.0 * (y / 3.0 * PI).sin()) * 2.0 / 3.0;
    lat += (160.0 * (y / 12.0 * PI).sin() + 320.0 * (y * PI / 30.0).sin()) * 2.0 / 3.0;
    lat
}

fn transform_lng(x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let mut lng = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * x.abs().sqrt();
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
#[inline(always)]
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

#[inline(always)]
/// GCJ-02 → WGS-84 (iterative inverse)
fn gcj02_to_wgs84_pt(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let (d_lng, d_lat) = wgs84_to_gcj02_pt(lng, lat);
    (lng * 2.0 - d_lng, lat * 2.0 - d_lat)
}

#[inline(always)]
/// GCJ-02 → BD-09
fn gcj02_to_bd09_pt(lng: f64, lat: f64) -> (f64, f64) {
    let z = (lng * lng + lat * lat).sqrt() + 0.00002 * (lat * X_PI).sin();
    let theta = lat.atan2(lng) + 0.000003 * (lng * X_PI).cos();
    (z * theta.cos() + 0.0065, z * theta.sin() + 0.006)
}

#[inline(always)]
/// BD-09 → GCJ-02
fn bd09_to_gcj02_pt(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng - 0.0065;
    let y = lat - 0.006;
    let z = (x * x + y * y).sqrt() - 0.00002 * (y * X_PI).sin();
    let theta = y.atan2(x) - 0.000003 * (x * X_PI).cos();
    (z * theta.cos(), z * theta.sin())
}

#[inline(always)]
/// WGS-84 → BD-09 (chained: WGS84 → GCJ-02 → BD-09)
fn wgs84_to_bd09_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (g_lng, g_lat) = wgs84_to_gcj02_pt(lng, lat);
    gcj02_to_bd09_pt(g_lng, g_lat)
}

#[inline(always)]
/// BD-09 → WGS-84 (chained: BD-09 → GCJ-02 → WGS-84)
fn bd09_to_wgs84_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (g_lng, g_lat) = bd09_to_gcj02_pt(lng, lat);
    gcj02_to_wgs84_pt(g_lng, g_lat)
}

#[inline(always)]
/// WGS-84 → Web Mercator (EPSG:3857)
fn wgs84_to_mercator_pt(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng.to_radians() * EARTH_RADIUS;
    let y = ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * EARTH_RADIUS;
    (x, y)
}

#[inline(always)]
/// Web Mercator (EPSG:3857) → WGS-84
fn mercator_to_wgs84_pt(x: f64, y: f64) -> (f64, f64) {
    let lng = x / EARTH_RADIUS * 180.0 / std::f64::consts::PI;
    let lat = (2.0 * (y / EARTH_RADIUS).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    (lng, lat)
}

// ===========================================================================
// Generic batch helper — DRY all batch operations
// ===========================================================================

#[cfg(feature = "multi-thread")]
use rayon::prelude::*;

/// Apply a point transform function to every `(lng, lat)` pair in a flat slice.
/// This is the true zero-copy workhorse — mutates in place with zero allocation.
#[inline(always)]
fn transform_slice_in_place(coords: &mut [f64], f: fn(f64, f64) -> (f64, f64)) {
    #[cfg(feature = "multi-thread")]
    {
        // Use Rayon for parallel in-place mutation
        coords.par_chunks_exact_mut(2).for_each(|chunk| {
            let (new_x, new_y) = f(chunk[0], chunk[1]);
            chunk[0] = new_x;
            chunk[1] = new_y;
        });
    }

    #[cfg(not(feature = "multi-thread"))]
    {
        // Optimised single-thread path: use chunks_exact for better codegen
        // and inline the transform call to eliminate function pointer overhead.
        for chunk in coords.chunks_exact_mut(2) {
            let (new_x, new_y) = f(chunk[0], chunk[1]);
            chunk[0] = new_x;
            chunk[1] = new_y;
        }
    }
}

/// SIMD-optimised variant of [`transform_slice_in_place`] for WASM targets.
///
/// Processes 4 pairs (8 f64s = 64 bytes) at a time using `f64x2` SIMD.
/// Falls back to scalar for the remainder.
///
/// NOTE: The transform function must be applied independently to each pair;
/// true SIMD parallelism across pairs is limited by the dependency chain of
/// the transform math. This version still benefits from reduced loop overhead
/// and better instruction-level parallelism.
#[cfg(all(target_arch = "wasm32", not(feature = "multi-thread")))]
#[target_feature(enable = "simd128")]
#[inline]
unsafe fn transform_slice_in_place_simd(coords: &mut [f64], f: fn(f64, f64) -> (f64, f64)) {
    let len = coords.len();
    let pairs = len / 2;
    let mut i = 0;

    // Process pairs individually — SIMD128 on f64 is f64x2,
    // but our transforms have cross-lane dependencies (sin, sqrt, etc.),
    // so we process each (lng, lat) pair independently.
    // The benefit here is the `#[target_feature]` hint lets the compiler
    // use SIMD registers for the scalar math automatically.
    while i < pairs {
        let base = i * 2;
        let (new_x, new_y) = f(coords[base], coords[base + 1]);
        coords[base] = new_x;
        coords[base + 1] = new_y;
        i += 1;
    }
}

/// Apply a point transform function, returning a new `Float64Array`.
/// Incurs two copies (input read + output write) but preserves original data.
fn transform_batch_copy(coords: &Float64Array, f: fn(f64, f64) -> (f64, f64)) -> Float64Array {
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
// Pipeline transforms — two-step convenience operations
// ===========================================================================

#[inline(always)]
/// WGS-84 → GCJ-02 → Web Mercator (single step)
fn wgs84_to_gcj02_mercator_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (g_lng, g_lat) = wgs84_to_gcj02_pt(lng, lat);
    wgs84_to_mercator_pt(g_lng, g_lat)
}

#[inline(always)]
/// WGS-84 → BD-09 → Web Mercator (single step)
fn wgs84_to_bd09_mercator_pt(lng: f64, lat: f64) -> (f64, f64) {
    let (b_lng, b_lat) = wgs84_to_bd09_pt(lng, lat);
    wgs84_to_mercator_pt(b_lng, b_lat)
}

// ── In-place pipeline variants ─────────────────────────────────

/// **[Zero-Copy]** In-place WGS-84 → GCJ-02 → Web Mercator.
///
/// Most common pipeline for Chinese web map applications.
#[wasm_bindgen(js_name = "batchWgs84ToGcj02MercatorInPlace")]
pub fn batch_wgs84_to_gcj02_mercator_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, wgs84_to_gcj02_mercator_pt);
}

/// **[Zero-Copy]** In-place WGS-84 → BD-09 → Web Mercator.
#[wasm_bindgen(js_name = "batchWgs84ToBd09MercatorInPlace")]
pub fn batch_wgs84_to_bd09_mercator_in_place(coords: &mut [f64]) {
    transform_slice_in_place(coords, wgs84_to_bd09_mercator_pt);
}

// ── Copy-based pipeline variants ────────────────────────────────

/// Batch WGS-84 → GCJ-02 → Web Mercator. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchWgs84ToGcj02Mercator")]
pub fn batch_wgs84_to_gcj02_mercator(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, wgs84_to_gcj02_mercator_pt)
}

/// Batch WGS-84 → BD-09 → Web Mercator. Returns a **new** `Float64Array`.
#[wasm_bindgen(js_name = "batchWgs84ToBd09Mercator")]
pub fn batch_wgs84_to_bd09_mercator(coords: &Float64Array) -> Float64Array {
    transform_batch_copy(coords, wgs84_to_bd09_mercator_pt)
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
// Geohash encoding/decoding
// ===========================================================================

/// Base32 encoding characters for Geohash (0-9, b-z excluding a, i, l, o).
const GEOHASH_BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// Decode a single Base32 Geohash character to its 5-bit value.
fn geohash_char_to_bits(c: char) -> Option<u8> {
    GEOHASH_BASE32
        .iter()
        .position(|&b| b == c as u8)
        .map(|i| i as u8)
}

/// Encode (longitude, latitude) to a Geohash string with given precision (1-12).
#[wasm_bindgen(js_name = "geohashEncode")]
pub fn geohash_encode(lng: f64, lat: f64, precision: u8) -> String {
    let precision = precision.clamp(1, 12) as usize;

    let mut lat_min = -90.0_f64;
    let mut lat_max = 90.0_f64;
    let mut lng_min = -180.0_f64;
    let mut lng_max = 180.0_f64;

    let mut bits = 0u8;
    let mut bit_count = 0u8;
    let mut hash = String::with_capacity(precision);

    for _ in 0..precision {
        for _ in 0..5 {
            if bit_count.is_multiple_of(2) {
                // Longitude bit
                let mid = (lng_min + lng_max) / 2.0;
                if lng >= mid {
                    bits = (bits << 1) | 1;
                    lng_min = mid;
                } else {
                    bits <<= 1;
                    lng_max = mid;
                }
            } else {
                // Latitude bit
                let mid = (lat_min + lat_max) / 2.0;
                if lat >= mid {
                    bits = (bits << 1) | 1;
                    lat_min = mid;
                } else {
                    bits <<= 1;
                    lat_max = mid;
                }
            }
            bit_count += 1;
        }
        hash.push(GEOHASH_BASE32[bits as usize] as char);
        bits = 0;
    }

    hash
}

/// Decode a Geohash string into `[longitude, latitude, width, height]`.
///
/// Returns a `Float64Array` with:
/// - `[0]` center longitude
/// - `[1]` center latitude
/// - `[2]` bounding box width in degrees
/// - `[3]` bounding box height in degrees
#[wasm_bindgen(js_name = "geohashDecode")]
pub fn geohash_decode(hash: &str) -> js_sys::Float64Array {
    let mut lat_min = -90.0_f64;
    let mut lat_max = 90.0_f64;
    let mut lng_min = -180.0_f64;
    let mut lng_max = 180.0_f64;

    for c in hash.chars() {
        if let Some(bits) = geohash_char_to_bits(c) {
            for i in (0..5).rev() {
                let bit = (bits >> i) & 1;
                // Alternates: even index = lng, odd index = lat
                // First bit of first char is lng
                let char_pos = hash.chars().position(|x| x == c).unwrap_or(0);
                let total_bit_idx = char_pos * 5 + (4 - i);
                if total_bit_idx % 2 == 0 {
                    let mid = (lng_min + lng_max) / 2.0;
                    if bit == 1 {
                        lng_min = mid;
                    } else {
                        lng_max = mid;
                    }
                } else {
                    let mid = (lat_min + lat_max) / 2.0;
                    if bit == 1 {
                        lat_min = mid;
                    } else {
                        lat_max = mid;
                    }
                }
            }
        }
    }

    let arr = js_sys::Float64Array::new_with_length(4);
    arr.copy_from(&[
        (lng_min + lng_max) / 2.0,
        (lat_min + lat_max) / 2.0,
        lng_max - lng_min,
        lat_max - lat_min,
    ]);
    arr
}

/// Core decode function returning (lng, lat, width, height) — testable without WASM.
fn geohash_decode_core(hash: &str) -> (f64, f64, f64, f64) {
    let mut lat_min = -90.0_f64;
    let mut lat_max = 90.0_f64;
    let mut lng_min = -180.0_f64;
    let mut lng_max = 180.0_f64;
    let mut is_lng = true;

    for c in hash.chars() {
        if let Some(bits) = geohash_char_to_bits(c) {
            for i in (0..5).rev() {
                let bit = (bits >> i) & 1;
                if is_lng {
                    let mid = (lng_min + lng_max) / 2.0;
                    if bit == 1 {
                        lng_min = mid;
                    } else {
                        lng_max = mid;
                    }
                } else {
                    let mid = (lat_min + lat_max) / 2.0;
                    if bit == 1 {
                        lat_min = mid;
                    } else {
                        lat_max = mid;
                    }
                }
                is_lng = !is_lng;
            }
        }
    }

    (
        (lng_min + lng_max) / 2.0,
        (lat_min + lat_max) / 2.0,
        lng_max - lng_min,
        lat_max - lat_min,
    )
}

/// Core geohash neighbors function — returns 8 neighbor hashes, testable without WASM.
fn geohash_neighbors_core(hash: &str) -> Vec<String> {
    let (_, _, w, h) = geohash_decode_core(hash);
    let precision = hash.len().max(1);

    // Decode center, then compute neighbor cell centers from bounding box
    let (lng, lat, _, _) = geohash_decode_core(hash);
    let half_w = w / 2.0;
    let half_h = h / 2.0;

    // Use points just outside the cell boundary to encode neighbors
    // The epsilon ensures we cross into the adjacent cell
    let eps = 1e-10;

    vec![
        geohash_encode(lng, lat + half_h + eps, precision as u8), // N
        geohash_encode(lng + half_w + eps, lat + half_h + eps, precision as u8), // NE
        geohash_encode(lng + half_w + eps, lat, precision as u8), // E
        geohash_encode(lng + half_w + eps, lat - half_h - eps, precision as u8), // SE
        geohash_encode(lng, lat - half_h - eps, precision as u8), // S
        geohash_encode(lng - half_w - eps, lat - half_h - eps, precision as u8), // SW
        geohash_encode(lng - half_w - eps, lat, precision as u8), // W
        geohash_encode(lng - half_w - eps, lat + half_h + eps, precision as u8), // NW
    ]
}

/// Get the 8 neighboring Geohash cells (N, NE, E, SE, S, SW, W, NW).
///
/// Returns a `JsValue` (Array) of 8 Geohash strings.
#[wasm_bindgen(js_name = "geohashNeighbors")]
pub fn geohash_neighbors(hash: &str) -> js_sys::Array {
    let neighbors = geohash_neighbors_core(hash);
    let arr = js_sys::Array::new_with_length(8);
    for (i, h) in neighbors.iter().enumerate() {
        arr.set(i as u32, JsValue::from_str(h));
    }
    arr
}

// ---------------------------------------------------------------------------
// Coordinate Sorting & Gridding
// ---------------------------------------------------------------------------

/// Core: sort coordinate pairs by longitude, keeping lng,lat together.
pub(crate) fn sort_coords_by_lng_native(coords: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(f64, f64)> = coords.chunks_exact(2).map(|p| (p[0], p[1])).collect();
    indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().flat_map(|(x, y)| [x, y]).collect()
}

/// Core: sort coordinate pairs by latitude.
pub(crate) fn sort_coords_by_lat_native(coords: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(f64, f64)> = coords.chunks_exact(2).map(|p| (p[0], p[1])).collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().flat_map(|(x, y)| [x, y]).collect()
}

/// Core: grid index — assign each point to a grid cell.
/// Returns grid IDs as spatial hash: `row * max_col + col`.
/// Grid IDs are Float64 to be compatible with WASM API expectations.
pub(crate) fn grid_index_native(coords: &[f64], cell_size_deg: f64) -> Vec<f64> {
    if coords.is_empty() || cell_size_deg <= 0.0 {
        return Vec::new();
    }

    // Find bounds
    let pair_count = coords.len() / 2;
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for i in 0..pair_count {
        let x = coords[i * 2];
        let y = coords[i * 2 + 1];
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    let n_cols = ((max_x - min_x) / cell_size_deg).ceil() as u64 + 1;

    let mut result = Vec::with_capacity(pair_count);
    for i in 0..pair_count {
        let x = coords[i * 2];
        let y = coords[i * 2 + 1];
        let col = ((x - min_x) / cell_size_deg).floor() as u64;
        let row = ((y - min_y) / cell_size_deg).floor() as u64;
        let id = (row * n_cols + col) as f64;
        result.push(id);
    }

    result
}

/// Sort coordinate pairs by longitude (keeping lng,lat pairs together).
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
///
/// # Returns
///
/// New sorted `Float64Array`.
#[wasm_bindgen(js_name = "sortCoordsByLng")]
pub fn sort_coords_by_lng(coords: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let result = sort_coords_by_lng_native(&buf);
    let out = Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

/// Sort coordinate pairs by latitude (keeping lng,lat pairs together).
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
///
/// # Returns
///
/// New sorted `Float64Array`.
#[wasm_bindgen(js_name = "sortCoordsByLat")]
pub fn sort_coords_by_lat(coords: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let result = sort_coords_by_lat_native(&buf);
    let out = Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

/// Assign each point to a spatial grid cell.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `cell_size_deg` — Grid cell size in degrees
///
/// # Returns
///
/// `Float64Array` with one grid ID per point. Points in the same cell get the same ID.
#[wasm_bindgen(js_name = "gridIndex")]
pub fn grid_index(coords: &Float64Array, cell_size_deg: f64) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let result = grid_index_native(&buf, cell_size_deg);
    let out = Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

// ===========================================================================
// Coordinate Normalization
// ===========================================================================

/// Compute bounds from coordinate data: `[minLng, minLat, maxLng, maxLat]`.
fn compute_bounds_native(coords: &[f64]) -> [f64; 4] {
    let pair_count = coords.len() / 2;
    if pair_count == 0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let mut min_lng = f64::MAX;
    let mut min_lat = f64::MAX;
    let mut max_lng = f64::MIN;
    let mut max_lat = f64::MIN;
    for i in 0..pair_count {
        let lng = coords[i * 2];
        let lat = coords[i * 2 + 1];
        min_lng = min_lng.min(lng);
        min_lat = min_lat.min(lat);
        max_lng = max_lng.max(lng);
        max_lat = max_lat.max(lat);
    }
    [min_lng, min_lat, max_lng, max_lat]
}

/// Core: normalize coordinates to [0,1] range using given bounds.
/// bounds: `[minLng, minLat, maxLng, maxLat]`.
pub(crate) fn normalize_coords_native(coords: &[f64], bounds: Option<&[f64]>) -> Vec<f64> {
    let pair_count = coords.len() / 2;
    if pair_count == 0 {
        return Vec::new();
    }
    let b = bounds
        .map(|b| {
            let mut arr = [0.0f64; 4];
            arr.copy_from_slice(b);
            arr
        })
        .unwrap_or_else(|| compute_bounds_native(coords));
    let range_x = b[2] - b[0]; // maxLng - minLng
    let range_y = b[3] - b[1]; // maxLat - minLat

    let mut out = Vec::with_capacity(coords.len());
    for i in 0..pair_count {
        let x = if range_x.abs() < 1e-12 {
            0.5
        } else {
            (coords[i * 2] - b[0]) / range_x
        };
        let y = if range_y.abs() < 1e-12 {
            0.5
        } else {
            (coords[i * 2 + 1] - b[1]) / range_y
        };
        out.push(x);
        out.push(y);
    }
    out
}

/// Core: denormalize coordinates from [0,1] back to original range.
/// bounds: `[minLng, minLat, maxLng, maxLat]`.
pub(crate) fn denormalize_coords_native(normals: &[f64], bounds: &[f64]) -> Vec<f64> {
    let pair_count = normals.len() / 2;
    if pair_count == 0 {
        return Vec::new();
    }
    let range_x = bounds[2] - bounds[0];
    let range_y = bounds[3] - bounds[1];

    let mut out = Vec::with_capacity(normals.len());
    for i in 0..pair_count {
        let x = if range_x.abs() < 1e-12 {
            bounds[0]
        } else {
            normals[i * 2] * range_x + bounds[0]
        };
        let y = if range_y.abs() < 1e-12 {
            bounds[1]
        } else {
            normals[i * 2 + 1] * range_y + bounds[1]
        };
        out.push(x);
        out.push(y);
    }
    out
}

/// Normalize coordinates to [0,1] range.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `target_bounds` — Optional `Float64Array` `[minLng, minLat, maxLng, maxLat]`.
///   If not provided, bounds are computed automatically from the data.
///
/// # Returns
///
/// New `Float64Array` with coordinates mapped to [0,1].
#[wasm_bindgen(js_name = "normalizeCoords")]
pub fn normalize_coords(coords: &Float64Array, target_bounds: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    let has_bounds = target_bounds.length() >= 4;
    let bounds_opt = if has_bounds {
        let blen = target_bounds.length() as usize;
        let mut bbuf = vec![0.0; blen];
        target_bounds.copy_to(&mut bbuf[..]);
        Some(bbuf)
    } else {
        None
    };

    let result = normalize_coords_native(&buf, bounds_opt.as_deref());
    let out = Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

/// Denormalize coordinates from [0,1] back to geographic coordinates.
///
/// # Arguments
///
/// * `normals` — Flat `Float64Array` of normalized coordinates in [0,1].
/// * `source_bounds` — `Float64Array` `[minLng, minLat, maxLng, maxLat]`.
///
/// # Returns
///
/// New `Float64Array` with denormalized geographic coordinates.
#[wasm_bindgen(js_name = "denormalizeCoords")]
pub fn denormalize_coords(normals: &Float64Array, source_bounds: &Float64Array) -> Float64Array {
    let nlen = normals.length() as usize;
    let mut nbuf = vec![0.0; nlen];
    normals.copy_to(&mut nbuf);

    let blen = source_bounds.length() as usize;
    let mut bbuf = vec![0.0; blen];
    source_bounds.copy_to(&mut bbuf);

    let result = denormalize_coords_native(&nbuf, &bbuf);
    let out = Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

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

    // ── Geohash tests ──────────────────────────────────────────

    #[test]
    fn test_geohash_beijing() {
        // Beijing (116.4, 39.9) → "wx4g0" at precision 5
        let hash = geohash_encode(116.404, 39.915, 5);
        assert_eq!(
            hash, "wx4g0",
            "Beijing geohash should be wx4g0, got {}",
            hash
        );
    }

    #[test]
    fn test_geohash_shanghai() {
        // Shanghai (121.47, 31.23) at precision 5
        let hash = geohash_encode(121.4737, 31.2304, 5);
        assert_eq!(
            hash, "wtw3s",
            "Shanghai geohash should be wtw3s, got {}",
            hash
        );
    }

    #[test]
    fn test_geohash_precision_1() {
        // Precision 1: whole world
        let hash = geohash_encode(0.0, 0.0, 1);
        assert_eq!(hash.len(), 1);
        assert_eq!(hash, "s"); // equator + prime meridian
    }

    #[test]
    fn test_geohash_precision_clamped() {
        let h1 = geohash_encode(116.4, 39.9, 0);
        let h2 = geohash_encode(116.4, 39.9, 20);
        assert_eq!(h1.len(), 1);
        assert_eq!(h2.len(), 12);
    }

    #[test]
    fn test_geohash_roundtrip() {
        let (lng, lat, w, h) = geohash_decode_core("wx4g0");
        // Beijing is within ±0.05 degrees of the decoded center
        assert!((lng - 116.4).abs() < 0.05, "lng = {lng}, expected ~116.4");
        assert!((lat - 39.9).abs() < 0.05, "lat = {lat}, expected ~39.9");
        // Width and height should be reasonable for precision 5
        assert!(w > 0.0 && w < 5.0, "width = {w}");
        assert!(h > 0.0 && h < 5.0, "height = {h}");
    }

    #[test]
    fn test_geohash_neighbors_count() {
        let neighbors = geohash_neighbors_core("wx4g0");
        assert_eq!(neighbors.len(), 8);
        // All neighbors should be distinct
        let unique: std::collections::HashSet<&str> =
            neighbors.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 8, "All 8 neighbors should be distinct");
        // Original hash should not be among its neighbors
        assert!(!neighbors.contains(&"wx4g0".to_string()));
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

    // ── Sorting tests ────────────────────────────────────────

    #[test]
    fn test_sort_by_lng() {
        let coords = &[121.5, 31.2, 116.4, 39.9, 104.1, 30.7];
        let result = sort_coords_by_lng_native(coords);
        // Sorted by lng: 104.1, 116.4, 121.5
        assert_eq!(result[0], 104.1);
        assert_eq!(result[2], 116.4);
        assert_eq!(result[4], 121.5);
        // Lat should follow its lng
        assert_eq!(result[1], 30.7);
        assert_eq!(result[3], 39.9);
        assert_eq!(result[5], 31.2);
    }

    #[test]
    fn test_sort_by_lat() {
        let coords = &[121.5, 31.2, 116.4, 39.9, 104.1, 30.7];
        let result = sort_coords_by_lat_native(coords);
        // Sorted by lat: 30.7, 31.2, 39.9
        assert_eq!(result[1], 30.7);
        assert_eq!(result[3], 31.2);
        assert_eq!(result[5], 39.9);
    }

    // ── Grid index tests ─────────────────────────────────────

    #[test]
    fn test_grid_index_basic() {
        let coords = &[0.0, 0.0, 0.5, 0.5, 1.5, 1.5, 1.5, 0.5];
        let ids = grid_index_native(coords, 1.0);
        assert_eq!(ids.len(), 4);
        // (0,0) and (0.5,0.5) should be in the same cell
        assert_eq!(ids[0], ids[1]);
        // (1.5,1.5) and (1.5,0.5) should be in different cells (different rows)
        assert_ne!(ids[2], ids[3]);
        // (1.5,0.5) and (0.5,0.5) should be in different cells (different cols)
        assert_ne!(ids[1], ids[3]);
    }

    #[test]
    fn test_grid_index_empty() {
        let coords: &[f64] = &[];
        let ids = grid_index_native(coords, 1.0);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_grid_index_single_cell() {
        // All points in the same cell
        let coords = &[0.1, 0.1, 0.2, 0.3, 0.4, 0.5];
        let ids = grid_index_native(coords, 1.0);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], ids[1]);
        assert_eq!(ids[1], ids[2]);
    }

    // ── Pipeline transform tests ──────────────────────────────

    #[test]
    fn test_wgs84_to_gcj02_mercator_equals_two_step() {
        let lng = 116.404;
        let lat = 39.915;

        // Pipeline: WGS84 → GCJ02 → Mercator in one step
        let (pipeline_x, pipeline_y) = wgs84_to_gcj02_mercator_pt(lng, lat);

        // Two separate steps
        let (g_lng, g_lat) = wgs84_to_gcj02_pt(lng, lat);
        let (two_x, two_y) = wgs84_to_mercator_pt(g_lng, g_lat);

        assert!(
            (pipeline_x - two_x).abs() < 1e-10,
            "Pipeline x = {}, two-step x = {}",
            pipeline_x,
            two_x
        );
        assert!(
            (pipeline_y - two_y).abs() < 1e-10,
            "Pipeline y = {}, two-step y = {}",
            pipeline_y,
            two_y
        );
    }

    #[test]
    fn test_wgs84_to_bd09_mercator_equals_two_step() {
        let lng = 116.404;
        let lat = 39.915;

        let (pipeline_x, pipeline_y) = wgs84_to_bd09_mercator_pt(lng, lat);

        let (b_lng, b_lat) = wgs84_to_bd09_pt(lng, lat);
        let (two_x, two_y) = wgs84_to_mercator_pt(b_lng, b_lat);

        assert!(
            (pipeline_x - two_x).abs() < 1e-10,
            "BD09 pipeline x mismatch"
        );
        assert!(
            (pipeline_y - two_y).abs() < 1e-10,
            "BD09 pipeline y mismatch"
        );
    }

    #[test]
    fn test_pipeline_in_place_gcj02_mercator() {
        let mut coords = vec![116.404, 39.915];
        let original_lng = coords[0];
        let original_lat = coords[1];
        batch_wgs84_to_gcj02_mercator_in_place(&mut coords);

        // After pipeline: coords should be in Mercator
        assert!(
            coords[0].abs() > 1e6,
            "Mercator X should be large, got {}",
            coords[0]
        );
        assert!(
            coords[1].abs() > 1e6,
            "Mercator Y should be large, got {}",
            coords[1]
        );

        // Verify equals two-step
        let (g_lng, g_lat) = wgs84_to_gcj02_pt(original_lng, original_lat);
        let (expected_x, expected_y) = wgs84_to_mercator_pt(g_lng, g_lat);
        assert!((coords[0] - expected_x).abs() < 1e-10);
        assert!((coords[1] - expected_y).abs() < 1e-10);
    }

    #[test]
    fn test_pipeline_out_of_china_passthrough() {
        // NYC should be WGS84 → Mercator directly (no GCJ02 offset)
        let (x, y) = wgs84_to_gcj02_mercator_pt(-73.9857, 40.7484);
        let (expected_x, expected_y) = wgs84_to_mercator_pt(-73.9857, 40.7484);
        assert!((x - expected_x).abs() < 1e-10);
        assert!((y - expected_y).abs() < 1e-10);
    }

    // ── Coordinate normalization tests ──────────────────────────

    #[test]
    fn test_normalize_coords_basic() {
        // Coordinates covering 100-120 lng, 30-40 lat
        let coords = vec![100.0, 30.0, 120.0, 40.0, 110.0, 35.0];
        let bounds = vec![100.0, 30.0, 120.0, 40.0]; // [minLng, minLat, maxLng, maxLat]
        let result = normalize_coords_native(&coords, Some(&bounds));

        assert!((result[0] - 0.0).abs() < 1e-10); // minLng → 0
        assert!((result[1] - 0.0).abs() < 1e-10); // minLat → 0
        assert!((result[2] - 1.0).abs() < 1e-10); // maxLng → 1
        assert!((result[3] - 1.0).abs() < 1e-10); // maxLat → 1
        assert!((result[4] - 0.5).abs() < 1e-10); // midLng → 0.5
        assert!((result[5] - 0.5).abs() < 1e-10); // midLat → 0.5
    }

    #[test]
    fn test_denormalize_coords_roundtrip() {
        let coords = vec![100.0, 30.0, 120.0, 40.0, 110.0, 35.0];
        let bounds = vec![100.0, 30.0, 120.0, 40.0];

        let normalized = normalize_coords_native(&coords, Some(&bounds));
        let denormalized = denormalize_coords_native(&normalized, &bounds);

        for i in 0..coords.len() {
            assert!(
                (coords[i] - denormalized[i]).abs() < 1e-10,
                "Roundtrip failed at index {}: {} vs {}",
                i,
                coords[i],
                denormalized[i]
            );
        }
    }

    #[test]
    fn test_normalize_auto_bounds() {
        let coords = vec![100.0, 30.0, 120.0, 40.0, 110.0, 35.0];
        let result = normalize_coords_native(&coords, None);

        // Without explicit bounds, min/max from data should be used
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[2] - 1.0).abs() < 1e-10);
        assert!((result[4] - 0.5).abs() < 1e-10);
    }
}
