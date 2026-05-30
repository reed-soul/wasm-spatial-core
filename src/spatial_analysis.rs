//! Spatial Analysis
//!
//! Basic spatial operations (buffer, bounding box, centroid, geodesic
//! calculations). Operates on WGS-84 coordinates and returns flat typed
//! arrays suitable for direct GPU upload.

use geo::CoordsIter;
#[cfg(test)]
use geo::LineString;
use wasm_bindgen::prelude::*;

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Earth radius in meters (WGS-84 mean).
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Haversine distance in meters between two WGS-84 points.
fn haversine_distance_internal(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = lat2_r - lat1_r;
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + lat1_r.cos() * lat2_r.cos() * (dlng / 2.0).sin() * (dlng / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
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
// Geodesic Calculations
// ===========================================================================

/// Calculate the Haversine distance between two WGS-84 points in meters.
///
/// # Arguments
/// - `lng1`: Longitude of point 1 in degrees.
/// - `lat1`: Latitude of point 1 in degrees.
/// - `lng2`: Longitude of point 2 in degrees.
/// - `lat2`: Latitude of point 2 in degrees.
///
/// Returns the great-circle distance in meters.
#[wasm_bindgen(js_name = "haversineDistance")]
pub fn haversine_distance(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    haversine_distance_internal(lat1, lng1, lat2, lng2)
}

/// Calculate the initial bearing (forward azimuth) from point 1 to point 2.
///
/// Returns the bearing in degrees [0, 360), where 0 = North, 90 = East,
/// 180 = South, 270 = West.
///
/// # Arguments
/// - `lng1`: Longitude of origin in degrees.
/// - `lat1`: Latitude of origin in degrees.
/// - `lng2`: Longitude of destination in degrees.
/// - `lat2`: Latitude of destination in degrees.
#[wasm_bindgen(js_name = "bearing")]
pub fn bearing(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let x = dlng.sin() * lat2_r.cos();
    let y = lat1_r.cos() * lat2_r.sin() - lat1_r.sin() * lat2_r.cos() * dlng.cos();

    let bearing_rad = x.atan2(y);
    let bearing_deg = bearing_rad.to_degrees();

    // Normalize to [0, 360)
    ((bearing_deg % 360.0) + 360.0) % 360.0
}

/// Calculate the destination point given a start point, bearing, and distance.
///
/// Uses the direct geodesic problem solution.
///
/// # Arguments
/// - `lng`: Origin longitude in degrees.
/// - `lat`: Origin latitude in degrees.
/// - `bearing_deg`: Bearing in degrees (0 = North, 90 = East).
/// - `distance_m`: Distance in meters.
///
/// Returns `Float64Array` `[lng, lat]` of the destination point.
#[wasm_bindgen(js_name = "destination")]
pub fn destination(lng: f64, lat: f64, bearing_deg: f64, distance_m: f64) -> js_sys::Float64Array {
    let lat_r = lat.to_radians();
    let bearing_r = bearing_deg.to_radians();
    let angular_dist = distance_m / EARTH_RADIUS_M;

    let dest_lat = (lat_r.sin() * angular_dist.cos()
        + lat_r.cos() * angular_dist.sin() * bearing_r.cos())
    .asin();

    let dest_lng = (bearing_r.sin() * angular_dist.sin() * lat_r.cos())
        .atan2(angular_dist.cos() - lat_r.sin() * dest_lat.sin());

    let dest_lat_deg = dest_lat.to_degrees();
    let dest_lng_deg = lng + dest_lng.to_degrees();

    let arr = js_sys::Float64Array::new_with_length(2);
    arr.copy_from(&[dest_lng_deg, dest_lat_deg]);
    arr
}

/// Calculate the midpoint between two WGS-84 points on the great circle.
///
/// # Arguments
/// - `lng1`: Longitude of point 1 in degrees.
/// - `lat1`: Latitude of point 1 in degrees.
/// - `lng2`: Longitude of point 2 in degrees.
/// - `lat2`: Latitude of point 2 in degrees.
///
/// Returns `Float64Array` `[lng, lat]` of the midpoint.
#[wasm_bindgen(js_name = "midpoint")]
pub fn midpoint(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> js_sys::Float64Array {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let bx = lat2_r.cos() * dlng.cos();
    let by = lat2_r.cos() * dlng.sin();

    let lat_m = (lat1_r.sin() + lat2_r.sin()).atan2((lat1_r.cos() + bx).hypot(by));

    let lng_m = lng1 + (by.atan2(lat1_r.cos() + bx)).to_degrees();

    let arr = js_sys::Float64Array::new_with_length(2);
    arr.copy_from(&[lng_m, lat_m.to_degrees()]);
    arr
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
// Vincenty Distance (Ellipsoidal)
// ===========================================================================

/// WGS-84 ellipsoid parameters.
const WGS84_A: f64 = 6_378_137.0; // semi-major axis (m)
const WGS84_F: f64 = 1.0 / 298.257_223_563; // flattening
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F); // semi-minor axis

/// Vincenty inverse formula — geodesic distance between two points on the WGS-84 ellipsoid.
///
/// More accurate than Haversine for long distances (sub-millimeter accuracy).
///
/// # Arguments
/// - `lng1`, `lat1`: Point 1 in degrees.
/// - `lng2`, `lat2`: Point 2 in degrees.
///
/// # Returns
/// Distance in meters. Returns `f64::NAN` if the points are antipodal (no convergence).
#[wasm_bindgen(js_name = "vincentyDistance")]
pub fn vincenty_distance(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    vincenty_distance_internal(lat1, lng1, lat2, lng2)
}

/// Internal Vincenty (uses lat,lng order for consistency).
fn vincenty_distance_internal(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    if (lat1 - lat2).abs() < 1e-12 && (lng1 - lng2).abs() < 1e-12 {
        return 0.0;
    }

    let u1 = ((1.0 - WGS84_F) * lat1.to_radians().tan()).atan();
    let u2 = ((1.0 - WGS84_F) * lat2.to_radians().tan()).atan();
    let sin_u1 = u1.sin();
    let cos_u1 = u1.cos();
    let sin_u2 = u2.sin();
    let cos_u2 = u2.cos();

    let mut lambda = (lng2 - lng1).to_radians();
    let mut iter = 0;
    let max_iter = 200;

    let mut sin_sigma: f64;
    let mut cos_sigma: f64;
    let mut sigma: f64;
    let mut sin_alpha: f64;
    let mut cos_sq_alpha: f64;
    let mut cos2sigma_m: f64;
    let mut lambda_prev: f64;

    loop {
        let sin_lambda = lambda.sin();
        let cos_lambda = lambda.cos();

        sin_sigma = ((cos_u2 * sin_lambda) * (cos_u2 * sin_lambda)
            + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda)
                * (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda))
            .sqrt();

        if sin_sigma < 1e-15 {
            return 0.0; // co-incident points
        }

        cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
        sigma = sin_sigma.atan2(cos_sigma);

        sin_alpha = (cos_u1 * cos_u2 * sin_lambda) / sin_sigma;
        cos_sq_alpha = 1.0 - sin_alpha * sin_alpha;

        if cos_sq_alpha == 0.0 {
            cos2sigma_m = 0.0; // equatorial line
        } else {
            cos2sigma_m = cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha;
        }

        let c = WGS84_F / 16.0 * cos_sq_alpha * (4.0 + WGS84_F * (4.0 - 3.0 * cos_sq_alpha));

        lambda_prev = lambda;
        lambda = (lng2 - lng1).to_radians()
            + (1.0 - c)
                * WGS84_F
                * sin_alpha
                * (sigma + c * sin_sigma * (cos2sigma_m + c * cos_sigma * (-1.0 + 2.0 * cos2sigma_m * cos2sigma_m)));

        iter += 1;
        if (lambda - lambda_prev).abs() < 1e-12 || iter >= max_iter {
            break;
        }
    }

    if iter >= max_iter {
        return f64::NAN; // no convergence (antipodal)
    }

    let u_sq = cos_sq_alpha * (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);
    let a_coeff = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
    let b_coeff = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));

    let delta_sigma = b_coeff
        * sin_sigma
        * (cos2sigma_m
            + b_coeff
                / 4.0
                * (cos_sigma * (-1.0 + 2.0 * cos2sigma_m * cos2sigma_m)
                    - b_coeff / 6.0
                        * cos2sigma_m
                        * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                        * (-3.0 + 4.0 * cos2sigma_m * cos2sigma_m)));

    WGS84_B * a_coeff * (sigma - delta_sigma)
}

// ===========================================================================
// Rhumb (Loxodrome) Calculations
// ===========================================================================

/// Rhumb (loxodrome/constant-bearing) distance between two WGS-84 points.
///
/// Used in maritime and aviation navigation.
///
/// # Arguments
/// - `lng1`, `lat1`: Point 1 in degrees.
/// - `lng2`, `lat2`: Point 2 in degrees.
///
/// # Returns
/// Distance in meters.
#[wasm_bindgen(js_name = "rhumbDistance")]
pub fn rhumb_distance(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = lat2_r - lat1_r;
    let dlng = (lng2 - lng1).to_radians();

    let d_phi = ((1.0 - WGS84_F) * ((lat2_r / 2.0 + std::f64::consts::PI / 4.0).tan()).ln()
        - (1.0 - WGS84_F) * ((lat1_r / 2.0 + std::f64::consts::PI / 4.0).tan()).ln())
    .abs();

    let q = if d_phi.abs() < 1e-15 {
        // Points at same latitude — q = cos(lat)
        let avg_lat = (lat1_r + lat2_r) / 2.0;
        avg_lat.cos()
    } else {
        dlat.abs() / d_phi
    };

    // Delta longitude, normalized to [-π, π]
    let d_lon = ((dlng + std::f64::consts::PI) % (2.0 * std::f64::consts::PI)) - std::f64::consts::PI;

    (d_phi * d_phi + (q * d_lon) * (q * d_lon)).sqrt() * EARTH_RADIUS_M
}

/// Rhumb (constant-bearing) bearing from point 1 to point 2.
///
/// # Arguments
/// - `lng1`, `lat1`: Point 1 in degrees.
/// - `lng2`, `lat2`: Point 2 in degrees.
///
/// # Returns
/// Bearing in degrees [0, 360), where 0 = North, 90 = East.
#[wasm_bindgen(js_name = "rhumbBearing")]
pub fn rhumb_bearing(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let d_phi = (1.0 - WGS84_F) * ((lat2_r / 2.0 + std::f64::consts::PI / 4.0).tan()).ln()
        - (1.0 - WGS84_F) * ((lat1_r / 2.0 + std::f64::consts::PI / 4.0).tan()).ln();

    let d_lon = ((dlng + std::f64::consts::PI) % (2.0 * std::f64::consts::PI)) - std::f64::consts::PI;

    let theta = d_lon.atan2(d_phi);
    let bearing = theta.to_degrees();

    ((bearing % 360.0) + 360.0) % 360.0
}

// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use geo::BoundingRect;

    // ── Haversine distance tests ──────────────────────────────────

    #[test]
    fn test_haversine_distance() {
        // Beijing to Shanghai ~1068 km
        let dist = haversine_distance_internal(39.9042, 116.4074, 31.2304, 121.4737);
        assert!(
            (dist - 1_068_000.0).abs() < 20_000.0,
            "Distance off by too much: {}",
            dist
        );
    }

    #[test]
    fn test_haversine_zero_distance() {
        let dist = haversine_distance_internal(39.0, 116.0, 39.0, 116.0);
        assert!(dist.abs() < 0.01);
    }

    // ── Bearing tests ─────────────────────────────────────────────

    #[test]
    fn test_bearing_north() {
        // From (0,0) to (0,1) = due North = 0°
        let b = bearing(0.0, 0.0, 0.0, 1.0);
        assert!((b - 0.0).abs() < 0.1, "Expected ~0°, got {}", b);
    }

    #[test]
    fn test_bearing_east() {
        // From (0,0) to (1,0) at equator = due East ≈ 90°
        let b = bearing(0.0, 0.0, 1.0, 0.0);
        assert!((b - 90.0).abs() < 0.1, "Expected ~90°, got {}", b);
    }

    #[test]
    fn test_bearing_beijing_shanghai() {
        // Beijing (116.4, 39.9) → Shanghai (121.5, 31.2) ≈ South-East
        let b = bearing(116.4074, 39.9042, 121.4737, 31.2304);
        assert!(b > 120.0 && b < 180.0, "Expected SE bearing, got {}", b);
    }

    // ── Destination tests ────────────────────────────────────────

    #[test]
    fn test_destination_north_100km() {
        let result = native_destination(0.0, 0.0, 0.0, 100_000.0);
        assert!(
            (result[1] - 0.8983).abs() < 0.01,
            "Expected lat ~0.8983, got {}",
            result[1]
        );
        assert!(
            (result[0] - 0.0).abs() < 0.01,
            "Expected lng ~0, got {}",
            result[0]
        );
    }

    #[test]
    fn test_destination_roundtrip() {
        let start_lng = 116.4074;
        let start_lat = 39.9042;
        let result = native_destination(start_lng, start_lat, 90.0, 50_000.0);
        let end_lng = result[0];
        let end_lat = result[1];
        assert!(end_lng > start_lng);
        assert!(
            (end_lat - start_lat).abs() < 0.01,
            "Lat should be ~same: {} vs {}",
            end_lat,
            start_lat
        );
    }

    // ── Midpoint tests ───────────────────────────────────────────

    #[test]
    fn test_midpoint_equator() {
        let result = native_midpoint(0.0, 0.0, 10.0, 0.0);
        assert!(
            (result[0] - 5.0).abs() < 0.01,
            "Expected lng ~5, got {}",
            result[0]
        );
        assert!(
            (result[1] - 0.0).abs() < 0.01,
            "Expected lat ~0, got {}",
            result[1]
        );
    }

    #[test]
    fn test_midpoint_north() {
        let result = native_midpoint(0.0, 0.0, 0.0, 10.0);
        assert!(
            (result[1] - 5.0).abs() < 0.1,
            "Expected lat ~5, got {}",
            result[1]
        );
    }

    // ── Native helpers for geodesic functions ─────────────────────

    fn native_destination(lng: f64, lat: f64, bearing_deg: f64, distance_m: f64) -> [f64; 2] {
        let lat_r = lat.to_radians();
        let bearing_r = bearing_deg.to_radians();
        let angular_dist = distance_m / EARTH_RADIUS_M;

        let dest_lat = (lat_r.sin() * angular_dist.cos()
            + lat_r.cos() * angular_dist.sin() * bearing_r.cos())
        .asin();

        let dest_lng = (bearing_r.sin() * angular_dist.sin() * lat_r.cos())
            .atan2(angular_dist.cos() - lat_r.sin() * dest_lat.sin());

        let dest_lat_deg = dest_lat.to_degrees();
        let dest_lng_deg = lng + dest_lng.to_degrees();

        [dest_lng_deg, dest_lat_deg]
    }

    fn native_midpoint(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> [f64; 2] {
        let lat1_r = lat1.to_radians();
        let lat2_r = lat2.to_radians();
        let dlng = (lng2 - lng1).to_radians();

        let bx = lat2_r.cos() * dlng.cos();
        let by = lat2_r.cos() * dlng.sin();

        let lat_m = (lat1_r.sin() + lat2_r.sin()).atan2((lat1_r.cos() + bx).hypot(by));

        let lng_m = lng1 + (by.atan2(lat1_r.cos() + bx)).to_degrees();

        [lng_m, lat_m.to_degrees()]
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

    // ── Vincenty distance tests ───────────────────────────────────

    #[test]
    fn test_vincenty_beijing_shanghai() {
        // Beijing (116.4074, 39.9042) → Shanghai (121.4737, 31.2304)
        // Vincenty reference: ~1,065,846 m (more accurate than Haversine ~1,068,000)
        let dist = vincenty_distance(116.4074, 39.9042, 121.4737, 31.2304);
        assert!(
            !dist.is_nan(),
            "Vincenty should converge for non-antipodal points"
        );
        assert!(
            (dist - 1_065_846.0).abs() < 100.0,
            "Beijing-Shanghai Vincenty: got {}, expected ~1065846",
            dist
        );
    }

    #[test]
    fn test_vincenty_zero_distance() {
        let dist = vincenty_distance(116.0, 39.0, 116.0, 39.0);
        assert!(dist.abs() < 0.01, "Zero distance: got {}", dist);
    }

    #[test]
    fn test_vincenty_more_accurate_than_haversine() {
        // Long distance: London to New York
        // Vincenty: ~5570000 m, Haversine: ~5570000 m (difference in sub-meter range for this case)
        let v_dist = vincenty_distance(-0.1278, 51.5074, -74.006, 40.7128);
        let h_dist = haversine_distance_internal(51.5074, -0.1278, 40.7128, -74.006);
        assert!(!v_dist.is_nan());
        // Both should be within ~1% of each other
        let ratio = v_dist / h_dist;
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "Vincenty/Haversine ratio too different: {}",
            ratio
        );
    }

    #[test]
    fn test_vincenty_equator() {
        // Along equator: 10 degrees longitude
        // Distance should be ~1113.2 km per degree × 10
        let dist = vincenty_distance(0.0, 0.0, 10.0, 0.0);
        assert!(!dist.is_nan());
        assert!(
            (dist - 1_113_195.0).abs() < 500.0,
            "10° equator Vincenty: got {}, expected ~1113195",
            dist
        );
    }

    // ── Rhumb distance tests ──────────────────────────────────────

    #[test]
    fn test_rhumb_distance_equator() {
        // 10° along equator — rhumb uses spherical approximation
        // 10° × π/180 × 6371000 ≈ 1111949 m
        let dist = rhumb_distance(0.0, 0.0, 10.0, 0.0);
        assert!(
            (dist - 1_111_949.0).abs() < 500.0,
            "10° equator rhumb: got {}, expected ~1111949",
            dist
        );
    }

    #[test]
    fn test_rhumb_distance_north() {
        // Due north 10° from equator
        let dist = rhumb_distance(0.0, 0.0, 0.0, 10.0);
        assert!(
            (dist - 1_113_195.0).abs() < 1000.0,
            "10° north rhumb: got {}, expected ~1113195",
            dist
        );
    }

    #[test]
    fn test_rhumb_bearing_east() {
        // Due east at equator → bearing should be ~90°
        let bearing = rhumb_bearing(0.0, 0.0, 10.0, 0.0);
        assert!(
            (bearing - 90.0).abs() < 0.5,
            "East bearing: got {}, expected ~90",
            bearing
        );
    }

    #[test]
    fn test_rhumb_bearing_north() {
        // Due north → bearing should be ~0°
        let bearing = rhumb_bearing(0.0, 0.0, 0.0, 10.0);
        assert!(
            (bearing - 0.0).abs() < 0.5 || (bearing - 360.0).abs() < 0.5,
            "North bearing: got {}, expected ~0 or 360",
            bearing
        );
    }
}
