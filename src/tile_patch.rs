//! Incremental tileset patching (Wave 1.1).
//!
//! URI stability: leaf tiles use `tile_{leaf_idx}.pnts` from `generate_tileset*`.
//! Patches replace content by URI key; unrelated URIs and blobs are preserved.

use std::collections::HashMap;

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::pnts::{TilesetResult, WasmTilesetResult};

/// Incremental tileset update — replace specific tile blobs and optionally tileset.json.
#[derive(Debug, Clone, Default)]
pub struct TilesetPatch {
    /// URI → new tile binary content.
    pub replaced_tiles: HashMap<String, Vec<u8>>,
    /// When set, replaces the entire tileset.json (e.g. after bounds/error change).
    pub tileset_json: Option<String>,
}

impl TilesetPatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace a single tile by URI.
    pub fn replace_tile(&mut self, uri: impl Into<String>, data: Vec<u8>) {
        self.replaced_tiles.insert(uri.into(), data);
    }

    /// Replace tileset.json wholesale.
    pub fn set_tileset_json(&mut self, json: impl Into<String>) {
        self.tileset_json = Some(json.into());
    }

    /// Approximate serialized patch size (for acceptance tests).
    pub fn patch_bytes(&self) -> usize {
        let tiles: usize = self.replaced_tiles.values().map(|b| b.len()).sum();
        let json = self.tileset_json.as_ref().map(|s| s.len()).unwrap_or(0);
        tiles + json
    }
}

/// Apply a patch to an existing tileset. Unrelated URIs and tile blobs are unchanged.
pub fn apply_patch(
    base: &TilesetResult,
    patch: &TilesetPatch,
) -> Result<TilesetResult, SpatialErrorDetail> {
    if patch.replaced_tiles.is_empty() && patch.tileset_json.is_none() {
        return Err(SpatialError::InvalidInput.with_detail("patch is empty"));
    }

    let mut tiles = base.tiles.clone();
    let tile_bounds = base.tile_bounds.clone();
    let tile_uris = base.tile_uris.clone();

    for (uri, data) in &patch.replaced_tiles {
        let idx = tile_uris.iter().position(|u| u == uri).ok_or_else(|| {
            SpatialError::TileError.with_detail(format!("unknown tile URI in patch: {uri}"))
        })?;
        tiles[idx] = data.clone();
    }

    if let Some(json) = &patch.tileset_json {
        validate_tileset_json_uris(json, &tile_uris)?;
    }

    let tileset_json = patch
        .tileset_json
        .clone()
        .unwrap_or_else(|| base.tileset_json.clone());

    Ok(TilesetResult {
        tileset_json,
        tiles,
        tile_bounds,
        tile_uris,
    })
}

/// Collect leaf `content.uri` values from a tileset.json document.
fn leaf_uris_from_tileset_json(json: &str) -> Result<Vec<String>, SpatialErrorDetail> {
    let val: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| SpatialError::TileError.with_detail(format!("invalid tileset JSON: {e}")))?;
    let root = val
        .get("root")
        .ok_or_else(|| SpatialError::TileError.with_detail("tileset JSON missing root node"))?;
    let mut uris = Vec::new();
    collect_content_uris(root, &mut uris);
    uris.sort();
    Ok(uris)
}

fn collect_content_uris(node: &serde_json::Value, uris: &mut Vec<String>) {
    if let Some(content) = node.get("content") {
        if let Some(uri) = content.get("uri").and_then(|u| u.as_str()) {
            uris.push(uri.to_string());
        }
    }
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child in children {
            collect_content_uris(child, uris);
        }
    }
}

/// Ensure a replacement tileset.json references the same leaf URIs as the tile blobs.
fn validate_tileset_json_uris(json: &str, tile_uris: &[String]) -> Result<(), SpatialErrorDetail> {
    let json_uris = leaf_uris_from_tileset_json(json)?;
    let mut expected: Vec<String> = tile_uris.to_vec();
    expected.sort();
    if json_uris != expected {
        return Err(SpatialError::TileError.with_detail(format!(
            "tileset JSON URIs {json_uris:?} do not match tile URIs {expected:?}"
        )));
    }
    Ok(())
}

// ===========================================================================
// WASM API
// ===========================================================================

#[wasm_bindgen(js_name = "TilesetPatch")]
pub struct WasmTilesetPatch {
    inner: TilesetPatch,
}

impl Default for WasmTilesetPatch {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = "TilesetPatch")]
impl WasmTilesetPatch {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: TilesetPatch::new(),
        }
    }

    /// Replace tile content for a URI (e.g. `tile_3.pnts`).
    #[wasm_bindgen(js_name = "setTile")]
    pub fn set_tile(&mut self, uri: &str, data: &[u8]) {
        self.inner.replace_tile(uri, data.to_vec());
    }

    /// Replace tileset.json content.
    #[wasm_bindgen(js_name = "setTilesetJson")]
    pub fn set_tileset_json(&mut self, json: &str) {
        self.inner.set_tileset_json(json);
    }

    /// Approximate patch payload size in bytes.
    #[wasm_bindgen(getter, js_name = "patchBytes")]
    pub fn patch_bytes(&self) -> usize {
        self.inner.patch_bytes()
    }
}

/// Apply an incremental patch to a tileset result.
#[wasm_bindgen(js_name = "applyTilesetPatch")]
pub fn apply_tileset_patch_js(
    base: &WasmTilesetResult,
    patch: &WasmTilesetPatch,
) -> Result<WasmTilesetResult, JsValue> {
    apply_patch(base.inner(), &patch.inner)
        .map(WasmTilesetResult::from_inner)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::Octree;
    use crate::pnts::generate_tileset;

    fn sample_tileset() -> TilesetResult {
        let positions: Vec<f32> = (0..300)
            .flat_map(|i| {
                let f = i as f32 * 0.1;
                [f, f * 0.5, f * 0.25]
            })
            .collect();
        let mut positions = positions;
        let tree = Octree::build(&mut positions, 100, 8);
        generate_tileset(&tree, &positions, None).unwrap()
    }

    #[test]
    fn test_apply_patch_single_tile() {
        let base = sample_tileset();
        let full_bytes = base.total_bytes() + base.tileset_json().len();
        assert!(base.tile_count() >= 2);

        let target_uri = base.tile_uri(0).unwrap().to_string();
        let other_uri = base.tile_uri(1).unwrap().to_string();
        let original_other = base.tile(1).unwrap().to_vec();

        let mut patch = TilesetPatch::new();
        patch.replace_tile(&target_uri, b"patched-tile-content".to_vec());

        let patched = apply_patch(&base, &patch).unwrap();

        assert_eq!(patched.tile_uri(0), Some(target_uri.as_str()));
        assert_eq!(patched.tile(0), Some(b"patched-tile-content".as_slice()));
        assert_eq!(patched.tile_uri(1), Some(other_uri.as_str()));
        assert_eq!(patched.tile(1), Some(original_other.as_slice()));
        assert!(patch.patch_bytes() < full_bytes);
    }

    #[test]
    fn test_patch_unknown_uri_errors() {
        let base = sample_tileset();
        let mut patch = TilesetPatch::new();
        patch.replace_tile("missing.pnts", vec![1, 2, 3]);
        assert!(apply_patch(&base, &patch).is_err());
    }

    #[test]
    fn test_patch_tileset_json_uri_mismatch_errors() {
        let base = sample_tileset();
        let mut patch = TilesetPatch::new();
        patch.set_tileset_json(r#"{"root":{"content":{"uri":"wrong.pnts"}}}"#.to_string());
        assert!(apply_patch(&base, &patch).is_err());
    }

    #[test]
    fn test_patch_tileset_json_uri_match_ok() {
        let base = sample_tileset();
        let mut patch = TilesetPatch::new();
        patch.set_tileset_json(base.tileset_json().to_string());
        assert!(apply_patch(&base, &patch).is_ok());
    }
}
