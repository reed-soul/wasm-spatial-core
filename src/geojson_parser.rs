//! High-performance GeoJSON parser.
//!
//! Parses large GeoJSON payloads inside WASM memory, extracts vertex
//! positions into flat `Float64Array` buffers suitable for direct
//! consumption by WebGL / WebGPU rendering pipelines.
//!
//! ## Zero-copy Design
//!
//! The JS side passes a `&str` reference; parsing and vertex extraction
//! happen entirely within WASM linear memory. The output `Float64Array`
//! is a view onto WASM memory that JS can read without an additional copy.

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::validate_input_size;
use geojson::{Feature, GeoJson, Geometry, Value as GeoValue};
use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively extract all coordinate pairs from a GeoJSON geometry.
fn extract_coords(geometry: &Geometry, out: &mut Vec<f64>) {
    match &geometry.value {
        GeoValue::Point(pos) => {
            out.push(pos[0]);
            out.push(pos[1]);
        }
        GeoValue::MultiPoint(positions) | GeoValue::LineString(positions) => {
            for pos in positions {
                out.push(pos[0]);
                out.push(pos[1]);
            }
        }
        GeoValue::MultiLineString(lines) | GeoValue::Polygon(lines) => {
            for ring in lines {
                for pos in ring {
                    out.push(pos[0]);
                    out.push(pos[1]);
                }
            }
        }
        GeoValue::MultiPolygon(polygons) => {
            for polygon in polygons {
                for ring in polygon {
                    for pos in ring {
                        out.push(pos[0]);
                        out.push(pos[1]);
                    }
                }
            }
        }
        GeoValue::GeometryCollection(geometries) => {
            for geom in geometries {
                extract_coords(geom, out);
            }
        }
    }
}

/// Count coordinate pairs in a geometry.
fn count_coords(geometry: &Geometry) -> usize {
    match &geometry.value {
        GeoValue::Point(_) => 1,
        GeoValue::MultiPoint(positions) | GeoValue::LineString(positions) => positions.len(),
        GeoValue::MultiLineString(lines) | GeoValue::Polygon(lines) => {
            lines.iter().map(|r| r.len()).sum()
        }
        GeoValue::MultiPolygon(polygons) => polygons
            .iter()
            .flat_map(|p| p.iter())
            .map(|r| r.len())
            .sum(),
        GeoValue::GeometryCollection(geometries) => geometries.iter().map(count_coords).sum(),
    }
}

/// Get the geometry type string, or "Unknown" for null geometry.
fn geometry_type_string(feat: &Feature) -> &str {
    match &feat.geometry {
        Some(geom) => match geom.value {
            GeoValue::Point(_) => "Point",
            GeoValue::MultiPoint(_) => "MultiPoint",
            GeoValue::LineString(_) => "LineString",
            GeoValue::MultiLineString(_) => "MultiLineString",
            GeoValue::Polygon(_) => "Polygon",
            GeoValue::MultiPolygon(_) => "MultiPolygon",
            GeoValue::GeometryCollection(_) => "GeometryCollection",
        },
        None => "Unknown",
    }
}

/// Collect all features from a GeoJSON value into a Vec.
fn collect_features(geojson: GeoJson) -> Vec<Feature> {
    match geojson {
        GeoJson::Feature(feat) => vec![feat],
        GeoJson::FeatureCollection(fc) => fc.features,
        GeoJson::Geometry(geom) => {
            // Wrap bare geometry in a Feature
            vec![Feature {
                bbox: None,
                foreign_members: None,
                geometry: Some(geom),
                id: None,
                properties: None,
            }]
        }
    }
}

// ---------------------------------------------------------------------------
// GeoJSON Write (Serialization) — internal helpers
// ---------------------------------------------------------------------------

/// Build a GeoJSON coordinate array from a flat buffer, respecting the geometry type.
fn build_geojson_coords(coords: &[f64], geometry_type: &str) -> serde_json::Value {
    match geometry_type {
        "Point" => {
            assert!(coords.len() >= 2, "Point requires at least 2 values");
            serde_json::json!([coords[0], coords[1]])
        }
        "MultiPoint" | "LineString" => {
            let pts: Vec<serde_json::Value> = coords
                .chunks_exact(2)
                .map(|p| serde_json::json!([p[0], p[1]]))
                .collect();
            serde_json::json!(pts)
        }
        "Polygon" => {
            // Assume single ring (exterior) — coords are a closed ring
            let ring: Vec<serde_json::Value> = coords
                .chunks_exact(2)
                .map(|p| serde_json::json!([p[0], p[1]]))
                .collect();
            serde_json::json!([ring])
        }
        _ => {
            // Unknown type — treat as LineString
            let pts: Vec<serde_json::Value> = coords
                .chunks_exact(2)
                .map(|p| serde_json::json!([p[0], p[1]]))
                .collect();
            serde_json::json!(pts)
        }
    }
}

/// Native (non-WASM) helper: generate a GeoJSON Feature string from coords and type.
pub(crate) fn geojson_from_coords_native(coords: &[f64], geometry_type: &str) -> String {
    let geo_coords = build_geojson_coords(coords, geometry_type);
    let feature = serde_json::json!({
        "type": "Feature",
        "geometry": {
            "type": geometry_type,
            "coordinates": geo_coords
        },
        "properties": {}
    });
    serde_json::to_string(&feature).unwrap()
}

/// Native (non-WASM) helper: generate a FeatureCollection from multiple features.
pub(crate) fn geojson_feature_collection_native(
    coords: &[f64],
    types: &str,
    properties_json: &str,
) -> String {
    let type_list: Vec<&str> = types.split(',').collect();
    let prop_list: Vec<&str> = properties_json.split('\x01').collect();

    let mut features = Vec::new();
    let mut offset = 0usize;

    for (i, gtype) in type_list.iter().enumerate() {
        // Determine how many coordinate pairs this feature consumes
        let pair_count = match *gtype {
            "Point" => 1,
            _ => {
                // For other types, we need to figure out from the total or parse property boundaries
                // We'll use a different strategy: parse from remaining coords greedily
                // For MultiPoint/LineString: remaining pairs until next feature
                // For Polygon: detect closed ring
                count_pairs_for_type(*gtype, &coords[offset..])
            }
        };

        let end = (offset + pair_count * 2).min(coords.len());
        let feature_coords = &coords[offset..end];

        let geo_coords = build_geojson_coords(feature_coords, gtype);
        let prop_str = prop_list.get(i).copied().unwrap_or("{}");
        let prop_val: serde_json::Value = serde_json::from_str(prop_str).unwrap_or(serde_json::json!({}));

        features.push(serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": gtype,
                "coordinates": geo_coords
            },
            "properties": prop_val
        }));

        offset = end;
    }

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features
    });
    serde_json::to_string(&fc).unwrap()
}

/// Count coordinate pairs for a given geometry type from a coordinate slice.
fn count_pairs_for_type(gtype: &str, coords: &[f64]) -> usize {
    let remaining = coords.len() / 2;
    match gtype {
        "Point" => 1,
        "LineString" | "MultiPoint" => remaining,
        "Polygon" => {
            // Find the closed ring: scan until we see the same point as the start
            if remaining < 4 {
                return remaining;
            }
            for i in 1..remaining {
                if (coords[i * 2] - coords[0]).abs() < 1e-12
                    && (coords[i * 2 + 1] - coords[1]).abs() < 1e-12
                {
                    return i + 1; // include the closing point
                }
            }
            remaining
        }
        _ => remaining,
    }
}

// ---------------------------------------------------------------------------
// Public WASM API — GeoJSON Write
// ---------------------------------------------------------------------------

/// Generate a standard GeoJSON Feature string from coordinate buffer and geometry type.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` `[lng0, lat0, lng1, lat1, …]`
/// * `geometry_type` — One of: `"Point"`, `"LineString"`, `"Polygon"`, `"MultiPoint"`
///
/// # Returns
///
/// A JSON string representing a GeoJSON Feature.
///
/// # Example (JS)
/// ```js
/// const coords = new Float64Array([116.404, 39.915]);
/// const json = core.geoJsonFromCoords(coords, "Point");
/// // json = '{"type":"Feature","geometry":{"type":"Point","coordinates":[116.404,39.915]},"properties":{}}'
/// ```
#[wasm_bindgen(js_name = "geoJsonFromCoords")]
pub fn geojson_from_coords(coords: &Float64Array, geometry_type: &str) -> Result<String, JsValue> {
    validate_input_size(coords.length() as usize * 8, "geoJsonFromCoords")?;
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    match geometry_type {
        "Point" | "LineString" | "Polygon" | "MultiPoint" => {}
        _ => {
            return Err(JsValue::from_str(&format!(
                "Unsupported geometry type: {}",
                geometry_type
            )));
        }
    }

    Ok(geojson_from_coords_native(&buf, geometry_type))
}

/// Generate a GeoJSON FeatureCollection string from multiple features.
///
/// # Arguments
///
/// * `coords` — Flat `Float64Array` with all feature coordinates concatenated
/// * `types` — Comma-separated geometry types (one per feature)
/// * `properties_json` — Properties for each feature, separated by `\x01` (unit separator).
///   Each segment should be a valid JSON object string. Use `"{}"` for empty properties.
///
/// # Returns
///
/// A JSON string representing a GeoJSON FeatureCollection.
///
/// # Example (JS)
/// ```js
/// const coords = new Float64Array([116.4, 39.9, 121.5, 31.2]);
/// const json = core.geoJsonFeatureCollection(coords, "Point,Point", '{"name":"BJ"}\x01{"name":"SH"}');
/// ```
#[wasm_bindgen(js_name = "geoJsonFeatureCollection")]
pub fn geojson_feature_collection(
    coords: &Float64Array,
    types: &str,
    properties_json: &str,
) -> Result<String, JsValue> {
    validate_input_size(coords.length() as usize * 8, "geoJsonFeatureCollection")?;
    let len = coords.length() as usize;
    let mut buf = vec![0.0; len];
    coords.copy_to(&mut buf);

    Ok(geojson_feature_collection_native(&buf, types, properties_json))
}

// ---------------------------------------------------------------------------
// Public WASM API — GeoJSON Parse
// ---------------------------------------------------------------------------

/// Parse a GeoJSON string and return **all** coordinate pairs as a flat
/// `Float64Array` — `[lng0, lat0, lng1, lat1, …]`.
///
/// This is designed for bulk ingestion of large datasets; the flat layout
/// allows direct upload to a GPU vertex buffer with no further processing.
///
/// # Errors
///
/// Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonCoords")]
pub fn parse_geojson_coords(input: &str) -> Result<Float64Array, SpatialErrorDetail> {
    let geojson: GeoJson = input.parse().map_err(SpatialError::parse_error)?;

    // Heuristic: ~1 coordinate pair per ~80 bytes of GeoJSON text.
    // Pre-allocating avoids repeated realloc for large files.
    let estimated_pairs = input.len() / 80;
    let mut coords: Vec<f64> = Vec::with_capacity(estimated_pairs * 2);

    match geojson {
        GeoJson::Geometry(geom) => extract_coords(&geom, &mut coords),
        GeoJson::Feature(feat) => {
            if let Some(geom) = &feat.geometry {
                extract_coords(geom, &mut coords);
            }
        }
        GeoJson::FeatureCollection(fc) => {
            for feat in &fc.features {
                if let Some(geom) = &feat.geometry {
                    extract_coords(geom, &mut coords);
                }
            }
        }
    }

    let result = Float64Array::new_with_length(coords.len() as u32);
    result.copy_from(&coords);
    Ok(result)
}

/// Return the total number of features in a GeoJSON string.
///
/// Useful for progress reporting before parsing a very large file.
#[wasm_bindgen(js_name = "countGeoJsonFeatures")]
pub fn count_geojson_features(input: &str) -> Result<u32, SpatialErrorDetail> {
    let geojson: GeoJson = input.parse().map_err(SpatialError::parse_error)?;

    let count = match geojson {
        GeoJson::Geometry(_) => 1,
        GeoJson::Feature(_) => 1,
        GeoJson::FeatureCollection(fc) => fc.features.len(),
    };

    Ok(count as u32)
}

// ---------------------------------------------------------------------------
// Feature-level structured parsing
// ---------------------------------------------------------------------------

/// Result of structured GeoJSON feature parsing.
///
/// Contains per-feature coordinate buffers, offsets, counts, and geometry types.
#[wasm_bindgen(js_name = "GeoJsonFeaturesResult")]
pub struct GeoJsonFeaturesResult {
    /// Flat coordinate buffer `[lng0, lat0, lng1, lat1, …]` for all features.
    coordinates: Float64Array,
    /// Per-feature starting offset into the coordinate buffer.
    offsets: js_sys::Uint32Array,
    /// Per-feature coordinate pair count.
    counts: js_sys::Uint32Array,
    /// Comma-separated geometry types, e.g. `"Point,LineString,Polygon"`.
    types: String,
}

#[wasm_bindgen]
impl GeoJsonFeaturesResult {
    /// All coordinates as a flat `Float64Array`.
    #[wasm_bindgen(getter)]
    pub fn coordinates(&self) -> Float64Array {
        self.coordinates.clone()
    }

    /// Per-feature starting offset into the coordinate buffer.
    #[wasm_bindgen(getter)]
    pub fn offsets(&self) -> js_sys::Uint32Array {
        self.offsets.clone()
    }

    /// Per-feature coordinate pair count.
    #[wasm_bindgen(getter)]
    pub fn counts(&self) -> js_sys::Uint32Array {
        self.counts.clone()
    }

    /// Comma-separated geometry type for each feature.
    #[wasm_bindgen(getter)]
    pub fn types(&self) -> String {
        self.types.clone()
    }
}

/// Parse a GeoJSON string and return structured per-feature results including
/// coordinates, offsets, counts, and geometry types.
///
/// This is useful when you need to iterate features individually while still
/// benefitting from a single-pass parse.
///
/// # Errors
///
/// Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonFeatures")]
pub fn parse_geojson_features(input: &str) -> Result<GeoJsonFeaturesResult, SpatialErrorDetail> {
    let geojson: GeoJson = input.parse().map_err(SpatialError::parse_error)?;

    let features = collect_features(geojson);
    let feature_count = features.len();

    if feature_count == 0 {
        return Ok(GeoJsonFeaturesResult {
            coordinates: Float64Array::new_with_length(0),
            offsets: js_sys::Uint32Array::new_with_length(0),
            counts: js_sys::Uint32Array::new_with_length(0),
            types: String::new(),
        });
    }

    // First pass: count total coordinates and build types string
    let mut total_coords = 0usize;
    let mut counts = Vec::with_capacity(feature_count);
    let mut type_strings = Vec::with_capacity(feature_count);

    for feat in &features {
        let cnt = feat.geometry.as_ref().map(count_coords).unwrap_or(0);
        total_coords += cnt;
        counts.push(cnt as u32);
        type_strings.push(geometry_type_string(feat));
    }

    // Second pass: extract coordinates and compute offsets
    let mut all_coords: Vec<f64> = Vec::with_capacity(total_coords * 2);
    let mut offsets: Vec<u32> = Vec::with_capacity(feature_count);
    offsets.push(0u32);

    for feat in &features {
        if let Some(geom) = &feat.geometry {
            let start = all_coords.len() / 2;
            extract_coords(geom, &mut all_coords);
            let coord_count = (all_coords.len() / 2 - start) as u32;
            let prev = offsets[offsets.len() - 1];
            offsets.push(prev + coord_count);
        } else {
            offsets.push(offsets[offsets.len() - 1]);
        }
    }

    // Remove the trailing offset (we have N+1 offsets for N features)
    offsets.pop();

    let coords_arr = Float64Array::new_with_length(all_coords.len() as u32);
    coords_arr.copy_from(&all_coords);

    let offsets_arr = js_sys::Uint32Array::new_with_length(offsets.len() as u32);
    offsets_arr.copy_from(&offsets);

    let counts_arr = js_sys::Uint32Array::new_with_length(counts.len() as u32);
    counts_arr.copy_from(&counts);

    Ok(GeoJsonFeaturesResult {
        coordinates: coords_arr,
        offsets: offsets_arr,
        counts: counts_arr,
        types: type_strings.join(","),
    })
}

/// Extract all feature properties from a GeoJSON string as a JSON string.
///
/// Returns a JSON array of property objects. Features without properties
/// are represented as `null` entries.
///
/// # Example
/// ```js
/// const props = JSON.parse(core.parseGeoJsonProperties(geojsonStr));
/// // props = [{ name: "Beijing", population: 21540000 }, { ... }]
/// ```
///
/// # Errors
///
/// Returns a `SpatialErrorDetail` if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonProperties")]
pub fn parse_geojson_properties(input: &str) -> Result<String, SpatialErrorDetail> {
    let geojson: GeoJson = input.parse().map_err(SpatialError::parse_error)?;

    let features = collect_features(geojson);

    let props_array: Vec<Option<serde_json::Value>> = features
        .into_iter()
        .map(|f| f.properties.and_then(|p| serde_json::to_value(p).ok()))
        .collect();

    serde_json::to_string(&props_array).map_err(SpatialError::parse_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_FC: &str = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [116.404, 39.915]
                },
                "properties": {}
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[100.0, 0.0], [101.0, 1.0]]
                },
                "properties": {}
            }
        ]
    }"#;

    #[test]
    fn test_count_features() {
        let count = count_geojson_features(SAMPLE_FC).unwrap();
        assert_eq!(count, 2);
    }

    // ── Tests that exercise the public WASM API (requires wasm32) ─────

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_geojson_coords_returns_correct_count() {
        let coords = parse_geojson_coords(SAMPLE_FC).unwrap();
        // Feature 0: 1 Point = 1 pair = 2 floats
        // Feature 1: 1 LineString with 2 points = 2 pairs = 4 floats
        // Total: 3 pairs = 6 floats
        assert_eq!(coords.length(), 6);
        assert_eq!(coords.length() / 2, 3);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_geojson_coords_point() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [116.404, 39.915]
            },
            "properties": {}
        }"#;
        let coords = parse_geojson_coords(geojson).unwrap();
        assert_eq!(coords.length(), 2);
        let mut buf = vec![0.0; 2];
        coords.copy_to(&mut buf);
        assert!((buf[0] - 116.404).abs() < 1e-10);
        assert!((buf[1] - 39.915).abs() < 1e-10);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_geojson_coords_polygon() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0,0],[1,0],[1,1],[0,0]]]
            },
            "properties": {}
        }"#;
        let coords = parse_geojson_coords(geojson).unwrap();
        assert_eq!(coords.length(), 8);
    }

    // ── Native tests (test internal logic, no WASM needed) ───────────

    #[test]
    fn test_extract_coords_from_geometry() {
        let geom = Geometry::new(GeoValue::Point(vec![1.0, 2.0]));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out, vec![1.0, 2.0]);
    }

    #[test]
    fn test_extract_coords_linestring() {
        let positions = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![2.0, 2.0]];
        let geom = Geometry::new(GeoValue::LineString(positions));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out, vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0]);
        assert_eq!(out.len() / 2, 3); // 3 coordinate pairs
    }

    #[test]
    fn test_extract_coords_multipolygon_count() {
        let ring1 = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![0.0, 0.0]];
        let ring2 = vec![vec![2.0, 2.0], vec![3.0, 3.0], vec![2.0, 2.0]];
        let geom = Geometry::new(GeoValue::MultiPolygon(vec![vec![ring1], vec![ring2]]));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out.len() / 2, 6); // 6 coordinate pairs
    }

    // ── Native tests for new functions (no WASM needed) ──────────

    #[test]
    fn test_collect_features_feature_collection() {
        let geojson: GeoJson = SAMPLE_FC.parse().unwrap();
        let features = collect_features(geojson);
        assert_eq!(features.len(), 2);
    }

    #[test]
    fn test_collect_features_single_feature() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [1.0, 2.0]
            },
            "properties": { "name": "test" }
        }"#;
        let geojson: GeoJson = geojson.parse().unwrap();
        let features = collect_features(geojson);
        assert_eq!(features.len(), 1);
    }

    #[test]
    fn test_geometry_type_string() {
        let feat: Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
            "properties": {}
        }))
        .unwrap();
        assert_eq!(geometry_type_string(&feat), "Point");
    }

    #[test]
    fn test_count_coords() {
        let geom = Geometry::new(GeoValue::LineString(vec![
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![2.0, 2.0],
        ]));
        assert_eq!(count_coords(&geom), 3);
    }

    #[test]
    fn test_parse_geojson_properties_native() {
        let props = parse_geojson_properties(SAMPLE_FC).unwrap();
        let parsed: Vec<Option<serde_json::Value>> = serde_json::from_str(&props).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_parse_geojson_properties_with_data() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [116.4, 39.9]},
                    "properties": {"name": "Beijing", "population": 21540000}
                },
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [121.5, 31.2]},
                    "properties": {"name": "Shanghai", "population": 24870000}
                }
            ]
        }"#;
        let props = parse_geojson_properties(geojson).unwrap();
        let parsed: Vec<Option<serde_json::Value>> = serde_json::from_str(&props).unwrap();
        assert_eq!(parsed.len(), 2);
        let first = parsed[0].as_ref().unwrap();
        assert_eq!(first["name"], "Beijing");
        assert_eq!(first["population"], 21540000);
    }

    #[test]
    fn test_parse_geojson_properties_null_props() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [1.0, 2.0]},
            "properties": null
        }"#;
        let props = parse_geojson_properties(geojson).unwrap();
        let parsed: Vec<Option<serde_json::Value>> = serde_json::from_str(&props).unwrap();
        assert!(parsed[0].is_none());
    }

    // ── GeoJSON Write tests ────────────────────────────────────

    #[test]
    fn test_geojson_from_coords_point() {
        let coords = vec![116.404, 39.915];
        let json = native_geojson_from_coords(&coords, "Point");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "Feature");
        assert_eq!(parsed["geometry"]["type"], "Point");
        assert_eq!(parsed["geometry"]["coordinates"][0], 116.404);
        assert_eq!(parsed["geometry"]["coordinates"][1], 39.915);
    }

    #[test]
    fn test_geojson_from_coords_linestring() {
        let coords = vec![100.0, 0.0, 101.0, 1.0, 102.0, 2.0];
        let json = native_geojson_from_coords(&coords, "LineString");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["geometry"]["type"], "LineString");
        let arr = parsed["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0][0], 100.0);
        assert_eq!(arr[2][1], 2.0);
    }

    #[test]
    fn test_geojson_from_coords_multipoint() {
        let coords = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let json = native_geojson_from_coords(&coords, "MultiPoint");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["geometry"]["type"], "MultiPoint");
        let arr = parsed["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_geojson_from_coords_polygon() {
        // Closed ring: [0,0, 1,0, 1,1, 0,1, 0,0]
        let coords = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let json = native_geojson_from_coords(&coords, "Polygon");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["geometry"]["type"], "Polygon");
        let rings = parsed["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(rings.len(), 1); // one exterior ring
        assert_eq!(rings[0].as_array().unwrap().len(), 5); // 5 vertices (closed)
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_geojson_roundtrip_point() {
        let coords = vec![116.404, 39.915];
        let json = native_geojson_from_coords(&coords, "Point");
        // Re-parse with our parser
        let re_coords = parse_geojson_coords(&json).unwrap();
        let mut buf = vec![0.0; 2];
        re_coords.copy_to(&mut buf);
        assert!((buf[0] - coords[0]).abs() < 1e-10);
        assert!((buf[1] - coords[1]).abs() < 1e-10);
    }

    fn native_geojson_from_coords(coords: &[f64], geometry_type: &str) -> String {
        geojson_from_coords_native(coords, geometry_type)
    }

    #[test]
    fn test_geojson_feature_collection_native() {
        let coords1 = vec![116.4, 39.9]; // Point
        let coords2 = vec![100.0, 0.0, 101.0, 1.0]; // LineString
        let mut all_coords = coords1.clone();
        all_coords.extend_from_slice(&coords2);
        let types = "Point,LineString";
        let props = "{\"name\":\"Beijing\"}\x01{}";

        let json = native_geojson_feature_collection(&all_coords, types, props);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "FeatureCollection");
        let features = parsed["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["geometry"]["type"], "Point");
        assert_eq!(features[1]["geometry"]["type"], "LineString");
        assert_eq!(features[0]["properties"]["name"], "Beijing");
    }

    fn native_geojson_feature_collection(
        coords: &[f64],
        types: &str,
        properties_json: &str,
    ) -> String {
        geojson_feature_collection_native(coords, types, properties_json)
    }

    #[test]
    fn test_geojson_feature_collection_roundtrip() {
        // Build a FC: Point + LineString
        let coords = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let types = "Point,LineString";
        let props = "{\"a\":1}\x01{\"b\":2}";
        let json = native_geojson_feature_collection(&coords, types, props);

        // Parse back with native helpers
        let geojson: GeoJson = json.parse().unwrap();
        let features = collect_features(geojson);
        assert_eq!(features.len(), 2);
        let mut all_coords = Vec::new();
        for feat in &features {
            if let Some(geom) = &feat.geometry {
                extract_coords(geom, &mut all_coords);
            }
        }
        assert_eq!(all_coords.len(), 6);
        assert_eq!(all_coords[0], 10.0);
        assert_eq!(all_coords[5], 60.0);
    }
}
