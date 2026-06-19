//! WebGPU compute integration stubs (Wave 4).
//!
//! Actual GPU kernels run in JavaScript via `wasm-spatial-core/webgpu` using
//! WGSL shaders from `shaders/`. WASM provides CPU fallback paths and layout
//! contracts shared with the GPU pipeline.

use wasm_bindgen::prelude::*;

// ===========================================================================
// Buffer layout contract (W4.2) — must match npm/webgpu.ts and shaders/README.md
// ===========================================================================

/// Bytes per vertex position (`[x, y, z]` as f32).
pub const POSITION_STRIDE_BYTES: usize = 12;

/// Floats per 4×4 transform matrix (column-major, WebGL convention).
pub const MATRIX_FLOAT_COUNT: usize = 16;

/// Bytes per heightfield cell (f32 elevation).
pub const HEIGHT_STRIDE_BYTES: usize = 4;

/// Bytes per mask cell (u8, 0 = outside, 1 = inside).
pub const MASK_STRIDE_BYTES: usize = 1;

/// Bytes per triangle index (u32).
pub const INDEX_STRIDE_BYTES: usize = 4;

/// Floats per symmetric quadric (upper-triangular 4×4).
pub const QUADRIC_FLOAT_COUNT: usize = 10;

/// Current WGSL shader bundle version string.
pub const SHADER_BUNDLE_VERSION: &str = "1.1.0";

// ===========================================================================
// WASM status API (W4.1)
// ===========================================================================

/// Whether the crate was built with WebGPU compute integration enabled.
#[wasm_bindgen(js_name = "supportsWebGpu")]
pub fn supports_webgpu() -> bool {
    cfg!(feature = "webgpu")
}

/// Human-readable WebGPU module status.
#[wasm_bindgen(js_name = "webGpuStatus")]
pub fn webgpu_status() -> String {
    #[cfg(feature = "webgpu")]
    {
        String::from(
            "WebGPU compute: AVAILABLE via JS module 'wasm-spatial-core/webgpu'.\n\
             Requires latest Chrome with navigator.gpu.\n\
             Kernels: transform_points_v1, heightfield_flatten_v1, mesh_quadrics_v1, mesh_edge_costs_v1.\n\
             CPU fallback uses WASM transformPointCloud / flattenTerrain / simplifyMeshQem.",
        )
    }
    #[cfg(not(feature = "webgpu"))]
    {
        String::from("WebGPU compute: DISABLED. Build with --features webgpu to enable.")
    }
}

/// WGSL shader bundle version for cache invalidation.
#[wasm_bindgen(js_name = "webGpuShaderVersion")]
pub fn webgpu_shader_version() -> String {
    SHADER_BUNDLE_VERSION.to_string()
}

/// CPU reference for GPU transform parity tests (column-major Mat4).
#[cfg(feature = "webgpu")]
pub fn transform_points_cpu_reference(positions: &[f32], matrix: &[f32]) -> Vec<f32> {
    crate::point_cloud_analysis::transform_points_core(positions, matrix)
}

/// CPU reference for GPU heightfield flatten parity tests (masked flatten, no feather).
#[cfg(all(feature = "webgpu", feature = "terrain-edit"))]
pub fn flatten_heightfield_cpu_reference(heights: &[f32], mask: &[u8], target: f32) -> Vec<f32> {
    let mut out = heights.to_vec();
    let _ = crate::terrain_edit::flatten_inside(&mut out, mask, target);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_constants() {
        assert_eq!(POSITION_STRIDE_BYTES, 3 * std::mem::size_of::<f32>());
        assert_eq!(MATRIX_FLOAT_COUNT, 16);
        assert_eq!(HEIGHT_STRIDE_BYTES, std::mem::size_of::<f32>());
        assert_eq!(QUADRIC_FLOAT_COUNT, 10);
    }

    #[test]
    fn test_shader_version() {
        assert!(!SHADER_BUNDLE_VERSION.is_empty());
    }

    #[cfg(feature = "webgpu")]
    #[test]
    fn test_supports_webgpu_enabled() {
        assert!(supports_webgpu());
    }
}
