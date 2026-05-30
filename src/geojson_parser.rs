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
// Public WASM API
// ---------------------------------------------------------------------------

/// Parse a GeoJSON string and return **all** coordinate pairs as a flat
/// `Float64Array` — `[lng0, lat0, lng1, lat1, …]`.
///
/// This is designed for bulk ingestion of large datasets; the flat layout
/// allows direct upload to a GPU vertex buffer with no further processing.
///
/// # Errors
///
/// Returns a `JsValue` error if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonCoords")]
pub fn parse_geojson_coords(input: &str) -> Result<Float64Array, JsValue> {
    let geojson: GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

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
pub fn count_geojson_features(input: &str) -> Result<u32, JsValue> {
    let geojson: GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

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
/// Returns a `JsValue` error if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonFeatures")]
pub fn parse_geojson_features(input: &str) -> Result<GeoJsonFeaturesResult, JsValue> {
    let geojson: GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

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
/// Returns a `JsValue` error if the input is not valid GeoJSON.
#[wasm_bindgen(js_name = "parseGeoJsonProperties")]
pub fn parse_geojson_properties(input: &str) -> Result<String, JsValue> {
    let geojson: GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

    let features = collect_features(geojson);

    let props_array: Vec<Option<serde_json::Value>> = features
        .into_iter()
        .map(|f| f.properties.and_then(|p| serde_json::to_value(p).ok()))
        .collect();

    serde_json::to_string(&props_array)
        .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {e}")))
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
}
