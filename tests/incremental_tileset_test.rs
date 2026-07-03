//! Incremental tileset encoding — tiles emitted per leaf without full retention.

use wasm_spatial_core::{generate_tileset, generate_tileset_incremental_abort, Octree};

#[test]
fn test_incremental_tileset_matches_full_tileset() {
    let mut positions: Vec<f32> = (0..1000)
        .flat_map(|i| [i as f32, (i * 2) as f32, (i * 3) as f32])
        .collect();
    let tree = Octree::build(&mut positions, 200, 8);
    let active = &positions[..tree.total_points() as usize * 3];

    let full = generate_tileset(&tree, active, None).unwrap();
    let mut collected: Vec<Vec<u8>> = Vec::new();
    let json = generate_tileset_incremental_abort(
        &tree,
        active,
        None,
        None,
        None,
        |_idx, _uri, bytes, _bounds| {
            collected.push(bytes.to_vec());
            Ok(())
        },
        || false,
    )
    .unwrap();

    assert_eq!(json, full.tileset_json());
    assert_eq!(collected.len(), full.tile_count() as usize);
    for (i, tile) in collected.iter().enumerate() {
        assert_eq!(tile.as_slice(), full.tile(i).unwrap());
    }
}
