//! # wasm-spatial-core
//!
//! A high-performance WebAssembly spatial data processing engine for frontend
//! Web3D/GIS applications. Offloads heavy spatial computation from the server
//! to the browser, including coordinate transformations, GeoJSON parsing,
//! point cloud pre-processing and more.
//!
//! ## Core Design Principles
//!
//! - **Zero-copy memory sharing** via `ArrayBuffer` / `SharedArrayBuffer`
//! - **Streaming & chunked parsing** for large datasets
//! - **Direct GPU pipeline feeding** — output buffers ready for WebGL/WebGPU
//!
//! © 2026 智启未来 (Zhiqi Weilai) — Qingxi

mod b3dm;
mod cesium_adapter;
mod coordinate;
mod errors;
mod geojson_parser;
mod geojson_streaming;
mod geotiff;
mod gltf_writer;
pub use gltf_writer::{mesh_to_glb, point_cloud_to_glb, terrain_to_glb, GltfBuilder};
pub use b3dm::{
    create_instanced_tileset, create_instanced_tileset_i3dm, create_mesh_tileset,
    encode_b3dm_tile, encode_i3dm_tile,
};
mod ifc_reader;
mod octree;
mod pnts;
mod spatial_analysis;
mod spatial_index;
mod topology;
mod utils;
mod vector_tile;

pub use octree::{Bounds, Octree, OctreeNode, DEFAULT_MAX_DEPTH, DEFAULT_MAX_POINTS_PER_NODE};
pub use pnts::{
    draco_status_js, encode_pnts_tile, estimate_point_spacing, generate_tileset, pad_len,
    parse_pnts_header, supports_draco_js, TilesetResult,
};
mod wkb_wkt;

use errors::input_too_large_js;
pub use errors::{SpatialError, SpatialErrorDetail};

#[cfg(feature = "point-cloud")]
mod point_cloud;

#[cfg(feature = "point-cloud")]
mod point_cloud_stream;

#[cfg(feature = "e57-support")]
mod e57;

#[cfg(feature = "point-cloud")]
mod worker;

mod obj;
mod ply;

// Re-export core functions for integration testing and advanced usage
pub use coordinate::{
    batch_bd09_to_gcj02_in_place, batch_bd09_to_wgs84_in_place, batch_gcj02_to_bd09_in_place,
    batch_gcj02_to_wgs84_in_place, batch_mercator_to_wgs84_in_place, batch_wgs84_to_bd09_in_place,
    batch_wgs84_to_gcj02_in_place, batch_wgs84_to_mercator_in_place,
};

#[cfg(feature = "point-cloud")]
pub use point_cloud::{
    parse_las_header_core, parse_las_points_core, random_decimate_core, read_f64_le, read_u16_le,
    read_u32_le, voxel_grid_decimate_core,
};

#[cfg(feature = "laz-support")]
pub use point_cloud::parse_laz_points_core;

#[cfg(feature = "e57-support")]
pub use e57::{is_e57_format, parse_e57_core, E57Result};

#[cfg(feature = "point-cloud")]
pub use point_cloud_stream::compute_region_byte_range;

#[cfg(feature = "laz-support")]
pub use point_cloud_stream::{laz_status, supports_laz, PointCloudStreamer};

pub use ifc_reader::{parse_ifc_geometry_core, IfcGeometryResult, IfcMesh};

pub use geotiff::{
    apply_color_ramp_core, contour_lines_core, encode_quantized_mesh_core,
    encode_terrain_tileset_core, hillshade_core, parse_geotiff_core, ColorRamp, GeotiffInfo,
    QuantizedMeshResult, TerrainTilesetResult,
};

// Re-export internal helpers for integration/stress testing.
// These are exposed via a public "test_exports" module that is only
// intended for testing — not part of the stable API.
#[doc(hidden)]
pub mod test_exports {
    pub use crate::geojson_parser::{
        count_geojson_features, geojson_feature_collection_native, geojson_from_coords_native,
        parse_geojson_coords,
    };
    #[cfg(any(test, feature = "test-helpers"))]
    pub use crate::point_cloud::test_helpers;
    pub use crate::topology::{polygon_intersection_native, polygon_union_native};
    pub use crate::utils::{
        clean_coords_native, deduplicate_coords_native, validate_coords_native,
    };
}

use wasm_bindgen::prelude::*;

// Dynamic input size limit — thread-safe via LazyLock + RwLock.
use std::sync::{LazyLock, RwLock};

static INPUT_SIZE_LIMIT: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(100 * 1024 * 1024));

/// WASM linear memory maximum limit (0 = no limit, use WASM default).
static WASM_MEMORY_MAX: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(0));

/// Get the current input size limit.
pub(crate) fn get_current_input_limit() -> usize {
    INPUT_SIZE_LIMIT
        .read()
        .map(|v| *v)
        .unwrap_or(DEFAULT_MAX_INPUT_SIZE)
}

/// Get the current WASM memory max limit (0 = no limit).
pub(crate) fn get_wasm_memory_max() -> usize {
    WASM_MEMORY_MAX.read().map(|v| *v).unwrap_or(0)
}

/// Initialize the WASM module. Call this once before any other function.
///
/// Sets up the panic hook for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

#[cfg(feature = "multi-thread")]
pub use wasm_bindgen_rayon::init_thread_pool;

/// Return the library version string.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Format Support Status
// ---------------------------------------------------------------------------

/// Check if E57 format is supported (requires `e57-support` feature).
#[wasm_bindgen(js_name = "supportsE57")]
pub fn supports_e57() -> bool {
    cfg!(feature = "e57-support")
}

/// Get E57 support status as a human-readable string.
#[wasm_bindgen(js_name = "e57Status")]
pub fn e57_status() -> String {
    #[cfg(feature = "e57-support")]
    {
        String::from(
            "E57 support: AVAILABLE (e57 crate v0.11.12). Pure Rust, compiles to WASM. \
             Supports reading Cartesian/Spherical coordinates, colors, and intensities.",
        )
    }
    #[cfg(not(feature = "e57-support"))]
    {
        String::from("E57 support: DISABLED. Build with --features e57-support to enable.")
    }
}

/// Parse E57 file (feature-gated: requires `e57-support`).
#[cfg(feature = "e57-support")]
pub use e57::parse_e57;

/// Parse E57 with streaming progress (feature-gated).
#[cfg(feature = "e57-support")]
pub use e57::parse_e57_stream;

/// WebWorker parallel processing support.
#[cfg(feature = "point-cloud")]
pub use worker::{supports_worker, WorkerHandle, WorkerOptions};

// ---------------------------------------------------------------------------
// Dynamic Input Size Limit
// ---------------------------------------------------------------------------

/// Dynamically set the maximum allowed input size in bytes.
///
/// Default is 100 MB. Set to 0 to disable the limit.
///
/// # Example (JS)
/// ```js
/// core.setInputSizeLimit(50 * 1024 * 1024); // 50 MB
/// ```
#[wasm_bindgen(js_name = "setInputSizeLimit")]
pub fn set_input_size_limit(bytes: usize) {
    if let Ok(mut val) = INPUT_SIZE_LIMIT.write() {
        *val = bytes;
    }
}

/// Get the current input size limit in bytes.
///
/// Returns 100 MB (104,857,600) if not changed.
#[wasm_bindgen(js_name = "getInputSizeLimit")]
pub fn get_input_size_limit() -> usize {
    get_current_input_limit()
}

/// Get the approximate number of allocated bytes in WASM linear memory.
///
/// This reads the current `memory.buffer.byteLength`. Note that WASM memory
/// only grows (never shrinks), so this value is the peak allocation size.
///
/// Returns 0 on non-WASM targets.
#[wasm_bindgen(js_name = "getAllocatedBytes")]
pub fn get_allocated_bytes() -> usize {
    wasm_memory_total()
}

// ---------------------------------------------------------------------------
// Memory Info
// ---------------------------------------------------------------------------
///
/// Provides insight into WASM linear memory allocation, useful for monitoring
/// large spatial data processing workloads.
///
/// **Note:** Only available in WASM runtime. On native, returns zeros.
#[wasm_bindgen(js_name = "memoryInfo")]
pub fn memory_info() -> MemoryInfo {
    let total = wasm_memory_total();
    MemoryInfo {
        total,
        used: total,  // approximation
        remaining: 0, // approximation
    }
}

/// WASM linear memory usage info.
#[wasm_bindgen]
pub struct MemoryInfo {
    total: usize,
    used: usize,
    remaining: usize,
}

#[wasm_bindgen]
impl MemoryInfo {
    /// Total WASM linear memory allocated (in bytes).
    #[wasm_bindgen(getter)]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Approximate used memory (in bytes).
    #[wasm_bindgen(getter)]
    pub fn used(&self) -> usize {
        self.used
    }

    /// Remaining free memory (in bytes).
    #[wasm_bindgen(getter)]
    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

/// Get total WASM linear memory size.
fn wasm_memory_total() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        let mem = wasm_bindgen::memory();
        let buffer =
            js_sys::Reflect::get(&mem, &"buffer".into()).expect("memory.buffer should exist");
        let byte_length = js_sys::Reflect::get(&buffer, &"byteLength".into())
            .expect("buffer.byteLength should exist");
        byte_length.as_f64().expect("byteLength should be a number") as usize
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0 // Not available in native testing
    }
}

/// Default maximum allowed input size: 100 MB.
pub(crate) const DEFAULT_MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

// ---------------------------------------------------------------------------
// WASM Memory Control (Task 4)
// ---------------------------------------------------------------------------

/// Set the maximum WASM linear memory in bytes.
///
/// When set to a non-zero value, `checkMemoryAvailable` and `buildOctree`
/// will pre-check that estimated memory usage does not exceed this limit.
/// Set to 0 (default) to disable the limit.
///
/// Note: This does NOT change the actual WASM memory.grow limit — that is
/// configured at module instantiation time. This is a software-level guard
/// that pre-checks before allocating.
///
/// # Example (JS)
/// ```js
/// core.setMaxWasmMemory(256 * 1024 * 1024); // 256 MB
/// ```
#[wasm_bindgen(js_name = "setMaxWasmMemory")]
pub fn set_max_wasm_memory(bytes: usize) {
    if let Ok(mut val) = WASM_MEMORY_MAX.write() {
        *val = bytes;
    }
}

/// Get the current WASM memory max limit.
///
/// Returns 0 if no limit is set (WASM default applies).
#[wasm_bindgen(js_name = "getMaxWasmMemory")]
pub fn get_max_wasm_memory() -> usize {
    get_wasm_memory_max()
}

/// Check if estimated memory is available given the current WASM memory limit.
///
/// Compares the estimated byte requirement against the configured maximum.
/// Always returns `true` if no limit is set (max == 0).
///
/// # Arguments
/// * `estimated_bytes` — Estimated memory needed for an operation.
///
/// # Returns
/// `true` if there is enough memory, `false` if the estimate exceeds the limit.
#[wasm_bindgen(js_name = "checkMemoryAvailable")]
pub fn check_memory_available(estimated_bytes: usize) -> bool {
    let max = get_wasm_memory_max();
    if max == 0 {
        return true;
    }

    // Current WASM memory usage
    let current = wasm_memory_total();

    // Check if current + estimated exceeds the limit
    current.saturating_add(estimated_bytes) <= max
}

/// Estimate memory required for octree construction.
///
/// Upper-bound estimate:
/// - Positions buffer: `num_points × 12` bytes (Float32 × 3)
/// - Reorder map: `num_points × 8` bytes (usize)
/// - Octree nodes: ~100 bytes per estimated node
/// - Temp buffers: ~50% overhead for intermediate state
///
/// # Arguments
/// * `num_points` — Number of points in the dataset.
///
/// # Returns
/// Estimated memory in bytes.
#[wasm_bindgen(js_name = "estimateOctreeMemory")]
pub fn estimate_octree_memory(num_points: u32) -> usize {
    let n = num_points as usize;
    let positions_bytes = n * 3 * 4; // f32
    let reorder_bytes = n * 8; // usize
                               // Rough estimate: 1 root + up to 2^21 internal nodes, but realistically
                               // num_points / maxPointsPerNode * 8 leaves * 140 bytes each
    let estimated_nodes = (n / 50000).max(1) * 9;
    let nodes_bytes = estimated_nodes * 140;
    // Temp buffers (partition, etc.)
    let temp_bytes = positions_bytes / 2;

    positions_bytes + reorder_bytes + nodes_bytes + temp_bytes + 1024 // 1KB overhead
}

/// Validate input length is reasonable. Returns an error if input exceeds the current limit.
#[inline]
pub(crate) fn validate_input_size(len: usize, label: &str) -> Result<(), JsValue> {
    let limit = get_current_input_limit();
    if limit > 0 && len > limit {
        Err(input_too_large_js(format!(
            "Input too large ({} > {} bytes): {}",
            len, limit, label
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_validate_input_size_ok() {
        assert!(validate_input_size(100, "test").is_ok());
        assert!(validate_input_size(0, "test").is_ok());
    }

    #[test]
    fn test_default_input_limit() {
        assert!(get_input_size_limit() >= 100 * 1024 * 1024);
    }

    #[test]
    fn test_set_input_limit() {
        set_input_size_limit(50 * 1024 * 1024);
        assert_eq!(get_input_size_limit(), 50 * 1024 * 1024);
        // Reset to default
        set_input_size_limit(DEFAULT_MAX_INPUT_SIZE);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_validate_input_size_too_large() {
        assert!(validate_input_size(100 * 1024 * 1024 + 1, "test").is_err());
    }

    // ===========================================================================
    // Memory Control Tests (Task 4)
    // ===========================================================================

    #[test]
    fn test_set_max_wasm_memory() {
        set_max_wasm_memory(128 * 1024 * 1024);
        assert_eq!(get_max_wasm_memory(), 128 * 1024 * 1024);
        // Reset
        set_max_wasm_memory(0);
        assert_eq!(get_max_wasm_memory(), 0);
    }

    #[test]
    fn test_check_memory_available_no_limit() {
        set_max_wasm_memory(0); // No limit
        assert!(check_memory_available(usize::MAX)); // Always true with no limit
    }

    #[test]
    fn test_check_memory_available_within_limit() {
        set_max_wasm_memory(1024 * 1024); // 1 MB limit
                                          // Result depends on current usage — just ensure no panic
        let _available = check_memory_available(100);
        set_max_wasm_memory(0); // Reset
    }

    #[test]
    fn test_estimate_octree_memory_basic() {
        // 1M points
        let estimate = estimate_octree_memory(1_000_000);
        // Positions: 12MB, reorder: 8MB, nodes: ~140 bytes * ~180 = 25KB, temp: ~6MB
        // Total should be roughly 26MB+
        assert!(estimate > 20_000_000, "Expected > 20MB, got {}", estimate);
        assert!(estimate < 100_000_000, "Expected < 100MB, got {}", estimate);
    }

    #[test]
    fn test_estimate_octree_memory_small() {
        let estimate = estimate_octree_memory(1000);
        // Positions: 12KB, reorder: 8KB, nodes: ~1KB, temp: ~6KB
        assert!(estimate > 10_000, "Expected > 10KB, got {}", estimate);
    }
}
