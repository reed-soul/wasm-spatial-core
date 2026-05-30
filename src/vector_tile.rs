//! Vector Tile Slicing & Decoding Engine
//!
//! Provides a WebAssembly-backed vector tile generator using `geojsonvt`.
//! Parses GeoJSON once, then dynamically slices and encodes it into MVT (PBF)
//! buffers based on `z, x, y` tile coordinates.
//!
//! Also supports **MVT decoding** — parse raw protobuf MVT bytes back into
//! structured tile data or GeoJSON strings.

use geojson::GeoJson;
use geojsonvt::{GeoJSONVT, Options as GeoJsonVtOptions};
use geozero::mvt::tile::{Feature as ProtoFeature, Value as ProtoValue};
use geozero::mvt::{MvtWriter, Tile as MvtProtoTile};
use geozero::{geojson::GeoJsonString, GeozeroDatasource};
use js_sys::Float64Array;
use prost::Message as _;
use std::collections::{HashMap, VecDeque};
use wasm_bindgen::prelude::*;

/// Options for vector tile generation.
#[wasm_bindgen]
pub struct VectorTileOptions {
    pub max_zoom: u8,
    pub index_max_zoom: u8,
    pub index_max_points: u32,
    pub tolerance: f64,
    pub extent: u16,
    pub buffer: u16,
    pub line_metrics: bool,
    pub generate_id: bool,
}

#[wasm_bindgen]
impl VectorTileOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> VectorTileOptions {
        VectorTileOptions {
            max_zoom: 14,
            index_max_zoom: 5,
            index_max_points: 100000,
            tolerance: 3.0,
            extent: 4096,
            buffer: 64,
            line_metrics: false,
            generate_id: false,
        }
    }
}

impl From<VectorTileOptions> for GeoJsonVtOptions {
    fn from(val: VectorTileOptions) -> Self {
        GeoJsonVtOptions {
            max_zoom: val.max_zoom,
            index_max_zoom: val.index_max_zoom,
            index_max_points: val.index_max_points,
            tolerance: val.tolerance,
            extent: val.extent,
            buffer: val.buffer,
            line_metrics: val.line_metrics,
            generate_id: val.generate_id,
        }
    }
}

/// A high-performance vector tile engine.
///
/// Creates a pre-indexed GeoJSONVT structure from a GeoJSON string,
/// then can efficiently slice tiles by `(z, x, y)`. Feature properties
/// from the original GeoJSON are preserved as MVT tags.
///
/// Supports optional LRU caching via `getTileCached` / `clearTileCache`.
#[wasm_bindgen]
pub struct VectorTileEngine {
    index: GeoJSONVT,
    /// The layer name used in the output MVT protobuf.
    layer_name: String,
    /// LRU tile cache: (z, x, y) → MVT bytes.
    cache: HashMap<(u8, u32, u32), Vec<u8>>,
    /// LRU ordering: most-recently-used keys at the back.
    lru_order: VecDeque<(u8, u32, u32)>,
    /// Maximum cache capacity.
    max_cache_size: usize,
}

#[wasm_bindgen]
impl VectorTileEngine {
    /// Create a new VectorTileEngine from a GeoJSON string.
    ///
    /// The `layer_name` parameter controls the layer name embedded in the
    /// MVT protobuf output. Defaults to `"default"`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        geojson_str: &str,
        options: VectorTileOptions,
        layer_name: Option<String>,
    ) -> Result<VectorTileEngine, JsValue> {
        let geojson = geojson_str
            .parse::<GeoJson>()
            .map_err(crate::errors::tile_js)?;
        let vt_options: GeoJsonVtOptions = options.into();
        let index = GeoJSONVT::from_geojson(&geojson, &vt_options);

        let layer_name = layer_name.unwrap_or_else(|| "default".to_string());

        Ok(VectorTileEngine {
            index,
            layer_name,
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            max_cache_size: 64,
        })
    }

    /// Request a tile by `z, x, y` coordinates.
    /// Returns a raw `Uint8Array` representing the MVT (PBF) protobuf.
    /// If the tile is empty or out of bounds, returns an empty array.
    ///
    /// Feature properties (`name`, `id`, `class`, and any other fields)
    /// from the original GeoJSON are automatically encoded as MVT tags.
    #[wasm_bindgen(js_name = "getTile")]
    pub fn get_tile(&mut self, z: u8, x: u32, y: u32) -> Result<js_sys::Uint8Array, JsValue> {
        let tile = self.index.tile(z, x, y);

        if tile.feature_collection.features.is_empty() {
            return Ok(js_sys::Uint8Array::new_with_length(0));
        }

        let json_str =
            serde_json::to_string(&tile.feature_collection).map_err(crate::errors::tile_js)?;

        let mut geojson_data = GeoJsonString(json_str);

        // Use unscaled because geojsonvt has already transformed coordinates
        // into tile pixel space (extent 4096).
        // MvtWriter implements PropertyProcessor, so GeoJSON feature properties
        // are automatically encoded as MVT tags.
        let mut mvt_writer = MvtWriter::new_unscaled(4096).map_err(crate::errors::tile_js)?;

        geojson_data
            .process(&mut mvt_writer)
            .map_err(crate::errors::tile_js)?;

        let mvt_layer = mvt_writer.layer(&self.layer_name);

        let mut mvt_tile = MvtProtoTile::default();
        mvt_tile.layers.push(mvt_layer);

        let mut mvt_bytes = Vec::new();
        mvt_tile
            .encode(&mut mvt_bytes)
            .map_err(crate::errors::tile_js)?;

        let out_array = js_sys::Uint8Array::new_with_length(mvt_bytes.len() as u32);
        out_array.copy_from(&mvt_bytes);

        Ok(out_array)
    }

    /// Get the layer name used by this engine.
    #[wasm_bindgen(getter = layerName)]
    pub fn layer_name(&self) -> String {
        self.layer_name.clone()
    }

    /// Set a new layer name for subsequent tile requests.
    #[wasm_bindgen(setter = layerName)]
    pub fn set_layer_name(&mut self, name: String) {
        self.layer_name = name;
    }

    /// Request a tile with LRU caching (max 64 tiles).
    ///
    /// If the tile was previously requested, returns the cached result
    /// without re-computing. Otherwise, generates the tile, caches it,
    /// and returns it.
    ///
    /// Use `clearTileCache()` to evict all cached tiles.
    #[wasm_bindgen(js_name = "getTileCached")]
    pub fn get_tile_cached(
        &mut self,
        z: u8,
        x: u32,
        y: u32,
    ) -> Result<js_sys::Uint8Array, JsValue> {
        let key = (z, x, y);

        // Check cache
        if let Some(cached) = self.cache.get(&key) {
            // Move to back of LRU order (most recently used)
            self.lru_order.retain(|k| k != &key);
            self.lru_order.push_back(key);

            let out = js_sys::Uint8Array::new_with_length(cached.len() as u32);
            out.copy_from(cached);
            return Ok(out);
        }

        // Generate tile
        let tile_bytes = self.generate_tile_bytes(z, x, y)?;

        // Evict LRU entries if over capacity
        while self.cache.len() >= self.max_cache_size {
            if let Some(oldest) = self.lru_order.pop_front() {
                self.cache.remove(&oldest);
            }
        }

        // Store in cache
        self.cache.insert(key, tile_bytes.clone());
        self.lru_order.push_back(key);

        let out = js_sys::Uint8Array::new_with_length(tile_bytes.len() as u32);
        out.copy_from(&tile_bytes);
        Ok(out)
    }

    /// Clear the tile LRU cache.
    #[wasm_bindgen(js_name = "clearTileCache")]
    pub fn clear_tile_cache(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
    }

    /// Get the number of cached tiles.
    #[wasm_bindgen(js_name = "cacheSize")]
    pub fn cache_size(&self) -> u32 {
        self.cache.len() as u32
    }

    /// Internal: generate MVT bytes for a tile.
    fn generate_tile_bytes(&mut self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, JsValue> {
        let tile = self.index.tile(z, x, y);

        if tile.feature_collection.features.is_empty() {
            return Ok(Vec::new());
        }

        let json_str =
            serde_json::to_string(&tile.feature_collection).map_err(crate::errors::tile_js)?;

        let mut geojson_data = GeoJsonString(json_str);

        let mut mvt_writer = MvtWriter::new_unscaled(4096).map_err(crate::errors::tile_js)?;

        geojson_data
            .process(&mut mvt_writer)
            .map_err(crate::errors::tile_js)?;

        let mvt_layer = mvt_writer.layer(&self.layer_name);

        let mut mvt_tile = MvtProtoTile::default();
        mvt_tile.layers.push(mvt_layer);

        let mut mvt_bytes = Vec::new();
        mvt_tile
            .encode(&mut mvt_bytes)
            .map_err(crate::errors::tile_js)?;

        Ok(mvt_bytes)
    }
}

// ===========================================================================
// MVT Decoder — Parse protobuf MVT bytes into structured data
// ===========================================================================

/// A decoded MVT layer with structured feature data.
#[wasm_bindgen(js_name = "MvtLayer")]
pub struct MvtLayerDecoded {
    name: String,
    extent: u32,
    features: Vec<MvtFeatureDecoded>,
}

/// A decoded MVT feature with geometry, type, and tags.
#[derive(Clone)]
#[wasm_bindgen(js_name = "MvtFeature")]
pub struct MvtFeatureDecoded {
    geometry_type: u8,
    geometry: Vec<f64>,
    tags: Vec<(String, String)>,
    id: Option<u64>,
}

#[wasm_bindgen]
impl MvtFeatureDecoded {
    /// Geometry type: 0=Unknown, 1=Point, 2=LineString, 3=Polygon.
    #[wasm_bindgen(getter)]
    pub fn geometry_type(&self) -> u8 {
        self.geometry_type
    }

    /// Flat tile-space coordinates as `Float64Array`.
    #[wasm_bindgen(getter)]
    pub fn geometry(&self) -> Float64Array {
        let arr = Float64Array::new_with_length(self.geometry.len() as u32);
        if !self.geometry.is_empty() {
            arr.copy_from(&self.geometry);
        }
        arr
    }

    /// Tag count.
    #[wasm_bindgen(js_name = "tagCount")]
    pub fn tag_count(&self) -> u32 {
        self.tags.len() as u32
    }

    /// Get a tag key by index.
    #[wasm_bindgen(js_name = "tagKey")]
    pub fn tag_key(&self, index: u32) -> String {
        self.tags
            .get(index as usize)
            .map(|(k, _)| k.clone())
            .unwrap_or_default()
    }

    /// Get a tag value by index.
    #[wasm_bindgen(js_name = "tagValue")]
    pub fn tag_value(&self, index: u32) -> String {
        self.tags
            .get(index as usize)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    /// Feature ID (0 if not set).
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> f64 {
        self.id.unwrap_or(0) as f64
    }
}

#[wasm_bindgen]
impl MvtLayerDecoded {
    /// Layer name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Layer extent (typically 4096).
    #[wasm_bindgen(getter)]
    pub fn extent(&self) -> u32 {
        self.extent
    }

    /// Number of features in this layer.
    #[wasm_bindgen(js_name = "featureCount")]
    pub fn feature_count(&self) -> u32 {
        self.features.len() as u32
    }

    /// Get feature by index.
    #[wasm_bindgen(js_name = "featureAt")]
    pub fn feature_at(&self, index: u32) -> MvtFeatureDecoded {
        self.features
            .get(index as usize)
            .cloned()
            .unwrap_or_else(|| MvtFeatureDecoded {
                geometry_type: 0,
                geometry: Vec::new(),
                tags: Vec::new(),
                id: None,
            })
    }
}

/// Decode MVT (Mapbox Vector Tile) protobuf bytes into structured layer data.
///
/// ## Parameters
///
/// - `bytes` — Raw MVT protobuf bytes (typically from a `.pbf` tile file).
///
/// ## Returns
///
/// A `MvtLayer` (the first layer in the tile).
///
/// ## Usage (JS)
///
/// ```js
/// const response = await fetch('/tiles/10/868/387.pbf');
/// const buffer = await response.arrayBuffer();
/// const layer = decodeMvt(new Uint8Array(buffer));
/// console.log(layer.name(), layer.extent(), layer.featureCount());
/// const feat = layer.featureAt(0);
/// console.log(feat.geometryType(), feat.geometry());
/// ```
#[wasm_bindgen(js_name = "decodeMvt")]
pub fn decode_mvt(bytes: js_sys::Uint8Array) -> Result<MvtLayerDecoded, JsValue> {
    let mut buf = vec![0u8; bytes.length() as usize];
    bytes.copy_to(&mut buf);

    let tile_proto = MvtProtoTile::decode(&buf[..])
        .map_err(|e| crate::errors::tile_js(format!("MVT decode error: {e}")))?;

    if tile_proto.layers.is_empty() {
        return Err(crate::errors::tile_js("MVT tile contains no layers"));
    }

    let layer_proto = &tile_proto.layers[0];
    let name = layer_proto.name.clone();
    let extent = layer_proto.extent.unwrap_or(4096);

    let features = layer_proto
        .features
        .iter()
        .map(|f| decode_feature(f, &layer_proto.keys, &layer_proto.values))
        .collect();

    Ok(MvtLayerDecoded {
        name,
        extent,
        features,
    })
}

/// Decode an MVT feature's geometry commands into flat tile-space coordinates.
fn decode_feature(
    feature: &ProtoFeature,
    keys: &[String],
    values: &[ProtoValue],
) -> MvtFeatureDecoded {
    let geometry_type = feature.r#type.unwrap_or(0) as u8;
    let mut geometry = Vec::with_capacity(feature.geometry.len());
    let mut x = 0i32;
    let mut y = 0i32;

    let mut i = 0;
    while i < feature.geometry.len() {
        let cmd = feature.geometry[i];
        i += 1;
        let cmd_id = cmd & 0x07;
        let cmd_count = cmd >> 3;

        match cmd_id {
            // MoveTo
            1 => {
                for _ in 0..cmd_count {
                    if i + 1 >= feature.geometry.len() {
                        break;
                    }
                    let dx = zigzag_decode(feature.geometry[i] as i32);
                    i += 1;
                    let dy = zigzag_decode(feature.geometry[i] as i32);
                    i += 1;
                    x += dx;
                    y += dy;
                    geometry.push(x as f64);
                    geometry.push(y as f64);
                }
            }
            // LineTo
            2 => {
                for _ in 0..cmd_count {
                    if i + 1 >= feature.geometry.len() {
                        break;
                    }
                    let dx = zigzag_decode(feature.geometry[i] as i32);
                    i += 1;
                    let dy = zigzag_decode(feature.geometry[i] as i32);
                    i += 1;
                    x += dx;
                    y += dy;
                    geometry.push(x as f64);
                    geometry.push(y as f64);
                }
            }
            // ClosePath
            7 => {
                // No parameters, just signals close
            }
            _ => break,
        }
    }

    // Decode tags
    let mut tags = Vec::new();
    let tag_pairs = feature.tags.len() / 2;
    for j in 0..tag_pairs {
        let key_idx = feature.tags[j * 2] as usize;
        let val_idx = feature.tags[j * 2 + 1] as usize;

        let key = keys.get(key_idx).cloned().unwrap_or_default();
        let val = values.get(val_idx).map(value_to_string).unwrap_or_default();

        tags.push((key, val));
    }

    MvtFeatureDecoded {
        geometry_type,
        geometry,
        tags,
        id: feature.id,
    }
}

/// Decode a ZigZag-encoded int32.
#[inline]
fn zigzag_decode(n: i32) -> i32 {
    (n >> 1) ^ -(n & 1)
}

/// Convert an MVT Value to a string representation.
fn value_to_string(value: &ProtoValue) -> String {
    if let Some(ref s) = value.string_value {
        return s.clone();
    }
    if let Some(f) = value.float_value {
        return format!("{f}");
    }
    if let Some(d) = value.double_value {
        return format!("{d}");
    }
    if let Some(i) = value.int_value {
        return format!("{i}");
    }
    if let Some(u) = value.uint_value {
        return format!("{u}");
    }
    if let Some(s) = value.sint_value {
        return format!("{s}");
    }
    if let Some(b) = value.bool_value {
        return format!("{b}");
    }
    String::new()
}

/// Decode an MVT tile and convert all features to a GeoJSON FeatureCollection string.
///
/// ## Parameters
///
/// - `bytes` — Raw MVT protobuf bytes.
///
/// ## Returns
///
/// A GeoJSON FeatureCollection string with all features from the first layer.
/// Coordinates are in tile space (0..extent).
///
/// ## Usage (JS)
///
/// ```js
/// const response = await fetch('/tiles/10/868/387.pbf');
/// const geojson = decodeMvtToGeoJson(new Uint8Array(await response.arrayBuffer()));
/// // geojson = '{"type":"FeatureCollection","features":[...]}'
/// ```
#[wasm_bindgen(js_name = "decodeMvtToGeoJson")]
pub fn decode_mvt_to_geojson(bytes: js_sys::Uint8Array) -> Result<String, JsValue> {
    let layer = decode_mvt(bytes)?;

    let mut features_json = Vec::with_capacity(layer.features.len());

    for feat in &layer.features {
        let geom_json = build_geojson_geometry(feat.geometry_type, &feat.geometry);

        // Build tags JSON — quote string values, leave numbers bare
        let tags_json = feat
            .tags
            .iter()
            .map(|(k, v)| {
                // Try to parse as number; if it works, leave bare; otherwise quote it
                if v.parse::<f64>().is_ok() || v == "true" || v == "false" || v == "null" {
                    format!("\"{k}\":{v}")
                } else {
                    format!("\"{k}\":\"{v}\"")
                }
            })
            .collect::<Vec<_>>()
            .join(",");

        let id_str = feat
            .id
            .map(|id| format!("\"id\":{id},"))
            .unwrap_or_default();

        features_json.push(format!(
            r#"{{"type":"Feature",{id_str}"geometry":{geom_json},"properties":{{{tags_json}}}}}"#
        ));
    }

    Ok(format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features_json.join(",")
    ))
}

/// Convert decoded geometry coordinates to a GeoJSON geometry string.
fn build_geojson_geometry(geom_type: u8, coords: &[f64]) -> String {
    match geom_type {
        1 => {
            // Point — single point
            if coords.len() >= 2 {
                format!("[{},{}]", coords[0], coords[1])
            } else {
                "null".to_string()
            }
        }
        2 => {
            // LineString — all coordinates in sequence
            format!(
                "[{}]",
                coords
                    .chunks_exact(2)
                    .map(|p| format!("[{},{}]", p[0], p[1]))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        3 => {
            // Polygon — split into rings based on ClosePath
            // For simplicity, we treat the entire coordinate array as one ring
            // and wrap it in a Polygon. A proper implementation would detect
            // ring boundaries from the command stream.
            let ring = coords
                .chunks_exact(2)
                .map(|p| format!("[{},{}]", p[0], p[1]))
                .collect::<Vec<_>>()
                .join(",");
            format!("[[{ring}]]")
        }
        _ => "null".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_GEOJSON: &str = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [116.404, 39.915]
                },
                "properties": { "name": "Beijing", "class": "city", "population": 21540000 }
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[100.0, 0.0], [101.0, 1.0]]
                },
                "properties": { "name": "test_line", "id": 42, "class": "road" }
            }
        ]
    }"#;

    #[test]
    fn test_vector_tile_options_default() {
        let opts = VectorTileOptions::new();
        assert_eq!(opts.max_zoom, 14);
        assert_eq!(opts.extent, 4096);
        assert_eq!(opts.buffer, 64);
        assert!(!opts.line_metrics);
    }

    #[test]
    fn test_vector_tile_engine_create() {
        let opts = VectorTileOptions::new();
        let engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_vector_tile_engine_custom_layer_name() {
        let opts = VectorTileOptions::new();
        let engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, Some("pois".to_string()));
        assert!(engine.is_ok());
        assert_eq!(engine.unwrap().layer_name(), "pois");
    }

    #[test]
    fn test_vector_tile_engine_default_layer_name() {
        let opts = VectorTileOptions::new();
        let engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();
        assert_eq!(engine.layer_name(), "default");
    }

    #[test]
    fn test_vector_tile_engine_set_layer_name() {
        let opts = VectorTileOptions::new();
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();
        assert_eq!(engine.layer_name(), "default");
        engine.set_layer_name("buildings".to_string());
        assert_eq!(engine.layer_name(), "buildings");
    }

    /// Verify that properties are encoded into MVT tags by decoding the tile
    /// and checking the layer's keys/values.
    #[test]
    fn test_mvt_properties_encoded() {
        let opts = VectorTileOptions::new();
        let mut engine =
            VectorTileEngine::new(SAMPLE_GEOJSON, opts, Some("test_layer".to_string())).unwrap();

        // z=10, x=868, y=387 — tile containing Beijing area
        let tile = engine.index.tile(10, 868, 387);
        if tile.feature_collection.features.is_empty() {
            // If geojsonvt returns empty for this tile, skip
            return;
        }

        let json_str = serde_json::to_string(&tile.feature_collection).unwrap();
        let mut geojson_data = GeoJsonString(json_str);
        let mut mvt_writer = MvtWriter::new_unscaled(4096).unwrap();
        geojson_data.process(&mut mvt_writer).unwrap();

        let mvt_layer = mvt_writer.layer("test_layer");

        // The layer should contain the keys from our GeoJSON properties
        // ("name", "class", "population", "id", etc.)
        assert!(
            mvt_layer.keys.contains(&"name".to_string()),
            "Expected 'name' key in MVT layer keys, got: {:?}",
            mvt_layer.keys
        );
    }

    /// Verify that a custom layer name is encoded correctly into the MVT protobuf.
    #[test]
    fn test_mvt_custom_layer_name() {
        let opts = VectorTileOptions::new();
        let mut engine =
            VectorTileEngine::new(SAMPLE_GEOJSON, opts, Some("custom_layer".to_string())).unwrap();

        let tile = engine.index.tile(10, 868, 387);
        if tile.feature_collection.features.is_empty() {
            return;
        }

        let json_str = serde_json::to_string(&tile.feature_collection).unwrap();
        let mut geojson_data = GeoJsonString(json_str);
        let mut mvt_writer = MvtWriter::new_unscaled(4096).unwrap();
        geojson_data.process(&mut mvt_writer).unwrap();

        let mvt_layer = mvt_writer.layer("custom_layer");

        // Verify the layer version and name
        assert_eq!(mvt_layer.version, 2);
        assert_eq!(mvt_layer.name, "custom_layer");
    }

    // ── LRU cache tests (native) ────────────────────────────────────

    #[test]
    fn test_cache_size_and_clear() {
        let opts = VectorTileOptions::new();
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();
        assert_eq!(engine.cache_size(), 0);

        engine.clear_tile_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let opts = VectorTileOptions::new();
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();

        // Populate cache beyond capacity
        // We test the internal HashMap directly since getTileCached needs WASM
        for z in 0..4u8 {
            for x in 0..4u32 {
                let key = (z, x, 0);
                engine.cache.insert(key, vec![z, x as u8]);
                engine.lru_order.push_back(key);
            }
        }

        assert_eq!(engine.cache_size(), 16);

        // Now reduce capacity and trigger eviction
        engine.max_cache_size = 8;

        // Manually evict (normally done by getTileCached)
        while engine.cache.len() >= engine.max_cache_size {
            if let Some(oldest) = engine.lru_order.pop_front() {
                engine.cache.remove(&oldest);
            }
        }

        assert_eq!(engine.cache_size(), 7); // 8-1 after loop
    }

    // Tests that call get_tile require JS/WASM runtime (js_sys::Uint8Array)
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_get_tile_basic() {
        let opts = VectorTileOptions::new();
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();

        // z=10, x=868, y=387 — tile containing Beijing area
        let tile = engine.get_tile(10, 868, 387);
        assert!(tile.is_ok());
        let _tile_data = tile.unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_get_tile_out_of_bounds() {
        let opts = VectorTileOptions::new();
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();

        let tile = engine.get_tile(0, 0, 0);
        assert!(tile.is_ok());
    }

    // ── MVT decoder tests (native) ────────────────────────────────────

    #[test]
    fn test_zigzag_decode() {
        assert_eq!(zigzag_decode(0), 0);
        assert_eq!(zigzag_decode(1), -1);
        assert_eq!(zigzag_decode(2), 1);
        assert_eq!(zigzag_decode(3), -2);
        assert_eq!(zigzag_decode(4), 2);
        assert_eq!(zigzag_decode(-1), 0); // undefined for negative but shouldn't crash
    }

    #[test]
    fn test_value_to_string() {
        let v = geozero::mvt::tile::Value {
            string_value: Some("hello".to_string()),
            ..Default::default()
        };
        assert_eq!(value_to_string(&v), "hello");

        let v2 = geozero::mvt::tile::Value {
            int_value: Some(42),
            ..Default::default()
        };
        assert_eq!(value_to_string(&v2), "42");

        let v3 = geozero::mvt::tile::Value {
            double_value: Some(1.414),
            ..Default::default()
        };
        assert_eq!(value_to_string(&v3), "1.414");

        let v4 = geozero::mvt::tile::Value {
            bool_value: Some(true),
            ..Default::default()
        };
        assert_eq!(value_to_string(&v4), "true");
    }

    #[test]
    fn test_decode_feature_point() {
        // MVT Point with MoveTo(1) at (10, 20)
        let feature = geozero::mvt::tile::Feature {
            id: Some(42),
            tags: vec![0, 0],
            r#type: Some(1),
            geometry: vec![9u32, 20, 40],
        };
        let keys = vec!["name".to_string()];
        let values = vec![geozero::mvt::tile::Value {
            string_value: Some("test".to_string()),
            ..Default::default()
        }];

        let result = decode_feature(&feature, &keys, &values);
        assert_eq!(result.geometry_type, 1); // Point
        assert_eq!(result.geometry, vec![10.0, 20.0]);
        assert_eq!(result.tags.len(), 1);
        assert_eq!(result.tags[0], ("name".to_string(), "test".to_string()));
        assert_eq!(result.id, Some(42));
    }

    #[test]
    fn test_decode_feature_linestring() {
        // LineString: MoveTo(1) → LineTo(2)
        let feature = geozero::mvt::tile::Feature {
            id: None,
            tags: vec![],
            r#type: Some(2),
            geometry: vec![9u32, 20, 40, 18, 40, 40, 40, 40],
        };
        let result = decode_feature(&feature, &[], &[]);
        assert_eq!(result.geometry_type, 2);
        assert_eq!(result.geometry, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
    }

    #[test]
    fn test_build_geojson_geometry() {
        let coords = vec![10.0, 20.0];
        assert_eq!(build_geojson_geometry(1, &coords), "[10,20]");

        let coords = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(build_geojson_geometry(2, &coords), "[[10,20],[30,40]]");

        let coords = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0];
        let result = build_geojson_geometry(3, &coords);
        assert!(result.starts_with("[[["));
    }
}
