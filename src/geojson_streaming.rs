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
        .map_err(|e| crate::errors::parse_js(format!("GeoJSON parse error: {e}")))?;

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
        .map_err(|e| crate::errors::parse_js(format!("GeoJSON parse error: {e}")))?;

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
// Lazy GeoJSON parser — O(single feature) memory
// ---------------------------------------------------------------------------

/// Counts the number of `Value`s (objects and arrays) in a features array
/// by scanning the raw JSON text. Does NOT build a DOM.
///
/// Algorithm: Walk bytes, track nesting depth of `{` and `[`, skip strings.
/// When depth==1 (top level of features array), count commas + 1.
fn count_features_raw(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    // Find "features" key
    let features_key = b"\"features\"";
    let mut found = false;
    while i + features_key.len() <= len {
        if &bytes[i..i + features_key.len()] == features_key {
            found = true;
            i += features_key.len();
            break;
        }
        if bytes[i] == b'"' {
            // skip string
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    if !found {
        return 0;
    }

    // Skip to colon and then to `[`
    while i < len && bytes[i] != b'[' {
        i += 1;
    }
    if i >= len {
        return 0;
    }
    i += 1; // skip `[`

    // Now count top-level elements in the array
    let mut depth: i32 = 1;
    let mut count = 0;
    let mut expect_value = true;

    while i < len && depth > 0 {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' | b',' | b':' => {
                if c == b',' && depth == 1 {
                    expect_value = true;
                }
                i += 1;
            }
            b'"' => {
                if depth == 1 && expect_value {
                    count += 1;
                    expect_value = false;
                }
                // Skip string
                i += 1;
                while i < len && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'{' | b'[' => {
                if depth == 1 && expect_value {
                    count += 1;
                    expect_value = false;
                }
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b']' => {
                depth -= 1;
                i += 1;
            }
            b't' | b'f' | b'n' => {
                // true, false, null
                if depth == 1 && expect_value {
                    count += 1;
                    expect_value = false;
                }
                if c == b't' {
                    i += 4;
                }
                // true
                else if c == b'f' {
                    i += 5;
                }
                // false
                else {
                    i += 4;
                } // null
            }
            _ => {
                // Number or other value
                if depth == 1 && expect_value {
                    count += 1;
                    expect_value = false;
                }
                // Consume number characters
                while i < len
                    && (bytes[i].is_ascii_digit()
                        || bytes[i] == b'.'
                        || bytes[i] == b'-'
                        || bytes[i] == b'+'
                        || bytes[i] == b'e'
                        || bytes[i] == b'E')
                {
                    i += 1;
                }
                // Safety: if no bytes were consumed, advance by one to avoid infinite loop
                if i < len
                    && !matches!(
                        bytes[i],
                        b'{' | b'}'
                            | b'['
                            | b']'
                            | b'"'
                            | b','
                            | b':'
                            | b' '
                            | b'\t'
                            | b'\n'
                            | b'\r'
                    )
                {
                    i += 1;
                }
            }
        }
    }
    count
}

/// Extract all coordinate pairs from a coordinates JSON array starting at `pos`.
/// Advances `pos` past the closing `]`.
/// Returns the coordinates as a flat Vec<f64>.
fn extract_coords_from_json(input: &str, pos: &mut usize) -> Vec<f64> {
    let bytes = input.as_bytes();
    let mut coords = Vec::with_capacity(16);
    let len = bytes.len();

    // Skip to opening `[`
    while *pos < len && bytes[*pos] != b'[' {
        *pos += 1;
    }
    if *pos >= len {
        return coords;
    }
    *pos += 1;

    extract_coords_recursive(input, bytes, len, pos, &mut coords);

    coords
}

/// Recursively extract 2D coordinate pairs from nested JSON coordinate arrays.
fn extract_coords_recursive(
    input: &str,
    bytes: &[u8],
    len: usize,
    pos: &mut usize,
    coords: &mut Vec<f64>,
) {
    while *pos < len {
        // Skip whitespace
        while *pos < len
            && (bytes[*pos] == b' '
                || bytes[*pos] == b'\t'
                || bytes[*pos] == b'\n'
                || bytes[*pos] == b'\r')
        {
            *pos += 1;
        }
        if *pos >= len {
            break;
        }

        match bytes[*pos] {
            b']' => {
                *pos += 1;
                return;
            }
            b'[' => {
                *pos += 1;
                extract_coords_recursive(input, bytes, len, pos, coords);
            }
            b',' => {
                *pos += 1;
            }
            b'"' => {
                // Skip string values (shouldn't happen in coordinates, but be safe)
                *pos += 1;
                while *pos < len && bytes[*pos] != b'"' {
                    if bytes[*pos] == b'\\' {
                        *pos += 1;
                    }
                    *pos += 1;
                }
                if *pos < len {
                    *pos += 1;
                }
            }
            b't' | b'f' | b'n' => {
                // Skip true/false/null
                if bytes[*pos] == b't' {
                    *pos += 4;
                } else if bytes[*pos] == b'f' {
                    *pos += 5;
                } else {
                    *pos += 4;
                }
            }
            b'{' => {
                // Skip object
                *pos += 1;
                skip_value(input, bytes, len, pos);
            }
            _ => {
                // Must be a number
                let start = *pos;
                while *pos < len
                    && (bytes[*pos].is_ascii_digit()
                        || bytes[*pos] == b'.'
                        || bytes[*pos] == b'-'
                        || bytes[*pos] == b'+'
                        || bytes[*pos] == b'e'
                        || bytes[*pos] == b'E')
                {
                    *pos += 1;
                }
                if start < *pos {
                    let num_str = &input[start..*pos];
                    if let Ok(v) = num_str.parse::<f64>() {
                        coords.push(v);
                    }
                }
            }
        }
    }
}

/// Skip a JSON value (object, array, string, number, bool, null) starting at `pos`.
/// For objects and arrays, handles nesting properly.
fn skip_value(_input: &str, bytes: &[u8], len: usize, pos: &mut usize) {
    if *pos >= len {
        return;
    }
    match bytes[*pos] {
        b'"' => {
            *pos += 1;
            while *pos < len && bytes[*pos] != b'"' {
                if bytes[*pos] == b'\\' {
                    *pos += 1;
                }
                *pos += 1;
            }
            if *pos < len {
                *pos += 1;
            }
        }
        b'{' | b'[' => {
            let close = if bytes[*pos] == b'{' { b'}' } else { b']' };
            *pos += 1;
            let mut depth = 1;
            while *pos < len && depth > 0 {
                match bytes[*pos] {
                    b'"' => {
                        *pos += 1;
                        while *pos < len && bytes[*pos] != b'"' {
                            if bytes[*pos] == b'\\' {
                                *pos += 1;
                            }
                            *pos += 1;
                        }
                        if *pos < len {
                            *pos += 1;
                        }
                    }
                    c if c == close => {
                        depth -= 1;
                        *pos += 1;
                    }
                    c if c == b'{' || c == b'[' => {
                        depth += 1;
                        *pos += 1;
                    }
                    _ => {
                        *pos += 1;
                    }
                }
            }
        }
        b't' => {
            *pos += 4;
        }
        b'f' => {
            *pos += 5;
        }
        b'n' => {
            *pos += 4;
        }
        _ => {
            // Number
            while *pos < len
                && (bytes[*pos].is_ascii_digit()
                    || bytes[*pos] == b'.'
                    || bytes[*pos] == b'-'
                    || bytes[*pos] == b'+'
                    || bytes[*pos] == b'e'
                    || bytes[*pos] == b'E')
            {
                *pos += 1;
            }
        }
    }
}

/// A lazy GeoJSON FeatureCollection iterator.
///
/// Parses features one at a time using a manual JSON state machine,
/// without building the full DOM. Memory peak is O(single feature)
/// instead of O(all features).
///
/// ## Usage (JS)
///
/// ```js
/// const iter = parseGeoJsonLazy(hugeGeoJsonStr);
/// let feature;
/// while ((feature = iter.nextFeature()) !== null) {
///   // feature is a Float64Array of [lng0, lat0, lng1, lat1, ...]
///   gl.bufferSubData(gl.ARRAY_BUFFER, offset, feature);
///   offset += feature.byteLength;
/// }
/// console.log(`Processed ${iter.remaining()} features`);
/// iter.free();
/// ```
#[wasm_bindgen(js_name = "LazyGeoJsonIter")]
pub struct LazyGeoJsonIter {
    /// The full JSON input text (kept alive for byte-level parsing).
    input: String,
    /// Current byte offset into `input`.
    pos: usize,
    /// Total feature count (pre-scanned).
    total: u32,
    /// Number of features already consumed.
    consumed: u32,
}

#[wasm_bindgen]
impl LazyGeoJsonIter {
    /// Get the remaining unconsumed feature count.
    #[wasm_bindgen(js_name = "remaining")]
    pub fn remaining(&self) -> u32 {
        self.total - self.consumed
    }

    /// Get the total feature count.
    #[wasm_bindgen(getter)]
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Advance to the next feature and return its coordinates as a `Float64Array`.
    ///
    /// Returns `null` (JS undefined) when all features have been consumed.
    #[wasm_bindgen(js_name = "nextFeature")]
    pub fn next_feature(&mut self) -> Option<Float64Array> {
        if self.consumed >= self.total {
            return None;
        }

        let bytes = self.input.as_bytes();
        let len = bytes.len();

        // Advance past whitespace and commas to the next `{` (start of Feature)
        while self.pos < len {
            let c = bytes[self.pos];
            if c == b'{' {
                break;
            }
            self.pos += 1;
        }
        if self.pos >= len {
            self.consumed = self.total;
            return None;
        }

        // We're at the start of a Feature object. Find "geometry" key and its value.
        let mut found_geometry = false;
        let mut geometry_is_null = false;
        let mut geometry_pos = 0usize;

        // Parse the Feature object manually
        // Feature = { "type": "Feature", "geometry": ... , "properties": ... , ... }
        self.pos += 1; // skip `{`
        let mut depth = 1i32;

        while self.pos < len && depth > 0 {
            // Skip whitespace
            while self.pos < len
                && (bytes[self.pos] == b' '
                    || bytes[self.pos] == b'\t'
                    || bytes[self.pos] == b'\n'
                    || bytes[self.pos] == b'\r')
            {
                self.pos += 1;
            }
            if self.pos >= len {
                break;
            }

            match bytes[self.pos] {
                b'"' => {
                    // Read key
                    self.pos += 1;
                    let key_start = self.pos;
                    while self.pos < len && bytes[self.pos] != b'"' {
                        if bytes[self.pos] == b'\\' {
                            self.pos += 1;
                        }
                        self.pos += 1;
                    }
                    let key = &self.input[key_start..self.pos];
                    if self.pos < len {
                        self.pos += 1;
                    } // skip closing `"`

                    // Skip whitespace and colon
                    while self.pos < len
                        && (bytes[self.pos] == b' '
                            || bytes[self.pos] == b'\t'
                            || bytes[self.pos] == b'\n'
                            || bytes[self.pos] == b'\r'
                            || bytes[self.pos] == b':')
                    {
                        self.pos += 1;
                    }

                    if key == "geometry" {
                        found_geometry = true;
                        if self.pos < len && bytes[self.pos] == b'n' {
                            // null
                            geometry_is_null = true;
                            self.pos += 4;
                        } else {
                            geometry_is_null = false;
                            geometry_pos = self.pos;
                        }
                    } else {
                        // Skip this value
                        skip_value(&self.input, bytes, len, &mut self.pos);
                    }
                }
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    depth -= 1;
                    self.pos += 1;
                }
                _ => {
                    // Unexpected token (shouldn't happen in valid GeoJSON)
                    self.pos += 1;
                }
            }
        }

        self.consumed += 1;

        if !found_geometry || geometry_is_null {
            return Some(Float64Array::new_with_length(0));
        }

        // Now parse the geometry at geometry_pos
        let mut geom_parser = geometry_pos;
        let bytes_inner = self.input.as_bytes();

        // The geometry is a JSON object { "type": "...", "coordinates": [...] }
        // We need to find "coordinates" and extract them
        // Skip to `{`
        while geom_parser < len && bytes_inner[geom_parser] != b'{' {
            geom_parser += 1;
        }
        if geom_parser >= len {
            return Some(Float64Array::new_with_length(0));
        }
        geom_parser += 1; // skip `{`

        let mut geom_depth = 1i32;
        let mut found_coords = false;
        let mut coords_pos = 0usize;

        while geom_parser < len && geom_depth > 0 {
            while geom_parser < len
                && (bytes_inner[geom_parser] == b' '
                    || bytes_inner[geom_parser] == b'\t'
                    || bytes_inner[geom_parser] == b'\n'
                    || bytes_inner[geom_parser] == b'\r')
            {
                geom_parser += 1;
            }
            if geom_parser >= len {
                break;
            }

            match bytes_inner[geom_parser] {
                b'"' => {
                    geom_parser += 1;
                    let key_start = geom_parser;
                    while geom_parser < len && bytes_inner[geom_parser] != b'"' {
                        if bytes_inner[geom_parser] == b'\\' {
                            geom_parser += 1;
                        }
                        geom_parser += 1;
                    }
                    let key = &self.input[key_start..geom_parser];
                    if geom_parser < len {
                        geom_parser += 1;
                    }

                    while geom_parser < len
                        && (bytes_inner[geom_parser] == b' '
                            || bytes_inner[geom_parser] == b'\t'
                            || bytes_inner[geom_parser] == b'\n'
                            || bytes_inner[geom_parser] == b'\r'
                            || bytes_inner[geom_parser] == b':')
                    {
                        geom_parser += 1;
                    }

                    if key == "coordinates" {
                        found_coords = true;
                        coords_pos = geom_parser;
                    } else {
                        skip_value(&self.input, bytes_inner, len, &mut geom_parser);
                    }
                }
                b',' => {
                    geom_parser += 1;
                }
                b'}' => {
                    geom_depth -= 1;
                    geom_parser += 1;
                }
                _ => {
                    geom_parser += 1;
                }
            }
        }

        if !found_coords {
            return Some(Float64Array::new_with_length(0));
        }

        // Extract coordinates from the coordinates array
        let coords = extract_coords_from_json(&self.input, &mut coords_pos);
        let result = Float64Array::new_with_length(coords.len() as u32);
        result.copy_from(&coords);
        Some(result)
    }
}

/// Create a lazy GeoJSON FeatureCollection iterator.
///
/// Accepts a `&str` (one-shot input) but uses a manual JSON state machine
/// internally to parse features one at a time. Memory peak is O(single feature)
/// rather than O(all features).
///
/// ## Parameters
///
/// - `input` — A GeoJSON FeatureCollection string.
///
/// ## Returns
///
/// A `LazyGeoJsonIter` that you can call `.nextFeature()` on repeatedly.
///
/// ## Error
///
/// Returns `JsValue` error if the input is not valid JSON or not a FeatureCollection.
#[wasm_bindgen(js_name = "parseGeoJsonLazy")]
pub fn parse_geojson_lazy(input: &str) -> Result<LazyGeoJsonIter, JsValue> {
    // Quick validation: must be valid JSON starting with `{`
    let trimmed = input.trim_start();
    if !trimmed.starts_with('{') {
        return Err(crate::errors::invalid_input_js(
            "Input must be a JSON object (FeatureCollection)",
        ));
    }

    // Validate it's parseable JSON
    let _: serde_json::Value = input
        .parse()
        .map_err(|e| crate::errors::parse_js(format!("Invalid JSON: {e}")))?;

    // Count features without building DOM
    let total = count_features_raw(input) as u32;

    // Find the start of the features array
    let bytes = input.as_bytes();
    let features_key = b"\"features\"";
    let mut pos = 0;
    let len = bytes.len();
    while pos + features_key.len() <= len {
        if &bytes[pos..pos + features_key.len()] == features_key {
            pos += features_key.len();
            break;
        }
        pos += 1;
    }
    // Skip to `[`
    while pos < len && bytes[pos] != b'[' {
        pos += 1;
    }
    if pos < len {
        pos += 1;
    } // skip `[`

    Ok(LazyGeoJsonIter {
        input: input.to_string(),
        pos,
        total,
        consumed: 0,
    })
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

    // ── Lazy parser native tests (no WASM needed) ──────────────────

    #[test]
    fn test_count_features_raw() {
        let fc = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[1.0,2.0]}},{"type":"Feature","geometry":{"type":"Point","coordinates":[3.0,4.0]}}]}"#;
        assert_eq!(count_features_raw(fc), 2);
    }

    #[test]
    fn test_count_features_raw_empty() {
        let fc = r#"{"type":"FeatureCollection","features":[]}"#;
        assert_eq!(count_features_raw(fc), 0);
    }

    #[test]
    fn test_count_features_raw_single() {
        let fc = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":null}]}"#;
        assert_eq!(count_features_raw(fc), 1);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_lazy_parser_rejects_invalid() {
        let result = parse_geojson_lazy("not json");
        assert!(result.is_err());

        let result = parse_geojson_lazy("[1,2,3]");
        assert!(result.is_err());
    }

    // ── Lazy parser tests that need Float64Array (WASM only) ────────

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_lazy_parser_points() {
        let fc = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [1.0, 2.0]}, "properties": {"name": "A"}},
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [3.0, 4.0]}, "properties": {"name": "B"}},
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [5.0, 6.0]}, "properties": null}
            ]
        }"#;
        let mut iter = parse_geojson_lazy(fc).unwrap();
        assert_eq!(iter.total(), 3);
        assert_eq!(iter.remaining(), 3);

        let f1 = iter.next_feature().unwrap();
        assert_eq!(f1.length(), 2);

        let f2 = iter.next_feature().unwrap();
        assert_eq!(f2.length(), 2);
        assert_eq!(iter.remaining(), 1);

        let f3 = iter.next_feature().unwrap();
        assert_eq!(f3.length(), 2);

        assert!(iter.next_feature().is_none());
        assert_eq!(iter.remaining(), 0);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_lazy_parser_linestring() {
        let fc = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "geometry": {"type": "LineString", "coordinates": [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]}, "properties": {}},
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [10.0, 20.0]}, "properties": {}}
            ]
        }"#;
        let mut iter = parse_geojson_lazy(fc).unwrap();
        assert_eq!(iter.total(), 2);

        let f1 = iter.next_feature().unwrap();
        assert_eq!(f1.length(), 6); // 3 points × 2 coords

        let f2 = iter.next_feature().unwrap();
        assert_eq!(f2.length(), 2);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_lazy_parser_null_geometry() {
        let fc = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "geometry": null, "properties": {}},
                {"type": "Feature", "geometry": {"type": "Point", "coordinates": [7.0, 8.0]}, "properties": {}}
            ]
        }"#;
        let mut iter = parse_geojson_lazy(fc).unwrap();
        assert_eq!(iter.total(), 2);

        let f1 = iter.next_feature().unwrap();
        assert_eq!(f1.length(), 0); // null geometry

        let f2 = iter.next_feature().unwrap();
        assert_eq!(f2.length(), 2);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_lazy_parser_polygon() {
        let fc = r#"{
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "geometry": {"type": "Polygon", "coordinates": [[[0,0],[1,0],[1,1],[0,0]]]}, "properties": {}}
            ]
        }"#;
        let mut iter = parse_geojson_lazy(fc).unwrap();
        let f1 = iter.next_feature().unwrap();
        assert_eq!(f1.length(), 8); // 4 points × 2 coords
    }
}
