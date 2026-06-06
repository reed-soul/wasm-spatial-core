//! WebGPU buffer layout contract tests (W4.2).

#[cfg(feature = "webgpu")]
#[test]
fn test_gpu_layout_matches_npm_constants() {
    use wasm_spatial_core::{
        HEIGHT_STRIDE_BYTES, INDEX_STRIDE_BYTES, MASK_STRIDE_BYTES, MATRIX_FLOAT_COUNT,
        POSITION_STRIDE_BYTES, SHADER_BUNDLE_VERSION,
    };

    assert_eq!(POSITION_STRIDE_BYTES, 12);
    assert_eq!(MATRIX_FLOAT_COUNT, 16);
    assert_eq!(HEIGHT_STRIDE_BYTES, 4);
    assert_eq!(MASK_STRIDE_BYTES, 1);
    assert_eq!(INDEX_STRIDE_BYTES, 4);
    assert_eq!(SHADER_BUNDLE_VERSION, "1.0.0");
}

#[cfg(feature = "webgpu")]
#[test]
fn test_transform_cpu_reference_for_gpu_parity() {
    use wasm_spatial_core::test_exports::transform_points_reference;

    let positions = vec![1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0];
    let mut matrix = vec![0.0f32; 16];
    matrix[0] = 0.0;
    matrix[1] = 1.0;
    matrix[4] = -1.0;
    matrix[5] = 0.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;

    let out = transform_points_reference(&positions, &matrix);
    assert!((out[0] - 0.0).abs() < 1e-5);
    assert!((out[1] - 1.0).abs() < 1e-5);
}
