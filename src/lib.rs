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
mod spatial_analysis;
mod spatial_index;
mod utils;
mod vector_tile;

#[cfg(feature = "point-cloud")]
mod point_cloud;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }
}
