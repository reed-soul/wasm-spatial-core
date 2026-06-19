//! CPU reference parity for GPU heightfield flatten kernel (W4.4).

#[cfg(all(feature = "webgpu", feature = "terrain-edit"))]
#[test]
fn test_flatten_heightfield_reference_matches_flatten_inside() {
    use wasm_spatial_core::{
        flatten_inside, rasterize_polygon_mask, test_exports::flatten_heightfield_reference,
    };

    let width = 32u32;
    let height = 32u32;
    let bounds = [0.0, 0.0, 1.0, 1.0];
    let polygon = vec![0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75, 0.25, 0.25];
    let mask = rasterize_polygon_mask(width, height, &bounds, &polygon).unwrap();

    let mut heights: Vec<f32> = (0..(width * height))
        .map(|i| (i as f32 * 0.37).sin() * 50.0 + 100.0)
        .collect();

    let target = 42.0f32;
    let reference = flatten_heightfield_reference(&heights, &mask, target);

    flatten_inside(&mut heights, &mask, target).unwrap();
    assert_eq!(heights, reference);
}

#[cfg(all(feature = "webgpu", feature = "terrain-edit"))]
#[test]
fn test_flatten_heightfield_reference_preserves_outside_mask() {
    use wasm_spatial_core::{rasterize_polygon_mask, test_exports::flatten_heightfield_reference};

    let width = 16u32;
    let height = 16u32;
    let bounds = [0.0, 0.0, 1.0, 1.0];
    let polygon = vec![0.4, 0.4, 0.6, 0.4, 0.6, 0.6, 0.4, 0.6, 0.4, 0.4];
    let mask = rasterize_polygon_mask(width, height, &bounds, &polygon).unwrap();

    let heights: Vec<f32> = (0..(width * height)).map(|i| i as f32).collect();
    let original = heights.clone();
    let target = 5.0f32;

    let out = flatten_heightfield_reference(&heights, &mask, target);
    for (i, (&o, &n)) in original.iter().zip(out.iter()).enumerate() {
        if mask[i] == 0 {
            assert_eq!(o, n);
        } else {
            assert_eq!(n, target);
        }
    }
}
