//! Topology analysis for spatial geometries.
//!
//! Provides polygon area, line/polygon length, Douglas-Peucker simplification,
//! and point-in-ring (ray-casting) operations on flat coordinate buffers.

use wasm_bindgen::prelude::*;

use geo::{BooleanOps, Coord, LineString, Polygon as GeoPolygon};

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
// Polygon Boolean Operations
// ---------------------------------------------------------------------------

/// Convert a flat closed-ring buffer `[lng0,lat0, ..., lng0,lat0]` to a `geo::Polygon`.
fn ring_to_geo_polygon(ring: &[f64]) -> GeoPolygon<f64> {
    let coords: Vec<Coord<f64>> = ring
        .chunks_exact(2)
        .map(|p| Coord { x: p[0], y: p[1] })
        .collect();
    let exterior = LineString(coords);
    GeoPolygon::new(exterior, vec![])
}

/// Convert a `geo::MultiPolygon` to a flat closed-ring buffer.
/// For single polygons, returns the exterior ring. For multi-polygons,
/// concatenates all exterior rings (each closed).
fn multipolygon_to_flat_rings(mp: &geo::MultiPolygon<f64>) -> Vec<f64> {
    let mut out = Vec::new();
    for polygon in mp {
        for coord in polygon.exterior().0.iter() {
            out.push(coord.x);
            out.push(coord.y);
        }
    }
    out
}

/// Core native polygon intersection — returns flat ring buffer(s).
pub fn polygon_intersection_native(ring1: &[f64], ring2: &[f64]) -> Vec<f64> {
    let p1 = ring_to_geo_polygon(ring1);
    let p2 = ring_to_geo_polygon(ring2);
    let result = p1.intersection(&p2);
    multipolygon_to_flat_rings(&result)
}

/// Core native polygon union — returns flat ring buffer(s).
pub fn polygon_union_native(ring1: &[f64], ring2: &[f64]) -> Vec<f64> {
    let p1 = ring_to_geo_polygon(ring1);
    let p2 = ring_to_geo_polygon(ring2);
    let result = p1.union(&p2);
    multipolygon_to_flat_rings(&result)
}

/// Compute the intersection of two simple polygons.
///
/// # Arguments
///
/// * `ring1` — First polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
/// * `ring2` — Second polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
///
/// # Returns
///
/// A `Float64Array` with the intersection ring(s). Empty if polygons don't intersect.
#[wasm_bindgen(js_name = "polygonIntersection")]
pub fn polygon_intersection(
    ring1: &js_sys::Float64Array,
    ring2: &js_sys::Float64Array,
) -> js_sys::Float64Array {
    let len1 = ring1.length() as usize;
    let mut buf1 = vec![0.0; len1];
    ring1.copy_to(&mut buf1);

    let len2 = ring2.length() as usize;
    let mut buf2 = vec![0.0; len2];
    ring2.copy_to(&mut buf2);

    let result = polygon_intersection_native(&buf1, &buf2);
    let out = js_sys::Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

/// Compute the union of two simple polygons.
///
/// # Arguments
///
/// * `ring1` — First polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
/// * `ring2` — Second polygon as flat closed ring `[lng0,lat0, ..., lng0,lat0]`
///
/// # Returns
///
/// A `Float64Array` with the union ring(s).
#[wasm_bindgen(js_name = "polygonUnion")]
pub fn polygon_union(
    ring1: &js_sys::Float64Array,
    ring2: &js_sys::Float64Array,
) -> js_sys::Float64Array {
    let len1 = ring1.length() as usize;
    let mut buf1 = vec![0.0; len1];
    ring1.copy_to(&mut buf1);

    let len2 = ring2.length() as usize;
    let mut buf2 = vec![0.0; len2];
    ring2.copy_to(&mut buf2);

    let result = polygon_union_native(&buf1, &buf2);
    let out = js_sys::Float64Array::new_with_length(result.len() as u32);
    out.copy_from(&result);
    out
}

// ---------------------------------------------------------------------------
// Spatial Relationship Predicates (DE-9IM)
// ---------------------------------------------------------------------------

/// Core: check if a point is inside a polygon (using `geo` crate).
pub fn contains_native(outer_ring: &[f64], point_x: f64, point_y: f64) -> bool {
    let poly = ring_to_geo_polygon(outer_ring);
    let point = geo::Point(Coord {
        x: point_x,
        y: point_y,
    });
    use geo::Contains;
    poly.contains(&point)
}

/// Core: check if two polygons touch (share boundary but not interior).
pub fn touches_native(ring1: &[f64], ring2: &[f64]) -> bool {
    let p1 = ring_to_geo_polygon(ring1);
    let p2 = ring_to_geo_polygon(ring2);
    use geo::Relate;
    p1.relate(&p2).is_touches()
}

/// Core: check if two polygons intersect (share any point).
pub fn intersects_native(ring1: &[f64], ring2: &[f64]) -> bool {
    let p1 = ring_to_geo_polygon(ring1);
    let p2 = ring_to_geo_polygon(ring2);
    use geo::Intersects;
    p1.intersects(&p2)
}

/// Core: check if two polygons are disjoint (share no points).
pub fn disjoint_native(ring1: &[f64], ring2: &[f64]) -> bool {
    let p1 = ring_to_geo_polygon(ring1);
    let p2 = ring_to_geo_polygon(ring2);
    use geo::Intersects;
    !p1.intersects(&p2)
}

/// Check if a point is inside a polygon using the `geo` crate's algorithm.
///
/// Alias for `isPointInRing` using the robust `geo::Contains` trait.
///
/// # Arguments
///
/// * `outer_ring` — Flat `Float64Array` `[lng0,lat0, ...]`
/// * `point_x` — Point longitude
/// * `point_y` — Point latitude
#[wasm_bindgen(js_name = "contains")]
pub fn contains(outer_ring: &js_sys::Float64Array, point_x: f64, point_y: f64) -> bool {
    let len = outer_ring.length() as usize;
    let mut buf = vec![0.0; len];
    outer_ring.copy_to(&mut buf);
    contains_native(&buf, point_x, point_y)
}

/// Check if two polygons touch (share boundary but not interior).
///
/// # Arguments
///
/// * `ring1` — First polygon as flat closed ring
/// * `ring2` — Second polygon as flat closed ring
#[wasm_bindgen(js_name = "touches")]
pub fn touches(ring1: &js_sys::Float64Array, ring2: &js_sys::Float64Array) -> bool {
    let len1 = ring1.length() as usize;
    let mut buf1 = vec![0.0; len1];
    ring1.copy_to(&mut buf1);
    let len2 = ring2.length() as usize;
    let mut buf2 = vec![0.0; len2];
    ring2.copy_to(&mut buf2);
    touches_native(&buf1, &buf2)
}

/// Check if two polygons intersect (share any point).
///
/// # Arguments
///
/// * `ring1` — First polygon as flat closed ring
/// * `ring2` — Second polygon as flat closed ring
#[wasm_bindgen(js_name = "polygonIntersects")]
pub fn polygon_intersects(ring1: &js_sys::Float64Array, ring2: &js_sys::Float64Array) -> bool {
    let len1 = ring1.length() as usize;
    let mut buf1 = vec![0.0; len1];
    ring1.copy_to(&mut buf1);
    let len2 = ring2.length() as usize;
    let mut buf2 = vec![0.0; len2];
    ring2.copy_to(&mut buf2);
    intersects_native(&buf1, &buf2)
}

/// Check if two polygons are disjoint (share no points at all).
///
/// # Arguments
///
/// * `ring1` — First polygon as flat closed ring
/// * `ring2` — Second polygon as flat closed ring
#[wasm_bindgen(js_name = "disjoint")]
pub fn disjoint(ring1: &js_sys::Float64Array, ring2: &js_sys::Float64Array) -> bool {
    let len1 = ring1.length() as usize;
    let mut buf1 = vec![0.0; len1];
    ring1.copy_to(&mut buf1);
    let len2 = ring2.length() as usize;
    let mut buf2 = vec![0.0; len2];
    ring2.copy_to(&mut buf2);
    disjoint_native(&buf1, &buf2)
}

// ---------------------------------------------------------------------------
// Convex & Concave Hull
// ---------------------------------------------------------------------------

/// Compute the convex hull of a set of 2D points using Andrew's monotone chain algorithm.
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
///
/// # Returns
/// Flat `Float64Array` of convex hull vertices (closed: first == last).
#[wasm_bindgen(js_name = "convexHull")]
pub fn convex_hull(coords: &js_sys::Float64Array) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    if n < 2 {
        let arr = js_sys::Float64Array::new_with_length(buf.len() as u32);
        if !buf.is_empty() {
            arr.copy_from(&buf);
        }
        return arr;
    }

    // Build points: (x, y, original_index) — use x for lng, y for lat
    let mut points: Vec<(f64, f64)> = buf.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    points.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap()
            .then(a.1.partial_cmp(&b.1).unwrap())
    });
    points.dedup();

    if points.len() == 1 {
        let out = vec![points[0].0, points[0].1, points[0].0, points[0].1];
        let arr = js_sys::Float64Array::new_with_length(4);
        arr.copy_from(&out);
        return arr;
    }

    // Andrew's monotone chain
    let cross = |o: &(f64, f64), a: &(f64, f64), b: &(f64, f64)| -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower: Vec<(f64, f64)> = Vec::new();
    for p in &points {
        while lower.len() >= 2 && cross(&lower[lower.len() - 2], &lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper: Vec<(f64, f64)> = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2 && cross(&upper[upper.len() - 2], &upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }

    // Combine: lower + upper (excluding last of each since it's duplicated)
    lower.pop();
    upper.pop();
    lower.extend_from_slice(&upper);

    // Close the ring
    if !lower.is_empty() {
        lower.push(lower[0]);
    }

    let mut out = Vec::with_capacity(lower.len() * 2);
    for (x, y) in &lower {
        out.push(*x);
        out.push(*y);
    }

    let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Compute an approximate concave hull using alpha shape (simplified).
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
/// - `alpha`: Controls concavity. Larger values → more convex (α → ∞ gives convex hull).
///   Smaller values → more concave. Typical range: 0.1–10.0.
///
/// # Returns
/// Flat `Float64Array` of concave hull vertices (closed: first == last).
#[wasm_bindgen(js_name = "concaveHull")]
pub fn concave_hull(coords: &js_sys::Float64Array, alpha: f64) -> js_sys::Float64Array {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    if n < 3 {
        let arr = js_sys::Float64Array::new_with_length(buf.len() as u32);
        if !buf.is_empty() {
            arr.copy_from(&buf);
        }
        return arr;
    }

    let mut points: Vec<(f64, f64)> = buf.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    points.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap()
            .then(a.1.partial_cmp(&b.1).unwrap())
    });
    points.dedup();

    if points.len() < 3 {
        let mut out = Vec::new();
        for p in &points {
            out.push(p.0);
            out.push(p.1);
        }
        // Close
        if out.len() >= 2 {
            out.push(out[0]);
            out.push(out[1]);
        }
        let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
        arr.copy_from(&out);
        return arr;
    }

    let result = concave_hull_core(&points, alpha);

    let mut out = Vec::with_capacity(result.len() * 2);
    for (x, y) in &result {
        out.push(*x);
        out.push(*y);
    }

    let arr = js_sys::Float64Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Simplified concave hull: start with Delaunay-like approach.
/// For each edge on the convex hull, check if any point lies within alpha * edge_length.
/// If not, remove interior points from that edge; otherwise, insert the closest interior point.
fn concave_hull_core(points: &[(f64, f64)], alpha: f64) -> Vec<(f64, f64)> {
    // Step 1: compute convex hull
    let hull = convex_hull_core(points);

    if alpha >= 1000.0 || hull.len() <= 3 {
        // Alpha too large or already a triangle → return convex hull (closed)
        let mut result = hull;
        if !result.is_empty() {
            result.push(result[0]);
        }
        return result;
    }

    // Step 2: iterative edge splitting — for each edge, find interior points
    // whose perpendicular distance is < alpha * edge_length and split
    let mut boundary: Vec<usize> = hull.iter().map(|p| find_point_index(points, *p)).collect();

    let mut changed = true;
    let max_iterations = 50;
    let mut iteration = 0;

    while changed && iteration < max_iterations {
        changed = false;
        iteration += 1;
        let mut new_boundary = Vec::new();

        for i in 0..boundary.len() {
            let j = (i + 1) % boundary.len();
            let pi = points[boundary[i]];
            let pj = points[boundary[j]];
            let edge_len = ((pj.0 - pi.0).hypot(pj.1 - pi.1)).max(1e-10);
            let threshold = alpha * edge_len;

            // Find the closest interior point to this edge
            let mut best_idx: Option<usize> = None;
            let mut best_dist = threshold;

            for (k, &pk) in points.iter().enumerate() {
                if boundary.contains(&k) && k != boundary[i] && k != boundary[j] {
                    continue;
                }
                if boundary.contains(&k) {
                    continue; // skip boundary points
                }

                let dist = point_to_segment_dist(pk, pi, pj);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = Some(k);
                }
            }

            if let Some(k) = best_idx {
                new_boundary.push(boundary[i]);
                new_boundary.push(k);
                changed = true;
            } else {
                new_boundary.push(boundary[i]);
            }
        }

        boundary = new_boundary;
    }

    // Convert boundary indices back to points, closing the ring
    let mut result: Vec<(f64, f64)> = boundary.iter().map(|&i| points[i]).collect();
    if !result.is_empty() {
        result.push(result[0]);
    }
    result
}

/// Core convex hull algorithm returning Vec of points (not closed).
/// Input must be sorted; output is not closed.
fn convex_hull_core_sorted(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = points.len();
    if n <= 1 {
        return points.to_vec();
    }

    let cross = |o: &(f64, f64), a: &(f64, f64), b: &(f64, f64)| -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower: Vec<(f64, f64)> = Vec::new();
    for p in points {
        while lower.len() >= 2 && cross(&lower[lower.len() - 2], &lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper: Vec<(f64, f64)> = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2 && cross(&upper[upper.len() - 2], &upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend_from_slice(&upper);
    lower
}

fn find_point_index(points: &[(f64, f64)], target: (f64, f64)) -> usize {
    points
        .iter()
        .position(|&p| (p.0 - target.0).abs() < 1e-12 && (p.1 - target.1).abs() < 1e-12)
        .unwrap_or(0)
}

/// Core convex hull — sorts and deduplicates input first.
fn convex_hull_core(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap()
            .then(a.1.partial_cmp(&b.1).unwrap())
    });
    sorted.dedup();
    convex_hull_core_sorted(&sorted)
}

fn point_to_segment_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-20 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.0 + t * dx;
    let proj_y = a.1 + t * dy;
    (p.0 - proj_x).hypot(p.1 - proj_y)
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

    // ── Boolean operation tests ────────────────────────────────

    #[test]
    fn test_polygon_intersection_overlapping() {
        // Two overlapping unit squares
        let ring1 = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        let ring2 = vec![1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 1.0, 3.0, 1.0, 1.0];
        let result = polygon_intersection_native(&ring1, &ring2);
        // Intersection should be [1,1] to [2,2] — non-empty
        assert!(!result.is_empty());
        assert!(result.len() >= 8); // at least a quad
    }

    #[test]
    fn test_polygon_intersection_disjoint() {
        // Two non-overlapping squares
        let ring1 = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let ring2 = vec![5.0, 5.0, 6.0, 5.0, 6.0, 6.0, 5.0, 6.0, 5.0, 5.0];
        let result = polygon_intersection_native(&ring1, &ring2);
        assert!(
            result.is_empty(),
            "Disjoint polygons should have empty intersection"
        );
    }

    #[test]
    fn test_polygon_union_overlapping() {
        let ring1 = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        let ring2 = vec![1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 1.0, 3.0, 1.0, 1.0];
        let result = polygon_union_native(&ring1, &ring2);
        assert!(!result.is_empty());
        // Union should be larger than either individual polygon
        assert!(result.len() >= 10);
    }

    #[test]
    fn test_polygon_union_identical() {
        let ring = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let result = polygon_union_native(&ring, &ring);
        assert!(!result.is_empty());
        // Union of identical polygons should equal one polygon
        let pair_count = result.len() / 2;
        assert!(pair_count >= 4);
    }

    #[test]
    fn test_polygon_intersection_contained() {
        // ring2 is completely inside ring1
        let ring1 = vec![0.0, 0.0, 4.0, 0.0, 4.0, 4.0, 0.0, 4.0, 0.0, 0.0];
        let ring2 = vec![1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 1.0, 3.0, 1.0, 1.0];
        let result = polygon_intersection_native(&ring1, &ring2);
        // Intersection should be ring2 (the contained polygon)
        assert!(!result.is_empty());
    }

    // ── Spatial relationship tests ────────────────────────────

    #[test]
    fn test_contains_inside() {
        let ring = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        assert!(contains_native(&ring, 1.0, 1.0));
    }

    #[test]
    fn test_contains_outside() {
        let ring = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        assert!(!contains_native(&ring, 5.0, 5.0));
    }

    #[test]
    fn test_contains_on_vertex() {
        // geo::Contains does not include boundary vertices.
        // A vertex on the polygon boundary is NOT strictly "contained".
        let ring = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        assert!(!contains_native(&ring, 2.0, 0.0)); // boundary vertex
        assert!(!contains_native(&ring, 0.0, 0.0)); // boundary vertex
    }

    #[test]
    fn test_intersects_overlapping() {
        let ring1 = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        let ring2 = vec![1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 1.0, 3.0, 1.0, 1.0];
        assert!(intersects_native(&ring1, &ring2));
    }

    #[test]
    fn test_intersects_disjoint() {
        let ring1 = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let ring2 = vec![5.0, 5.0, 6.0, 5.0, 6.0, 6.0, 5.0, 6.0, 5.0, 5.0];
        assert!(!intersects_native(&ring1, &ring2));
    }

    #[test]
    fn test_disjoint_true() {
        let ring1 = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let ring2 = vec![5.0, 5.0, 6.0, 5.0, 6.0, 6.0, 5.0, 6.0, 5.0, 5.0];
        assert!(disjoint_native(&ring1, &ring2));
    }

    #[test]
    fn test_disjoint_false() {
        let ring1 = vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
        let ring2 = vec![1.0, 1.0, 3.0, 1.0, 3.0, 3.0, 1.0, 3.0, 1.0, 1.0];
        assert!(!disjoint_native(&ring1, &ring2));
    }

    #[test]
    fn test_touches_adjacent() {
        // Two squares that share an edge
        let ring1 = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let ring2 = vec![1.0, 0.0, 2.0, 0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 0.0];
        assert!(touches_native(&ring1, &ring2));
    }

    #[test]
    fn test_touches_not_touching() {
        let ring1 = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let ring2 = vec![5.0, 5.0, 6.0, 5.0, 6.0, 6.0, 5.0, 6.0, 5.0, 5.0];
        assert!(!touches_native(&ring1, &ring2));
    }

    // ── Convex hull tests ────────────────────────────────────────

    #[test]
    fn test_convex_hull_square() {
        let points = vec![
            0.0, 0.0, // SW
            1.0, 0.0, // SE
            1.0, 1.0, // NE
            0.0, 1.0, // NW
            0.5, 0.5, // center (interior, should be excluded from hull)
        ];
        let pts: Vec<(f64, f64)> = points.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        let hull = convex_hull_core(&pts);
        // Hull should contain 4 corner vertices (interior point excluded)
        assert!(
            hull.len() <= 5,
            "hull has {} points: {:?}",
            hull.len(),
            hull
        );
        // All hull points should be on the boundary (x=0 or x=1 or y=0 or y=1)
        for (x, y) in &hull {
            assert!(
                (x.abs() < 1e-9
                    || (x - 1.0).abs() < 1e-9
                    || y.abs() < 1e-9
                    || (y - 1.0).abs() < 1e-9),
                "Interior point ({}, {}) in hull",
                x,
                y
            );
        }
    }

    #[test]
    fn test_convex_hull_collinear() {
        // Three collinear points
        let points = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        let hull = convex_hull_core(&points);
        // Should be just 2 unique points
        assert!(hull.len() <= 3);
    }

    #[test]
    fn test_convex_hull_triangle() {
        let points = vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)];
        let hull = convex_hull_core(&points);
        // Triangle: 3 vertices
        assert_eq!(hull.len(), 3);
    }

    #[test]
    fn test_convex_hull_single_point() {
        let points = vec![(5.0, 5.0)];
        let hull = convex_hull_core(&points);
        assert_eq!(hull.len(), 1);
    }

    // ── Concave hull tests ───────────────────────────────────────

    #[test]
    fn test_concave_hull_with_interior() {
        // L-shaped set of points
        let points: Vec<(f64, f64)> = vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (1.0, 1.0),
            (1.0, 2.0),
            (0.0, 2.0),
        ];
        let hull = concave_hull_core(&points, 0.5);
        // Concave hull should capture the L-shape (more vertices than convex)
        assert!(hull.len() >= 5);
        // Closed
        assert_eq!(hull.first(), hull.last());
    }

    #[test]
    fn test_concave_hull_large_alpha_equals_convex() {
        let points: Vec<(f64, f64)> = vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (1.0, 1.0),
            (1.0, 2.0),
            (0.0, 2.0),
        ];
        let concave = concave_hull_core(&points, 10000.0);
        let mut convex = convex_hull_core(&points);
        convex.push(convex[0]); // close it for comparison
                                // Large alpha should give approximately convex hull
        assert_eq!(concave.len(), convex.len());
    }

    #[test]
    fn test_concave_hull_triangle() {
        // Triangle — concave == convex
        let points = vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)];
        let hull = concave_hull_core(&points, 1.0);
        // concave_hull_core adds a closing point
        assert_eq!(hull.len(), 4); // 3 + closing
        assert_eq!(hull.first(), hull.last());
    }
}
