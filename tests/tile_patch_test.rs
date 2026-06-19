//! Integration tests for incremental tileset patching (Wave 1.1).

use wasm_spatial_core::{apply_patch, generate_tileset, Octree, TilesetPatch};

fn sample_tileset() -> wasm_spatial_core::TilesetResult {
    let positions: Vec<f32> = (0..600)
        .flat_map(|i| {
            let f = i as f32 * 0.05;
            [f, f * 0.3, f * 0.1]
        })
        .collect();
    let mut positions = positions;
    let tree = Octree::build(&mut positions, 80, 8);
    generate_tileset(&tree, &positions, None).unwrap()
}

#[test]
fn test_single_tile_patch_preserves_other_uris() {
    let base = sample_tileset();
    assert!(base.tile_count() >= 2);

    let uri0 = base.tile_uri(0).unwrap().to_string();
    let uri1 = base.tile_uri(1).unwrap().to_string();
    let blob1 = base.tile(1).unwrap().to_vec();
    let full_size = base.total_bytes() + base.tileset_json().len();

    let mut patch = TilesetPatch::new();
    patch.replace_tile(&uri0, b"replacement-pnts".to_vec());

    let patched = apply_patch(&base, &patch).unwrap();

    assert_eq!(patched.tile_uri(0), Some(uri0.as_str()));
    assert_eq!(patched.tile(0), Some(b"replacement-pnts".as_slice()));
    assert_eq!(patched.tile_uri(1), Some(uri1.as_str()));
    assert_eq!(patched.tile(1), Some(blob1.as_slice()));
    assert!(patch.patch_bytes() < full_size);
}

#[test]
fn test_patch_tileset_json_rejects_uri_mismatch() {
    let base = sample_tileset();
    let mut patch = TilesetPatch::new();
    patch.set_tileset_json(r#"{"asset":{"version":"1.0"}}"#);

    let err = apply_patch(&base, &patch).unwrap_err();
    assert_eq!(err.code(), "TILE_ERROR");
}

#[test]
fn test_patch_tileset_json_with_matching_uris() {
    let base = sample_tileset();
    let mut patch = TilesetPatch::new();
    patch.set_tileset_json(base.tileset_json().to_string());

    let patched = apply_patch(&base, &patch).unwrap();
    assert_eq!(patched.tileset_json(), base.tileset_json());
    assert_eq!(patched.tile_count(), base.tile_count());
}
