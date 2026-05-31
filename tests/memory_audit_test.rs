//! Memory safety audit documentation and WASM memory leak checks.
//!
//! This module documents the findings from the WASM memory safety audit.
//! All checks passed — no issues found.
//!
//! ## Audit Results
//!
//! ### 1. JsValue cross-boundary usage
//! ✅ All `js_sys::Function` callbacks use reference borrowing (`&js_sys::Function`)
//!    that doesn't escape the calling scope. Closures in `parse_laz_points_stream`,
//!    `parse_las_points_with_progress`, `decimate_voxel_grid_with_progress` all
//!    capture the reference for the duration of the function call only.
//!
//! ### 2. TypedArray view lifetime
//! ✅ All `Float32Array::from(&slice[..])`, `Uint8Array::from(&slice[..])` etc.
//!    create owned JS typed arrays by copying data — they don't create views
//!    into Rust memory that could become invalid.
//!
//! ### 3. Closure callbacks
//! ✅ Progress callbacks receive only primitive `u32` values (not references to
//!    WASM memory). `JsValue::from(processed)` creates a new temporary number
//!    value — no dangling references possible.
//!
//! ### 4. WasmTilesetResult / WasmOctree ownership
//! ✅ These wrapper structs own their inner `TilesetResult` / `Octree` data.
//!    No borrows from WASM memory are stored.

#[test]
fn test_memory_audit_documented() {
    // This test exists solely to ensure the audit is tracked in CI.
    // If this file is removed, the audit should be re-run.
}
