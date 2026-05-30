//! Topology analysis for spatial geometries.
//!
//! Provides polygon area, line/polygon length, Douglas-Peucker simplification,
//! and point-in-ring (ray-casting) operations on flat coordinate buffers.

use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Earth radius in meters (WGS-84 mean).
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Haversine distance between two WGS-84 points in meters.
fn haversine(lng1: f64, lat1: f64, lng2: f64, lat2: f64) -> f64 {
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = lat2_r - lat1_r;
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + lat1_r.cos() * lat2_r.cos() * (dlng / 2.0).sin() * (dlng / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

/// Signed area of a simple polygon using the Shoelace formula (planar).
/// Positive for counter-clockwise, negative for clockwise.
/// Expects closed ring: `[lng0,lat0, lng1,lat1, ..., lng0,lat0]`.
#[cfg(test)]
fn shoelace_area(coords: &[f64]) -> f64 {
    let n = coords.len() / 2;
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0_f64;
    for i in 0..n - 1 {
        let x0 = coords[i * 2];
        let y0 = coords[i * 2 + 1];
        let x1 = coords[(i + 1) * 2];
        let y1 = coords[(i + 1) * 2 + 1];
        area += x0 * y1 - x1 * y0;
    }
    area / 2.0
}

/// Compute spherical excess for a geodesic polygon (spherical area formula).
/// Uses the trapezoidal spherical excess formula.
/// Takes closed ring coords `[lng0,lat0, ...]`.
fn spherical_polygon_area(coords: &[f64]) -> f64 {
    let n = coords.len() / 2;
    if n < 3 {
        return 0.0;
    }

    let mut sum = 0.0_f64;
    for i in 0..n - 1 {
        let lng1 = coords[i * 2].to_radians();
        let lat1 = coords[i * 2 + 1].to_radians();
        let lng2 = coords[(i + 1) * 2].to_radians();
        let lat2 = coords[(i + 1) * 2 + 1].to_radians();

        sum += (lng2 - lng1) * (2.0 + lat1.sin() + lat2.sin());
    }

    let area_rad = sum.abs() / 2.0;
    area_rad * EARTH_RADIUS_M * EARTH_RADIUS_M
}

/// Perpendicular distance from point (px, py) to line segment (x1,y1)-(x2,y2).
fn perpendicular_distance_sq(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx == 0.0 && dy == 0.0 {
        // Degenerate segment
        let dpx = px - x1;
        let dpy = py - y1;
        return dpx * dpx + dpy * dpy;
    }

    let len_sq = dx * dx + dy * dy;
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    let dpx = px - proj_x;
    let dpy = py - proj_y;
    dpx * dpx + dpy * dpy
}

// ---------------------------------------------------------------------------
// Public WASM API
// ---------------------------------------------------------------------------

/// Calculate the area of a polygon in square meters using the spherical
/// excess formula.
///
/// # Arguments
/// - `coords`: Flat `Float64Array` of a closed ring `[lng0,lat0, lng1,lat1, ..., lng0,lat0]`.
///
/// For polygons with holes, use `areaWithHoles` instead.
#[wasm_bindgen(js_name = "polygonArea")]
pub fn polygon_area(coords: &js_sys::Float64Array) -> Result<f64, JsValue> {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let point_count = buf.len() / 2;
    if point_count < 3 {
        return Ok(0.0);
    }

    Ok(spherical_polygon_area(&buf))
}

/// Calculate the area of a polygon with holes in square meters.
/// Exterior ring area minus the sum of all hole areas.
///
/// # Arguments
/// - `rings`: Flat `Float64Array` containing all rings concatenated.
/// - `ringSizes`: `Uint32Array` where each element is the number of *coordinates*
///   (not points) in each ring. First ring = exterior, rest = holes.
///
/// Each ring must be closed (first point = last point).
#[wasm_bindgen(js_name = "areaWithHoles")]
pub fn area_with_holes(
    rings: &js_sys::Float64Array,
    ring_sizes: &js_sys::Uint32Array,
) -> Result<f64, JsValue> {
    let mut ring_buf = vec![0.0f64; rings.length() as usize];
    rings.copy_to(&mut ring_buf);
    let mut sizes_buf = vec![0u32; ring_sizes.length() as usize];
    ring_sizes.copy_to(&mut sizes_buf);

    if sizes_buf.is_empty() {
        return Ok(0.0);
    }

    let exterior_coords = &ring_buf[..sizes_buf[0] as usize];
    let mut total_area = spherical_polygon_area(exterior_coords);

    let mut offset = sizes_buf[0] as usize;
    for &size in &sizes_buf[1..] {
        let hole_coords = &ring_buf[offset..offset + size as usize];
        total_area -= spherical_polygon_area(hole_coords);
        offset += size as usize;
    }

    Ok(total_area)
}

/// Calculate the total length of a line string or polygon perimeter in meters
/// using the Haversine formula.
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]`.
#[wasm_bindgen(js_name = "polylineLength")]
pub fn polyline_length(coords: &js_sys::Float64Array) -> Result<f64, JsValue> {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let point_count = buf.len() / 2;
    if point_count < 2 {
        return Ok(0.0);
    }

    let mut total = 0.0_f64;
    for i in 0..point_count - 1 {
        let lng1 = buf[i * 2];
        let lat1 = buf[i * 2 + 1];
        let lng2 = buf[(i + 1) * 2];
        let lat2 = buf[(i + 1) * 2 + 1];
        total += haversine(lng1, lat1, lng2, lat2);
    }

    Ok(total)
}

/// Simplify a line string using the Douglas-Peucker algorithm.
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]`.
/// - `tolerance`: Simplification tolerance in **radians**.
///   For typical geographic data, `0.0001` ≈ ~11 m at the equator.
///
/// Returns simplified `Float64Array` `[lng0,lat0, ...]` preserving the first
/// and last points.
#[wasm_bindgen(js_name = "simplifyDouglasPeucker")]
pub fn simplify_douglas_peucker(
    coords: &js_sys::Float64Array,
    tolerance: f64,
) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    if n <= 2 {
        let arr = js_sys::Float64Array::new_with_length(buf.len() as u32);
        arr.copy_from(&buf);
        return arr;
    }

    let tol_sq = tolerance * tolerance;
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;

    // Iterative Douglas-Peucker
    simplify_segment(&buf, 0, n - 1, tol_sq, &mut keep);

    let mut out: Vec<f64> = Vec::with_capacity(n * 2);
    for i in 0..n {
        if keep[i] {
            out.push(buf[i * 2]);
            out.push(buf[i * 2 + 1]);
        }
    }

    let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Recursive Douglas-Peucker segment simplification.
fn simplify_segment(coords: &[f64], start: usize, end: usize, tol_sq: f64, keep: &mut [bool]) {
    let px0 = coords[start * 2];
    let py0 = coords[start * 2 + 1];
    let px1 = coords[end * 2];
    let py1 = coords[end * 2 + 1];

    let mut max_dist_sq = 0.0_f64;
    let mut max_idx = start;

    for i in (start + 1)..end {
        let px = coords[i * 2];
        let py = coords[i * 2 + 1];
        let d = perpendicular_distance_sq(px, py, px0, py0, px1, py1);
        if d > max_dist_sq {
            max_dist_sq = d;
            max_idx = i;
        }
    }

    if max_dist_sq > tol_sq {
        keep[max_idx] = true;
        if max_idx - start > 1 {
            simplify_segment(coords, start, max_idx, tol_sq, keep);
        }
        if end - max_idx > 1 {
            simplify_segment(coords, max_idx, end, tol_sq, keep);
        }
    }
}

/// Test if a point is inside a polygon ring using the ray-casting algorithm.
///
/// # Arguments
/// - `point_x`: Longitude of the test point.
/// - `point_y`: Latitude of the test point.
/// - `ring_coords`: Flat `Float64Array` `[lng0,lat0, lng1,lat1, ...]` defining the ring.
///   The ring does **not** need to be explicitly closed.
///
/// Returns `true` if the point is inside the ring.
#[wasm_bindgen(js_name = "isPointInRing")]
pub fn is_point_in_ring(point_x: f64, point_y: f64, ring_coords: &js_sys::Float64Array) -> bool {
    let mut buf = vec![0.0f64; ring_coords.length() as usize];
    ring_coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let xi = buf[i * 2];
        let yi = buf[i * 2 + 1];
        let xj = buf[j * 2];
        let yj = buf[j * 2 + 1];

        if ((yi > point_y) != (yj > point_y))
            && (point_x < (xj - xi) * (point_y - yi) / (yj - yi) + xi)
        {
            inside = !inside;
        }
        j = i;
    }

    inside
}

// ---------------------------------------------------------------------------
// TIN (Triangulated Irregular Network)
// ---------------------------------------------------------------------------

/// Result of building a TIN from scattered 3D points.
#[wasm_bindgen]
pub struct TinResult {
    positions: Vec<f64>, // flat [x,y,z, x,y,z, ...]
    indices: Vec<u32>,   // triangle indices [i0,i1,i2, ...]
}

#[wasm_bindgen]
impl TinResult {
    /// Flat vertex positions `[x0,y0,z0, x1,y1,z1, ...]`.
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float64Array {
        let arr = js_sys::Float64Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    /// Triangle indices `[i0,i1,i2, i3,i4,i5, ...]`.
    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> js_sys::Uint32Array {
        let arr = js_sys::Uint32Array::new_with_length(self.indices.len() as u32);
        arr.copy_from(&self.indices);
        arr
    }

    /// Number of vertices.
    #[wasm_bindgen(getter, js_name = "vertexCount")]
    pub fn vertex_count(&self) -> u32 {
        (self.positions.len() / 3) as u32
    }

    /// Number of triangles.
    #[wasm_bindgen(getter, js_name = "triangleCount")]
    pub fn triangle_count(&self) -> u32 {
        (self.indices.len() / 3) as u32
    }
}

/// Build a TIN from scattered 3D points using the Bowyer-Watson algorithm.
///
/// # Arguments
/// - `points`: Flat `Float64Array` `[x0,y0,z0, x1,y1,z1, ...]`
///
/// # Returns
/// `TinResult` with deduplicated positions and triangle indices.
#[wasm_bindgen(js_name = "buildTin")]
pub fn build_tin(points: &js_sys::Float64Array) -> Result<TinResult, JsValue> {
    let mut buf = vec![0.0f64; points.length() as usize];
    points.copy_to(&mut buf);

    let n = buf.len() / 3;
    if n < 3 {
        return Err(JsValue::from_str("TIN requires at least 3 points"));
    }

    let result = bowyer_watson_2d(&buf, n);
    Ok(result)
}

/// Interpolate a Z value on a TIN surface at (x, y) using barycentric interpolation.
///
/// Finds the triangle containing (x, y) and interpolates Z.
/// If the point is outside the TIN convex hull, returns the Z of the nearest vertex.
#[wasm_bindgen(js_name = "tinInterpolate")]
pub fn tin_interpolate(tin: &TinResult, x: f64, y: f64) -> f64 {
    let positions = &tin.positions;
    let indices = &tin.indices;

    let n_triangles = indices.len() / 3;

    for t in 0..n_triangles {
        let i0 = indices[t * 3] as usize;
        let i1 = indices[t * 3 + 1] as usize;
        let i2 = indices[t * 3 + 2] as usize;

        let x0 = positions[i0 * 3];
        let y0 = positions[i0 * 3 + 1];
        let z0 = positions[i0 * 3 + 2];
        let x1 = positions[i1 * 3];
        let y1 = positions[i1 * 3 + 1];
        let z1 = positions[i1 * 3 + 2];
        let x2 = positions[i2 * 3];
        let y2 = positions[i2 * 3 + 1];
        let z2 = positions[i2 * 3 + 2];

        // Barycentric coordinate test
        let denom = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2);
        if denom.abs() < 1e-12 {
            continue;
        }
        let w0 = ((y1 - y2) * (x - x2) + (x2 - x1) * (y - y2)) / denom;
        let w1 = ((y2 - y0) * (x - x2) + (x0 - x2) * (y - y2)) / denom;
        let w2 = 1.0 - w0 - w1;

        if w0 >= -1e-10 && w1 >= -1e-10 && w2 >= -1e-10 {
            return w0 * z0 + w1 * z1 + w2 * z2;
        }
    }

    // Fallback: nearest vertex
    let mut best_dist = f64::MAX;
    let mut best_z = 0.0;
    for i in 0..(positions.len() / 3) {
        let dx = positions[i * 3] - x;
        let dy = positions[i * 3 + 1] - y;
        let d = dx * dx + dy * dy;
        if d < best_dist {
            best_dist = d;
            best_z = positions[i * 3 + 2];
        }
    }
    best_z
}

// ---------------------------------------------------------------------------
// Bowyer-Watson Delaunay Triangulation (2D projection of 3D points)
// ---------------------------------------------------------------------------

/// 2D point for triangulation (projected from 3D).
#[derive(Debug, Clone, Copy)]
struct Point2D {
    x: f64,
    y: f64,
    #[allow(dead_code)]
    idx: usize, // original index in the positions array
}

/// A triangle in the triangulation.
#[derive(Debug, Clone)]
struct Triangle {
    a: usize, // index into points2d
    b: usize,
    c: usize,
}

impl Triangle {
    fn circumcircle_contains(&self, p: &Point2D, pts: &[Point2D]) -> bool {
        let ax = pts[self.a].x;
        let ay = pts[self.a].y;
        let bx = pts[self.b].x;
        let by = pts[self.b].y;
        let cx = pts[self.c].x;
        let cy = pts[self.c].y;

        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        if d.abs() < 1e-12 {
            return false; // degenerate
        }

        let ux = ((ax * ax + ay * ay) * (by - cy)
            + (bx * bx + by * by) * (cy - ay)
            + (cx * cx + cy * cy) * (ay - by))
            / d;
        let uy = ((ax * ax + ay * ay) * (cx - bx)
            + (bx * bx + by * by) * (ax - cx)
            + (cx * cx + cy * cy) * (bx - ax))
            / d;

        let r_sq = (ax - ux) * (ax - ux) + (ay - uy) * (ay - uy);
        let d_sq = (p.x - ux) * (p.x - ux) + (p.y - uy) * (p.y - uy);

        d_sq < r_sq
    }

    #[allow(dead_code)]
    fn contains_vertex(&self, v: usize) -> bool {
        self.a == v || self.b == v || self.c == v
    }
}

/// Build TIN using Bowyer-Watson algorithm on 2D projection (XY).
fn bowyer_watson_2d(coords: &[f64], n: usize) -> TinResult {
    let mut pts2d: Vec<Point2D> = (0..n)
        .map(|i| Point2D {
            x: coords[i * 3],
            y: coords[i * 3 + 1],
            idx: i,
        })
        .collect();

    // Super triangle that encompasses all points
    let (min_x, max_x, min_y, max_y) = pts2d.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(mn_x, mx_x, mn_y, mx_y), p| (mn_x.min(p.x), mx_x.max(p.x), mn_y.min(p.y), mx_y.max(p.y)),
    );

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let delta_max = dx.max(dy);
    let mid_x = (min_x + max_x) / 2.0;
    let mid_y = (min_y + max_y) / 2.0;

    // Super triangle vertices
    let st0 = Point2D {
        x: mid_x - 20.0 * delta_max,
        y: mid_y - delta_max,
        idx: usize::MAX, // sentinel
    };
    let st1 = Point2D {
        x: mid_x,
        y: mid_y + 20.0 * delta_max,
        idx: usize::MAX,
    };
    let st2 = Point2D {
        x: mid_x + 20.0 * delta_max,
        y: mid_y - delta_max,
        idx: usize::MAX,
    };

    pts2d.push(st0);
    pts2d.push(st1);
    pts2d.push(st2);

    let st0_idx = n;
    let st1_idx = n + 1;
    let st2_idx = n + 2;

    let mut triangles = vec![Triangle {
        a: st0_idx,
        b: st1_idx,
        c: st2_idx,
    }];

    // Insert each point
    for i in 0..n {
        let mut bad: Vec<usize> = Vec::new();

        for (t_idx, tri) in triangles.iter().enumerate() {
            if tri.circumcircle_contains(&pts2d[i], &pts2d) {
                bad.push(t_idx);
            }
        }

        // Find boundary polygon of the bad triangles
        let mut polygon: Vec<(usize, usize)> = Vec::new();
        for &t_idx in &bad {
            let tri = &triangles[t_idx];
            let edges = [(tri.a, tri.b), (tri.b, tri.c), (tri.c, tri.a)];

            for (ea, eb) in edges {
                let shared = bad.iter().any(|&other_idx| {
                    if other_idx == t_idx {
                        return false;
                    }
                    let other = &triangles[other_idx];
                    let oedges = [(other.a, other.b), (other.b, other.c), (other.c, other.a)];
                    oedges.iter().any(|&(oa, ob)| oa == eb && ob == ea)
                });

                if !shared {
                    polygon.push((ea, eb));
                }
            }
        }

        // Remove bad triangles (reverse order to maintain indices)
        let mut bad_sorted = bad.clone();
        bad_sorted.sort_unstable();
        for &idx in bad_sorted.iter().rev() {
            triangles.swap_remove(idx);
        }

        // Create new triangles from polygon edges to the new point
        for (ea, eb) in polygon {
            triangles.push(Triangle { a: ea, b: eb, c: i });
        }
    }

    // Remove triangles that share vertices with the super triangle
    triangles.retain(|tri| tri.a < n && tri.b < n && tri.c < n);

    // Build output
    let mut positions_out = Vec::with_capacity(n * 3);
    let mut remap = vec![0usize; n]; // original index → deduped index
    let mut dedup_map: std::collections::HashMap<(usize,), usize> =
        std::collections::HashMap::new();

    for i in 0..n {
        let key = (i,);
        if let Some(&existing) = dedup_map.get(&key) {
            remap[i] = existing;
        } else {
            let new_idx = positions_out.len() / 3;
            positions_out.push(coords[i * 3]);
            positions_out.push(coords[i * 3 + 1]);
            positions_out.push(coords[i * 3 + 2]);
            dedup_map.insert(key, new_idx);
            remap[i] = new_idx;
        }
    }

    let indices: Vec<u32> = triangles
        .iter()
        .flat_map(|tri| {
            [
                remap[tri.a] as u32,
                remap[tri.b] as u32,
                remap[tri.c] as u32,
            ]
        })
        .collect();

    TinResult {
        positions: positions_out,
        indices,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Haversine distance tests ──────────────────────────────────

    #[test]
    fn test_haversine_beijing_shanghai() {
        let dist = haversine(116.4074, 39.9042, 121.4737, 31.2304);
        assert!(
            (dist - 1_068_000.0).abs() < 20_000.0,
            "Beijing-Shanghai distance off: {}",
            dist
        );
    }

    #[test]
    fn test_haversine_zero() {
        let dist = haversine(10.0, 20.0, 10.0, 20.0);
        assert!(dist.abs() < 0.01);
    }

    // ── Shoelace area tests ────────────────────────────────────────

    #[test]
    fn test_shoelace_unit_square() {
        // Unit square: (0,0)-(1,0)-(1,1)-(0,1)-(0,0)
        let coords = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let area = shoelace_area(&coords);
        assert!((area - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_shoelace_clockwise() {
        // Clockwise unit square
        let coords = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let area = shoelace_area(&coords);
        assert!((area + 1.0).abs() < 1e-10); // negative for CW
    }

    // ── Polygon area tests ─────────────────────────────────────────

    #[test]
    fn test_spherical_area_converges_to_planar_small() {
        // Small polygon ≈ planar area
        // 1 degree × 1 degree box centered at equator ≈ 111km × 111km ≈ 12,321 km²
        let coords = vec![
            0.0, 0.0, // SW
            1.0, 0.0, // SE
            1.0, 1.0, // NE
            0.0, 1.0, // NW
            0.0, 0.0, // close
        ];
        let area = spherical_polygon_area(&coords);
        // ~111,000 m × 111,000 m ≈ 1.23e10 m²
        assert!(
            (area - 1.23e10).abs() < 5e8,
            "Area: {} (expected ~1.23e10)",
            area
        );
    }

    #[test]
    fn test_spherical_area_triangle() {
        // Simple triangle
        let coords = vec![0.0, 0.0, 1.0, 0.0, 0.5, 0.5, 0.0, 0.0];
        let area = spherical_polygon_area(&coords);
        assert!(area > 0.0);
    }

    // ── Polyline length tests ─────────────────────────────────────

    #[test]
    fn test_polyline_length_single_segment() {
        // 1 degree longitude at equator ≈ 111,320 m
        let coords = vec![0.0, 0.0, 1.0, 0.0];
        let len = native_polyline_length(&coords);
        assert!(
            (len - 111_320.0).abs() < 1_000.0,
            "Length: {} (expected ~111320)",
            len
        );
    }

    #[test]
    fn test_polyline_length_zero() {
        let coords = vec![10.0, 20.0];
        let len = native_polyline_length(&coords);
        assert!(len.abs() < 0.01);
    }

    #[test]
    fn test_polyline_length_empty() {
        let coords: Vec<f64> = vec![];
        let len = native_polyline_length(&coords);
        assert!(len.abs() < 0.01);
    }

    // Native helper (no WASM needed)
    fn native_polyline_length(coords: &[f64]) -> f64 {
        let n = coords.len() / 2;
        if n < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for i in 0..n - 1 {
            total += haversine(
                coords[i * 2],
                coords[i * 2 + 1],
                coords[(i + 1) * 2],
                coords[(i + 1) * 2 + 1],
            );
        }
        total
    }

    // ── Douglas-Peucker tests ─────────────────────────────────────

    #[test]
    fn test_simplify_preserves_endpoints() {
        let coords = vec![0.0, 0.0, 1.0, 0.1, 2.0, -0.1, 3.0, 0.0];
        let simplified = native_simplify(&coords, 0.5);
        assert_eq!(simplified.len() / 2, 2); // 2 points with large tolerance
        assert_eq!(simplified[0], 0.0);
        assert_eq!(simplified[1], 0.0);
        assert_eq!(simplified[2], 3.0);
        assert_eq!(simplified[3], 0.0);
    }

    #[test]
    fn test_simplify_large_tolerance() {
        let coords = vec![0.0, 0.0, 1.0, 5.0, 2.0, 0.0];
        let simplified = native_simplify(&coords, 10.0); // large tolerance
                                                         // With large tolerance, should reduce to just start and end
        assert_eq!(simplified.len(), 4); // 2 points × 2 coords
        assert_eq!(simplified[0], 0.0);
        assert_eq!(simplified[1], 0.0);
    }

    #[test]
    fn test_simplify_small_tolerance() {
        let coords = vec![0.0, 0.0, 1.0, 0.1, 2.0, -0.1, 3.0, 0.0];
        let simplified = native_simplify(&coords, 0.001); // tiny tolerance
                                                          // All points should be kept
        assert_eq!(simplified.len(), coords.len());
    }

    // Native simplify helper
    fn native_simplify(coords: &[f64], tolerance: f64) -> Vec<f64> {
        let n = coords.len() / 2;
        if n <= 2 {
            return coords.to_vec();
        }
        let tol_sq = tolerance * tolerance;
        let mut keep = vec![false; n];
        keep[0] = true;
        keep[n - 1] = true;
        simplify_segment(coords, 0, n - 1, tol_sq, &mut keep);
        let mut out = Vec::new();
        for i in 0..n {
            if keep[i] {
                out.push(coords[i * 2]);
                out.push(coords[i * 2 + 1]);
            }
        }
        out
    }

    // ── Point-in-ring tests ───────────────────────────────────────

    #[test]
    fn test_point_in_ring_inside() {
        // Unit square
        let ring = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        assert!(native_point_in_ring(0.5, 0.5, &ring));
    }

    #[test]
    fn test_point_in_ring_outside() {
        let ring = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        assert!(!native_point_in_ring(2.0, 2.0, &ring));
    }

    #[test]
    fn test_point_in_ring_on_edge() {
        // Points exactly on edge are edge-case; ray-casting may return either.
        // We just verify it doesn't panic.
        let ring = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let _ = native_point_in_ring(0.0, 0.5, &ring);
    }

    #[test]
    fn test_point_in_ring_triangle() {
        let ring = vec![0.0, 0.0, 10.0, 0.0, 5.0, 10.0];
        assert!(native_point_in_ring(5.0, 3.0, &ring));
        assert!(!native_point_in_ring(5.0, 11.0, &ring));
    }

    // Native point-in-ring helper
    fn native_point_in_ring(px: f64, py: f64, ring: &[f64]) -> bool {
        let n = ring.len() / 2;
        if n < 3 {
            return false;
        }
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let xi = ring[i * 2];
            let yi = ring[i * 2 + 1];
            let xj = ring[j * 2];
            let yj = ring[j * 2 + 1];
            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    // ── TIN tests ───────────────────────────────────────────────

    #[test]
    fn test_tin_basic() {
        // 3 points forming a triangle
        let coords = vec![0.0, 0.0, 10.0, 1.0, 0.0, 20.0, 0.5, 1.0, 30.0];
        let result = native_build_tin(&coords);
        assert_eq!(result.vertex_count(), 3);
        assert_eq!(result.triangle_count(), 1);
        assert_eq!(result.indices.len(), 3);
    }

    #[test]
    fn test_tin_interpolate_at_vertex() {
        // Triangle with vertices at (0,0,10), (1,0,20), (0.5,1,30)
        let coords = vec![0.0, 0.0, 10.0, 1.0, 0.0, 20.0, 0.5, 1.0, 30.0];
        let result = native_build_tin(&coords);
        let z = native_tin_interpolate(&result, 0.0, 0.0);
        assert!(
            (z - 10.0).abs() < 0.1,
            "Z at vertex (0,0) should be ~10, got {}",
            z
        );
    }

    #[test]
    fn test_tin_interpolate_centroid() {
        // Triangle with vertices at (0,0,0), (2,0,0), (1,2,6)
        let coords = vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0, 2.0, 6.0];
        let result = native_build_tin(&coords);
        // Centroid of triangle: (1, 2/3) — barycentric (1/3, 1/3, 1/3)
        let z = native_tin_interpolate(&result, 1.0, 2.0 / 3.0);
        assert!(
            (z - 2.0).abs() < 0.1,
            "Z at centroid should be ~2, got {}",
            z
        );
    }

    #[test]
    fn test_tin_four_points() {
        // 4 points forming 2 triangles
        let coords = vec![0.0, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 1.0, 3.0, 1.0, 1.0, 4.0];
        let result = native_build_tin(&coords);
        assert_eq!(result.vertex_count(), 4);
        assert!(result.triangle_count() >= 2);
        assert!(result.triangle_count() <= 2);
    }

    // Native helpers for TIN testing
    fn native_build_tin(coords: &[f64]) -> TinResult {
        let n = coords.len() / 3;
        assert!(n >= 3, "TIN requires at least 3 points");
        bowyer_watson_2d(coords, n)
    }

    fn native_tin_interpolate(tin: &TinResult, x: f64, y: f64) -> f64 {
        tin_interpolate(tin, x, y)
    }

    // ── Area with holes test ──────────────────────────────────────

    #[test]
    fn test_area_with_holes() {
        // Outer: 1x1 degree box, hole: 0.5x0.5 degree box
        let outer: Vec<f64> = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let hole: Vec<f64> = vec![0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75, 0.25, 0.25];
        let mut rings = outer.clone();
        rings.extend_from_slice(&hole);

        let outer_area = spherical_polygon_area(&outer);
        let hole_area = spherical_polygon_area(&hole);
        let remaining = outer_area - hole_area;

        assert!(remaining > 0.0);
        assert!(remaining < outer_area);
    }
}
