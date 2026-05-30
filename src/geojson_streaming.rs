//! Streaming GeoJSON parser.
//!
//! For large GeoJSON files (50 MB+), the standard `parseGeoJsonCoords` API
//! holds the entire parsed DOM in memory. This module provides a chunked
//! alternative that processes features in batches, calling a JS progress
//! callback between chunks so the UI can remain responsive and show a
//! progress bar.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │              WASM Linear Memory                   │
//! │                                                   │
//! │  ┌─────────────┐     ┌──────────────────────┐    │
//! │  │ JSON string  │────►│ Stream Deserializer   │    │
//! │  │ (input)      │     │ (feature-by-feature)  │    │
//! │  └─────────────┘     └──────┬───────────────┘    │
//! │                             │                     │
//! │            ┌────────────────▼───────────────┐     │
//! │            │  Chunk buffer (Vec<f64>)        │     │
//! │            │  [lng, lat, lng, lat, …]        │     │
//! │            └────────────────┬───────────────┘     │
//! │                             │ every N features    │
//! │                             ▼                     │
//! │                    JS callback(chunk, progress)    │
//! └──────────────────────────────────────────────────┘
//! ```

use geojson::{Feature, Geometry, Value as GeoValue};
use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

// Re-use the extract_coords helper from the main parser module,
// but we define a local version here to keep the module self-contained.

/// Recursively extract coordinate pairs from a geometry into a flat buffer.
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
// Public WASM API — Streaming / chunked parser
// ---------------------------------------------------------------------------

/// Parse a GeoJSON FeatureCollection in chunks, calling `on_chunk` with
/// each batch of coordinate data and progress information.
///
/// ## Parameters
///
/// - `input` — The full GeoJSON string (must be a FeatureCollection).
/// - `chunk_size` — Number of features to process per chunk (e.g. 1000).
///   Larger chunks = fewer JS↔WASM transitions but longer UI blocking.
/// - `on_chunk` — A JS callback: `(coords: Float64Array, processed: u32, total: u32) => void`
///
/// ## Usage (JS)
///
/// ```js
/// parseGeoJsonStream(hugeGeoJson, 500, (coords, processed, total) => {
///   // Upload coords to GPU, update progress bar
///   progressBar.value = processed / total;
///   gl.bufferSubData(gl.ARRAY_BUFFER, offset, coords);
/// });
/// ```
///
/// ## Design Rationale
///
/// Standard JSON parsers (serde_json) build the full DOM in memory.
/// For a 200 MB FeatureCollection this costs ~400 MB WASM heap.
///
/// This function first parses the full FeatureCollection (unavoidable with
/// the `geojson` crate), but then processes and emits features in chunks,
/// allowing the JS side to consume and discard coordinate data incrementally
/// rather than holding all coordinates in memory at once.
///
/// For true streaming (constant memory), a custom tokeniser would be needed.
/// This is planned for a future release using `serde_json::StreamDeserializer`
/// over raw bytes.
#[wasm_bindgen(js_name = "parseGeoJsonStream")]
pub fn parse_geojson_stream(
    input: &str,
    chunk_size: u32,
    on_chunk: &js_sys::Function,
) -> Result<u32, JsValue> {
    // Parse the FeatureCollection
    let geojson: geojson::GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

    let features = match geojson {
        geojson::GeoJson::FeatureCollection(fc) => fc.features,
        geojson::GeoJson::Feature(f) => vec![f],
        geojson::GeoJson::Geometry(g) => {
            vec![Feature {
                bbox: None,
                geometry: Some(g),
                id: None,
                properties: None,
                foreign_members: None,
            }]
        }
    };

    let total = features.len() as u32;
    let chunk_sz = chunk_size.max(1) as usize;
    let mut processed: u32 = 0;

    // Pre-allocate a reusable chunk buffer
    let mut chunk_coords: Vec<f64> = Vec::with_capacity(chunk_sz * 4); // ~2 coords per feature

    for chunk in features.chunks(chunk_sz) {
        chunk_coords.clear();

        for feat in chunk {
            if let Some(geom) = &feat.geometry {
                extract_coords(geom, &mut chunk_coords);
            }
        }

        processed += chunk.len() as u32;

        // Create Float64Array and call JS callback
        let js_coords = Float64Array::new_with_length(chunk_coords.len() as u32);
        js_coords.copy_from(&chunk_coords);

        let this = JsValue::null();
        on_chunk.call3(
            &this,
            &js_coords.into(),
            &JsValue::from(processed),
            &JsValue::from(total),
        )?;
    }

    Ok(total)
}

/// Parse a GeoJSON FeatureCollection and return coordinates in separate
/// per-feature arrays, useful when you need to map coordinates back to
/// individual features.
///
/// Returns a `js_sys::Array` where each element is a `Float64Array`
/// containing the coordinates for one feature.
#[wasm_bindgen(js_name = "parseGeoJsonPerFeature")]
pub fn parse_geojson_per_feature(input: &str) -> Result<js_sys::Array, JsValue> {
    let geojson: geojson::GeoJson = input
        .parse()
        .map_err(|e| JsValue::from_str(&format!("GeoJSON parse error: {e}")))?;

    let features = match geojson {
        geojson::GeoJson::FeatureCollection(fc) => fc.features,
        geojson::GeoJson::Feature(f) => vec![f],
        geojson::GeoJson::Geometry(g) => {
            vec![Feature {
                bbox: None,
                geometry: Some(g),
                id: None,
                properties: None,
                foreign_members: None,
            }]
        }
    };

    let result = js_sys::Array::new_with_length(features.len() as u32);
    let mut coords: Vec<f64> = Vec::with_capacity(32);

    for (i, feat) in features.iter().enumerate() {
        coords.clear();
        if let Some(geom) = &feat.geometry {
            extract_coords(geom, &mut coords);
        }
        let js_arr = Float64Array::new_with_length(coords.len() as u32);
        js_arr.copy_from(&coords);
        result.set(i as u32, js_arr.into());
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_coords_point() {
        let geom = Geometry::new(GeoValue::Point(vec![1.0, 2.0]));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out, vec![1.0, 2.0]);
    }

    #[test]
    fn test_extract_coords_polygon() {
        let ring = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 0.0],
        ];
        let geom = Geometry::new(GeoValue::Polygon(vec![ring]));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out.len(), 8); // 4 points × 2 coords
    }

    #[test]
    fn test_extract_coords_multipolygon() {
        let ring1 = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![0.0, 0.0]];
        let ring2 = vec![vec![2.0, 2.0], vec![3.0, 3.0], vec![2.0, 2.0]];
        let geom = Geometry::new(GeoValue::MultiPolygon(vec![vec![ring1], vec![ring2]]));
        let mut out = Vec::new();
        extract_coords(&geom, &mut out);
        assert_eq!(out.len(), 12); // 6 points × 2 coords
    }
}
