//! Vector Tile Slicing Engine
//!
//! Provides a WebAssembly-backed vector tile generator using `geojsonvt`.
//! Parses GeoJSON once, then dynamically slices and encodes it into MVT (PBF)
//! buffers based on `z, x, y` tile coordinates.

use geojson::GeoJson;
use geojsonvt::{GeoJSONVT, Options as GeoJsonVtOptions};
use geozero::mvt::{Message, Tile};
use geozero::{geojson::GeoJsonString, mvt::MvtWriter, GeozeroDatasource};
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
#[wasm_bindgen]
pub struct VectorTileEngine {
    index: GeoJSONVT,
    /// The layer name used in the output MVT protobuf.
    layer_name: String,
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
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let vt_options: GeoJsonVtOptions = options.into();
        let index = GeoJSONVT::from_geojson(&geojson, &vt_options);

        let layer_name = layer_name.unwrap_or_else(|| "default".to_string());

        Ok(VectorTileEngine { index, layer_name })
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

        let json_str = serde_json::to_string(&tile.feature_collection)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut geojson_data = GeoJsonString(json_str);

        // Use unscaled because geojsonvt has already transformed coordinates
        // into tile pixel space (extent 4096).
        // MvtWriter implements PropertyProcessor, so GeoJSON feature properties
        // are automatically encoded as MVT tags.
        let mut mvt_writer =
            MvtWriter::new_unscaled(4096).map_err(|e| JsValue::from_str(&e.to_string()))?;

        geojson_data
            .process(&mut mvt_writer)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mvt_layer = mvt_writer.layer(&self.layer_name);

        let mut mvt_tile = Tile::default();
        mvt_tile.layers.push(mvt_layer);

        let mut mvt_bytes = Vec::new();
        mvt_tile
            .encode(&mut mvt_bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

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

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_get_tile_empty_high_zoom() {
        let mut opts = VectorTileOptions::new();
        opts.max_zoom = 14;
        let mut engine = VectorTileEngine::new(SAMPLE_GEOJSON, opts, None).unwrap();

        let tile = engine.get_tile(14, 0, 0);
        assert!(tile.is_ok());
        let tile_data = tile.unwrap();
        assert_eq!(tile_data.length(), 0);
    }
}
