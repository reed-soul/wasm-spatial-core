//! Local ENU (East-North-Up) tangent frame for site-scale precision (Wave 2.6).
//!
//! Geodetic coordinates are handled in f64; rendering offsets relative to the
//! anchor are exposed as f32 for GPU-friendly buffers.

use wasm_bindgen::prelude::*;

use crate::cesium_adapter::wgs84_to_cartesian3_single;
use crate::errors::SpatialError;

const WGS84_A: f64 = 6378137.0;
const WGS84_B: f64 = 6_356_752.314_245_179;
const WGS84_A_SQ: f64 = WGS84_A * WGS84_A;
const WGS84_B_SQ: f64 = WGS84_B * WGS84_B;
const WGS84_E_SQ: f64 = 1.0 - WGS84_B_SQ / WGS84_A_SQ;
const WGS84_EP_SQ: f64 = (WGS84_A_SQ - WGS84_B_SQ) / WGS84_B_SQ;

/// Local East-North-Up frame anchored at a WGS84 geodetic point.
#[derive(Debug, Clone, Copy)]
pub struct EnuFrame {
    pub anchor_lng: f64,
    pub anchor_lat: f64,
    pub anchor_alt: f64,
    pub anchor_ecef: [f64; 3],
    sin_lat: f64,
    cos_lat: f64,
    sin_lon: f64,
    cos_lon: f64,
}

impl EnuFrame {
    /// Create an ENU frame from a WGS84 anchor `[longitude°, latitude°, altitude m]`.
    pub fn from_anchor(lng: f64, lat: f64, alt: f64) -> Self {
        let (x, y, z) = wgs84_to_cartesian3_single(lng, lat, alt);
        let lat_rad = lat.to_radians();
        let lon_rad = lng.to_radians();
        Self {
            anchor_lng: lng,
            anchor_lat: lat,
            anchor_alt: alt,
            anchor_ecef: [x, y, z],
            sin_lat: lat_rad.sin(),
            cos_lat: lat_rad.cos(),
            sin_lon: lon_rad.sin(),
            cos_lon: lon_rad.cos(),
        }
    }

    /// Convert a WGS84 geodetic point to ENU meters relative to the anchor.
    pub fn wgs84_to_enu(&self, lng: f64, lat: f64, alt: f64) -> [f64; 3] {
        let (x, y, z) = wgs84_to_cartesian3_single(lng, lat, alt);
        self.ecef_delta_to_enu(
            x - self.anchor_ecef[0],
            y - self.anchor_ecef[1],
            z - self.anchor_ecef[2],
        )
    }

    /// Convert ENU meters relative to the anchor back to WGS84 geodetic degrees.
    pub fn enu_to_wgs84(&self, east: f64, north: f64, up: f64) -> [f64; 3] {
        let (dx, dy, dz) = self.enu_delta_to_ecef(east, north, up);
        let x = self.anchor_ecef[0] + dx;
        let y = self.anchor_ecef[1] + dy;
        let z = self.anchor_ecef[2] + dz;
        let (lng, lat, alt) = cartesian3_to_wgs84(x, y, z);
        [lng, lat, alt]
    }

    fn ecef_delta_to_enu(&self, dx: f64, dy: f64, dz: f64) -> [f64; 3] {
        let east = -self.sin_lon * dx + self.cos_lon * dy;
        let north = -self.sin_lat * self.cos_lon * dx - self.sin_lat * self.sin_lon * dy
            + self.cos_lat * dz;
        let up =
            self.cos_lat * self.cos_lon * dx + self.cos_lat * self.sin_lon * dy + self.sin_lat * dz;
        [east, north, up]
    }

    fn enu_delta_to_ecef(&self, east: f64, north: f64, up: f64) -> (f64, f64, f64) {
        let dx = -self.sin_lon * east - self.sin_lat * self.cos_lon * north
            + self.cos_lat * self.cos_lon * up;
        let dy = self.cos_lon * east - self.sin_lat * self.sin_lon * north
            + self.cos_lat * self.sin_lon * up;
        let dz = self.cos_lat * north + self.sin_lat * up;
        (dx, dy, dz)
    }
}

/// Batch WGS84 `[lng, lat, alt, ...]` → ENU `[e, n, u, ...]` (f64).
pub fn batch_wgs84_to_enu_core(coords: &[f64], frame: &EnuFrame) -> Vec<f64> {
    assert!(
        coords.len().is_multiple_of(3),
        "coords length must be a multiple of 3"
    );
    let mut out = Vec::with_capacity(coords.len());
    for triple in coords.chunks_exact(3) {
        let enu = frame.wgs84_to_enu(triple[0], triple[1], triple[2]);
        out.extend_from_slice(&enu);
    }
    out
}

/// Batch ENU `[e, n, u, ...]` → WGS84 `[lng, lat, alt, ...]` (f64).
pub fn batch_enu_to_wgs84_core(coords: &[f64], frame: &EnuFrame) -> Vec<f64> {
    assert!(
        coords.len().is_multiple_of(3),
        "coords length must be a multiple of 3"
    );
    let mut out = Vec::with_capacity(coords.len());
    for triple in coords.chunks_exact(3) {
        let wgs = frame.enu_to_wgs84(triple[0], triple[1], triple[2]);
        out.extend_from_slice(&wgs);
    }
    out
}

/// Batch WGS84 → ENU as f32 rendering offsets relative to anchor.
pub fn batch_wgs84_to_enu_f32_core(coords: &[f64], frame: &EnuFrame) -> Vec<f32> {
    batch_wgs84_to_enu_core(coords, frame)
        .into_iter()
        .map(|v| v as f32)
        .collect()
}

fn cartesian3_to_wgs84(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);
    let theta = (z * WGS84_A).atan2(p * WGS84_B);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();
    let lat = (z + WGS84_EP_SQ * WGS84_B * sin_theta.powi(3))
        .atan2(p - WGS84_E_SQ * WGS84_A * cos_theta.powi(3));
    let sin_lat = lat.sin();
    let n = WGS84_A / (1.0 - WGS84_E_SQ * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;
    (lon.to_degrees(), lat.to_degrees(), alt)
}

// ===========================================================================
// WASM API
// ===========================================================================

/// WASM handle for a local ENU coordinate frame.
#[wasm_bindgen]
pub struct WasmEnuFrame {
    inner: EnuFrame,
}

#[wasm_bindgen(js_name = "createEnuFrame")]
pub fn create_enu_frame(anchor: &js_sys::Float64Array) -> Result<WasmEnuFrame, JsValue> {
    if anchor.length() < 3 {
        return Err(SpatialError::InvalidInput
            .with_detail("anchor must be [longitude, latitude, altitude]")
            .into());
    }
    Ok(WasmEnuFrame {
        inner: EnuFrame::from_anchor(
            anchor.get_index(0),
            anchor.get_index(1),
            anchor.get_index(2),
        ),
    })
}

#[wasm_bindgen]
impl WasmEnuFrame {
    #[wasm_bindgen(getter, js_name = "anchorLng")]
    pub fn anchor_lng(&self) -> f64 {
        self.inner.anchor_lng
    }

    #[wasm_bindgen(getter, js_name = "anchorLat")]
    pub fn anchor_lat(&self) -> f64 {
        self.inner.anchor_lat
    }

    #[wasm_bindgen(getter, js_name = "anchorAlt")]
    pub fn anchor_alt(&self) -> f64 {
        self.inner.anchor_alt
    }

    /// Convert flat WGS84 `[lng, lat, alt, ...]` to ENU `[e, n, u, ...]` (f64).
    #[wasm_bindgen(js_name = "wgs84ToEnu")]
    pub fn wgs84_to_enu(&self, coords: &[f64]) -> Result<js_sys::Float64Array, JsValue> {
        if !coords.len().is_multiple_of(3) {
            return Err(SpatialError::InvalidInput
                .with_detail("coords length must be a multiple of 3")
                .into());
        }
        let out = batch_wgs84_to_enu_core(coords, &self.inner);
        Ok(js_sys::Float64Array::from(&out[..]))
    }

    /// Convert flat ENU `[e, n, u, ...]` to WGS84 `[lng, lat, alt, ...]` (f64).
    #[wasm_bindgen(js_name = "enuToWgs84")]
    pub fn enu_to_wgs84(&self, coords: &[f64]) -> Result<js_sys::Float64Array, JsValue> {
        if !coords.len().is_multiple_of(3) {
            return Err(SpatialError::InvalidInput
                .with_detail("coords length must be a multiple of 3")
                .into());
        }
        let out = batch_enu_to_wgs84_core(coords, &self.inner);
        Ok(js_sys::Float64Array::from(&out[..]))
    }

    /// Convert flat WGS84 to ENU f32 rendering offsets `[e, n, u, ...]`.
    #[wasm_bindgen(js_name = "wgs84ToEnuF32")]
    pub fn wgs84_to_enu_f32(&self, coords: &[f64]) -> Result<js_sys::Float32Array, JsValue> {
        if !coords.len().is_multiple_of(3) {
            return Err(SpatialError::InvalidInput
                .with_detail("coords length must be a multiple of 3")
                .into());
        }
        let out = batch_wgs84_to_enu_f32_core(coords, &self.inner);
        Ok(js_sys::Float32Array::from(&out[..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enu_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[test]
    fn test_anchor_maps_to_origin() {
        let frame = EnuFrame::from_anchor(116.391, 39.907, 50.0);
        let enu = frame.wgs84_to_enu(116.391, 39.907, 50.0);
        assert!(enu[0].abs() < 1e-6);
        assert!(enu[1].abs() < 1e-6);
        assert!(enu[2].abs() < 1e-6);
    }

    #[test]
    fn test_roundtrip_at_1km_east() {
        let frame = EnuFrame::from_anchor(116.391, 39.907, 50.0);
        let enu = [1000.0, 0.0, 0.0];
        let wgs = frame.enu_to_wgs84(enu[0], enu[1], enu[2]);
        let back = frame.wgs84_to_enu(wgs[0], wgs[1], wgs[2]);
        let err = enu_distance(enu, back);
        assert!(
            err < 1e-3,
            "round-trip error {err} m exceeds 1 mm tolerance at 1 km"
        );
    }

    #[test]
    fn test_roundtrip_at_1km_north_and_up() {
        let frame = EnuFrame::from_anchor(116.391, 39.907, 50.0);
        for enu in [[0.0, 1000.0, 0.0], [0.0, 0.0, 100.0], [700.0, 700.0, 50.0]] {
            let wgs = frame.enu_to_wgs84(enu[0], enu[1], enu[2]);
            let back = frame.wgs84_to_enu(wgs[0], wgs[1], wgs[2]);
            let err = enu_distance(enu, back);
            assert!(err < 1e-3, "round-trip error {err} m for {enu:?}");
        }
    }

    #[test]
    fn test_batch_wgs84_to_enu() {
        let frame = EnuFrame::from_anchor(0.0, 0.0, 0.0);
        let coords = [0.0, 0.0, 0.0, 0.001, 0.0, 10.0];
        let out = batch_wgs84_to_enu_core(&coords, &frame);
        assert_eq!(out.len(), 6);
        assert!(out[0].abs() < 1e-3 && out[1].abs() < 1e-3);
    }

    #[test]
    fn test_f32_offsets() {
        let frame = EnuFrame::from_anchor(116.0, 39.0, 0.0);
        let coords = [116.0, 39.0, 0.0];
        let f32 = batch_wgs84_to_enu_f32_core(&coords, &frame);
        assert_eq!(f32.len(), 3);
        assert!(f32[0].abs() < 1e-4);
    }
}
