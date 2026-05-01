//! Coordinate system transformations.
//!
//! Provides high-frequency CRS projection conversions between common spatial
//! reference systems (WGS84, GCJ-02, BD-09, Web Mercator, etc.) directly
//! inside the browser via WASM.
//!
//! ## Design Notes
//!
//! All public APIs operate on flat `Float64Array` buffers to minimise
//! serialisation overhead between JS and WASM (zero-copy where possible).

use wasm_bindgen::prelude::*;
use js_sys::Float64Array;

// ---------------------------------------------------------------------------
// WGS-84  ↔  GCJ-02  (China offset)
// ---------------------------------------------------------------------------

const A: f64 = 6378245.0;
const EE: f64 = 0.006_693_421_62;

fn transform_lat(x: f64, y: f64) -> f64 {
    let mut lat = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y
        + 0.2 * x.abs().sqrt();
    lat += (20.0 * (6.0 * x * std::f64::consts::PI).sin()
        + 20.0 * (2.0 * x * std::f64::consts::PI).sin())
        * 2.0
        / 3.0;
    lat += (20.0 * (y * std::f64::consts::PI).sin()
        + 40.0 * (y / 3.0 * std::f64::consts::PI).sin())
        * 2.0
        / 3.0;
    lat += (160.0 * (y / 12.0 * std::f64::consts::PI).sin()
        + 320.0 * (y * std::f64::consts::PI / 30.0).sin())
        * 2.0
        / 3.0;
    lat
}

fn transform_lng(x: f64, y: f64) -> f64 {
    let mut lng = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y
        + 0.1 * x.abs().sqrt();
    lng += (20.0 * (6.0 * x * std::f64::consts::PI).sin()
        + 20.0 * (2.0 * x * std::f64::consts::PI).sin())
        * 2.0
        / 3.0;
    lng += (20.0 * (x * std::f64::consts::PI).sin()
        + 40.0 * (x / 3.0 * std::f64::consts::PI).sin())
        * 2.0
        / 3.0;
    lng += (150.0 * (x / 12.0 * std::f64::consts::PI).sin()
        + 300.0 * (x / 30.0 * std::f64::consts::PI).sin())
        * 2.0
        / 3.0;
    lng
}

fn out_of_china(lng: f64, lat: f64) -> bool {
    !(73.66..=135.05).contains(&lng) || !(3.86..=53.55).contains(&lat)
}

/// WGS-84 → GCJ-02 for a single point.
fn wgs84_to_gcj02_single(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let mut d_lat = transform_lat(lng - 105.0, lat - 35.0);
    let mut d_lng = transform_lng(lng - 105.0, lat - 35.0);
    let rad_lat = lat / 180.0 * std::f64::consts::PI;
    let magic = 1.0 - EE * rad_lat.sin() * rad_lat.sin();
    let sqrt_magic = magic.sqrt();
    d_lat = (d_lat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrt_magic) * std::f64::consts::PI);
    d_lng = (d_lng * 180.0) / (A / sqrt_magic * rad_lat.cos() * std::f64::consts::PI);
    (lng + d_lng, lat + d_lat)
}

/// GCJ-02 → WGS-84 for a single point (iterative).
fn gcj02_to_wgs84_single(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let (d_lng, d_lat) = wgs84_to_gcj02_single(lng, lat);
    (lng * 2.0 - d_lng, lat * 2.0 - d_lat)
}

// ---------------------------------------------------------------------------
// Public WASM API — batch operations on flat Float64Array
// ---------------------------------------------------------------------------

/// Batch-convert a flat `[lng0, lat0, lng1, lat1, …]` array from **WGS-84**
/// to **GCJ-02**. Returns a new `Float64Array` of the same length.
///
/// This is the primary high-performance entry point for coordinate conversion;
/// data stays in linear WASM memory and avoids per-point JS ↔ WASM round-trips.
#[wasm_bindgen(js_name = "batchWgs84ToGcj02")]
pub fn batch_wgs84_to_gcj02(coords: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0f64; len];
    coords.copy_to(&mut buf);

    for i in (0..len).step_by(2) {
        let (lng, lat) = wgs84_to_gcj02_single(buf[i], buf[i + 1]);
        buf[i] = lng;
        buf[i + 1] = lat;
    }

    let result = Float64Array::new_with_length(len as u32);
    result.copy_from(&buf);
    result
}

/// Batch-convert a flat `[lng0, lat0, lng1, lat1, …]` array from **GCJ-02**
/// to **WGS-84**. Returns a new `Float64Array` of the same length.
#[wasm_bindgen(js_name = "batchGcj02ToWgs84")]
pub fn batch_gcj02_to_wgs84(coords: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0f64; len];
    coords.copy_to(&mut buf);

    for i in (0..len).step_by(2) {
        let (lng, lat) = gcj02_to_wgs84_single(buf[i], buf[i + 1]);
        buf[i] = lng;
        buf[i + 1] = lat;
    }

    let result = Float64Array::new_with_length(len as u32);
    result.copy_from(&buf);
    result
}

/// Batch-convert a flat `[lng, lat, …]` array from **WGS-84** to
/// **Web Mercator (EPSG:3857)**. Returns `[x, y, …]` in metres.
#[wasm_bindgen(js_name = "batchWgs84ToMercator")]
pub fn batch_wgs84_to_mercator(coords: &Float64Array) -> Float64Array {
    let len = coords.length() as usize;
    let mut buf = vec![0f64; len];
    coords.copy_to(&mut buf);

    const EARTH_RADIUS: f64 = 6378137.0;
    for i in (0..len).step_by(2) {
        let lng = buf[i];
        let lat = buf[i + 1];
        buf[i] = lng.to_radians() * EARTH_RADIUS;
        buf[i + 1] =
            ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * EARTH_RADIUS;
    }

    let result = Float64Array::new_with_length(len as u32);
    result.copy_from(&buf);
    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgs84_to_gcj02_in_china() {
        let (lng, lat) = wgs84_to_gcj02_single(116.404, 39.915);
        assert!((lng - 116.410).abs() < 0.01);
        assert!((lat - 39.917).abs() < 0.01);
    }

    #[test]
    fn test_out_of_china_passthrough() {
        let (lng, lat) = wgs84_to_gcj02_single(-73.9857, 40.7484);
        assert!((lng - (-73.9857)).abs() < 1e-10);
        assert!((lat - 40.7484).abs() < 1e-10);
    }

    #[test]
    fn test_roundtrip_gcj02() {
        let original_lng = 116.404;
        let original_lat = 39.915;
        let (g_lng, g_lat) = wgs84_to_gcj02_single(original_lng, original_lat);
        let (w_lng, w_lat) = gcj02_to_wgs84_single(g_lng, g_lat);
        assert!((w_lng - original_lng).abs() < 0.001);
        assert!((w_lat - original_lat).abs() < 0.001);
    }
}
