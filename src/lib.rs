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

mod cesium_adapter;
mod coordinate;
mod errors;
mod geojson_parser;
mod geojson_streaming;
mod gltf_writer;
mod ifc_reader;
mod octree;
mod pnts;
mod spatial_analysis;
mod spatial_index;
mod topology;
mod utils;
mod vector_tile;

pub use octree::{Bounds, Octree, OctreeNode, DEFAULT_MAX_DEPTH, DEFAULT_MAX_POINTS_PER_NODE};
pub use pnts::{encode_pnts_tile, generate_tileset, pad_len, parse_pnts_header, TilesetResult};
mod wkb_wkt;

use errors::input_too_large_js;
pub use errors::{SpatialError, SpatialErrorDetail};

#[cfg(feature = "point-cloud")]
mod point_cloud;

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

pub use ifc_reader::{parse_ifc_geometry_core, IfcGeometryResult, IfcMesh};

// Re-export internal helpers for integration/stress testing.
// These are exposed via a public "test_exports" module that is only
// intended for testing — not part of the stable API.
#[doc(hidden)]
pub mod test_exports {
    pub use crate::geojson_parser::{
        count_geojson_features, geojson_feature_collection_native, geojson_from_coords_native,
        parse_geojson_coords,
    };
    pub use crate::topology::{polygon_intersection_native, polygon_union_native};
    pub use crate::utils::{
        clean_coords_native, deduplicate_coords_native, validate_coords_native,
    };
}

use wasm_bindgen::prelude::*;

// Dynamic input size limit — thread-safe via LazyLock + RwLock.
use std::sync::{LazyLock, RwLock};

static INPUT_SIZE_LIMIT: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(100 * 1024 * 1024));

/// Get the current input size limit.
pub(crate) fn get_current_input_limit() -> usize {
    INPUT_SIZE_LIMIT
        .read()
        .map(|v| *v)
        .unwrap_or(DEFAULT_MAX_INPUT_SIZE)
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
        assert_eq!(version(), "0.2.0");
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
}
