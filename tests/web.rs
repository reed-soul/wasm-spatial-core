//! Browser-based wasm-bindgen tests.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use wasm_spatial_core::version;

#[wasm_bindgen_test]
fn test_version_in_browser() {
    assert_eq!(version(), "0.2.0-alpha.1");
}
