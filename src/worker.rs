//! WebWorker Parallel Point Cloud Processing
//!
//! CPU-intensive point cloud operations (octree building, tileset generation)
//! are offloaded to Web Workers to prevent UI freezing.
//!
//! Two processing modes:
//! - **Worker mode**: Creates an inline Web Worker via Blob URL for true parallelism.
//!   Requires COOP/COEP headers for `SharedArrayBuffer` support.
//! - **Chunked mode**: Processes data in batches on the main thread, yielding
//!   control via `setTimeout(0)` between chunks to keep the UI responsive.
//!
//! Worker mode is preferred when available; chunked mode is the fallback.
//!
//! NOTE: The Web Worker implementation uses JS interop. The Rust side provides
//! helper functions and the `WorkerHandle` struct. Actual Worker creation and
//! message passing is done in JavaScript (see the demo for examples).

use wasm_bindgen::prelude::*;

use crate::errors::SpatialErrorDetail;

// ===========================================================================
// Worker support detection
// ===========================================================================

/// Check if Web Worker is available in the current environment.
///
/// Returns `true` in browser contexts with Worker support.
#[wasm_bindgen(js_name = "supportsWorker")]
pub fn supports_worker() -> bool {
    // In WASM running in a browser, Workers are generally available.
    // The actual limitation is COOP/COEP headers for SharedArrayBuffer.
    let has_worker = js_sys::eval("typeof Worker !== 'undefined'")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    has_worker
}

/// Check if the environment has COOP/COEP headers set (for SharedArrayBuffer).
///
/// Returns `true` if `SharedArrayBuffer` is available and functional.
#[wasm_bindgen(js_name = "supportsSharedArrayBuffer")]
pub fn supports_shared_array_buffer() -> bool {
    let result = js_sys::eval(
        "typeof SharedArrayBuffer !== 'undefined' && new SharedArrayBuffer(8).byteLength === 8",
    );
    match result {
        Ok(val) => val.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

// ===========================================================================
// Worker processing — Handle
// ===========================================================================

/// Handle for a background point cloud processing task.
///
/// Use this with `processPointCloudInWorker` to manage a background job.
/// The Worker is created on the JS side; this Rust struct tracks state.
#[wasm_bindgen]
pub struct WorkerHandle {
    point_count: u32,
    max_points_per_node: u32,
    max_depth: u32,
    cancelled: bool,
}

#[wasm_bindgen]
impl WorkerHandle {
    /// Number of points being processed.
    #[wasm_bindgen(getter, js_name = "pointCount")]
    pub fn point_count(&self) -> u32 {
        self.point_count
    }

    /// Cancel the processing task.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Check if the task has been cancelled.
    #[wasm_bindgen(getter, js_name = "isCancelled")]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

/// Process point cloud data in a Web Worker.
///
/// Creates a `WorkerHandle` to track the processing state. The actual Worker
/// creation and communication happens on the JavaScript side.
///
/// # Parameters
///
/// - `point_count`: Number of points to process
/// - `options`: Optional JS object with `maxPointsPerNode` and `maxDepth`
///
/// # Returns
///
/// A `WorkerHandle` for tracking the task.
///
/// # Example (JavaScript)
///
/// ```js
/// const handle = core.processPointCloudInWorker(positions.length / 3, {
///   maxPointsPerNode: 50000,
///   maxDepth: 21,
/// });
///
/// // Create the actual worker
/// const workerBlob = new Blob([`
///   self.onmessage = function(e) {
///     const { positions, maxPointsPerNode, maxDepth } = e.data;
///     // Process here...
///     self.postMessage({ type: 'complete', result: tileset });
///   };
/// `], { type: 'application/javascript' });
/// const worker = new Worker(URL.createObjectURL(workerBlob));
///
/// handle.onProgress((pct, msg) => console.log(`${pct}%: ${msg}`));
/// handle.onComplete((result) => { /* use tileset */ });
/// ```
#[wasm_bindgen(js_name = "processPointCloudInWorker")]
pub fn process_point_cloud_in_worker(
    point_count: u32,
    options: JsValue,
) -> Result<WorkerHandle, SpatialErrorDetail> {
    let opts = if options.is_object() {
        Some(&js_sys::Object::from(options))
    } else {
        None
    };

    let max_points_per_node = if let Some(o) = opts {
        js_sys::Reflect::get(o, &JsValue::from_str("maxPointsPerNode"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as u32)
            .unwrap_or(50000)
    } else {
        50000
    };

    let max_depth = if let Some(o) = opts {
        js_sys::Reflect::get(o, &JsValue::from_str("maxDepth"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as u32)
            .unwrap_or(21)
    } else {
        21
    };

    Ok(WorkerHandle {
        point_count,
        max_points_per_node,
        max_depth,
        cancelled: false,
    })
}

// ===========================================================================
// Chunked processing helpers — Pure Rust
// ===========================================================================

/// Compute axis-aligned bounds from positions.
///
/// Returns `(min_x, min_y, min_z, max_x, max_y, max_z)`.
pub(crate) fn compute_bounds(positions: &[f32]) -> (f32, f32, f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut min_z = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut max_z = f32::MIN;

    for i in (0..positions.len()).step_by(3) {
        let x = positions[i];
        let y = positions[i + 1];
        let z = positions[i + 2];
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        min_z = min_z.min(z);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        max_z = max_z.max(z);
    }

    (min_x, min_y, min_z, max_x, max_y, max_z)
}

/// Build a tileset result object from positions and colors.
///
/// This is a helper that creates a JS object with point count, bounds, and
/// tileset parameters. Used by the chunked processing pipeline.
fn build_tileset_result(
    positions: &[f32],
    max_points_per_node: u32,
    max_depth: u32,
) -> JsValue {
    let result = js_sys::Object::new();
    let point_count = positions.len() / 3;

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("pointCount"),
        &JsValue::from(point_count as u32),
    )
    .ok();

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("maxPointsPerNode"),
        &JsValue::from(max_points_per_node),
    )
    .ok();

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("maxDepth"),
        &JsValue::from(max_depth),
    )
    .ok();

    // Bounds
    if !positions.is_empty() {
        let (min_x, min_y, min_z, max_x, max_y, max_z) = compute_bounds(positions);
        let bounds = js_sys::Object::new();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("minX"), &JsValue::from(min_x)).ok();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("minY"), &JsValue::from(min_y)).ok();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("minZ"), &JsValue::from(min_z)).ok();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("maxX"), &JsValue::from(max_x)).ok();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("maxY"), &JsValue::from(max_y)).ok();
        js_sys::Reflect::set(&bounds, &JsValue::from_str("maxZ"), &JsValue::from(max_z)).ok();
        js_sys::Reflect::set(&result, &JsValue::from_str("bounds"), &bounds).ok();
    }

    result.into()
}

/// Process point cloud data in chunks on the main thread.
///
/// This is a synchronous helper that computes bounds and creates a result.
/// The actual async chunking with `setTimeout(0)` is best done in JS.
///
/// Use this from JavaScript like:
///
/// ```js
/// async function processChunked(positions, chunkSize, callback) {
///   const total = positions.length / 3;
///   const chunks = Math.ceil(total / chunkSize);
///   for (let i = 0; i < chunks; i++) {
///     // Process chunk i...
///     callback(i, chunks, (i + 1) * chunkSize);
///     await new Promise(r => setTimeout(r, 0)); // yield to event loop
///   }
///   return core.generateTileset(positions, 50000, 21);
/// }
/// ```
#[wasm_bindgen(js_name = "chunkedProcessingInfo")]
pub fn chunked_processing_info(
    point_count: u32,
    chunk_size: Option<u32>,
) -> js_sys::Object {
    let chunk_size = chunk_size.unwrap_or(50000);
    let total_chunks = (point_count + chunk_size - 1) / chunk_size;

    let result = js_sys::Object::new();
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("totalChunks"),
        &JsValue::from(total_chunks),
    )
    .ok();
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("chunkSize"),
        &JsValue::from(chunk_size),
    )
    .ok();
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("pointCount"),
        &JsValue::from(point_count),
    )
    .ok();

    result
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_bounds_empty() {
        let (min_x, min_y, min_z, max_x, max_y, max_z) = compute_bounds(&[]);
        assert_eq!(min_x, f32::MAX);
        assert_eq!(max_x, f32::MIN);
        assert_eq!(min_y, f32::MAX);
        assert_eq!(max_y, f32::MIN);
        assert_eq!(min_z, f32::MAX);
        assert_eq!(max_z, f32::MIN);
    }

    #[test]
    fn test_compute_bounds_single_point() {
        let positions = vec![5.0_f32, 10.0, -3.0];
        let (min_x, min_y, min_z, max_x, max_y, max_z) = compute_bounds(&positions);
        assert_eq!(min_x, 5.0);
        assert_eq!(max_x, 5.0);
        assert_eq!(min_y, 10.0);
        assert_eq!(max_y, 10.0);
        assert_eq!(min_z, -3.0);
        assert_eq!(max_z, -3.0);
    }

    #[test]
    fn test_compute_bounds_multiple_points() {
        let positions = vec![
            1.0_f32, 2.0, 3.0, // point 0
            10.0, -5.0, 0.0,   // point 1
            -1.0, 8.0, 6.0,    // point 2
        ];
        let (min_x, min_y, min_z, max_x, max_y, max_z) = compute_bounds(&positions);
        assert_eq!(min_x, -1.0);
        assert_eq!(max_x, 10.0);
        assert_eq!(min_y, -5.0);
        assert_eq!(max_y, 8.0);
        assert_eq!(min_z, 0.0);
        assert_eq!(max_z, 6.0);
    }

    #[test]
    fn test_worker_handle_cancel() {
        let mut handle = WorkerHandle {
            point_count: 1000,
            max_points_per_node: 50000,
            max_depth: 21,
            cancelled: false,
        };
        assert!(!handle.is_cancelled());
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[test]
    fn test_worker_handle_creation() {
        let handle = WorkerHandle {
            point_count: 100000,
            max_points_per_node: 1000,
            max_depth: 18,
            cancelled: false,
        };
        assert_eq!(handle.point_count(), 100000);
        assert_eq!(handle.max_points_per_node, 1000);
        assert_eq!(handle.max_depth, 18);
    }
}
