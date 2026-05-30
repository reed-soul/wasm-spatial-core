//! TopoJSON parsing and conversion.
//!
//! TopoJSON is a topology-aware extension of GeoJSON that encodes geometry as
//! arcs (shared line segments) rather than redundant coordinate arrays. This
//! module parses TopoJSON and converts it to coordinate buffers and GeoJSON.

use wasm_bindgen::prelude::*;

use crate::validate_input_size;

// ===========================================================================
// Types
// ===========================================================================

/// A decoded arc: list of [x, y] points (delta-decoded).
type DecodedArc = Vec<[f64; 2]>;

/// A collection of decoded arcs.
#[derive(Default)]
struct ArcCollection {
    arcs: Vec<DecodedArc>,
}

impl ArcCollection {
    fn decode_arc_at(&self, arc_ref: &[i64]) -> Vec<[f64; 2]> {
        if arc_ref.is_empty() {
            return vec![];
        }
        let idx = arc_ref[0];
        let abs_idx = idx.unsigned_abs() as usize;
        if abs_idx >= self.arcs.len() {
            return vec![];
        }
        let mut pts = self.arcs[abs_idx].clone();
        if idx < 0 {
            pts.reverse();
        }
        pts
    }

    fn decode_ring(&self, ring_ref: &[i64]) -> Vec<[f64; 2]> {
        let mut ring_pts = Vec::new();
        for &arc_idx in ring_ref {
            let mut pts = self.decode_arc_at(&[arc_idx]);
            if !ring_pts.is_empty() && !pts.is_empty() {
                pts.remove(0);
            }
            ring_pts.extend(pts);
        }
        ring_pts
    }
}

/// An arc reference in TopoJSON: can be a single integer or an array of integers.
fn parse_arc_ref(val: &serde_json::Value) -> Vec<i64> {
    if let Some(n) = val.as_i64() {
        return vec![n];
    }
    if let Some(arr) = val.as_array() {
        let mut result = Vec::new();
        for v in arr {
            if let Some(n) = v.as_i64() {
                result.push(n);
            }
        }
        return result;
    }
    vec![]
}

/// A ring reference: array of arc indices (each can be an integer or array).
fn parse_ring_ref(val: &serde_json::Value) -> Vec<Vec<i64>> {
    if let Some(arr) = val.as_array() {
        let mut result = Vec::new();
        for v in arr {
            result.push(parse_arc_ref(v));
        }
        return result;
    }
    vec![]
}

// ===========================================================================
// Transform helpers
// ===========================================================================

/// Apply TopoJSON transform (quantization) to a point.
fn apply_transform(
    x: f64,
    y: f64,
    transform: &Option<serde_json::Value>,
) -> (f64, f64) {
    if let Some(t) = transform {
        if let (Some(s), Some(tr)) = (t.get("scale"), t.get("translate")) {
            let sx = s.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0);
            let sy = s.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
            let tx = tr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let ty = tr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
            return (x * sx + tx, y * sy + ty);
        }
    }
    (x, y)
}

/// Parse arcs from the TopoJSON topology.
fn parse_arcs_json(topology: &serde_json::Value) -> Vec<DecodedArc> {
    let mut arcs = Vec::new();
    if let Some(arc_arr) = topology.get("arcs").and_then(|a| a.as_array()) {
        for arc in arc_arr {
            if let Some(points) = arc.as_array() {
                let mut decoded = Vec::new();
                let mut x = 0.0_f64;
                let mut y = 0.0_f64;
                for pt in points {
                    if let Some(coords) = pt.as_array() {
                        if coords.len() >= 2 {
                            x += coords[0].as_f64().unwrap_or(0.0);
                            y += coords[1].as_f64().unwrap_or(0.0);
                            decoded.push([x, y]);
                        }
                    }
                }
                arcs.push(decoded);
            }
        }
    }
    arcs
}

/// Collect all geometries from the objects field of a TopoJSON topology.
fn collect_geometries(topology: &serde_json::Value) -> Vec<&serde_json::Value> {
    let mut geometries = Vec::new();
    if let Some(objects) = topology.get("objects").and_then(|o| o.as_object()) {
        for (_key, value) in objects {
            let geom_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if geom_type == "GeometryCollection" {
                if let Some(geoms) = value.get("geometries").and_then(|g| g.as_array()) {
                    for g in geoms {
                        geometries.push(g);
                    }
                }
            } else {
                geometries.push(value);
            }
        }
    }
    geometries
}

// ===========================================================================
// Geometry decoding
// ===========================================================================

/// Decode a geometry's coordinates from arcs and apply transform.
fn decode_geometry_coords(
    geometry: &serde_json::Value,
    arcs: &ArcCollection,
    transform: &Option<serde_json::Value>,
) -> Vec<[f64; 2]> {
    let geom_type = geometry.get("type").and_then(|t| t.as_str()).unwrap_or("");

    let mut out: Vec<[f64; 2]> = Vec::new();

    match geom_type {
        "Point" => {
            if let Some(coords) = geometry.get("coordinates").and_then(|c| c.as_array()) {
                if coords.len() >= 2 {
                    let (x, y) = apply_transform(
                        coords[0].as_f64().unwrap_or(0.0),
                        coords[1].as_f64().unwrap_or(0.0),
                        transform,
                    );
                    out.push([x, y]);
                }
            }
        }
        "MultiPoint" => {
            if let Some(arr) = geometry.get("coordinates").and_then(|c| c.as_array()) {
                for pt in arr {
                    if let Some(c) = pt.as_array() {
                        if c.len() >= 2 {
                            let (x, y) = apply_transform(
                                c[0].as_f64().unwrap_or(0.0),
                                c[1].as_f64().unwrap_or(0.0),
                                transform,
                            );
                            out.push([x, y]);
                        }
                    }
                }
            }
        }
        "LineString" => {
            if let Some(arcs_field) = geometry.get("arcs") {
                let arc_ref = parse_arc_ref(arcs_field);
                let mut pts = arcs.decode_arc_at(&arc_ref);
                for pt in &mut pts {
                    let (tx, ty) = apply_transform(pt[0], pt[1], transform);
                    pt[0] = tx;
                    pt[1] = ty;
                }
                out.extend(pts);
            }
        }
        "MultiLineString" => {
            if let Some(indices) = geometry.get("arcs").and_then(|a| a.as_array()) {
                for arc_ref_val in indices {
                    let arc_ref = parse_arc_ref(arc_ref_val);
                    let mut pts = arcs.decode_arc_at(&arc_ref);
                    for pt in &mut pts {
                        let (tx, ty) = apply_transform(pt[0], pt[1], transform);
                        pt[0] = tx;
                        pt[1] = ty;
                    }
                    out.extend(pts);
                }
            }
        }
        "Polygon" => {
            if let Some(rings_val) = geometry.get("arcs") {
                let rings = parse_ring_ref(rings_val);
                for ring_ref in &rings {
                    let mut ring_pts = Vec::new();
                    for &arc_idx in ring_ref {
                        let mut pts = arcs.decode_arc_at(&[arc_idx]);
                        if !ring_pts.is_empty() && !pts.is_empty() {
                            pts.remove(0);
                        }
                        ring_pts.extend(pts);
                    }
                    for pt in &mut ring_pts {
                        let (tx, ty) = apply_transform(pt[0], pt[1], transform);
                        pt[0] = tx;
                        pt[1] = ty;
                    }
                    out.extend(ring_pts);
                }
            }
        }
        "MultiPolygon" => {
            if let Some(polys_val) = geometry.get("arcs").and_then(|a| a.as_array()) {
                for poly_val in polys_val {
                    let rings = parse_ring_ref(poly_val);
                    for ring_ref in &rings {
                        let mut ring_pts = Vec::new();
                        for &arc_idx in ring_ref {
                            let mut pts = arcs.decode_arc_at(&[arc_idx]);
                            if !ring_pts.is_empty() && !pts.is_empty() {
                                pts.remove(0);
                            }
                            ring_pts.extend(pts);
                        }
                        for pt in &mut ring_pts {
                            let (tx, ty) = apply_transform(pt[0], pt[1], transform);
                            pt[0] = tx;
                            pt[1] = ty;
                        }
                        out.extend(ring_pts);
                    }
                }
            }
        }
        "GeometryCollection" => {
            if let Some(geometries) = geometry.get("geometries").and_then(|g| g.as_array()) {
                for geom in geometries {
                    out.extend(decode_geometry_coords(geom, arcs, transform));
                }
            }
        }
        _ => {}
    }

    out
}

// ===========================================================================
// Core functions (testable without WASM)
// ===========================================================================

/// Parse TopoJSON and return all coordinates as a flat `[lng0, lat0, lng1, lat1, ...]` buffer.
pub(crate) fn parse_topojson_core(input: &str) -> Result<Vec<f64>, String> {
    validate_input_size(input.len(), "parseTopojson").map_err(|e| e.as_string().unwrap_or_default())?;

    let topology: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("Invalid TopoJSON: {}", e))?;

    let arc_vecs = parse_arcs_json(&topology);
    let arc_collection = ArcCollection { arcs: arc_vecs };
    let transform = topology.get("transform").cloned();
    let geometries = collect_geometries(&topology);

    let mut all_coords: Vec<[f64; 2]> = Vec::new();
    for geom in &geometries {
        all_coords.extend(decode_geometry_coords(geom, &arc_collection, &transform));
    }

    let mut out = Vec::with_capacity(all_coords.len() * 2);
    for [x, y] in all_coords {
        out.push(x);
        out.push(y);
    }

    Ok(out)
}

/// Convert TopoJSON to GeoJSON string.
pub(crate) fn topojson_to_geojson_core(input: &str) -> Result<String, String> {
    validate_input_size(input.len(), "topojsonToGeojson").map_err(|e| e.as_string().unwrap_or_default())?;

    let topology: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("Invalid TopoJSON: {}", e))?;

    let arc_vecs = parse_arcs_json(&topology);
    let arc_collection = ArcCollection { arcs: arc_vecs };
    let transform = topology.get("transform").cloned();
    let geometries = collect_geometries(&topology);

    let mut features = Vec::new();

    for geom in &geometries {
        let geom_type = geom.get("type").and_then(|t| t.as_str()).unwrap_or("Unknown");
        let coords = decode_geometry_coords(geom, &arc_collection, &transform);

        let json_coords = build_geojson_coords(geom_type, &coords);

        let feature = serde_json::json!({
            "type": "Feature",
            "properties": geom.get("properties").cloned().unwrap_or(serde_json::json!({})),
            "geometry": {
                "type": geom_type,
                "coordinates": json_coords,
            }
        });
        features.push(feature);
    }

    let geojson = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    });

    serde_json::to_string_pretty(&geojson)
        .map_err(|e| format!("Failed to serialize GeoJSON: {}", e))
}

fn build_geojson_coords(geom_type: &str, coords: &[[f64; 2]]) -> serde_json::Value {
    let pts: Vec<Vec<f64>> = coords.iter().map(|c| vec![c[0], c[1]]).collect();

    match geom_type {
        "Point" => {
            if pts.len() == 1 {
                serde_json::json!([pts[0][0], pts[0][1]])
            } else {
                serde_json::json!(null)
            }
        }
        "MultiPoint" | "LineString" => serde_json::json!(pts),
        "MultiLineString" => serde_json::json!([pts]),
        "Polygon" => serde_json::json!([pts]),
        "MultiPolygon" => {
            // All coords go into a single ring in a single polygon
            serde_json::json!([[pts]])
        }
        _ => serde_json::json!(pts),
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Parse TopoJSON and return all geometry coordinates as a flat `Float64Array`.
///
/// # Arguments
/// - `input`: TopoJSON string.
///
/// # Returns
/// Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
#[wasm_bindgen(js_name = "parseTopojson")]
pub fn parse_topojson(input: &str) -> Result<js_sys::Float64Array, JsValue> {
    let coords = parse_topojson_core(input).map_err(|e| JsValue::from_str(&e))?;
    let arr = js_sys::Float64Array::new_with_length(coords.len() as u32);
    if !coords.is_empty() {
        arr.copy_from(&coords);
    }
    Ok(arr)
}

/// Convert TopoJSON to a GeoJSON FeatureCollection string.
///
/// # Arguments
/// - `input`: TopoJSON string.
///
/// # Returns
/// GeoJSON string.
#[wasm_bindgen(js_name = "topojsonToGeojson")]
pub fn topojson_to_geojson(input: &str) -> Result<String, JsValue> {
    topojson_to_geojson_core(input).map_err(|e| JsValue::from_str(&e))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_arc_basic() {
        let arc: ArcVec = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];
        let collection = ArcCollection { arcs: vec![arc] };
        let pts = collection.decode_arc_at(&[0]);
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], [0.0, 0.0]);
        assert_eq!(pts[1], [1.0, 0.0]);
        assert_eq!(pts[2], [1.0, 1.0]); // cumulative: 1+0, 0+1
    }

    #[test]
    fn test_decode_arc_negative() {
        let arc: Vec<Vec<f64>> = vec![vec![2.0, 3.0], vec![1.0, 2.0]];
        let collection = ArcCollection { arcs: vec![arc] };
        let pts = collection.decode_arc_at(&[-0]);
        // Reversed: [3, 5] → [2, 3] → [0, 0]
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], [3.0, 5.0]);
        assert_eq!(pts[1], [2.0, 3.0]);
    }

    #[test]
    fn test_parse_topojson_simple() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "point1": {
                    "type": "Point",
                    "coordinates": [0.5, 0.5]
                },
                "line1": {
                    "type": "LineString",
                    "arcs": [[0]]
                }
            },
            "arcs": [
                [[0, 0], [1, 0], [0, 1]]
            ]
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 8); // 1 point + 3 arc points
        assert!((coords[0] - 0.5).abs() < 1e-10);
        assert!((coords[1] - 0.5).abs() < 1e-10);
        assert!((coords[2] - 0.0).abs() < 1e-10);
        assert!((coords[3] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_topojson_with_transform() {
        let topojson = r#"{
            "type": "Topology",
            "transform": {
                "scale": [0.01, 0.01],
                "translate": [100.0, 40.0]
            },
            "objects": {
                "point1": {
                    "type": "Point",
                    "coordinates": [5, 10]
                }
            },
            "arcs": []
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 2);
        assert!((coords[0] - 100.05).abs() < 1e-10);
        assert!((coords[1] - 40.1).abs() < 1e-10);
    }

    #[test]
    fn test_parse_topojson_polygon() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "poly1": {
                    "type": "Polygon",
                    "arcs": [[[0]]]
                }
            },
            "arcs": [
                [[0, 0], [1, 0], [0, 1], [-1, -1]]
            ]
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 8); // 4 points × 2
        assert_eq!(coords[0], 0.0);
        assert_eq!(coords[1], 0.0);
        assert_eq!(coords[2], 1.0);
        assert_eq!(coords[3], 0.0);
        assert_eq!(coords[4], 1.0);
        assert_eq!(coords[5], 1.0);
        assert_eq!(coords[6], 0.0);
        assert_eq!(coords[7], 0.0);
    }

    #[test]
    fn test_parse_topojson_geometry_collection() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "collection": {
                    "type": "GeometryCollection",
                    "geometries": [
                        {
                            "type": "Point",
                            "coordinates": [1.0, 2.0]
                        },
                        {
                            "type": "Point",
                            "coordinates": [3.0, 4.0]
                        }
                    ]
                }
            },
            "arcs": []
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 4);
        assert_eq!(coords[0], 1.0);
        assert_eq!(coords[1], 2.0);
        assert_eq!(coords[2], 3.0);
        assert_eq!(coords[3], 4.0);
    }

    #[test]
    fn test_topojson_to_geojson_basic() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "point1": {
                    "type": "Point",
                    "coordinates": [0.5, 0.5]
                }
            },
            "arcs": []
        }"#;

        let geojson = topojson_to_geojson_core(topojson).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&geojson).unwrap();
        assert_eq!(parsed["type"], "FeatureCollection");
        assert_eq!(parsed["features"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["features"][0]["geometry"]["type"], "Point");
    }

    #[test]
    fn test_topojson_to_geojson_polygon() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "poly1": {
                    "type": "Polygon",
                    "arcs": [[[0]]]
                }
            },
            "arcs": [
                [[0, 0], [2, 0], [0, 2], [-2, -2]]
            ]
        }"#;

        let geojson = topojson_to_geojson_core(topojson).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&geojson).unwrap();
        assert_eq!(parsed["features"][0]["geometry"]["type"], "Polygon");
        assert!(parsed["features"][0]["geometry"]["coordinates"].is_array());
    }

    #[test]
    fn test_invalid_topojson() {
        let result = parse_topojson_core("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_multipolygon() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "mp": {
                    "type": "MultiPolygon",
                    "arcs": [[[[0]]]]
                }
            },
            "arcs": [
                [[0, 0], [1, 0], [0, 1], [-1, -1]]
            ]
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 8); // 4 points × 2 coords
    }

    #[test]
    fn test_linestring_with_arc() {
        let topojson = r#"{
            "type": "Topology",
            "objects": {
                "line1": {
                    "type": "LineString",
                    "arcs": [[0]]
                }
            },
            "arcs": [
                [[1, 0], [0, 1]]
            ]
        }"#;

        let coords = parse_topojson_core(topojson).unwrap();
        assert_eq!(coords.len(), 4); // 2 points
        assert_eq!(coords[0], 1.0);
        assert_eq!(coords[1], 0.0);
        assert_eq!(coords[2], 1.0);
        assert_eq!(coords[3], 1.0);
    }
}
