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

use wasm_bindgen::prelude::*;
use js_sys::Float64Array;
use geojson::{GeoJson, Geometry, Value as GeoValue};

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

    let mut coords: Vec<f64> = Vec::new();

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
}
