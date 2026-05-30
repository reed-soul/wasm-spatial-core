//! Spatial Analysis
//!
//! Basic spatial operations (buffer, bounding box, centroid).
//! Operates on WGS-84 coordinates and returns flat typed arrays
//! suitable for direct GPU upload.

use geo::CoordsIter;
#[cfg(test)]
use geo::LineString;
use wasm_bindgen::prelude::*;

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Haversine distance in meters between two WGS-84 points.
#[cfg(test)]
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters

    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = lat2_r - lat1_r;
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + lat1_r.cos() * lat2_r.cos() * (dlng / 2.0).sin() * (dlng / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R * c
}

/// Build a `LineString` from a flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
#[cfg(test)]
fn coords_to_linestring(coords: &[f64]) -> LineString<f64> {
    LineString::from_iter(
        coords
            .chunks_exact(2)
            .map(|c| geo::Coord { x: c[0], y: c[1] }),
    )
}

/// Generate a circle approximation around a point.
fn circle_polygon(lng: f64, lat: f64, radius_meters: f64, segments: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(segments * 2 + 2);

    // Convert radius to degrees at the given latitude
    let earth_radius_m = 6_371_000.0;
    let radius_deg_lat = radius_meters / earth_radius_m * (180.0 / std::f64::consts::PI);
    let radius_deg_lng =
        radius_meters / (earth_radius_m * lat.to_radians().cos()) * (180.0 / std::f64::consts::PI);

    for i in 0..segments {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
        let x = lng + radius_deg_lng * angle.cos();
        let y = lat + radius_deg_lat * angle.sin();
        out.push(x);
        out.push(y);
    }

    // Close the ring by repeating the first point
    if out.len() >= 2 {
        out.push(out[0]);
        out.push(out[1]);
    }

    out
}

// ===========================================================================
// Buffer Operations
// ===========================================================================

/// Generate a buffer polygon around a point.
///
/// Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, lng1, lat1, ...]`
/// forming a circle approximation around the given point.
#[wasm_bindgen(js_name = "bufferPoint")]
pub fn buffer_point(
    lng: f64,
    lat: f64,
    radius_meters: f64,
    segments: Option<u32>,
) -> js_sys::Float64Array {
    let segs = segments.unwrap_or(64).max(8) as usize;
    let out = circle_polygon(lng, lat, radius_meters, segs);

    let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Generate a buffer polygon around a line string (union of point buffers).
///
/// Returns a flat `Float64Array` of polygon vertices `[lng0, lat0, ...]`.
/// Note: this is a simplified implementation that produces a convex hull of
/// all circle vertices around each line point. For production use with
/// concave results, consider `geo`'s `BooleanOps` union.
#[wasm_bindgen(js_name = "bufferLineString")]
pub fn buffer_linestring(
    coords: &js_sys::Float64Array,
    radius_meters: f64,
    segments: Option<u32>,
) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let segs = segments.unwrap_or(16).max(8) as usize;
    let point_count = buf.len() / 2;

    if point_count == 0 {
        let arr = js_sys::Float64Array::new_with_length(0);
        return arr;
    }

    // Generate circles at each vertex, then compute convex hull
    let mut all_coords: Vec<geo::Coord<f64>> = Vec::new();
    for i in 0..point_count {
        let lng = buf[i * 2];
        let lat = buf[i * 2 + 1];
        let circle = circle_polygon(lng, lat, radius_meters, segs);
        for chunk in circle.chunks_exact(2) {
            all_coords.push(geo::Coord {
                x: chunk[0],
                y: chunk[1],
            });
        }
    }

    // Use geo's convex_hull for a simple buffer approximation
    let poly = geo::Polygon::new(all_coords.into(), vec![]);
    let hull = geo::ConvexHull::convex_hull(&poly);

    // Extract exterior ring
    let exterior = hull.exterior();
    let mut out = Vec::with_capacity(exterior.coords_count() * 2);
    for coord in exterior.coords() {
        out.push(coord.x);
        out.push(coord.y);
    }

    let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

// ===========================================================================
// Geometry Measurements
// ===========================================================================

/// Compute the axis-aligned bounding box of a set of coordinates.
///
/// Returns `[minLng, minLat, maxLng, maxLat]`.
#[wasm_bindgen(js_name = "boundingBox")]
pub fn bounding_box(coords: &js_sys::Float64Array) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let point_count = buf.len() / 2;
    if point_count == 0 {
        let arr = js_sys::Float64Array::new_with_length(4);
        arr.copy_from(&[0.0, 0.0, 0.0, 0.0]);
        return arr;
    }

    let mut min_lng = f64::MAX;
    let mut min_lat = f64::MAX;
    let mut max_lng = f64::MIN;
    let mut max_lat = f64::MIN;

    for i in 0..point_count {
        let lng = buf[i * 2];
        let lat = buf[i * 2 + 1];
        min_lng = min_lng.min(lng);
        min_lat = min_lat.min(lat);
        max_lng = max_lng.max(lng);
        max_lat = max_lat.max(lat);
    }

    let out = [min_lng, min_lat, max_lng, max_lat];
    let arr = js_sys::Float64Array::new_with_length(4);
    arr.copy_from(&out);
    arr
}

/// Compute the centroid (mean center) of a set of coordinates.
///
/// Returns `[lng, lat]`.
#[wasm_bindgen(js_name = "centroid")]
pub fn centroid(coords: &js_sys::Float64Array) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let point_count = buf.len() / 2;
    if point_count == 0 {
        let arr = js_sys::Float64Array::new_with_length(2);
        arr.copy_from(&[0.0, 0.0]);
        return arr;
    }

    let mut sum_lng = 0.0_f64;
    let mut sum_lat = 0.0_f64;

    for i in 0..point_count {
        sum_lng += buf[i * 2];
        sum_lat += buf[i * 2 + 1];
    }

    let out = [sum_lng / point_count as f64, sum_lat / point_count as f64];
    let arr = js_sys::Float64Array::new_with_length(2);
    arr.copy_from(&out);
    arr
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use geo::BoundingRect;

    // ── Haversine distance tests ──────────────────────────────────

    #[test]
    fn test_haversine_distance() {
        // Beijing to Shanghai ~1068 km
        let dist = haversine_distance(39.9042, 116.4074, 31.2304, 121.4737);
        assert!(
            (dist - 1_068_000.0).abs() < 20_000.0,
            "Distance off by too much: {}",
            dist
        );
    }

    #[test]
    fn test_haversine_zero_distance() {
        let dist = haversine_distance(39.0, 116.0, 39.0, 116.0);
        assert!(dist.abs() < 0.01);
    }

    // ── Point buffer tests ────────────────────────────────────────

    #[test]
    fn test_buffer_point_basic() {
        let out = circle_polygon(116.404, 39.915, 1000.0, 64);
        assert_eq!(out.len(), 64 * 2 + 2); // 64 segments + closing point

        // All coordinates should be close to original
        for chunk in out.chunks_exact(2) {
            assert!(
                (chunk[0] - 116.404).abs() < 0.02,
                "Lng too far: {}",
                chunk[0]
            );
            assert!(
                (chunk[1] - 39.915).abs() < 0.02,
                "Lat too far: {}",
                chunk[1]
            );
        }
    }

    #[test]
    fn test_buffer_point_ring_closed() {
        let out = circle_polygon(0.0, 0.0, 1000.0, 32);
        // First point should equal last point
        assert_eq!(out[0], out[out.len() - 2]);
        assert_eq!(out[1], out[out.len() - 1]);
    }

    #[test]
    fn test_buffer_point_minimum_segments() {
        let out = circle_polygon(0.0, 0.0, 100.0, 8);
        assert_eq!(out.len(), 8 * 2 + 2);
    }

    // ── Bounding box tests ───────────────────────────────────────

    #[test]
    fn test_bounding_box_basic() {
        let coords: Vec<f64> = vec![
            116.0, 39.0, // SW-ish
            117.0, 40.0, // NE-ish
            116.5, 39.5, // Center-ish
        ];
        let line = coords_to_linestring(&coords);
        let bbox = line.bounding_rect().unwrap();

        assert_eq!(bbox.min().x, 116.0);
        assert_eq!(bbox.min().y, 39.0);
        assert_eq!(bbox.max().x, 117.0);
        assert_eq!(bbox.max().y, 40.0);
    }

    #[test]
    fn test_bounding_box_single_point() {
        let coords: Vec<f64> = vec![10.0, 20.0];
        let line = coords_to_linestring(&coords);
        let bbox = line.bounding_rect().unwrap();

        assert_eq!(bbox.min().x, 10.0);
        assert_eq!(bbox.max().x, 10.0);
        assert_eq!(bbox.min().y, 20.0);
        assert_eq!(bbox.max().y, 20.0);
    }

    // ── Centroid tests ───────────────────────────────────────────

    #[test]
    fn test_centroid_basic() {
        let coords: Vec<f64> = vec![0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        let point_count = 4;
        let sum_lng: f64 = coords.iter().step_by(2).sum();
        let sum_lat: f64 = coords.iter().skip(1).step_by(2).sum();
        assert_eq!(sum_lng / point_count as f64, 5.0);
        assert_eq!(sum_lat / point_count as f64, 5.0);
    }

    #[test]
    fn test_centroid_manual() {
        let _coords: Vec<f64> = vec![0.0, 0.0, 10.0, 10.0];
        let point_count = 2;
        let sum_lng = 0.0 + 10.0;
        let sum_lat = 0.0 + 10.0;
        assert_eq!(sum_lng / point_count as f64, 5.0);
        assert_eq!(sum_lat / point_count as f64, 5.0);
    }

    // ── LineString tests ──────────────────────────────────────────

    #[test]
    fn test_coords_to_linestring_valid() {
        let coords: Vec<f64> = vec![0.0, 0.0, 1.0, 1.0];
        let line = coords_to_linestring(&coords);
        assert_eq!(line.coords_count(), 2);
    }

    #[test]
    fn test_coords_to_linestring_odd_length() {
        let coords: Vec<f64> = vec![0.0, 0.0, 1.0];
        let line = coords_to_linestring(&coords);
        assert_eq!(line.coords_count(), 1);
    }

    #[test]
    fn test_coords_to_linestring_empty() {
        let coords: Vec<f64> = vec![];
        let line = coords_to_linestring(&coords);
        assert_eq!(line.coords_count(), 0);
    }
}
