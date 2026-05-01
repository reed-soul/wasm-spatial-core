//! Vector Tile Slicing Engine
//!
//! Provides a WebAssembly-backed vector tile generator using `geojsonvt`.
//! Parses GeoJSON once, then dynamically slices and encodes it into MVT (PBF)
//! buffers based on `z, x, y` tile coordinates.

use wasm_bindgen::prelude::*;
use geojson::GeoJson;
use geojsonvt::{GeoJSONVT, Options as GeoJsonVtOptions};
use geozero::{GeozeroDatasource, geojson::GeoJsonString, mvt::MvtWriter};
use geozero::mvt::{Tile, Message};

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
#[wasm_bindgen]
pub struct VectorTileEngine {
    index: GeoJSONVT,
}

#[wasm_bindgen]
impl VectorTileEngine {
    /// Create a new VectorTileEngine from a GeoJSON string.
    #[wasm_bindgen(constructor)]
    pub fn new(geojson_str: &str, options: VectorTileOptions) -> Result<VectorTileEngine, JsValue> {
        let geojson = geojson_str.parse::<GeoJson>().map_err(|e| JsValue::from_str(&e.to_string()))?;
        let vt_options: GeoJsonVtOptions = options.into();
        let index = GeoJSONVT::from_geojson(&geojson, &vt_options);

        Ok(VectorTileEngine { index })
    }

    /// Request a tile by `z, x, y` coordinates.
    /// Returns a raw `Uint8Array` representing the MVT (PBF) protobuf.
    /// If the tile is empty or out of bounds, returns an empty array.
    #[wasm_bindgen(js_name = "getTile")]
    pub fn get_tile(&mut self, z: u8, x: u32, y: u32) -> Result<js_sys::Uint8Array, JsValue> {
        let tile = self.index.tile(z, x, y);
        
        if tile.feature_collection.features.is_empty() {
            return Ok(js_sys::Uint8Array::new_with_length(0));
        }

        let json_str = serde_json::to_string(&tile.feature_collection)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut geojson_data = GeoJsonString(json_str);
        
        // Use unscaled because geojsonvt has already transformed coordinates into tile pixel space (extent 4096)
        let mut mvt_writer = MvtWriter::new_unscaled(4096)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        geojson_data.process(&mut mvt_writer)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
        let mvt_layer = mvt_writer.layer("default");
        
        let mut mvt_tile = Tile::default();
        mvt_tile.layers.push(mvt_layer);
        
        let mut mvt_bytes = Vec::new();
        mvt_tile.encode(&mut mvt_bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let out_array = js_sys::Uint8Array::new_with_length(mvt_bytes.len() as u32);
        out_array.copy_from(&mvt_bytes);
        
        Ok(out_array)
    }
}
