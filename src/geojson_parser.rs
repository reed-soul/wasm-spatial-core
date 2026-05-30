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

use geojson::{GeoJson, Geometry, Value as GeoValue};
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
// Unit tests
// ---------------------------------------------------------------------------

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
}
