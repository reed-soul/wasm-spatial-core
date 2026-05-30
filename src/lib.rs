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
mod geojson_parser;
mod geojson_streaming;
mod gltf_writer;
mod ifc_reader;
mod spatial_analysis;
mod spatial_index;
mod topology;
mod utils;
mod vector_tile;

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

use wasm_bindgen::prelude::*;

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

/// Memory usage information for the WASM module.
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

/// Maximum allowed input size: 100 MB.
const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Validate input length is reasonable. Returns an error if input exceeds 100 MB.
#[inline]
pub(crate) fn validate_input_size(len: usize, label: &str) -> Result<(), JsValue> {
    if len > MAX_INPUT_SIZE {
        Err(JsValue::from_str(&format!(
            "Input too large ({} > {} bytes): {}",
            len, MAX_INPUT_SIZE, label
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
        assert_eq!(version(), "0.2.0-alpha.1");
    }

    #[test]
    fn test_validate_input_size_ok() {
        assert!(validate_input_size(100, "test").is_ok());
        assert!(validate_input_size(0, "test").is_ok());
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_validate_input_size_too_large() {
        assert!(validate_input_size(100 * 1024 * 1024 + 1, "test").is_err());
    }
}
